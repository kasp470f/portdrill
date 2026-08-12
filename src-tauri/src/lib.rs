mod commands;
mod config;
mod models;
mod tunnel_manager;

use commands::AppState;
use tunnel_manager::TunnelManager;
use tauri::Manager;
use tauri::Emitter;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
mod macos {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    pub fn set_dock_visible(visible: bool) {
        unsafe {
            let ns_app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let policy: i32 = if visible { 0 } else { 1 }; // 0 = regular, 1 = accessory
            let _: () = msg_send![ns_app, setActivationPolicy: policy];
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rules = config::load_rules();

    let state = AppState {
        rules: Mutex::new(rules),
        tunnel_manager: TunnelManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_rules,
            commands::create_rule,
            commands::update_rule,
            commands::delete_rule,
            commands::toggle_rule,
            commands::get_tunnel_status,
            commands::reorder_rules,
        ])
        .setup(|app| {
            let has_tray = Arc::new(AtomicBool::new(false));

            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let check_update = MenuItem::with_id(app, "check_update", "Check for Updates", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &check_update, &separator, &quit])?;

            let tray_result = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("PortDrill")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        // Ensure Dock icon is visible when showing the window on macOS
                        #[cfg(target_os = "macos")]
                        {
                            macos::set_dock_visible(true);
                        }
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "check_update" => {
                        // Ensure Dock icon is visible when showing the window on macOS
                        #[cfg(target_os = "macos")]
                        {
                            macos::set_dock_visible(true);
                        }
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("check-for-update", ());
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
                        // Ensure Dock icon is visible when showing the window on macOS
                        #[cfg(target_os = "macos")]
                        {
                            macos::set_dock_visible(true);
                        }
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
                    // Hide the window instead of closing and hide Dock on macOS
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    {
                        macos::set_dock_visible(false);
                    }
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
