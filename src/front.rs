use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use crate::api::{
    Anymod, edit_mod, get_dependencies, get_latest_version, list_mods, replace_mods, search_mods,
    upgrade_mods,
};
use crate::downloader::download_multiple_mods;
use crate::mfio::{MFText, ainput, parse_to_int, press_enter, select};
use crate::profile::{add_lock, build_working_profile, list_locks, read_full_config, remove_lock};
use crate::structs::{Config, Profile};
use crate::utils::{generate_hash, get_confdir, get_confpath, get_jar_filename};

pub async fn add_mod(modname: &str) -> Result<(), Box<dyn Error>> {
    // Print text
    println!(":out: Adding a mod...");

    // Create a working profile
    let working_profile = build_working_profile().await?;

    // Get the latest version
    let mod_version = get_latest_version(&modname.to_string(), &working_profile).await?;

    let mod_list = list_mods(&working_profile).await?;

    for anymod in mod_list.1 {
        if anymod.hash == mod_version.hash {
            return Err(format!(
                "The mod {} is already installed",
                mod_version.title.as_ref().ok_or("No title")?
            )
            .into());
        }
    }

    // Download this version
    replace_mods(
        vec![&mod_version.hash],
        vec![mod_version.clone()],
        &working_profile,
    )
    .await?;

    // Print text
    println!(
        ":out: Downloaded {} ({})",
        &mod_version.title.unwrap(),
        &mod_version.filename
    );

    // Check for existing dependencies
    match mod_version.depends {
        Some(dependencies) => {
            // Get the dependencies' info
            let dependencies = get_dependencies(&dependencies, &working_profile.client).await?;

            // Print all existing dependencies: their names and types (required or optional)
            for dependency in dependencies {
                println!(":dep: {} {}", dependency.0, dependency.1);
            }
        }
        None => {}
    }
    Ok(())
}

/// Creates config file
pub async fn create_profile() -> Result<(), Box<dyn Error>> {
    // Print the text
    println!(":out: Press enter to choose mods directory");

    // Ask user to press enter
    press_enter().await?;

    // Get selected folder
    let modsfolder = {
        let buffer = ainput(":: Enter the path to mods folder: ").await?;
        let path = Path::new(&buffer);
        if !path.exists() {
            return Err("No folder with such name".into());
        }
        buffer.trim().to_string()
    };

    // Get minecraft version

    let gameversion = ainput(":out: Type a Minecraft version: ").await?;

    // A list of loaders
    let loaders: Vec<(&str, &str)> = vec![
        ("Quilt", "quilt"),
        ("Fabric", "fabric"),
        ("Forge", "forge"),
        ("NeoForge", "neoforge"),
    ];

    // Ask user to select a loader
    let loader = select("Choose a loader", loaders).await?;

    // Ask to enter the name of the profile
    let name = ainput(":out: What should this profile be called? ").await?;

    // Get a full config
    let mut current_config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    // Create a new profile
    let new_profile = Profile {
        active: true,
        name,
        modsfolder,
        gameversion,
        loader: loader.to_string(),
        hash: generate_hash().await?,
    };

    // Set every previous profile as inactive
    for profile in current_config.profile.iter_mut() {
        profile.active = false;
    }

    // Push the new profile in the config
    current_config.profile.push(new_profile);

    // Translate into toml string
    let string_toml = toml::to_string(&current_config)?;

    // Get config directory
    let config_dir = get_confdir().await?;

    // Get a config path
    let config_path = get_confpath().await?;

    // Create a config folder if it doesn't exist
    tokio::fs::create_dir_all(config_dir).await?;

    // Write a config
    tokio::fs::write(config_path, string_toml).await?;

    // Success
    Ok(())
}

/// Deletes one selected profile
pub async fn delete_profile(all: u32) -> Result<(), Box<dyn Error>> {
    // Get a mutable config
    let mut config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    let path = get_confpath().await?;

    if all == 1 {
        tokio::fs::remove_file(path).await?;
    }

    // Create a profile menu
    let profiles: Vec<(&str, String)> = config
        .profile
        .iter()
        .map(|profile| (profile.name.as_str(), profile.hash.clone()))
        .collect();

    // If there's no profiles
    if profiles.is_empty() {
        return Err("There are no profiles yet".into());
    };

    // Get a selected profile
    let selected_value = select("Which profile to delete?", profiles).await?;

    // Leave all profiles that don't have the same hash
    config
        .profile
        .retain(|profile| profile.hash != selected_value);

    // Translate into toml string
    let string_toml = toml::to_string(&config)?;

    // Get a config path
    let config_path = get_confpath().await?;

    // Write a config
    tokio::fs::write(config_path, string_toml).await?;

    // Success
    Ok(())
}

/// Switches profile to selected one
pub async fn switch_profile() -> Result<(), Box<dyn Error>> {
    // Get a mutable config
    let mut config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    // Create a profile menu
    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|profile| {
            let name = if profile.active {
                format!(
                    "[{}{}*{}] {} [{} {}] [{}]",
                    MFText::Bold,
                    MFText::Underline,
                    MFText::Reset,
                    profile.name,
                    profile.loader,
                    profile.gameversion,
                    profile.modsfolder
                )
            } else {
                format!(
                    "[{}{} {}] {} [{} {}] [{}]",
                    MFText::Bold,
                    MFText::Underline,
                    MFText::Reset,
                    profile.name,
                    profile.loader,
                    profile.gameversion,
                    profile.modsfolder
                )
            };
            (name, profile.hash.clone())
        })
        .collect();

    // Get a selected profile hash
    let selected_hash = select("Which profile to switch to?", profiles).await?;

    // Set a selected profile to active and others to inactive
    for profile in config.profile.iter_mut() {
        if profile.hash == *selected_hash {
            profile.active = true
        } else {
            profile.active = false;
        }
    }

    // Translate into toml string
    let string_toml = toml::to_string(&config)?;

    // Get a config path
    let config_path = get_confpath().await?;

    // Write a config
    tokio::fs::write(config_path, string_toml).await?;

    // Success
    Ok(())
}

