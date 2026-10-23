//! Windows installs: setup programs (with silent switches for the common
//! installer frameworks), MSI, MSIX, and Start Menu shortcuts. This builds
//! everywhere; the functions only run on Windows.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::{base_name, Ctx, Outcome};
use crate::model::{App, InstallKind, Installed};
use crate::system::{self, powershell, ps_quote};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Flavor {
    Nsis,
    Inno,
    Squirrel,
    Burn,
    InstallShield,
    Unknown,
}

/// Look for an installer framework's marker inside the file.
fn sniff(path: &Path) -> Flavor {
    let needles: &[(&[u8], Flavor)] = &[
        (b"Nullsoft", Flavor::Nsis),
        (b"NSIS Error", Flavor::Nsis),
        (b"Inno Setup", Flavor::Inno),
        (b"SquirrelSetup", Flavor::Squirrel),
        (b"Squirrel.Windows", Flavor::Squirrel),
        (b".wixburn", Flavor::Burn),
        (b"InstallShield", Flavor::InstallShield),
    ];
    let Ok(mut f) = std::fs::File::open(path) else { return Flavor::Unknown };
    let mut buf = vec![0u8; 1 << 20];
    let mut carry: Vec<u8> = Vec::new();
    let mut read_total = 0usize;
    // The markers live in the first few MB of the stub.
    while read_total < 32 << 20 {
        let n = match f.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        read_total += n;
        let mut hay = std::mem::take(&mut carry);
        hay.extend_from_slice(&buf[..n]);
        for (needle, flavor) in needles {
            if hay.windows(needle.len()).any(|w| w == *needle) {
                return *flavor;
            }
        }
        carry = hay[hay.len().saturating_sub(32)..].to_vec();
    }
    Flavor::Unknown
}

fn silent_args(f: Flavor) -> &'static [&'static str] {
    match f {
        Flavor::Nsis => &["/S"],
        Flavor::Inno => &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"],
        Flavor::Squirrel => &["--silent"],
        Flavor::Burn => &["/quiet", "/norestart"],
        Flavor::InstallShield => &["/s"],
        Flavor::Unknown => &[],
    }
}

