/*
 ____                      _                 _
|  _ \  _____      ___ __ | | ___   __ _  __| | ___ _ __
| | | |/ _ \ \ /\ / / '_ \| |/ _ \ / _` |/ _` |/ _ \ '__|
| |_| | (_) \ V  V /| | | | | (_) | (_| | (_| |  __/ |
|____/ \___/ \_/\_/ |_| |_|_|\___/ \__,_|\__,_|\___|_|

*/

// Internal modules
use crate::Anymod;
use crate::consts::USER_AGENT;
use crate::structs::WorkingProfile;

// Standard imports
use std::path::Path;
use std::result::Result;

// External crates
use futures::future::join_all;
use std::sync::Arc;
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;

/// Downloads a single file
pub async fn download_mod(
    anymod: &Anymod,
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a destination directory if it doesn't exist
    create_dir_all(&working_profile.profile.modsfolder).await?;

    // Create a file path
    let path = Path::new(&working_profile.profile.modsfolder).join(&anymod.filename);

    // Send the download request
    let mut response = working_profile
        .client
        .get(&anymod.url)
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
pub async fn download_multiple_mods(
    files: Vec<Anymod>,
    working_profile: Arc<WorkingProfile>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<JoinHandle<Result<(), ()>>> = vec![];

    for file in files {
        let wp = working_profile.clone();
        tasks.push(tokio::spawn(async move {
            match download_mod(&file, &wp).await {
                Ok(_) => {}
                Err(e) => eprintln!(":err: {e}"),
            }
            Ok(())
        }));
    }

    join_all(tasks).await;

    // Success
    Ok(())
}
