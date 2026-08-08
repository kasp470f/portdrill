mod commands;
mod config;
mod models;
mod tunnel_manager;

use commands::AppState;
use tunnel_manager::TunnelManager;
use tauri::Manager;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rules = config::load_rules();

    let state = AppState {
        rules: Mutex::new(rules),
        tunnel_manager: TunnelManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::toggle_rule,
            commands::get_tunnel_status,
        ])
        .setup(|app| {
            let has_tray = Arc::new(AtomicBool::new(false));

            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let tray_result = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("TunnelApp")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app);

            if tray_result.is_ok() {
                has_tray.store(true, Ordering::SeqCst);
            }

            app.manage(has_tray);

            // Health check thread
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(5));
                let state: tauri::State<AppState> = handle.state();
                state.tunnel_manager.health_check(&handle);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let has_tray: tauri::State<Arc<AtomicBool>> = window.state();
                if has_tray.load(Ordering::SeqCst) {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
