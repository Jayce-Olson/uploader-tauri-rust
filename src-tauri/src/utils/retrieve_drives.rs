use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use winapi::um::fileapi::{GetDriveTypeW, GetLogicalDriveStringsW};
use winapi::um::winbase::DRIVE_REMOVABLE;

use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

use windows::core::PCWSTR;

#[tauri::command]
#[cfg(target_os = "windows")]
pub fn list_devices() -> Vec<(String, String)> {
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
        return vec![(
            "Error retrieving drives.".to_string(),
            "Unknown".to_string(),
        )];
    }

    // Below looks like a tumor but it is just splitting the buffer into individual strings, filtering out empty strings, converting the slices to stirngs,
    // filters out drives that are not removable (removable drives), then uses another tumor to map over the current drives and retrieves the names for the drives,
    // and then finally collects a tuple of (drive, drive_name) to the drives vector.

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
