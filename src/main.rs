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
use mfio::MFText;
use profile::{
    add_lock, build_working_profile, create_profile, delete_all_profiles, delete_profile,
    list_locks, list_profiles, remove_lock, switch_profile,
};
use structs::*;
use utils::get_jar_filename;

#[tokio::main]
// The start of the main function
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Calling initialise to start the program
    match initialise().await {
        Ok(_) => (),
        Err(error) => async_eprintln!(":err: {error}").await,
    };

    // Exit
    Ok(())
}

/// The start of the main async function
async fn initialise() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read the commandline arguments
    let args: Vec<String> = std::env::args().collect();

    // Get the first argument
    match args.get(1).map(String::as_str) {
        // minefetch add %mod_id_or_slug%
        Some("add") => match args.get(2).map(String::as_str) // Get the mod id / slug
        { 
            Some(modname) => {
                // Print text
                async_println!(":out: Adding a mod...").await;

                // Create a working profile
                let working_profile = build_working_profile().await?;

                // Get the latest version
                let mod_version =
                    fetch_latest_version(&modname.to_string(), &working_profile).await?;

                // Download this version
                download_file(
                    &working_profile.profile.modsfolder,
                    &mod_version.0,
                    &mod_version.1,
                    &working_profile.client,
                )
                .await?;

                // Print text
                async_println!(":out: Downloaded {} ({})", &modname, &mod_version.0).await;

                // Check for existing dependencies
                match mod_version.2 {
                    Some(dependencies) => {
                        // Get the dependencies' info
                        let dependencies =
                            get_dependencies(&dependencies, &working_profile.client).await?;
                        
                        // Print all existing dependencies: their names and types (required or optional)
                        for dependency in dependencies {
                            async_println!(":dep: {} {}", dependency.0, dependency.1).await;
                        }
                    }
                    None => {}
                }
            }

            // If the prompt is empty
            _ => display_help_msg(&HELP_MESSAGE).await,
        },

        // minefetch profile %subcommand%
        Some("profile") => match args.get(2).map(String::as_str) {
            // minefetch profile create
            Some("create") => create_profile().await?,

            // minefetch profile delete (all or only one) 
            Some("delete") => match args.get(3).map(String::as_str) {
                // minefetch profile delete all
                Some("all") => delete_all_profiles().await?,
                
                // minefetch profile delete
                _ => delete_profile().await?,
            },

            // minefetch profile switch
            Some("switch") => switch_profile().await?,

            // minefetch profile list
            Some("list") => list_profiles().await?,

            // If the prompt is empty
            _ => display_help_msg(&HELP_MESSAGE).await,
        },

        // minefetch version
        Some("version") => async_println!(":out: {} {}", NAME, PROGRAM_VERSION).await,

        // minefetch search %query%
        Some("search") => match args.get(2) {
            /*
            Doesn't use the value from args because
            program joins all the strings starting
            from the third argument
            */
            Some(_) => {
                // Join all the strings to form a query
                let query = args[2..].join(" ");

                // Create a working profile
                let working_profile = build_working_profile().await?;

                /*
                    search_mods() prompts a user to select mods in menu.
                    So, 'files' contains a list of mods to install.
                */
                let files = search_mods(&query, &working_profile).await?;

                // Download 'files'
                download_multiple_files(
                    files,
                    &working_profile.profile.modsfolder,
                    &working_profile.client,
                )
                .await?;
            }

            // Display the help message if empty
            None => display_help_msg(&HELP_MESSAGE).await,
        },

        // minefetch update OR minefetch upgrade
        Some("upgrade") | Some("update") => {
            // Create a working profile
            let working_profile = build_working_profile().await?;

            // Returns a list of new files of mods to install
            let files = upgrade_mods(&working_profile).await?;

            // If empty then there're no mods to update
            if files.len() == 0 {
                async_println!(":out: All mods are up to date!").await;
                return Ok(());
            }

            // Download 'files'
            download_multiple_files(
                files,
                &working_profile.profile.modsfolder,
                &working_profile.client,
            )
            .await?;
        }

        // minefetch list
        Some("list") => {
            // Create a working profile
            let working_profile = build_working_profile().await?;

            /*
                'match' is used here because if there's some
                error like a problem with internet connection then
                the program must output the list using only local data
            */
            match list_mods(&working_profile).await {
                Ok((size, versions)) => {
                    // If there're no mods in the profile
                    if size == 0 {
                        return Err("There are no mods yet".into());
                    }

                    // Print text
                    async_println!(
                        ":out: There are {}{}{} mods in profile {}:",
                        MFText::Bold,
                        size,
                        MFText::Reset,
                        working_profile.profile.name
                    )
                    .await;

                    // Set the counter
                    let mut counter: usize = 1;

                    // Go through the versions' list
                    for (_, version) in versions {
                        // Print text
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

                        // Increase the counter
                        counter += 1
                    }
                }
                // If there's some error then try to display mods' list locally 
                Err(error) => {
                    // Print the error
                    async_eprintln!(":err: {}", error).await;

                    // Get a mods' folder
                    let path = Path::new(&working_profile.profile.modsfolder);

                    // Read the dir
                    let mut entries = tokio::fs::read_dir(path).await?;

                    // Set the counter
                    let mut counter: usize = 1;

                    // Go through files in the dir
                    while let Some(entry) = entries.next_entry().await? {
                        
                        // Get the filename if the file has a .jar extension
                        if let Some(filename) = get_jar_filename(&entry).await {
                            // Print filename
                            async_println!("[{}] {}", counter, filename).await;
                            
                            // Increase the counter
                            counter += 1;
                        }
                    }
                }
            };
        }

        // minefetch lock %subcommand%
        Some("lock") => match args.get(2).map(String::as_str) {
            // minefetch lock add
            Some("add") => {
                // Create a working profile
                let working_profile = build_working_profile().await?;

                // Add a lock through interactive menu
                add_lock(&working_profile).await?;
            }

            // minefetch lock remove
            Some("remove") => {
                // Create a working profile
                let working_profile = build_working_profile().await?;

                // Remove a lock through interactive menu
                remove_lock(&working_profile).await?;
            }

            // minefetch lock list
            Some("list") => {
                // Create a working profile
                let working_profile = build_working_profile().await?;
                let locks = list_locks(&working_profile).await?;

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

            // Display the help message in other cases
            Some(_) => display_help_msg(&HELP_MESSAGE).await,
            None => display_help_msg(&HELP_MESSAGE).await,
        },

        // minefetch help
        Some("help") => {
            display_help_msg(&HELP_MESSAGE).await;
        }

        // minefetch debug
        Some("debug") => {
            // Display some information
            println!(":dbg: {} / {} / {}", NAME, PROGRAM_VERSION, USER_AGENT);
        }

        // minefetch edit
        Some("edit") => {
            // Create a working profile
            let working_profile = build_working_profile().await?;

            // Call an interactive dialog
            edit_mod(&working_profile).await?;
        }

        // Display a help message in other case
        Some(_) => display_help_msg(&HELP_MESSAGE).await,

        // If arguments are empty
        None => async_println!(":out: No arguments provided").await,
    }

    // Success
    Ok(())
}
