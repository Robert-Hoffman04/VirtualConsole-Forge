#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_cores,
            commands::build_wad_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vc-tauri");
}
