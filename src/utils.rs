/*
 _   _ _   _ _ _ _   _
| | | | |_(_) (_) |_(_) ___  ___
| | | | __| | | | __| |/ _ \/ __|
| |_| | |_| | | | |_| |  __/\__ \
 \___/ \__|_|_|_|\__|_|\___||___/

*/

// Standard imports
use std::path::PathBuf;
use std::result::Result;

// Internal modules
use crate::Path;
use crate::async_eprintln;

// External crates
use rand::Rng;
use rand::distr::Alphanumeric;
use sha1::{Digest, Sha1};
use tokio::fs::DirEntry;
use tokio::io::{self, AsyncReadExt};
use uu_whoami::whoami;

/// Generates random 64 char string
pub async fn generate_hash() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Get a random hash
    let random_hash = tokio::task::spawn_blocking(|| {
        rand::rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect::<String>()
    })
    .await?;

    // Return it
    Ok(random_hash)
}

/// Returns Vec<String> of hashes in given path
pub async fn get_hashes(
    path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Get files from path
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(_) => return Err(":out: There are no mods yet".into()),
    };

    // Create a hash list
    let mut hashes: Vec<String> = Vec::new();

    // Create tasks list
    let mut tasks = vec![];

    // Go through every object in path
    while let Some(entry) = entries.next_entry().await? {
        // Get a path to the file / folder
        let path = entry.path();

        // If it's a file
        if path.is_file() {
            // Add a task
            tasks.push(tokio::task::spawn(
                async move { calculate_sha1(&path).await },
            ));
        }
    }

    // Join tasks
    for task in tasks {
        match task.await {
            Ok(Ok(hash)) => hashes.push(hash),

            Ok(Err(error)) => async_eprintln!("Error processing hash: {error}").await,

            Err(error) => async_eprintln!("Task error: {error}").await,
        }
    }

    // If there're no files
    if hashes.is_empty() {
        return Err("No valid entries found to calculate hashes".into());
    }

    // Return hashes
    Ok(hashes)
}

/// Calculates hash for a file
pub async fn calculate_sha1<P: AsRef<Path>>(path: P) -> io::Result<String> {
    // Open the file
    let mut file = tokio::fs::File::open(&path).await?;

    // Create a hasher
    let mut hasher = Sha1::new();

    // Create a buffer
    let mut buffer = vec![0; 8192];

    /*
        A loop which reads bytes from the
        file and appeands them to the hasher
    */
    loop {
        // Read to the buffer and count bytes count
        let bytes_read = file.read(&mut buffer).await?;

        // If bytes count equals zero
        if bytes_read == 0 {
            break;
        }

        // Update the hasher
        hasher.update(&buffer[..bytes_read]);
    }

    // Return result
    Ok(format!("{:x}", hasher.finalize()))
}

/// Deletes files in folder with same hash
pub async fn remove_mods_by_hash(
    modsfolder: &str,
    hashes_to_remove: &Vec<&String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read mods' folder
    let mut entries = tokio::fs::read_dir(modsfolder).await?;

    // Go through every file in the folder
    while let Some(entry) = entries.next_entry().await? {
        // Get a path to the file / folder
        let path = entry.path();

        // If it's a file
        if path.is_file() {
            // Get a hash
            let file_hash = calculate_sha1(&path).await?;

            // If the hash in the hash list then remove a file
            if hashes_to_remove.contains(&&file_hash) {
                tokio::fs::remove_file(&path).await?;
            }
        }
    }

    // Success
    Ok(())
}

/// Gets a filename of the file if it's a .jar file
pub async fn get_jar_filename(entry: &DirEntry) -> Option<String> {
    // Get a path
    let path = entry.path();

    // If it's a file and it has a .jar extension then return a String
    if path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("jar") {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from);
    }

    // If not then return nothing
    None
}

/// Gets a home folder (Not sure if it works for windows)
pub async fn get_homedir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Get the username
    let username = whoami().expect("Couldn't get the username");

    // Join with the base home directory depending on the OS
    #[cfg(any(target_os = "linux",target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
    let homedir = PathBuf::from("/home/").join(username);

    #[cfg(target_os = "macos")]
    let homedir = PathBuf::from("/Users/").join(username);

    #[cfg(target_os = "windows")]
    let homedir = PathBuf::from("C:\\Users\\").join(username);

    // Return the home directory
    Ok(homedir)
}

/// Gets a config path
pub async fn get_confpath() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Get a home folder
    let homedir = get_homedir().await?;

    /*
        Join the home folder with
        the config location and return
    */
    Ok(homedir
        .join(".config")
        .join("minefetch")
        .join("config.toml"))
}

/// Gets a config directory
pub async fn get_confdir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // Get a home folder
    let homedir = get_homedir().await?;

    /*
        Join the home folder with the
        config folder location and return
    */
    Ok(homedir.join(".config").join("minefetch"))
}
