// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::io;
use std::path::PathBuf;
use tauri::Manager;


fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![add_x, move_files,move_files_with_progress])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

#[tauri::command]
fn add_x(current_value: i32) -> i32 {
    let x = 5;
    current_value + x
}

#[tauri::command]
fn list_devices() -> Vec<String> {
    let mut devices = Vec::new();
    if let Ok(entries) = fs::read_dir("C:/media") { // entries will hold the unwrapped return value of fs::read_dir
        for entry in entries {
            if let Ok(entry) = entry {
                devices.push(entry.path().display().to_string());
            }
        }
    }
    devices // devices will be returned
}


#[tauri::command]
fn move_files(src: String, dest: String) -> Result<(), String> { // "()" return type means no meaningful value which is akin to void (Result<success return val, failure return val>)
    let src_path = PathBuf::from(src); // PathBuff is a type that handles differences in operating system paths ("/" vs "\")
    let dest_path = PathBuf::from(dest); // This isn't casting, but conversion. This is converting the type to PathBuf

    if let Err(e) = fs::copy(&src_path, &dest_path) { 
        return Err(format!("Failed to move files: {}", e));
    }

    Ok(()) // Basically return void value
}

#[tauri::command]
fn move_files_with_progress(
    app_handle: tauri::AppHandle,
    src: String,
    dest: String,
) -> Result<(), String> {
    let src_path = PathBuf::from(src);
    let dest_path = PathBuf::from(dest);

    // Simulate progress (replace with actual logic) - It is possible I may need to do fs::copy for each individual file/sub directory
    for progress in (0..=100).step_by(10) {
        app_handle.emit_all("file-progress", progress).unwrap(); // Part of Tauri, emits values to all listeners attached to "file-progress"
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // fs::copy(&src_path, &dest_path).map_err(|e| format!("Failed to move files: {}", e))?; // I am going to write the not shorthand version of this for clarification and because I am still new
    if let Err(e) = fs::copy(&src_path, &dest_path){ 
      return Err(format!("Failed to move files: {}", e))
    }
    Ok(())
}