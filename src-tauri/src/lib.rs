mod commands;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState {
                settings: std::sync::Mutex::new(commands::load_settings(app.handle())),
                last_repos: std::sync::Mutex::new(Vec::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::probe_git,
            commands::scan_repos,
            commands::read_status,
            commands::add_repo,
            commands::batch_fetch,
            commands::batch_pull,
            commands::fetch_repos,
            commands::pull_repos
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
