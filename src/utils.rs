/*
 _   _ _   _ _ _ _   _           
| | | | |_(_) (_) |_(_) ___  ___ 
| | | | __| | | | __| |/ _ \/ __|
| |_| | |_| | | | |_| |  __/\__ \
 \___/ \__|_|_|_|\__|_|\___||___/

*/

// Standard imports
use std::result::Result;

// External crates
use crate::async_eprintln;
use crate::Path;
use rand::distributions::Alphanumeric;
use rand::Rng;
use sha1::{Digest, Sha1};
use tokio::fs::DirEntry;
use tokio::io::{self, AsyncReadExt};

/// Generates random 64 char string
pub async fn generate_hash() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let random_hash = tokio::task::spawn_blocking(|| {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect::<String>()
    })
    .await?;
    Ok(random_hash)
}

/// Returns Vec<String> of hashes in given path
pub async fn get_hashes(
    path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(_) => return Err(":: There are no mods yet".into()),
    };

    let mut hashes: Vec<String> = Vec::new();

    let mut tasks = vec![];

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            tasks.push(tokio::task::spawn(async move {
                let hash = calculate_sha1(&path).await;
                hash
            }));
        }
    }

    for task in tasks {
        match task.await {
            Ok(Ok(hash)) => hashes.push(hash),
            Ok(Err(e)) => async_eprintln!("Error processing hash: {e}").await,
            Err(e) => async_eprintln!("Task error: {e}").await,
        }
    }
    if hashes.is_empty() {
        return Err("No valid entries found to calculate hashes".into());
    }
    Ok(hashes)
}

/// Calculates hash of a file
pub async fn calculate_sha1<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = tokio::fs::File::open(&path).await?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Deletes files in folder with same hash
pub async fn remove_mods_by_hash(
    modsfolder: &str,
    hashes_to_remove: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut entries = tokio::fs::read_dir(modsfolder).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_hash = calculate_sha1(&path).await?;
            if hashes_to_remove.contains(&file_hash) {
                tokio::fs::remove_file(&path).await?;
            }
        }
    }

    Ok(())
}

/// Finds all .jar files in directory
pub async fn get_jar_filename(entry: &DirEntry) -> Option<String> {
    let path = entry.path();
    if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("jar") {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .map(String::from);
    }
    None
}
