//! Background update checks, notifications and (optional) automatic updates.

use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::ops::{self, state};

fn notify(h: &AppHandle, title: &str, body: &str) {
    if let Err(e) = h.notification().builder().title(title).body(body).show() {
        log::warn!("notification failed: {e}");
    }
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1..=4 => names.join(", "),
        n => format!("{} and {} more", names[..3].join(", "), n - 3),
    }
}

/// One background round: check, notify, maybe update.
pub fn run_once(h: &AppHandle) {
    let (ids, settings) = {
        let st = state(h);
        let s = st.store.lock().unwrap();
        let ids: Vec<String> = s
            .apps
            .iter()
            .filter(|a| !s.settings.only_check_installed || a.installed.is_some() || a.settings.track_only)
            .map(|a| a.id.clone())
            .collect();
        (ids, s.settings.clone())
    };
    if ids.is_empty() {
        return;
    }
    log::info!("background check of {} apps", ids.len());
    ops::check_many(h, ids);

    // Updates we haven't announced yet.
    let (fresh, auto): (Vec<(String, String)>, Vec<(String, String)>) = {
        let st = state(h);
        let mut s = st.store.lock().unwrap();
        let mut fresh = vec![];
        let mut auto = vec![];
        for a in s.apps.iter_mut() {
            if !a.has_update() {
                continue;
            }
            let v = a.latest.as_ref().map(|l| l.version.clone()).unwrap_or_default();
            if settings.background_updates && ops::auto_updatable(a) {
                auto.push((a.id.clone(), a.name.clone()));
            } else if a.notified_version.as_deref() != Some(v.as_str()) && !a.settings.skip_notifications {
                fresh.push((a.id.clone(), format!("{} {v}", a.name)));
            }
            a.notified_version = Some(v);
        }
        s.save_apps();
        (fresh, auto)
    };

    if settings.notifications && !fresh.is_empty() {
        let names: Vec<String> = fresh.iter().map(|(_, n)| n.clone()).collect();
        let title = if names.len() == 1 { "Update available".to_string() } else { format!("{} updates available", names.len()) };
        notify(h, &title, &list(&names));
    }

    let mut updated = vec![];
    let mut failed = vec![];
    for (id, name) in auto {
        match ops::install(h, &id, None) {
            Ok(a) => updated.push(format!(
                "{} {}",
                a.name,
                a.installed.map(|i| i.version).unwrap_or_default()
            )),
            Err(e) => failed.push(format!("{name}: {e:#}")),
        }
    }
    if settings.notifications && !updated.is_empty() {
        notify(h, &format!("Updated {} app{}", updated.len(), if updated.len() == 1 { "" } else { "s" }), &list(&updated));
    }
    if settings.notifications && !failed.is_empty() {
        notify(h, "Some updates failed", &list(&failed));
    }
    let _ = h.emit("apps-reloaded", ());
    self_check(h);
}

/// Check for a new Obtainit; notify, or install it when allowed.
pub fn self_check(h: &AppHandle) {
    let settings = state(h).store.lock().unwrap().settings.clone();
    if !settings.self_update_check {
        return;
    }
    let st = crate::self_update::check(h);
    if !st.available {
        return;
    }
    let v = st.latest.clone().unwrap_or_default();
    if settings.self_update_auto && st.can_install {
        // Background installs only for updates that don't need a password.
        if matches!(st.method, crate::self_update::Method::AppImage | crate::self_update::Method::Nsis | crate::self_update::Method::Msi) {
            if settings.notifications {
                notify(h, "Updating Obtainit", &format!("Installing {v} and restarting"));
            }
            if let Err(e) = crate::self_update::apply(h) {
                log::error!("self-update failed: {e:#}");
                notify(h, "Obtainit update failed", &format!("{e:#}"));
            }
            return;
        }
    }
    if settings.notifications && crate::self_update::should_notify(&st) {
        notify(h, "Obtainit update available", &format!("Version {v} is ready. Open Settings → About to update."));
    }
}

pub fn start(h: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(4));
        let on_start = state(&h).store.lock().unwrap().settings.check_on_start;
        if on_start {
            run_once(&h);
        } else {
            self_check(&h);
        }
        let mut last = Instant::now();
        loop {
            std::thread::sleep(Duration::from_secs(30));
            let mins = state(&h).store.lock().unwrap().settings.check_interval_minutes;
            if mins == 0 {
                continue;
            }
            if last.elapsed() >= Duration::from_secs(mins as u64 * 60) {
                last = Instant::now();
                run_once(&h);
            }
        }
    });
}
