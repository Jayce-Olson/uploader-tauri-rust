// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use winapi::um::fileapi::{GetDriveTypeW, GetLogicalDriveStringsW};
use winapi::um::winbase::DRIVE_REMOVABLE;

use windows::Win32::Foundation::{BOOL, HANDLE};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_PRIVILEGES,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/* I am still learning Rust so please excuse my excesive amount of comments */

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_devices, copy_dir])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

#[tauri::command]
fn copy_dir(src: String, dest: String) -> Result<(), String> {
    if let Err(e) = elevate_privileges() {
        println!("Failed to elevate privileges: {}", e);
    } else {
        println!("Privileges elevated successfully.");
    }
    let src_path = PathBuf::from(src);
    let dest_path = PathBuf::from(dest);

    // Call the recursive copy function, map errors to strings
    match copy_dir_recursive(&src_path, &dest_path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to copy directory: {}", e)),
    }
}

// Tauri commands do not allow std::io::Result return types so I needed to make this seperate
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    println!("{}", src.display());
    if src.is_dir() {
        fs::create_dir_all(dest)?; // Create destination directory - if there is an error, the "?" will make this statement automatically return, with the error
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            println!("{}", entry.file_name().to_string_lossy());
            let entry_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if entry_path.is_dir() {
                copy_dir_recursive(&entry_path, &dest_path)?; // Recursive call for directories
            } else {
                let _ = fs::copy(&entry_path, &dest_path); // I do not return an error if this is bad, because I do not want files that are corrupt, or base windows files like IndexerVolumeGuid to end the process.
            }
        }
    } else {
        fs::copy(src, dest)?; // Copy a single file
    }
    Ok(())
}

#[tauri::command]
fn list_devices() -> Vec<String> {
    /*
        Below an array of buffers is created. It uses the unicharacter 16 bit format (needed later to pass to GetLogicalDriveStringsW)mand is initilized with
        a size of 30. The array is mutable and each posistion is initilized with the value of 0, which represents \0, which signifies the end of a string. I chose 30
        because for this applications context it is unlikely the user will have more than 30 drives attached, it is unlikely they will have more than 10 but I chose 30
        to be safe. In the event that there is more than 30, because I chose an array for the data structure, extra data will be automatically trunacated instead of causing
        an error.The drive strings will likely look like this: C:\0D:\0E:\0\0 and the array like this: [67, 58, 92, 0, 68, 58, 92, 0, 69, 58, 92, 0, 0, 0, ...]
        Theoretically a vector may have been a better data structure so that any additional drives could just be appended, but due to the incredibly-wildly low chance
        of that happening, I stuck with a faster-fixed size array.
    */
    let mut buffer: [u16; 30] = [0; 30];
    /*
        Below is unsafe because it is calling windows API (a C function) that Rusts memory safety will not have controll over (it can't garentee memory safety).
        "unsafe" allow the below code to compile. GetLogicalDriveStringsW() requires a raw pointer. A mutiple reference is unable to be passed through, which is
        why .as_mut_ptr() is used to return a mutable pointer to buffer.
    */
    let length = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };

    if length == 0 {
        return vec!["Error retrieving drives.".to_string()];
    }

    let drives: Vec<String> = buffer
        .split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .map(|s| OsString::from_wide(s).to_string_lossy().to_string())
        .filter(|drive| {
            let drive_type =
                unsafe { GetDriveTypeW(drive.encode_utf16().collect::<Vec<u16>>().as_ptr()) };
            drive_type == DRIVE_REMOVABLE
        })
        .collect();

    drives
}
fn elevate_privileges() -> Result<(), String> {
    unsafe {
        let mut token_handle: HANDLE = HANDLE(0);

        // Open the process token
        let result: BOOL = OpenProcessToken(
            GetCurrentProcess(),
            windows::Win32::Security::TOKEN_ADJUST_PRIVILEGES,
            &mut token_handle,
        );

        if !result.as_bool() {
            return Err("Failed to open process token.".to_string());
        }

        let mut privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: Default::default(),
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        // Use the wide-character version of LookupPrivilegeValue
        let luid_result = LookupPrivilegeValueW(
            None,                             // Local system
            windows::w!("SeBackupPrivilege"), // Wide string literal
            &mut privileges.Privileges[0].Luid,
        );

        if !luid_result.as_bool() {
            return Err("Failed to lookup privilege value for SeBackupPrivilege.".to_string());
        }

        privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        // Adjust the token privileges
        let adjust_result =
            AdjustTokenPrivileges(token_handle, false, Some(&privileges), 0, None, None);

        if !adjust_result.as_bool() {
            return Err("Failed to adjust token privileges.".to_string());
        }

        Ok(())
    }
}
