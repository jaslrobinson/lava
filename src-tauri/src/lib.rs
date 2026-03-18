mod commands;
mod klwp_import;
mod plugins;
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
    let close_editor = MenuItemBuilder::with_id("close_editor", "Close Editor").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit All").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_editor)
        .item(&toggle_wallpaper)
        .separator()
        .item(&close_editor)
        .item(&quit)
        .build()?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("LAVA - Live Animated Visuals for Arch")
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
            "close_editor" => {
                // Close editor only — wallpaper keeps running
                app.exit(0);
            }
            "quit" => {
                // Quit everything — kill wallpaper + exit
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
            manager.register(Box::new(providers::radar::RadarProvider));
            manager.register(Box::new(providers::hyprland::HyprlandProvider::new()));

            for provider in plugins::load_plugins() {
                manager.register(provider);
            }

            let data = manager.data();

            // AI provider needs shared data to read the current artist from music provider
            manager.register(Box::new(providers::ai::AiProvider::new(data.clone())));

            app.manage(data.clone());

            let provider_handle = manager.start(app.handle().clone());
            app.manage(provider_handle);

            // Start Hyprland event listener — writes directly to shared data + emits events
            providers::hyprland::HyprlandProvider::start_event_listener(
                app.handle().clone(),
                data,
            );

            setup_tray(app)?;

            // Start audio visualizer capture (non-blocking, spawns background thread)
            let audio_bands = providers::audio::new_shared_bands();
            providers::audio::start_audio_capture(app.handle().clone(), audio_bands);

            // Write editor PID file so wallpaper can detect us
            let _ = std::fs::write("/tmp/lava-editor.pid", std::process::id().to_string());

            // Watch for show-editor signal from wallpaper process
            commands::wallpaper::start_signal_watcher(app.handle().clone());

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Always allow close — standalone wallpaper keeps running independently
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::formula::evaluate_formula,
            commands::formula::get_provider_data,
            commands::project::save_project,
            commands::project::load_project,
            commands::project::import_komp,
            commands::project::export_komp,
            commands::project::list_project_fonts,
            commands::project::list_system_fonts,
            commands::project::copy_asset_to_project,
            commands::project::extract_apk_icon,
            commands::project::write_icon_file,
            commands::project::extract_image_layer,
            commands::wallpaper::start_wallpaper_mode,
            commands::wallpaper::stop_wallpaper_mode,
            commands::window::hide_editor,
            commands::window::show_editor,
            commands::window::quit_app,
            commands::window::is_wallpaper_running,
            commands::window::open_url,
            commands::window::music_control,
            commands::window::launch_app,
            commands::window::adjust_volume,
            commands::settings::load_settings,
            commands::settings::save_settings,
            commands::settings::set_autostart,
            commands::apps::list_apps,
            commands::apps::resolve_icon,
            commands::apps::get_window_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