/// Lists all profiles
pub async fn list_profiles() -> Result<(), Box<dyn Error>> {
    // Get all profiles
    let config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    // Print the profiles
    for profile in config.profile {
        // If it's active then add an asterisk
        if profile.active {
            println!(
                "[{}{}*{}] {} [{} {}] [{}]",
                MFText::Bold,
                MFText::Underline,
                MFText::Reset,
                profile.name,
                profile.loader,
                profile.gameversion,
                profile.modsfolder
            )
        } else {
            println!(
                "[{}{} {}] {} [{} {}] [{}]",
                MFText::Bold,
                MFText::Underline,
                MFText::Reset,
                profile.name,
                profile.loader,
                profile.gameversion,
                profile.modsfolder
            )
        }
    }

    // Success
    Ok(())
}

pub async fn search(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    // Join all the strings to form a query
    let query = args[2..].join(" ");

    // Create a working profile
    let working_profile = build_working_profile().await?;

    /*
    Get the current mods' list to compare with
    versions that user will try to install. It's needed to
    ensure that there won't be any duplicates of mods
    */

    let (size, mod_list) = list_mods(&working_profile).await.unwrap_or_default();

    /*
        search_mods() prompts a user to select mods in menu.
        So, 'files' contains a list of mods to install.
    */
    let hits = search_mods(&query, &working_profile).await?;

    // Print all hits
    for number in (0..hits.len()).rev() {
        if let Some(hit) = hits.get(number) {
            println!("[{}] {}", number + 1, hit.title);
        }
    }

    // Parse the user input
    let selected_string = ainput(":out: Select mods to install: ").await?;

    // Create a selected number list
    let numbers = parse_to_int(selected_string)?;

    let mut versions: Vec<Anymod> = Vec::new();

    // Print the options
    for number in &numbers {
        // Get a git by its number in the list
        match hits.get(*number) {
            // If it's in the range
            Some(version) => {
                // If there're mods installed in the profile
                if size != 0 {
                    let mut installed: bool = false;

                    for anymod in &mod_list {
                        if anymod.project_id == version.project_id {
                            eprintln!(
                                ":wrn: The mod {} is already installed, skipping",
                                version.title
                            );
                            installed = true;
                            break;
                        }
                    }
                    if !installed {
                        let project_id = &version.project_id;
                        let version = get_latest_version(project_id, &working_profile).await?;
                        versions.push(version);
                    }
                }
            }
            None => return Err("The number is out of range".into()),
        };
    }

    // Download 'files'
    download_multiple_mods(versions, Arc::new(working_profile.clone())).await?;

    Ok(())
}

pub async fn upgrade() -> Result<(), Box<dyn Error>> {
    // Create a working profile
    let working_profile = build_working_profile().await?;

    // Returns a list of new files of mods to install
    let (old_mods, new_mods) = upgrade_mods(&working_profile).await?;

    // If empty then there're no mods to update
    if new_mods.len() == 0 {
        println!(":out: All mods are up to date!");
        return Ok(());
    }

    // Download 'files'
    replace_mods(
        old_mods.iter().map(|hash| hash).collect(),
        new_mods,
        &working_profile,
    )
    .await?;

    Ok(())
}

pub async fn list() -> Result<(), Box<dyn Error>> {
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
            println!(
                ":out: There are {}{}{} mods in profile {}:",
                MFText::Bold,
                size,
                MFText::Reset,
                working_profile.profile.name
            );

            for (num, anymod) in versions.iter().enumerate() {
                println!(
                    "[{}{}{}] {}{}{} ({})",
                    MFText::Bold,
                    num + 1,
                    MFText::Reset,
                    MFText::Bold,
                    anymod.title.clone().unwrap_or_default(),
                    MFText::Reset,
                    anymod.filename,
                );
            }
        }
        // If there's some error then try to display mods' list locally
        Err(error) => {
            // Print the error
            eprintln!(":err: {}", error);

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
                    println!("[{}] {}", counter, filename);

                    // Increase the counter
                    counter += 1;
                }
            }
        }
    };

    Ok(())
}

pub async fn fadd_lock() -> Result<(), Box<dyn Error>> {
    // Create a working profile
    let working_profile = build_working_profile().await?;

    // Add a lock through interactive menu
    add_lock(&working_profile).await?;

    Ok(())
}

pub async fn rm_lock() -> Result<(), Box<dyn Error>> {
    // Create a working profile
    let working_profile = build_working_profile().await?;

    // Remove a lock through interactive menu
    remove_lock(&working_profile).await?;

    Ok(())
}

pub async fn fedit_mod() -> Result<(), Box<dyn Error>> {
    // Create a working profile
    let working_profile = build_working_profile().await?;

    // Call an interactive dialog
    edit_mod(&working_profile).await?;

    Ok(())
}

pub async fn ls_lock() -> Result<(), Box<dyn Error>> {
    // Create a working profile
    let working_profile = build_working_profile().await?;
    let locks = list_locks(&working_profile).await?;

    for (size, name, filename) in locks {
        println!(
            "[{}{}{}] {}{}{} ({})",
            MFText::Bold,
            size,
            MFText::Reset,
            MFText::Bold,
            name,
            MFText::Reset,
            filename
        );
    }

    Ok(())
}
