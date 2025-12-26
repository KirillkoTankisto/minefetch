/*
 _   _ _   _ _ _ _   _
| | | | |_(_) (_) |_(_) ___  ___
| | | | __| | | | __| |/ _ \/ __|
| |_| | |_| | | | |_| |  __/\__ \
 \___/ \__|_|_|_|\__|_|\___||___/

*/

// Standard imports
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;

// External crates
use rand::Rng;
use rand::distr::Alphanumeric;
use sha1::{Digest, Sha1};
use tokio::task::spawn_blocking;

/// Generates random 64 char string
pub async fn generate_hash() -> Result<String, Box<dyn std::error::Error>> {
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
pub async fn get_hashes(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut dir = tokio::fs::read_dir(path)
        .await
        .map_err(|_| "There's no mods yet")?;

    let mut paths: Vec<PathBuf> = Vec::new();

    while let Some(entry) = dir.next_entry().await? {
        if entry.file_type().await?.is_file() {
            paths.push(entry.path());
        }
    }

    let mut handles = Vec::with_capacity(paths.len());

    for p in paths {
        if p.extension().unwrap_or_default() == "jar" {
            handles.push(spawn_blocking(move || calculate_sha1(&p)));
        }
    }

    if handles.is_empty() {
        return Ok(Vec::new());
    }

    // Collect results
    let mut hashes = Vec::with_capacity(handles.len());

    for handle in handles {
        match handle.await {
            Ok(Ok(hash)) => hashes.push(hash),
            Ok(Err(e)) => eprintln!("Error processing hash: {e}"),
            Err(e) => eprintln!("Task join error: {e}"),
        }
    }

    if hashes.is_empty() {
        return Err("No valid entries found to calculate hashes".into());
    }

    Ok(hashes)
}

/// Synchronous SHA-1 calculation using a buffered reader (used inside spawn_blocking)
fn calculate_sha1(path: &Path) -> std::io::Result<String> {
    let f = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, f); // 64 KiB buffer
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    // `finalize()`/format as hex — matches your original formatting.
    Ok(format!("{:x}", hasher.finalize()))
}

/// Deletes files in folder with same hash
pub async fn remove_mods_by_hash(
    modsfolder: &str,
    hashes_to_remove: &Vec<&String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read mods' folder
    let mut entries = tokio::fs::read_dir(modsfolder).await?;

    // Go through every file in the folder
    while let Some(entry) = entries.next_entry().await? {
        // Get a path to the file / folder
        let path = entry.path();

        // If it's a file
        if path.is_file() {
            // Get a hash
            let file_hash = calculate_sha1(&path)?;

            // If the hash in the hash list then remove a file
            if hashes_to_remove.contains(&&file_hash) {
                tokio::fs::remove_file(&path).await?;
            }
        }
    }

    // Success
    Ok(())
}

/// Gets a home folder (Not sure if it works for windows)
pub async fn get_homedir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let homedir = env::home_dir().ok_or("Can't get the home directory")?;

    Ok(homedir)
}

/// Gets a config path
pub async fn get_confpath() -> Result<PathBuf, Box<dyn std::error::Error>> {
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
pub async fn get_confdir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Get a home folder
    let homedir = get_homedir().await?;

    /*
        Join the home folder with the
        config folder location and return
    */
    Ok(homedir.join(".config").join("minefetch"))
}
