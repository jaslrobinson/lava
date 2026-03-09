mod commands;
mod klwp_import;
mod project;
mod providers;

use providers::manager::ProviderManager;
use tauri::Manager;

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

            let data = manager.data();
            app.manage(data);

            manager.start(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::formula::evaluate_formula,
            commands::project::save_project,
            commands::project::load_project,
            commands::project::import_klwp,
            commands::wallpaper::start_wallpaper_mode,
            commands::wallpaper::stop_wallpaper_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
