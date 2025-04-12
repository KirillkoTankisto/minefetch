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

use reqwest::Client;
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
    add_lock, create_profile, delete_all_profiles, delete_profile, list_locks, list_profiles,
    read_config, remove_lock, switch_profile,
};
use structs::*;
use utils::get_jar_filename;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match initialise().await {
        Ok(_) => (),
        Err(error) => async_eprintln!(":err: {error}").await,
    };
    Ok(())
}

/// The start of the main async function
async fn initialise() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("add") => match args.get(2).map(String::as_str) {
            Some(modname) => {
                async_println!(":: Adding mod...").await;

                let profile: Profile = read_config().await?;

                let params = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[&profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[&profile.gameversion])?,
                    ),
                ];

                let client = reqwest::Client::new();

                let modversion =
                    fetch_latest_version(&modname.to_string(), &client, &params, &profile).await?;

                download_file(&profile.modsfolder, &modversion.0, &modversion.1, &client).await?;

                async_println!(":: Downloaded {} ({})", &modname, &modversion.0).await;

                match modversion.2 {
                    Some(dep) => {
                        let dependencies = get_dependencies(&dep, &client).await?;
                        for dependency in dependencies {
                            async_println!(":deps: {} {}", dependency.0, dependency.1).await;
                        }
                    }
                    None => {}
                }
            }

            _ => async_println!(":: Usage: minefetch add <modname>").await,
        },

        Some("profile") => match args.get(2).map(String::as_str) {
            Some("create") => create_profile().await?,

            Some("delete") => match args.get(3).map(String::as_str) {
                Some("all") => delete_all_profiles().await?,
                _ => delete_profile().await?,
            },

            Some("switch") => switch_profile().await?,

            Some("list") => list_profiles().await?,

            _ => {
                async_eprintln!(
                    ":: Usage: minefetch profile < create | delete | delete all | switch | list >"
                )
                .await
            }
        },

        Some("version") => async_println!(":: {} {}", NAME, PROGRAM_VERSION).await,

        Some("search") => match args.get(2) {
            Some(_) => {
                let query = args[2..].join(" ");

                let profile: Profile = read_config().await?;

                let facets = json!([
                    [format!("categories:{}", profile.loader)],
                    [format!("versions:{}", profile.gameversion)],
                    ["project_type:mod"],
                ]);

                let fetch_params: Vec<(String, String)> = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[&profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[&profile.gameversion])?,
                    ),
                ];

                let client = reqwest::Client::new();

                let files = search_mods(&query, facets, &client, &fetch_params, &profile).await?;

                download_multiple_files(files, &profile.modsfolder, &client).await?;
            }

            None => async_println!(":: Usage: minefetch search <query>").await,
        },

        Some("upgrade") | Some("update") => {
            let profile: Profile = read_config().await?;

            let files = upgrade_mods(&profile).await?;

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
                    let mut counter: usize = 1;
                    for (_, version) in versions {
                        async_println!(
                            "[{}] {} ({})",
                            counter,
                            format!("\x1b[{}\x1b[0m", version.name),
                            version
                                .files
                                .iter()
                                .find(|file| file.primary)
                                .ok_or("No primary file found")
                                .unwrap()
                                .filename
                        )
                        .await;
                        counter += 1
                    }
                }
                Err(error) => {
                    async_eprintln!(":err: {}", error).await;
                    let path = Path::new(&profile.modsfolder);
                    let mut entries = tokio::fs::read_dir(path).await?;
                    let mut counter: usize = 1;
                    while let Some(entry) = entries.next_entry().await? {
                        if let Some(path) = get_jar_filename(&entry).await {
                            async_println!("[{}] {}", counter, path).await;
                            counter += 1;
                        }
                    }
                }
            };
        }

        Some("lock") => match args.get(2).map(String::as_str) {
            Some("add") => {
                let client = Client::new();
                let profile = read_config().await?;
                add_lock(&profile, &client).await?;
            }

            Some("remove") => {
                let client = Client::new();
                let profile = read_config().await?;
                remove_lock(&profile, &client).await?;
            }

            Some("list") => {
                let client = Client::new();
                let profile = read_config().await?;
                let locks = list_locks(&client, &profile).await?;
                for (size, name, filename) in locks {
                    async_println!("[{}] {} ({})", size, name, filename).await;
                }
            }

            Some(_) => async_println!(":: Usage: minefetch lock < add | remove | list >").await,
            None => (),
        },

        Some(_) => async_println!(":: There is no such command!").await,

        None => async_println!(":: No arguments provided").await,
    }
    Ok(())
}
