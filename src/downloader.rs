/*
 ____                      _                 _
|  _ \  _____      ___ __ | | ___   __ _  __| | ___ _ __
| | | |/ _ \ \ /\ / / '_ \| |/ _ \ / _` |/ _` |/ _ \ '__|
| |_| | (_) \ V  V /| | | | | (_) | (_| | (_| |  __/ |
|____/ \___/ \_/\_/ |_| |_|_|\___/ \__,_|\__,_|\___|_|

*/

// Internal modules
use crate::consts::USER_AGENT;
use crate::{Dependency, get_dependencies};

// Standard imports
use std::path::{Path, PathBuf};
use std::result::Result;

// External crates
use reqwest::Client;
use tokio::io::AsyncWriteExt;

/// Downloads a single file
pub async fn download_file(
    path: &str,
    filename: &str,
    url: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a destination directory if it doesn't exist
    tokio::fs::create_dir_all(path).await?;

    // Create a file path
    let path = std::path::Path::new(path).join(&filename);

    // Send the download request
    let mut response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    // Create a file
    let mut file = tokio::fs::File::create(path).await?;

    // Write into file gradually
    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
    }

    // Success
    Ok(())
}

/// Downloads multiple files
pub async fn download_multiple_files(
    files: Vec<(String, String, Option<Vec<Dependency>>)>,
    path: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    // Tasks' list
    let mut handles = Vec::new();

    // Destination path
    let base_path = Path::new(path);

    // Going through all files that must be downloaded
    for (filename, url, dependencies) in files {
        // Copy the client
        let client_download = client.clone();
        let client_dependencies = client.clone();

        // Get the path
        let sanitized_path = PathBuf::from(base_path);

        // Check if this path safe (inside base_path)
        if !sanitized_path.starts_with(base_path) {
            eprintln!(
                ":err: Potential path traversal attack detected: {:?}",
                sanitized_path
            );
            continue;
        }

        // Create a task
        let handle = tokio::spawn(async move {
            // Print the text
            println!(":out: Downloading {}", &filename);

            // Convert path to &str
            let path_str = match sanitized_path.to_str() {
                Some(path) => path,
                None => {
                    eprintln!(":err: Invalid UTF-8 path for {}", filename);
                    return; // Exit the task early
                }
            };

            // Download a file
            match download_file(path_str, &filename, &url, &client_download).await {
                Ok(_) => {}
                Err(error) => {
                    eprintln!(":err: Failed to download {}: {}", filename, error)
                }
            }
        });

        // Check if the mod has any dependencies
        match dependencies {
            Some(dep) => {
                // Get a list of dependencies
                let list = get_dependencies(&dep, &client_dependencies).await?;

                // Print the list
                for dependency in list {
                    println!(":dep: {} {}", dependency.0, dependency.1);
                }
            }
            None => {}
        }

        // Append to the tasks' list
        handles.push(handle);
    }

    // Execute the tasks
    for handle in handles {
        if let Err(error) = handle.await {
            eprintln!(":err: Task panicked: {:?}", error);
        }
    }

    // Success
    Ok(())
}
