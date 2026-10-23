# Obtainit

An [Obtainium](https://github.com/ImranR98/Obtainium)-style updater for **Linux and Windows** desktops.
Track apps on GitHub, GitLab, Codeberg/Forgejo or any download page, and install new releases as
soon as they're published.

- **Engine:** [lifter](https://github.com/cjrh/lifter) (Rust), turned into a library: `crates/lifter-core`
- **UI:** Material 3 (Google's [`@material/web`](https://github.com/material-components/material-web)
  components and the same seed-color theming that Flutter apps like Autonomix and Obtainium use)
- **Shell:** [Tauri 2](https://tauri.app): small native binary, system tray, notifications

## Features

**Sources**
- GitHub (with optional token), GitLab (incl. self-hosted), Codeberg / Forgejo / Gitea
- Any web page: lifter's HTML method (CSS selector + link regex + optional version selector)
- Any JSON API: lifter's `api_json` method (JSONPath for version, URLs and commit)

**What it can install**

| Linux | Windows |
|---|---|
| AppImage (+ app menu entry) | Setup `.exe` (silent install for NSIS, Inno Setup, Squirrel, WiX) |
| Portable binaries into `~/.local/bin` | MSI (`msiexec /passive`) |
| Binaries pulled out of `.tar.gz/.xz/.zst/.bz2/.zip/.gz` (lifter) | MSIX |
| Whole archives unpacked to a folder (+ menu entry) | Portable `.exe` and `.zip` apps (+ Start menu shortcut) |
| `.deb`, `.rpm`, Arch packages (via `pkexec`, like Autonomix) | |
| Flatpak bundles (user install), Snaps | |

**Per-app options (Obtainium's, plus lifter's)**
- Include prereleases · fall back to older releases · verify GitHub's "latest" tag
- Filter release titles / notes by regex (invertible) · sort by date, version or API order
- Minimum release age · use latest file upload as release date
- Version from tag, title or date · version extraction regex + match group · "any change" mode (for `nightly` tags, with commit tracking)
- File-name filter regex (invertible) · automatic OS/CPU matching (lifter's scoring) · force an install type
- Files to extract from archives and rename (lifter's `target_filename(s)_to_extract_from_archive` / `desired_filename`)
- Silent install / custom installer arguments · menu shortcut · SHA-256 verification (GitHub digests)
- Track-only · exempt from background updates · skip notifications · ignore a version
- Custom launch command · command to run after each update

**App-wide**
- Background checks on a schedule, notifications, optional automatic updates
- Tray icon, close-to-tray, start with the system (hidden)
- Search, filter chips, categories, favorites, sorting, "Update all"
- Import Obtainit backups, **Obtainium exports**, **lifter.config** files, or URL lists
- Export to Obtainit JSON or **lifter.config** (to run the same downloads headless with `lifter`)
- Logs page (lifter-core's messages included)
- Works down to phone-width windows; compact list option

**Themes**
- Auto / Light / Dark mode, with a separate theme for the light and the dark slot
- Material You default themes generated from any palette color, plus an optional separate accent color
- Templates: Paper, Solarized, Nord, Rosé Dawn, Black (OLED), Dracula, Catppuccin Mocha, Midnight
- Custom themes: pick background, surface, text and accent from a full RGB/HSV spectrum picker,
  with a live preview of the whole app and contrast warnings; or duplicate a template and tweak it

**Updates itself**
- Checks github.com/Shantil-K/Obtainit releases on startup and on the app schedule
- Detects how it was installed (AppImage, .deb, .rpm, Windows setup, MSI), downloads the matching
  prebuilt file, verifies its SHA-256, installs it and restarts (optionally fully automatic)

## Layout

```
crates/lifter-core/   lifter's engine as a library: http (retry/backoff), scrape (json/html),
                      assets (platform scoring), archive (extraction), version, config (lifter.config)
src-tauri/src/
  sources/            GitHub, GitLab, Forgejo, HTML, JSON
  resolve.rs          release filtering pipeline + asset ranking
  install/            portable (lifter), linux (AppImage/deb/rpm/pacman/flatpak/snap), windows
  ops.rs              check / download / verify / install / uninstall
  scheduler.rs        background checks, notifications, auto-updates
  import_export.rs    Obtainit, Obtainium, lifter.config, URL lists
src/                  Svelte 5 + Material 3 UI
```

## Building

Requirements: Rust (stable), Node 20+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/)
(on Debian/Ubuntu: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`).

```bash
npm install
npm run tauri dev        # run with hot reload
npm run tauri build      # Linux: .deb, .rpm, .AppImage   Windows: NSIS .exe, .msi
```

Windows installers must be built on Windows (or by the GitHub Actions workflow in `.github/workflows`).

Tests:

```bash
cargo test                                         # unit tests
cargo test -p obtainit -- --ignored --nocapture    # real downloads/installs into temp folders
```

UI preview in a normal browser with a fake backend: `VITE_MOCK=1 npx vite`, then open
`http://localhost:1420/?page=app&id=ripgrep` (also `?page=add`, `?page=settings`, `?theme=dark`).

## Releases and lifter updates

```bash
scripts/release.sh patch     # or minor / major / 1.2.3
```

This bumps the version, tags `vX.Y.Z` and pushes. `.github/workflows/build.yml` then builds the
Linux and Windows installers and publishes the GitHub release, and installed copies update themselves.

Upstream lifter is watched daily by `.github/workflows/lifter-upstream.yml`. When cjrh/lifter publishes
a release newer than the one in `crates/lifter-core/UPSTREAM`, it opens a pull request listing the
changed engine files and commits. lifter-core is lifter's engine restructured as a library, so
upstream changes are ported by hand on that branch. Merging the PR runs
`.github/workflows/auto-release.yml`, which bumps the version and publishes a new release —
and every installed Obtainit picks it up on its next check.

One-time repository setting for the bot PRs: *Settings → Actions → General → Workflow permissions →
"Allow GitHub Actions to create and approve pull requests"*.

## Where things are stored

| | Linux | Windows |
|---|---|---|
| Apps & settings | `~/.config/obtainit/` | `%APPDATA%\obtainit\` |
| Log | `~/.local/share/obtainit/obtainit.log` | `%APPDATA%\obtainit\obtainit.log` |
| Single binaries (default) | `~/.local/bin` | `%LOCALAPPDATA%\Obtainit\bin` |
| AppImages / unpacked apps (default) | `~/Applications` | `%LOCALAPPDATA%\Obtainit\Apps` |

## License

AGPL-3.0-or-later, because the engine comes from lifter (AGPL-3.0).
