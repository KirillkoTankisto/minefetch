/*
 ____                      _                 _
|  _ \  _____      ___ __ | | ___   __ _  __| | ___ _ __
| | | |/ _ \ \ /\ / / '_ \| |/ _ \ / _` |/ _` |/ _ \ '__|
| |_| | (_) \ V  V /| | | | | (_) | (_| | (_| |  __/ |
|____/ \___/ \_/\_/ |_| |_|_|\___/ \__,_|\__,_|\___|_|

*/

// Internal modules
use crate::api::Anymod;
use crate::consts::USER_AGENT;
use crate::structs::WorkingProfile;

// Standard imports
use std::path::Path;
use std::result::Result;
use std::sync::Arc;

// External crates
use futures::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::fs::create_dir_all;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;

/// Downloads a single file
pub async fn download_mod(
    anymod: &Anymod,
    working_profile: &WorkingProfile,
    bar: ProgressBar,
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

    let total = response.content_length().ok_or("But nobody came")?;

    bar.set_length(total);

    // Create a file
    let mut file = tokio::fs::File::create(path).await?;

    // Write into file gradually
    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
        bar.inc(chunk.len() as u64);
    }

    bar.finish();

    // Success
    Ok(())
}

/// Downloads multiple files
pub async fn download_multiple_mods(
    files: Vec<Anymod>,
    working_profile: Arc<WorkingProfile>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks: Vec<JoinHandle<Result<(), ()>>> = Vec::with_capacity(files.len());

    let multibar = Arc::new(MultiProgress::new());
    let style =
        ProgressStyle::with_template("{wide_msg} {bar:50} {percent}%").expect("valid template");

    for file in files {
        let wp = working_profile.clone();
        let mb = multibar.clone();
        let st = style.clone();
        tasks.push(tokio::spawn(async move {
            let bar = mb.add(ProgressBar::new(0));
            bar.set_style(st);
            bar.set_message(file.clone().title.unwrap_or(file.filename.clone()));
            match download_mod(&file, &wp, bar).await {
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
