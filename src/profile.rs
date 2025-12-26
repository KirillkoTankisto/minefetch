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

// External crates
use reqwest::Client;

// Internal imports
use crate::cache::list_mods_cached;
use crate::mfio::select;
use crate::structs::{Config, Locks, MFHashMap, Profile, WorkingProfile};
use crate::utils::get_confpath;

/// Returns single active Profile
pub async fn read_config() -> Result<Profile, Box<dyn std::error::Error>> {
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
        .ok_or_else(|| "No active profile found".into())
}

/// Returns full Config
pub async fn read_full_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Get config path
    let config_path = get_confpath().await?;

    // Read config
    let contents = tokio::fs::read_to_string(&config_path).await?;

    // Parse config
    let config: Config = toml::from_str(&contents)?;

    // Return full config, including inactive profiles
    Ok(config)
}

/// Gets a list of locks
pub async fn get_locks(profile: &Profile) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
pub async fn add_lock(working_profile: &WorkingProfile) -> Result<(), Box<dyn std::error::Error>> {
    // Get a mod list
    let versions = list_mods_cached(&working_profile).await?;

    // Create a mutable mod menu
    let mut modmenu: Vec<(String, String)> = Vec::new();

    // Push the version files into 'modmenu'
    for version in versions {
        modmenu.push((version.title.unwrap(), version.hash))
    }

    // Select a hash
    let hash = select("Choose a mod to lock", modmenu).await?;

    // Write into the lock
    write_lock(&working_profile.profile, hash).await?;

    // Success
    Ok(())
}

/// Writes a new lock into the file
pub async fn write_lock(profile: &Profile, hash: String) -> Result<(), Box<dyn std::error::Error>> {
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
) -> Result<(), Box<dyn std::error::Error>> {
    // Get a mutable lock list
    let mut locks = get_locks(&working_profile.profile).await?;

    // Create a mutable lock menu
    let mut lockmenu: Vec<(String, String)> = Vec::new();

    // Get a mod list
    let mods = list_mods_cached(&working_profile).await?;

    /*
        Go through all locks and get an
        info for each using mod list
    */
    for lock in &locks {
        // Get a each version info using its hash
        let (name, filename) = match mods
            .iter()
            .find_map(|v| if v.hash == *lock { Some(v) } else { None })
        {
            // If it's in the mod list then clone its info
            Some(value) => (value.title.clone(), value.filename.clone()),

            /*
                If not then continue
                (This case mustn't even happen unless
                the mod was deleted by user)
            */
            None => continue,
        };

        // Push the info into lock menu
        lockmenu.push((
            format!("{} ({})", name.unwrap(), filename),
            lock.to_string(),
        ));
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
) -> Result<&mut MFHashMap, Box<dyn std::error::Error>> {
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
) -> Result<Vec<(usize, String, String)>, Box<dyn std::error::Error>> {
    // Get a locks' list
    let locks = get_locks(&working_profile.profile).await?;

    // Get a mods' list
    let mods = list_mods_cached(&working_profile).await?;

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
        let (name, filename) = match mods
            .iter()
            .find_map(|v| if v.hash == *lock { Some(v) } else { None })
        {
            // If it's in the mod list then clone its info
            Some(value) => (value.title.clone(), value.filename.clone()),

            /*
                If not then continue
                (This case mustn't even happen unless
                the mod was deleted by user)
            */
            None => continue,
        };

        // Push into the result
        result.push((counter, name.unwrap(), filename));

        // Append to the counter
        counter += 1;
    }

    // Return the list
    Ok(result)
}

/// Creates a WorkingProfile which contains a Client and a Profile
pub async fn build_working_profile() -> Result<WorkingProfile, Box<dyn std::error::Error>> {
    // Read the profile
    let profile = read_config().await?;

    // Create a client
    let client = Client::new();

    // Create a WorkingProfile structure
    let working_profile = WorkingProfile { profile, client };

    // Return the WorkingProfile
    Ok(working_profile)
}
