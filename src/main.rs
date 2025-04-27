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

// External crates
use reqwest::Client;
use serde_json::json;

// Internal modules
mod api;
mod consts;
mod coreutils;
mod downloader;
mod helpmsg;
mod mfio;
mod profile;
mod structs;
mod utils;
use api::*;
use consts::*;
use downloader::*;
use helpmsg::display_help_msg;
use mfio::{MFText, select};
use profile::{
    add_lock, create_profile, delete_all_profiles, delete_profile, list_locks, list_profiles,
    read_config, remove_lock, switch_profile,
};
use structs::*;
use utils::{filename_from_url, get_jar_filename, remove_mods_by_hash};

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
                async_println!(":out: Adding a mod...").await;

                let profile: Profile = read_config().await?;

                let params = &[
                    ("loaders", &serde_json::to_string(&[&profile.loader])?),
                    (
                        "game_versions",
                        &serde_json::to_string(&[&profile.gameversion])?,
                    ),
                ];

                let client = Client::new();

                let modversion =
                    fetch_latest_version(&modname.to_string(), &client, params, &profile).await?;

                download_file(&profile.modsfolder, &modversion.0, &modversion.1, &client).await?;

                async_println!(":out: Downloaded {} ({})", &modname, &modversion.0).await;

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

            _ => async_println!(":out: Usage: minefetch add <modname>").await,
        },

        Some("profile") => match args.get(2).map(String::as_str) {
            Some("create") => create_profile().await?,

            Some("delete") => match args.get(3).map(String::as_str) {
                Some("all") => delete_all_profiles().await?,
                _ => delete_profile().await?,
            },

            Some("switch") => switch_profile().await?,

            Some("list") => list_profiles().await?,

            _ => async_eprintln!(
                ":out: Usage: minefetch profile < create | delete | delete all | switch | list >"
            )
            .await,
        },

        Some("version") => async_println!(":out: {} {}", NAME, PROGRAM_VERSION).await,

        Some("search") => match args.get(2) {
            Some(_) => {
                let query = args[2..].join(" ");

                let profile: Profile = read_config().await?;

                let facets = json!([
                    [format!("categories:{}", profile.loader)],
                    [format!("versions:{}", profile.gameversion)],
                    ["project_type:mod"],
                ]);

                let params = &[
                    ("loaders", &serde_json::to_string(&[&profile.loader])?),
                    (
                        "game_versions",
                        &serde_json::to_string(&[&profile.gameversion])?,
                    ),
                ];

                let client = Client::new();

                let files = search_mods(&query, facets, &client, params, &profile).await?;

                download_multiple_files(files, &profile.modsfolder, &client).await?;
            }

            None => async_println!(":out: Usage: minefetch search <query>").await,
        },

        Some("upgrade") | Some("update") => {
            let profile = read_config().await?;
            let client = Client::new();

            let files = upgrade_mods(&profile, &client).await?;

            let client = Client::new();

            download_multiple_files(files, &profile.modsfolder, &client).await?;
        }

        Some("list") => {
            let profile: Profile = read_config().await?;
            let client = Client::new();

            match list_mods(&profile, &client).await {
                Ok((size, versions)) => {
                    if size == 0 {
                        return Err("There are no mods yet".into());
                    }
                    async_println!(
                        ":out: There are \x1b[1;97m{}\x1b[0m mods in profile {}:",
                        size,
                        profile.name
                    )
                    .await;
                    let mut counter: usize = 1;
                    for (_, version) in versions {
                        async_println!(
                            "[{}{}{}] {}{}{} ({})",
                            MFText::Bold,
                            counter,
                            MFText::Reset,
                            MFText::Bold,
                            version.name,
                            MFText::Reset,
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
                    async_println!(
                        "[{}{}{}] {}{}{} ({})",
                        MFText::Bold,
                        size,
                        MFText::Reset,
                        MFText::Bold,
                        name,
                        MFText::Reset,
                        filename
                    )
                    .await;
                }
            }

            Some(_) => async_println!(":out: Usage: minefetch lock < add | remove | list >").await,
            None => (),
        },

        Some("help") => {
            display_help_msg(&HELP_MESSAGE).await;
        }

        Some("debug") => {
            println!(":dbg: {} {} {}", NAME, PROGRAM_VERSION, USER_AGENT);
        }

        Some("edit") => {
            let client = Client::new();
            let profile = read_config().await?;

            let modlist = list_mods(&profile, &client).await?;
            let mut menu: Vec<(String, Version)> = Vec::new();

            for modification in modlist.1 {
                menu.push((
                    format!(
                        "{} ({})",
                        modification.1.name,
                        modification
                            .1
                            .files
                            .iter()
                            .find(|file| file.primary)
                            .unwrap()
                            .filename
                    ),
                    modification.1,
                ));
            }

            let mod_to_edit = select("Select a mod to edit", menu).await?;

            let params = &[
                ("loaders", &serde_json::to_string(&[&profile.loader])?),
                (
                    "game_versions",
                    &serde_json::to_string(&[&profile.gameversion])?,
                ),
            ];

            let url = reqwest::Url::parse_with_params(
                format!(
                    "https://api.modrinth.com/v2/project/{}/version",
                    mod_to_edit.project_id
                )
                .as_str(),
                params,
            )?;

            let response = client
                .get(url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await?
                .text()
                .await?;

            let parsed: VersionsList = serde_json::from_str(&response)?;

            let mut versions_to_install: Vec<(String, String)> = Vec::new();

            for version in parsed {
                versions_to_install.push((
                    version.name,
                    version
                        .files
                        .iter()
                        .find(|file| file.primary)
                        .unwrap()
                        .url
                        .clone(),
                ));
            }

            let version_to_install =
                select("Choose a version to install", versions_to_install).await?;

            let filename = filename_from_url(&version_to_install);

            download_file(&profile.modsfolder, filename, &version_to_install, &client).await?;

            async_println!(":out: Downloaded {filename}").await;

            remove_mods_by_hash(
                &profile.modsfolder,
                &vec![
                    &mod_to_edit
                        .files
                        .iter()
                        .find(|file| file.primary)
                        .unwrap()
                        .hashes
                        .sha1,
                ],
            )
            .await?;

            async_println!(
                ":out: Deleted {}",
                &mod_to_edit
                    .files
                    .iter()
                    .find(|file| file.primary)
                    .unwrap()
                    .filename
            )
            .await
        }

        Some(_) => {
            async_println!(":out: There is no such command! Type 'minefetch help' for help message")
                .await
        }

        None => async_println!(":out: No arguments provided").await,
    }
    Ok(())
}
