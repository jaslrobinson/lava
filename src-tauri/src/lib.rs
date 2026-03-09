mod commands;
mod klwp_import;
mod project;
mod providers;

use providers::manager::ProviderManager;
use tauri::Emitter;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_editor = MenuItemBuilder::with_id("show_editor", "Show Editor").build(app)?;
    let toggle_wallpaper =
        MenuItemBuilder::with_id("toggle_wallpaper", "Start/Stop Wallpaper").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_editor)
        .item(&toggle_wallpaper)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("KLLW - Kustom Linux Live Wallpaper")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_editor" => {
                if let Some(window) = app.get_webview_window("main") {
                    window.show().ok();
                    window.set_focus().ok();
                }
            }
            "toggle_wallpaper" => {
                if let Some(window) = app.get_webview_window("main") {
                    if commands::wallpaper::is_wallpaper_active() {
                        // Stop wallpaper — invoke the stop command logic
                        window.emit("tray-stop-wallpaper", ()).ok();
                    } else {
                        // Show editor so user can start wallpaper from there
                        window.show().ok();
                        window.set_focus().ok();
                        window.emit("tray-start-wallpaper", ()).ok();
                    }
                }
            }
            "quit" => {
                commands::wallpaper::kill_wallpaper_process();
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
                    window.show().ok();
                    window.set_focus().ok();
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let mut manager = ProviderManager::new();

            manager.register(Box::new(providers::datetime::DateTimeProvider));
            manager.register(Box::new(providers::battery::BatteryProvider::new()));
            manager.register(Box::new(
                providers::sysinfo_provider::SysInfoProvider::new(),
            ));
            manager.register(Box::new(
                providers::resource_monitor::ResourceMonitorProvider::new(),
            ));
            manager.register(Box::new(providers::music::MusicProvider));
            manager.register(Box::new(providers::network::NetworkProvider));
            manager.register(Box::new(providers::traffic::TrafficProvider::new()));
            let (weather, forecast) = providers::weather::create_providers();
            manager.register(Box::new(weather));
            manager.register(Box::new(forecast));

            let data = manager.data();
            app.manage(data);

            manager.start(app.handle().clone());

            setup_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if commands::wallpaper::is_wallpaper_active() {
                    // Wallpaper is running — hide instead of closing
                    api.prevent_close();
                    window.hide().ok();
                }
                // If wallpaper is NOT active, allow normal close/quit
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::formula::evaluate_formula,
            commands::project::save_project,
            commands::project::load_project,
            commands::project::import_klwp,
            commands::wallpaper::start_wallpaper_mode,
            commands::wallpaper::stop_wallpaper_mode,
            commands::window::hide_editor,
            commands::window::show_editor,
            commands::window::quit_app,
            commands::window::is_wallpaper_running,
            commands::settings::load_settings,
            commands::settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