/// Split a command-line string on spaces, keeping quoted parts together.
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut q = false;
    for c in s.chars() {
        match c {
            '"' => q = !q,
            ' ' | '\t' if !q => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Run a program; if Windows says it needs elevation (error 740), retry
/// through a UAC prompt.
fn run_maybe_elevated(program: &str, args: &[String]) -> Result<()> {
    log::info!("running: {program} {}", args.join(" "));
    match system::command(program).args(args).status() {
        Ok(st) if st.success() => Ok(()),
        // 3010 = success, reboot required.
        Ok(st) if st.code() == Some(3010) => Ok(()),
        Ok(st) => bail!("{program} exited with {st}"),
        Err(e) if e.raw_os_error() == Some(740) => {
            let arglist = if args.is_empty() {
                String::new()
            } else {
                format!(
                    " -ArgumentList {}",
                    args.iter().map(|a| ps_quote(a)).collect::<Vec<_>>().join(",")
                )
            };
            let code = powershell(&format!(
                "$p = Start-Process -FilePath {}{} -Verb RunAs -Wait -PassThru; $p.ExitCode",
                ps_quote(program),
                arglist
            ))?;
            match code.trim() {
                "0" | "3010" | "" => Ok(()),
                c => bail!("{program} exited with code {c}"),
            }
        }
        Err(e) => Err(e).with_context(|| format!("starting {program}")),
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase", default)]
struct UninstallEntry {
    display_name: String,
    display_version: Option<String>,
    uninstall_string: Option<String>,
    quiet_uninstall_string: Option<String>,
    install_location: Option<String>,
    display_icon: Option<String>,
}

/// Search the registry's Uninstall keys for an entry whose name contains
/// one of `names`.
fn find_uninstall_entry(names: &[String]) -> Option<UninstallEntry> {
    if !cfg!(windows) {
        return None;
    }
    let script = r"$p = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*','HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
Get-ItemProperty $p -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName } | Select-Object DisplayName,DisplayVersion,UninstallString,QuietUninstallString,InstallLocation,DisplayIcon | ConvertTo-Json -Compress";
    let json = powershell(script).ok()?;
    let entries: Vec<UninstallEntry> = if json.trim_start().starts_with('[') {
        serde_json::from_str(&json).ok()?
    } else {
        vec![serde_json::from_str(&json).ok()?]
    };
    let lowered: Vec<String> = names.iter().map(|n| n.to_lowercase()).filter(|n| n.len() > 1).collect();
    // Exact name first, then "contains".
    let exact = entries.iter().position(|e| lowered.contains(&e.display_name.to_lowercase()));
    let idx = exact.or_else(|| {
        entries.iter().position(|e| {
            let d = e.display_name.to_lowercase();
            lowered.iter().any(|n| d.contains(n.as_str()))
        })
    })?;
    entries.into_iter().nth(idx)
}

fn names_for(app: &App) -> Vec<String> {
    vec![app.name.clone(), base_name(app), base_name(app).replace('-', " ")]
}

fn exe_from_entry(e: &UninstallEntry) -> Option<String> {
    if let Some(icon) = &e.display_icon {
        let p = icon.split(',').next().unwrap_or(icon).trim_matches('"').to_string();
        if p.to_lowercase().ends_with(".exe") && !p.to_lowercase().contains("unins") && Path::new(&p).exists() {
            return Some(p);
        }
    }
    None
}

pub fn installer(ctx: &Ctx) -> Result<Outcome> {
    let s = &ctx.app.settings;
    let args: Vec<String> = if !s.installer_args.trim().is_empty() {
        split_args(&s.installer_args)
    } else if s.silent_install {
        let f = sniff(ctx.file);
        log::info!("[{}] installer type: {f:?}", ctx.app.name);
        silent_args(f).iter().map(|a| a.to_string()).collect()
    } else {
        vec![]
    };
    // Windows refuses to run a setup .exe without the .exe extension.
    let file = ctx.file.to_string_lossy().into_owned();
    run_maybe_elevated(&file, &args)?;
    let entry = find_uninstall_entry(&names_for(ctx.app));
    Ok(Outcome {
        executable: entry.as_ref().and_then(exe_from_entry),
        package_name: entry.map(|e| e.display_name),
        ..Default::default()
    })
}

fn msi_product_code(file: &str) -> Option<String> {
    powershell(&format!(
        "$i = New-Object -ComObject WindowsInstaller.Installer; \
         $db = $i.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$i,@({},0)); \
         $v = $db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@(\"SELECT Value FROM Property WHERE Property='ProductCode'\")); \
         [void]$v.GetType().InvokeMember('Execute','InvokeMethod',$null,$v,$null); \
         $r = $v.GetType().InvokeMember('Fetch','InvokeMethod',$null,$v,$null); \
         $r.GetType().InvokeMember('StringData','GetProperty',$null,$r,1)",
        ps_quote(file)
    ))
    .ok()
    .filter(|c| c.starts_with('{'))
}

pub fn msi(ctx: &Ctx) -> Result<Outcome> {
    let file = ctx.file.to_string_lossy().into_owned();
    let code = msi_product_code(&file);
    let mut args = vec!["/i".to_string(), file.clone(), "/norestart".into()];
    if !ctx.app.settings.installer_args.trim().is_empty() {
        args.extend(split_args(&ctx.app.settings.installer_args));
    } else if ctx.app.settings.silent_install {
        args.push("/passive".into());
    }
    run_maybe_elevated("msiexec", &args)?;
    let entry = find_uninstall_entry(&names_for(ctx.app));
    Ok(Outcome {
        executable: entry.as_ref().and_then(exe_from_entry),
        package_name: code.or(entry.map(|e| e.display_name)),
        ..Default::default()
    })
}

pub fn msix(ctx: &Ctx) -> Result<Outcome> {
    let file = ctx.file.to_string_lossy().into_owned();
    powershell(&format!("Add-AppxPackage -Path {} -ForceApplicationShutdown", ps_quote(&file)))?;
    Ok(Outcome::default())
}

pub fn uninstall(app: &App, inst: &Installed) -> Result<()> {
    match inst.kind {
        InstallKind::Msi => {
            let code = inst.package_name.clone().filter(|c| c.starts_with('{'));
            if let Some(code) = code {
                return run_maybe_elevated("msiexec", &["/x".into(), code, "/passive".into(), "/norestart".into()]);
            }
        }
        InstallKind::Msix => bail!("remove {} from Windows Settings → Apps", app.name),
        _ => {}
    }
    let mut names = names_for(app);
    if let Some(n) = &inst.package_name {
        names.insert(0, n.clone());
    }
    let entry = find_uninstall_entry(&names)
        .with_context(|| format!("couldn't find {} in the list of installed programs", app.name))?;
    let cmd = entry
        .quiet_uninstall_string
        .or(entry.uninstall_string)
        .context("the program has no uninstall command")?;
    // "C:\path\unins000.exe" /SILENT  or  MsiExec.exe /X{GUID}
    let (prog, rest) = if let Some(stripped) = cmd.strip_prefix('"') {
        let end = stripped.find('"').unwrap_or(stripped.len());
        (stripped[..end].to_string(), stripped[end..].trim_start_matches('"').to_string())
    } else if let Some(i) = cmd.to_lowercase().find(".exe") {
        (cmd[..i + 4].to_string(), cmd[i + 4..].to_string())
    } else {
        (cmd.clone(), String::new())
    };
    let mut args = split_args(&rest);
    if args.is_empty() {
        let p = prog.to_lowercase();
        if p.contains("unins") {
            args = vec!["/VERYSILENT".into(), "/SUPPRESSMSGBOXES".into(), "/NORESTART".into()];
        } else if p.contains("uninstall") {
            args = vec!["/S".into()];
        }
    }
    run_maybe_elevated(&prog, &args)
}

/// Version reported by Windows for an installed program.
pub fn installed_version(app: &App, inst: &Installed) -> Option<String> {
    let mut names = names_for(app);
    if let Some(n) = &inst.package_name {
        names.insert(0, n.clone());
    }
    find_uninstall_entry(&names).and_then(|e| e.display_version)
}

/// Start Menu shortcut in Programs\Obtainit.
pub fn create_shortcut(ctx: &Ctx, exe: &Path) -> Result<String> {
    let dir = dirs::data_dir()
        .unwrap_or_default()
        .join(r"Microsoft\Windows\Start Menu\Programs\Obtainit");
    std::fs::create_dir_all(&dir)?;
    let name: String = ctx.app.name.chars().filter(|c| !r#"\/:*?"<>|"#.contains(*c)).collect();
    let lnk: PathBuf = dir.join(format!("{name}.lnk"));
    let workdir = exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    powershell(&format!(
        "$s = (New-Object -ComObject WScript.Shell).CreateShortcut({}); $s.TargetPath = {}; $s.WorkingDirectory = {}; $s.Save()",
        ps_quote(&lnk.to_string_lossy()),
        ps_quote(&exe.to_string_lossy()),
        ps_quote(&workdir)
    ))?;
    Ok(lnk.to_string_lossy().into_owned())
}

/// Put the bin folder on the user's PATH.
pub fn add_to_user_path(dir: &str) -> Result<()> {
    powershell(&format!(
        "$d = {}; $p = [Environment]::GetEnvironmentVariable('Path','User'); \
         if (-not (($p -split ';') -contains $d)) {{ [Environment]::SetEnvironmentVariable('Path', (($p.TrimEnd(';')) + ';' + $d).TrimStart(';'), 'User') }}",
        ps_quote(dir)
    ))?;
    Ok(())
}
