use std::fs::{self, File};
use std::io::{self, BufRead, ErrorKind};
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn copy_dir(src: String, dest: String) -> Result<(), String> {
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
                Err(_e) => {
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
        if !final_dest.exists() && final_dest.is_dir() {
            fs::create_dir_all(final_dest.clone())?;
        }
        println!("{}", final_dest.display());
        return Ok(final_dest);
    }
    return Err(io::Error::new(ErrorKind::NotFound, "File not found"));
    //Ok(dest.clone());
}
