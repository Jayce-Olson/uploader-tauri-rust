// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils; // Declare the utils module

use utils::copy; // Import the copy module
use utils::retrieve_drives;

/* I am still learning Rust so please excuse my excesive amount of comments */

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            retrieve_drives::list_devices,
            copy::copy_dir
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
