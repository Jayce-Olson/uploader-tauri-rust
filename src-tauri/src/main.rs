// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::{self, File};
use std::io::{self, BufRead, ErrorKind};
use std::path::{Path, PathBuf};

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use winapi::um::fileapi::{GetDriveTypeW, GetLogicalDriveStringsW};
use winapi::um::winbase::DRIVE_REMOVABLE;

use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HANDLE};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_PRIVILEGES,
};
use windows::Win32::Storage::FileSystem::*;
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
    let src_path = PathBuf::from(src);
    let dest_path = PathBuf::from(dest);

    // There is probably a better way to do this but I am still new to this, please forgivith me Keefer..
    let new_src = src_path.clone();
    let new_dest: PathBuf = match setup(&new_src, &dest_path) {
        Ok(path) => {
            println!("Function returned PathBuf: {}", path.display());
            path
        }
        Err(e) => return Err(format!("Failed to find schema and ID: {}", e)),
    };

    println!("{}", new_dest.display());

    // Call the recursive copy function, map errors to strings
    println!("Copying {} to {}", src_path.display(), new_dest.display());
    match copy_dir_recursive(&src_path, &new_dest) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to copy directory: {}", e)),
    }
}

// Tauri commands do not allow std::io::Result return types so I needed to make this seperate
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    println!("{}", src.display());
    if src.is_dir() {
        fs::create_dir_all(dest)?; // Create directory at destination - if there is an error, the "?" will make this statement automatically return, with the error

        for entry in fs::read_dir(src)? {
            match entry {
                Ok(entry) => {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.eq_ignore_ascii_case("IndexerVolumeGuid")
                            || file_name.eq_ignore_ascii_case("System Volume Information")
                        {
                            println!("Skipping file: {}", file_name);
                            continue;
                        }
                    }

                    println!("{}", entry.file_name().to_string_lossy());
                    let entry_path = entry.path();
                    let dest_path = dest.join(entry.file_name());
                    if entry_path.is_dir() {
                        copy_dir_recursive(&entry_path, &dest_path)?; // Recursive call for directories
                    } else {
                        fs::copy(&entry_path, &dest_path)?; // return an error if this fails - windows files and corrupted files have already been filtered out
                    }
                }
                Err(e) => {
                    eprintln!("Error reading directory: {}", e);
                }
            }
        }
    } else {
        fs::copy(src, dest)?; // Copy a single file
    }
    Ok(())
}

