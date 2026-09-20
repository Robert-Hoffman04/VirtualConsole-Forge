#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_cores,
            commands::build_wad_command,
            commands::preview_core_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vc-tauri");
}
