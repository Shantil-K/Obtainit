mod commands;
#[cfg(test)]
mod e2e_tests;
mod import_export;
mod install;
mod logs;
mod model;
mod ops;
mod resolve;
mod scheduler;
mod self_update;
mod sources;
mod store;
mod system;

use std::collections::HashSet;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

fn show_main(h: &AppHandle) {
    if let Some(w) = h.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn build_tray(h: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(h, "show", "Open Obtainit", true, None::<&str>)?;
    let check = MenuItem::with_id(h, "check", "Check for updates", true, None::<&str>)?;
    let quit = MenuItem::with_id(h, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(h)?;
    let menu = Menu::with_items(h, &[&show, &check, &sep, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Obtainit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|h, e| match e.id().as_ref() {
            "show" => show_main(h),
            "check" => {
                let h = h.clone();
                std::thread::spawn(move || scheduler::run_once(&h));
            }
            "quit" => h.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = h.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(h)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logs::init();
    log::info!("Obtainit {} starting", env!("CARGO_PKG_VERSION"));
    let store = store::Store::load();
    let start_hidden =
        std::env::args().any(|a| a == "--minimized") && store.settings.start_minimized
            || std::env::args().any(|a| a == "--hidden");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|h, _args, _cwd| show_main(h)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(ops::State { store: Mutex::new(store), busy: Mutex::new(HashSet::new()) })
        .setup(move |app| {
            let h = app.handle().clone();
            if let Err(e) = build_tray(&h) {
                log::warn!("tray icon unavailable: {e}");
            }
            if let Some(w) = app.get_webview_window("main") {
                let h2 = h.clone();
                w.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        let to_tray = ops::state(&h2).store.lock().unwrap().settings.close_to_tray;
                        if to_tray {
                            api.prevent_close();
                            if let Some(w) = h2.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                    }
                });
                if !start_hidden {
                    let _ = w.show();
                }
            }
            scheduler::start(h);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::preview_app,
            commands::add_app,
            commands::update_app,
            commands::delete_app,
            commands::check_apps,
            commands::install_app,
            commands::update_all,
            commands::uninstall_app,
            commands::mark_installed,
            commands::mark_not_installed,
            commands::ignore_version,
            commands::prefer_asset,
            commands::launch_app,
            commands::can_launch,
            commands::app_folder,
            commands::save_settings,
            commands::system_info,
            commands::add_bin_to_path,
            commands::import_file,
            commands::import_text,
            commands::export_file,
            commands::get_logs,
            commands::clear_logs,
            commands::log_path,
            commands::self_update_status,
            commands::self_update_check,
            commands::self_update_apply,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Obtainit");
}
