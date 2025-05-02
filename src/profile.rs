/*
 __  __ _            _____    _       _       ____             __ _ _
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__   |  _ \ _ __ ___  / _(_) | ___
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \  | |_) | '__/ _ \| |_| | |/ _ \
| |  | | | | | |  __/  _|  __/ || (__| | | | |  __/| | | (_) |  _| | |  __/
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| |_|   |_|  \___/|_| |_|_|\___|

*/

// Standard imports
use std::path::{Path, PathBuf};
use std::result::Result;
use std::vec;

// External crates
use reqwest::Client;
use rfd::AsyncFileDialog;

// Internal imports
use crate::api::list_mods;
use crate::async_println;
use crate::mfio::{MFText, ainput, press_enter, select};
use crate::structs::{Config, Locks, MFHashMap, Profile, WorkingProfile};
use crate::utils::{generate_hash, get_confdir, get_confpath};

/// Returns single active Profile
pub async fn read_config() -> Result<Profile, Box<dyn std::error::Error + Send + Sync>> {
    // Get config path
    let config_path = get_confpath().await?;

    // Read config
    let contents = match tokio::fs::read_to_string(&config_path).await {
        Ok(contents) => contents,
        Err(_) => return Err("There's no config yet, type minefetch profile create".into()),
    };

    // Parse config
    let config: Config = toml::from_str(&contents)?;

    // Return active profile
    config
        .profile
        .into_iter()
        .find(|profile| profile.active) // Searching for only active one
        .ok_or_else(|| ":out: No active profile found".into())
}

/// Returns full Config
pub async fn read_full_config() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    // Get config path
    let config_path = get_confpath().await?;

    // Read config
    let contents = tokio::fs::read_to_string(&config_path).await?;

    // Parse config
    let config: Config = toml::from_str(&contents)?;

    // Return full config, including inactive profiles
    Ok(config)
}

/// Creates config file
pub async fn create_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Print the text
    async_println!(":out: Press enter to choose mods directory").await;

    // Ask user to press enter
    press_enter().await?;

    // Get selected folder
    let modsfolder = match AsyncFileDialog::new().pick_folder().await {
        // Get a folder path
        Some(file) => file
            .path()
            .to_str()
            .ok_or_else(|| "Invalid UTF-8")?
            .to_string(),
        // If function user didn't choose any folder
        None => {
            let buffer = ainput(
                ":out: Cannot launch the gui folder picker\n:: Enter the path to mods folder: ",
            )
            .await?;
            let path = Path::new(&buffer);
            if !path.exists() {
                return Err("No folder with such name".into());
            }
            buffer.trim().to_string()
        }
    };

    // Get minecraft version

    let gameversion = ainput(":out: Type a Minecraft version: ").await?;

    // A list of loaders
    let loaders: Vec<(String, String)> = vec![
        ("Quilt".to_string(), "quilt".to_string()),
        ("Fabric".to_string(), "fabric".to_string()),
        ("Forge".to_string(), "forge".to_string()),
        ("NeoForge".to_string(), "neoforge".to_string()),
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
        loader,
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
pub async fn delete_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        .map(|profile| (profile.name.clone(), profile.hash.clone()))
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

/// Deletes config file completely
pub async fn delete_all_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read a config first to make sure that it won't anything else
    match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    // Get a config path
    let config_path = get_confpath().await?;

    // Delete a config
    tokio::fs::remove_file(config_path).await?;

    // Success
    Ok(())
}

/// Switches profile to selected one
pub async fn switch_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        if profile.hash == selected_hash {
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
pub async fn list_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            async_println!(
                "[{}{}*{}] {} [{} {}] [{}]",
                MFText::Bold,
                MFText::Underline,
                MFText::Reset,
                profile.name,
                profile.loader,
                profile.gameversion,
                profile.modsfolder
            )
            .await
        } else {
            async_println!(
                "[{}{} {}] {} [{} {}] [{}]",
                MFText::Bold,
                MFText::Underline,
                MFText::Reset,
                profile.name,
                profile.loader,
                profile.gameversion,
                profile.modsfolder
            )
            .await
        }
    }

    // Success
    Ok(())
}

/// Gets a list of locks
pub async fn get_locks(
    profile: &Profile,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Get a lock path
    let locks_path = get_locks_path(profile);

    // Read a lock list
    let string = match tokio::fs::read_to_string(locks_path).await {
        Ok(string) => string,
        Err(_) => {
            return Err(format!("The profile {} doesn't have any locks yet", profile.name).into());
        }
    };

    // Parse the toml string
    let locks: Locks = toml::from_str(&string)?;

    // If empty then return an error
    if locks.lock.is_empty() {
        return Err(format!("The profile {} doesn't have any locks yet", profile.name).into());
    }

    // Return locks
    Ok(locks.lock)
}