fn setup(src: &PathBuf, dest: &PathBuf) -> io::Result<PathBuf> {
    if src.is_dir() {
        for entry in fs::read_dir(src)? {
            match entry {
                Ok(entry) => {
                    let entry_path = entry.path();
                    return setup(&entry_path, &dest);
                }
                Err(e) => {
                    continue;
                }
            }
        }
    } else {
        let file = File::open(src)?;
        let reader = io::BufReader::new(file);

        let mut unit_id = String::new();
        let mut schema = String::new();

        // Process each line
        for line in reader.lines() {
            let line = line?;

            if line.starts_with("EOT UNIT ID:") {
                unit_id = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("Customer ID:") {
                schema = line.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }
        let dir_name = format!("{} {}", schema, unit_id);
        let new_dest = dest.join(&schema);
        let final_dest: PathBuf = dest.join(schema).join(dir_name);

        // Create the directory
        println!("Creating directory: {}", new_dest.display());
        if !new_dest.exists() && new_dest.is_dir() {
            fs::create_dir_all(&new_dest)?;
            println!("{}", final_dest.display());
        }
        if (!final_dest.exists() && final_dest.is_dir()) {
            fs::create_dir_all(final_dest.clone())?;
        }
        println!("{}", final_dest.display());
        return Ok(final_dest);
    }
    return Err(io::Error::new(ErrorKind::NotFound, "File not found"));
    //Ok(dest.clone());
}

#[tauri::command]
#[cfg(target_os = "windows")]
fn list_devices() -> Vec<(String, String)> {
    /*
        Below an array of buffers is created. It uses the unicharacter 16 bit format (needed later to pass to GetLogicalDriveStringsW)mand is initilized with
        a size of 30. The array is mutable and each posistion is initilized with the value of 0, which represents \0, which signifies the end of a string. I chose 30
        because for this applications context it is unlikely the user will have more than 30 drives attached, it is unlikely they will have more than 10 but I chose 30
        to be safe. In the event that there is more than 30, because I chose an array for the data structure, extra data will be automatically trunacated instead of causing
        an error.The drive strings will likely look like this: C:\0D:\0E:\0\0 and the array like this: [67, 58, 92, 0, 68, 58, 92, 0, 69, 58, 92, 0, 0, 0, ...]
        Theoretically a vector may have been a better data structure so that any additional drives could just be appended, but due to the incredibly-wildly low chance
        of that happening, I stuck with a faster-fixed size array.
    */
    use winapi::um::winnt::PCWSTR;
    let mut buffer: [u16; 30] = [0; 30];
    /*
        Below is unsafe because it is calling windows API (a C function) that Rusts memory safety will not have controll over (it can't garentee memory safety).
        "unsafe" allow the below code to compile. GetLogicalDriveStringsW() requires a raw pointer. A mutiple reference is unable to be passed through, which is
        why .as_mut_ptr() is used to return a mutable pointer to buffer.
    */
    let length = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };

    if length == 0 {
        return vec![(
            "Error retrieving drives.".to_string(),
            "Unknown".to_string(),
        )];
    }

    // Below looks like a tumor but it is just splitting the buffer into individual strings, filtering out empty strings, converting the slices to stirngs,
    // filters out drives that are not removable (removable drives), and then finally collects them to the drives vector.
    // let drives: Vec<String> = buffer
    //     .split(|&c| c == 0)
    //     .filter(|s| !s.is_empty())
    //     .map(|s| OsString::from_wide(s).to_string_lossy().to_string())
    //     .filter(|drive| {
    //         let drive_type =
    //             unsafe { GetDriveTypeW(drive.encode_utf16().collect::<Vec<u16>>().as_ptr()) };
    //         drive_type == DRIVE_REMOVABLE
    //     })
    //     .collect();
    // println!("{}", drives.join("\n") + "\n");

    let drives_with_names: Vec<(String, String)> = buffer
        .split(|&c| c == 0)
        .filter(|s| !s.is_empty())
        .map(|s| OsString::from_wide(s).to_string_lossy().to_string())
        .filter(|drive| {
            let drive_type =
                unsafe { GetDriveTypeW(drive.encode_utf16().collect::<Vec<u16>>().as_ptr()) };
            drive_type == DRIVE_REMOVABLE
        })
        .map(|drive| {
            // Prepare buffer for volume name and file system name
            let mut volume_name = vec![0u16; 256];
            let mut file_system_name = vec![0u16; 256];

            // Convert drive path to wide string
            let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();

            // Retrieve volume name
            let volume_name_str = unsafe {
                let success = GetVolumeInformationW(
                    PCWSTR(drive_wide.as_ptr() as *const u16),
                    Some(&mut volume_name),
                    None,                        // Volume serial number (not used)
                    None,                        // Maximum component length (not used)
                    None,                        // File system flags (not used)
                    Some(&mut file_system_name), // File system name buffer
                );

                if success.as_bool() {
                    OsString::from_wide(
                        &volume_name[..volume_name
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(volume_name.len())],
                    )
                    .to_string_lossy()
                    .to_string()
                } else {
                    "Unknown".to_string()
                }
            };

            // Return a tuple of the drive and its volume name
            (drive, volume_name_str)
        })
        .collect();

    // Print results
    for (drive, volume_name) in &drives_with_names {
        println!("Drive: {}, Volume Name: {}", drive, volume_name);
    }
    // Now `drives_with_names` can still be used because it was not consumed
    drives_with_names
}

// fn elevate_privileges() -> Result<(), String> {
//     unsafe {
//         let mut token_handle: HANDLE = HANDLE(0);

//         // Open the process token
//         let result: BOOL = OpenProcessToken(
//             GetCurrentProcess(),
//             windows::Win32::Security::TOKEN_ADJUST_PRIVILEGES,
//             &mut token_handle,
//         );

//         if !result.as_bool() {
//             return Err("Failed to open process token.".to_string());
//         }

//         let mut privileges = TOKEN_PRIVILEGES {
//             PrivilegeCount: 1,
//             Privileges: [LUID_AND_ATTRIBUTES {
//                 Luid: Default::default(),
//                 Attributes: SE_PRIVILEGE_ENABLED,
//             }],
//         };

//         // Use the wide-character version of LookupPrivilegeValue
//         let luid_result = LookupPrivilegeValueW(
//             None,                             // Local system
//             windows::w!("SeBackupPrivilege"), // Wide string literal
//             &mut privileges.Privileges[0].Luid,
//         );

//         if !luid_result.as_bool() {
//             return Err("Failed to lookup privilege value for SeBackupPrivilege.".to_string());
//         }

//         privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

//         // Adjust the token privileges
//         let adjust_result =
//             AdjustTokenPrivileges(token_handle, false, Some(&privileges), 0, None, None);

//         if !adjust_result.as_bool() {
//             return Err("Failed to adjust token privileges.".to_string());
//         }

//         Ok(())
//     }
// }
