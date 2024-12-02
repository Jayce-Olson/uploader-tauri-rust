// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn add_x(current_value: i32) -> i32 {
    let x = 5;
    current_value + x
}

// Also in main.rs
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![add_x])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