/// Adds a lock
pub async fn add_lock(
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get a mod list
    let (_, versions) = list_mods(&working_profile).await?;

    // Create a mutable mod menu
    let mut modmenu: Vec<(String, String)> = Vec::new();

    // Push the version files into 'modmenu'
    for version in versions {
        modmenu.push((
            version
                .1
                .files
                .iter()
                .find(|file| file.primary)
                .ok_or("No primary file found")
                .map(|file| file.filename.clone())?,
            version.0,
        ))
    }

    // Select a hash
    let hash = select("Choose a mod to lock", modmenu).await?;

    // Write into the lock
    write_lock(&working_profile.profile, hash).await?;

    // Success
    Ok(())
}

/// Writes a new lock into the file
pub async fn write_lock(
    profile: &Profile,
    hash: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get a mutable lock list
    let mut locks = match get_locks(&profile).await {
        Ok(locks) => locks,
        Err(_) => Vec::new(),
    };

    // Push a new hash into the lock list
    locks.push(hash);

    // Create a new lock list structure
    let new_locks = Locks { lock: locks };

    // Get a locks' path
    let locks_path = get_locks_path(&profile);

    // Write into the file
    tokio::fs::write(locks_path, toml::to_string(&new_locks)?).await?;

    // Success
    Ok(())
}

/// Removes a lock
pub async fn remove_lock(
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get a mutable lock list
    let mut locks = get_locks(&working_profile.profile).await?;

    // Create a mutable lock menu
    let mut lockmenu: Vec<(String, String)> = Vec::new();

    // Get a mod list
    let (_, mods) = list_mods(&working_profile).await?;

    /*
        Go through all locks and get an
        info for each using mod list
    */
    for lock in &locks {
        // Get a each version info using its hash
        let (name, filename) = match mods.get_key_value(lock) {
            // If it's in the mod list then clone its info
            Some(value) => (
                value.1.name.clone(),
                value
                    .1
                    .files
                    .iter()
                    .find(|file| file.primary)
                    .ok_or("No primary file found")
                    .map(|file| file.filename.clone())?,
            ),

            /*
                If not then continue
                (This case mustn't even happen unless
                the mod was deleted by user)
            */
            None => continue,
        };

        // Push the info into lock menu
        lockmenu.push((format!("{} ({})", name, filename), lock.to_string()));
    }

    // Choose a hash
    let hash = select("Choose a mod to unlock", lockmenu).await?;

    /*
        Keep all locks that
        are not equal to the selected hash
    */
    locks.retain(|lock| lock != &hash);

    // Create a new lock structure
    let locks = Locks { lock: locks };

    // Translate into toml string
    let locks_to_str = match toml::to_string(&locks) {
        Ok(locks) => locks,
        Err(error) => return Err(error.into()),
    };

    // Get a locks' path
    let lockspath = get_locks_path(&working_profile.profile);

    // Write into the file
    tokio::fs::write(lockspath, locks_to_str).await?;

    // Success
    Ok(())
}

/// Removes locks (hashes) from the hashmap
pub async fn remove_locked_ones(
    hashmap: &mut MFHashMap,
    locks: Vec<String>,
) -> Result<&mut MFHashMap, Box<dyn std::error::Error + Send + Sync>> {
    /*
        A loop which removes
        the locks from the hashmap
    */
    for lock in locks {
        hashmap.remove_entry(&lock);
    }

    // Return the modified hashmap
    Ok(hashmap)
}

/// Gets the locks' path
pub fn get_locks_path(profile: &Profile) -> PathBuf {
    // Join the mods' folder path with the locks' filename
    return Path::join(Path::new(&profile.modsfolder), "locks.toml");
}

/// Lists all locks
pub async fn list_locks(
    working_profile: &WorkingProfile,
) -> Result<Vec<(usize, String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    // Get a locks' list
    let locks = get_locks(&working_profile.profile).await?;

    // Get a mods' list
    let (_, mods) = list_mods(&working_profile).await?;

    // Set the counter
    let mut counter: usize = 1;

    // Create a mutable locks' list for output
    let mut result: Vec<(usize, String, String)> = Vec::new();

    /*
        Go through all locks and get an
        info for each using mod list
    */
    for lock in locks {
        // Get a version by hash
        let (name, filename) = match mods.get_key_value(&lock) {
            // If the mod exists
            Some(version) => (
                version.1.name.clone(),
                version
                    .1
                    .files
                    .iter()
                    .find(|file| file.primary)
                    .ok_or("No primary file found")
                    .map(|file| file.filename.clone())?,
            ),
            // If not then skip
            None => continue,
        };

        // Push into the result
        result.push((counter, name, filename));

        // Append to the counter
        counter += 1;
    }

    // Return the list
    Ok(result)
}

/// Creates a WorkingProfile which contains a Client and a Profile
pub async fn build_working_profile()
-> Result<WorkingProfile, Box<dyn std::error::Error + Send + Sync>> {
    // Read the profile
    let profile = read_config().await?;

    // Create a client
    let client = Client::new();

    // Create a WorkingProfile structure
    let working_profile = WorkingProfile { profile, client };

    // Return the WorkingProfile
    Ok(working_profile)
}
