/*
 ____                      _                 _
|  _ \  _____      ___ __ | | ___   __ _  __| | ___ _ __
| | | |/ _ \ \ /\ / / '_ \| |/ _ \ / _` |/ _` |/ _ \ '__|
| |_| | (_) \ V  V /| | | | | (_) | (_| | (_| |  __/ |
|____/ \___/ \_/\_/ |_| |_|_|\___/ \__,_|\__,_|\___|_|

*/

// Internal modules
use crate::consts::USER_AGENT;
use crate::{Dependency, async_eprintln, async_println, get_dependencies};

// Standard imports
use std::path::{Path, PathBuf};
use std::result::Result;

// External crates
use reqwest::Client;
use tokio::io::AsyncWriteExt;

/// Downloads single file
pub async fn download_file(
    path: &str,
    filename: &str,
    url: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::fs::create_dir_all(path).await?;

    let path = std::path::Path::new(path).join(&filename);

    let mut response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?;

    let mut file = tokio::fs::File::create(path).await?;

    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
    }

    Ok(())
}

/// Downloads multiple files
pub async fn download_multiple_files(
    files: Vec<(String, String, Option<Vec<Dependency>>)>,
    path: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    let base_path = Path::new(path);

    for (filename, url, deps) in files {
        let client = client.clone();

        let sanitized_path = PathBuf::from(base_path);

        if !sanitized_path.starts_with(base_path) {
            async_eprintln!(
                ":err: Potential path traversal attack detected: {:?}",
                sanitized_path
            )
            .await;
            continue;
        }

        let handle = tokio::spawn(async move {
            async_println!(":out: Downloading {}", &filename).await;
            let path_str = match sanitized_path.to_str() {
                Some(path) => path,
                None => {
                    async_eprintln!(":err: Invalid UTF-8 path for {}", filename).await;
                    return; // Exit the task early
                }
            };
            match download_file(path_str, &filename, &url, &client).await {
                Ok(_) => {}
                Err(error) => {
                    async_eprintln!(":err: Failed to download {}: {}", filename, error).await
                }
            }
        });

        let client = Client::new();

        match deps {
            Some(dep) => {
                let list = get_dependencies(&dep, &client).await?;
                for dependency in list {
                    async_println!(":dep: {} {}", dependency.0, dependency.1).await;
                }
            }
            None => {}
        }

        handles.push(handle);
    }

    for handle in handles {
        if let Err(error) = handle.await {
            async_eprintln!(":err: Task panicked: {:?}", error).await;
        }
    }

    Ok(())
}
