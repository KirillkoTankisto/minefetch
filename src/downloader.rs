// Standard imports
use crate::{async_eprintln, async_println, get_dependencies, Dependency};
use std::path::{Path, PathBuf};
use std::result::Result;
// External crates

use tokio::io::AsyncWriteExt;

/// Downloads single file
pub async fn download_file(
    path: &str,
    filename: &str,
    url: &str,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::fs::create_dir_all(path).await?;

    let path = std::path::Path::new(path).join(&filename);

    let mut response = client.get(url).send().await?;

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
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    let base_path = Path::new(path);

    for (filename, url, deps) in files {
        let client = client.clone();

        let sanitized_path = PathBuf::from(base_path);

        if !sanitized_path.starts_with(base_path) {
            async_eprintln!(
                "Potential path traversal attack detected: {:?}",
                sanitized_path
            )
            .await;
            continue;
        }

        let handle = tokio::spawn(async move {
            async_println!(":: Downloading {}", &filename).await;
            let path_str = match sanitized_path.to_str() {
                Some(s) => s,
                None => {
                    async_eprintln!(":err: Invalid UTF-8 path for {}", filename).await;
                    return; // Exit the task early
                }
            };
            match download_file(path_str, &filename, &url, &client).await {
                Ok(_) => {}
                Err(e) => async_eprintln!(":err: Failed to download {}: {}", filename, e).await,
            }
        });

        match deps {
            Some(dep) => {
                let list = get_dependencies(&dep).await?;
                for i in list {
                    async_println!(":: {} {}", i.0, i.1).await;
                }
            }
            None => {}
        }

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            async_eprintln!("Task panicked: {:?}", e).await;
        }
    }

    Ok(())
}
