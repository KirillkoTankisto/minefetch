/*
 __  __       _         _____                 _   _
|  \/  | __ _(_)_ __   |  ___|   _ _ __   ___| |_(_) ___  _ __
| |\/| |/ _` | | '_ \  | |_ | | | | '_ \ / __| __| |/ _ \| '_ \
| |  | | (_| | | | | | |  _|| |_| | | | | (__| |_| | (_) | | | |
|_|  |_|\__,_|_|_| |_| |_|   \__,_|_| |_|\___|\__|_|\___/|_| |_|

*/

// Standard imports
use std::path::Path;
use std::result::Result;
use std::vec;

// External crates
use serde_json::json;

// Internal modules
mod api;
mod consts;
mod downloader;
mod mfio;
mod profile;
mod structs;
mod utils;
use api::*;
use consts::*;
use downloader::*;
use profile::{
    create_profile, delete_all_profiles, delete_profile, list_profiles, read_config, switch_profile,
};
use structs::*;
use utils::get_jar_filename;

/// The start of the main async function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("add") => match args.get(2).map(String::as_str) {
            Some(s) => {
                async_println!(":: Adding mod...").await;

                let profile: Profile = match read_config().await {
                    Ok(profile) => profile,
                    Err(e) => return Err(e),
                };

                let params = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[profile.gameversion])?,
                    ),
                ];

                let client = reqwest::Client::new();

                let modversion = match fetch_latest_version(&s.to_string(), &client, &params).await
                {
                    Ok(modversion) => modversion,
                    Err(e) => {
                        async_eprintln!(":err: {e}").await;
                        return Ok(());
                    }
                };

                download_file(&profile.modsfolder, &modversion.0, &modversion.1, &client).await?;

                async_println!(":: Downloaded {} ({})", &s, &modversion.0).await;

                match modversion.2 {
                    Some(dep) => {
                        let list = get_dependencies(&dep, &client).await?;
                        for i in list {
                            async_println!(":deps: {} {}", i.0, i.1).await;
                        }
                    }
                    None => {}
                }
            }

            _ => async_println!(":: Usage: minefetch add <modname>").await,
        },

        Some("profile") => match args.get(2).map(String::as_str) {
            Some("create") => match create_profile().await {
                Ok(()) => (),
                Err(e) => async_eprintln!(":err: {e}").await,
            },

            Some("delete") => match args.get(3).map(String::as_str) {
                Some("all") => match delete_all_profiles().await {
                    Ok(()) => (),
                    Err(e) => async_eprintln!(":err: {e}").await,
                },
                _ => match delete_profile().await {
                    Ok(()) => (),
                    Err(e) => async_eprintln!(":err: {e}").await,
                },
            },

            Some("switch") => match switch_profile().await {
                Ok(()) => (),
                Err(e) => async_eprintln!(":err: {e}").await,
            },

            Some("list") => match list_profiles().await {
                Ok(()) => (),
                Err(e) => async_eprintln!(":err: {e}").await,
            },

            _ => {
                async_eprintln!(
                    ":: Usage: minefetch profile <create|delete|delete all|switch|list>"
                )
                .await
            }
        },

        Some("version") => async_println!(":: {} {}", NAME, PROGRAM_VERSION).await,

        Some("search") => match args.get(2) {
            Some(_) => {
                let query = args[2..].join(" ");

                let profile: Profile = match read_config().await {
                    Ok(profile) => profile,
                    Err(e) => {
                        async_eprintln!("{}", e).await;
                        return Ok(());
                    }
                };

                let facets = json!([
                    [format!("categories:{}", profile.loader)],
                    [format!("versions:{}", profile.gameversion)],
                    ["project_type:mod"],
                ]);

                let fetch_params: Vec<(String, String)> = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[profile.gameversion])?,
                    ),
                ];

                let client = reqwest::Client::new();

                let files = match search_mods(&query, facets, &client, &fetch_params).await {
                    Ok(files) => files,
                    Err(e) => {
                        async_eprintln!("{}", e).await;
                        return Ok(());
                    }
                };

                download_multiple_files(files, &profile.modsfolder, &client).await?;
            }

            None => async_println!(":: Usage: minefetch search <query>").await,
        },

        Some("upgrade") | Some("update") => {
            let profile: Profile = read_config().await?;
            let files = match upgrade_mods(&profile).await {
                Ok(files) => files,
                Err(e) => {
                    async_eprintln!("{}", e).await;
                    return Ok(());
                }
            };
            let client = reqwest::Client::new();
            download_multiple_files(files, &profile.modsfolder, &client).await?;
        }

        Some("list") => {
            let profile: Profile = read_config().await?;
            let client = reqwest::Client::new();
            match list_mods(&profile, &client).await {
                Ok((size, versions)) => {
                    if size == 0 {
                        return Err(":: There are no mods yet".into());
                    }
                    async_println!(
                        ":: There are \x1b[1;97m{}\x1b[0m mods in profile {}:",
                        size,
                        profile.name
                    )
                    .await;
                    let mut a: u32 = 1;
                    for (_, i) in versions {
                        async_println!(
                            "[{}] {} ({})",
                            a,
                            format!("\x1b[1;97m{}\x1b[0m", i.name),
                            i.files
                                .iter()
                                .find(|file| file.primary)
                                .ok_or(":err: No primary file found")
                                .unwrap()
                                .filename
                        )
                        .await;
                        a += 1
                    }
                }
                Err(e) => {
                    async_eprintln!("{}", e).await;
                    let path = Path::new(&profile.modsfolder);
                    let mut entries = tokio::fs::read_dir(path).await?;
                    let mut i = 1;
                    while let Some(entry) = entries.next_entry().await? {
                        if let Some(path) = get_jar_filename(&entry).await {
                            async_println!("[{}] {}", i, path).await;
                            i += 1;
                        }
                    }
                }
            };
        }

        Some(_) => async_println!(":: There is no such command!").await,

        None => async_println!(":: No arguments provided").await,
    }
    Ok(())
}
