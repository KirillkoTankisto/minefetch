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
    let config_path = get_confpath().await?;

    let contents = match tokio::fs::read_to_string(&config_path).await {
        Ok(contents) => contents,
        Err(_) => return Err("There's no config yet, type minefetch profile create".into()),
    };
    let config: Config = toml::from_str(&contents)?;

    config
        .profile
        .into_iter()
        .find(|profile| profile.active) // Searching for only active one
        .ok_or_else(|| ":out: No active profile found".into())
}

/// Returns full Config
pub async fn read_full_config() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let config_path = get_confpath().await?;
    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

/// Creates config file
pub async fn create_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    async_println!(":out: Press enter to choose mods directory").await;

    press_enter().await?;

    let modsfolder = match AsyncFileDialog::new().pick_folder().await {
        Some(file) => file
            .path()
            .to_str()
            .ok_or_else(|| "Invalid UTF-8")?
            .to_string(),
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

    let gameversion = ainput(":out: Type a Minecraft version: ").await?;

    let loaders: Vec<(String, String)> = vec![
        ("Quilt".to_string(), "quilt".to_string()),
        ("Fabric".to_string(), "fabric".to_string()),
        ("Forge".to_string(), "forge".to_string()),
        ("NeoForge".to_string(), "neoforge".to_string()),
    ];

    let loader = select("Choose a loader", loaders).await?;

    let name = ainput(":out: What should this profile be called? ").await?;

    let mut current_config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => Config::default(),
    };

    let new_profile = Profile {
        active: true,
        name,
        modsfolder,
        gameversion,
        loader,
        hash: generate_hash().await?,
    };

    for profile in current_config.profile.iter_mut() {
        profile.active = false;
    }

    current_config.profile.push(new_profile);

    let string_toml = toml::to_string(&current_config)?;
    let config_path = get_confpath().await?;
    let config_dir = get_confdir().await?;

    tokio::fs::create_dir_all(config_dir).await?;
    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes one selected profile
pub async fn delete_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|profile| (profile.name.clone(), profile.hash.clone()))
        .collect();

    if profiles.is_empty() {
        return Err("There are no profiles yet".into());
    };
    let selected_value = select("Which profile to delete?", profiles).await?;

    config
        .profile
        .retain(|profile| profile.hash != selected_value);

    let string_toml = toml::to_string(&config)?;
    let config_path = get_confpath().await?;

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes config file completely
pub async fn delete_all_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    let config_path = get_confpath().await?;

    tokio::fs::remove_file(config_path).await?;
    Ok(())
}

/// Switches profile to selected one
pub async fn switch_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

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

    let selected_value = select("Which profile to switch to?", profiles).await?;

    for profile in config.profile.iter_mut() {
        if profile.hash == selected_value {
            profile.active = true
        } else {
            profile.active = false;
        }
    }

    let string_toml = toml::to_string(&config)?;
    let config_path = get_confpath().await?;

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Lists all profiles
pub async fn list_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = match read_full_config().await {
        Ok(config) => config,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };
    for profile in config.profile {
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
    Ok(())
}

pub async fn get_locks(
    profile: &Profile,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let locks_path = get_lock_dir(profile);
    let string = match tokio::fs::read_to_string(locks_path).await {
        Ok(string) => string,
        Err(_) => {
            return Err(format!("The profile {} doesn't have any locks yet", profile.name).into());
        }
    };
    let locks: Locks = toml::from_str(&string)?;
    if locks.lock.is_empty() {
        return Err(format!("The profile {} doesn't have any locks yet", profile.name).into());
    }
    Ok(locks.lock)
}

pub async fn add_lock(
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_, versions) = list_mods(&working_profile).await?;
    let mut locklist: Vec<(String, String)> = Vec::new();

    for i in versions {
        locklist.push((
            i.1.files
                .iter()
                .find(|file| file.primary)
                .ok_or("No primary file found")
                .map(|file| file.filename.clone())?,
            i.0,
        ))
    }

    let hash = select("Choose a mod to lock", locklist).await?;
    let profile = read_config().await?;

    write_lock(&profile, hash).await?;
    Ok(())
}

pub async fn write_lock(
    profile: &Profile,
    hash: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut locks = match get_locks(&profile).await {
        Ok(locks) => locks,
        Err(_) => Vec::new(),
    };

    locks.push(hash);

    let new_locks = Locks { lock: locks };
    let locks_path = get_lock_dir(&profile);

    tokio::fs::write(locks_path, toml::to_string(&new_locks)?).await?;
    Ok(())
}

pub async fn remove_lock(
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut locks = get_locks(&working_profile.profile).await?;
    let mut locklist: Vec<(String, String)> = Vec::new();
    let (_, mods) = list_mods(&working_profile).await?;
    for lock in &locks {
        let (name, filename) = match mods.get_key_value(lock) {
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
            None => continue,
        };
        locklist.push((format!("{} ({})", name, filename), lock.to_string()));
    }

    let hash = select("Choose a mod to unlock", locklist).await?;

    locks.retain(|lock| lock != &hash);

    let locks = Locks { lock: locks };

    let locks_to_str = match toml::to_string(&locks) {
        Ok(locks) => locks,
        Err(error) => return Err(error.into()),
    };

    let lockspath = get_lock_dir(&working_profile.profile);

    tokio::fs::write(lockspath, locks_to_str).await?;

    Ok(())
}

pub async fn remove_locked_ones(
    hashmap: &mut MFHashMap,
    locks: Vec<String>,
) -> Result<&mut MFHashMap, Box<dyn std::error::Error + Send + Sync>> {
    for lock in locks {
        hashmap.remove_entry(&lock);
    }
    Ok(hashmap)
}

pub fn get_lock_dir(profile: &Profile) -> PathBuf {
    return Path::join(Path::new(&profile.modsfolder), "locks.toml");
}

pub async fn list_locks(
    working_profile: &WorkingProfile,
) -> Result<Vec<(usize, String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let locks = get_locks(&working_profile.profile).await?;
    let (_, mods) = list_mods(&working_profile).await?;
    let mut counter: usize = 1;
    let mut result: Vec<(usize, String, String)> = Vec::new();
    for lock in locks {
        let (name, filename) = match mods.get_key_value(&lock) {
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
            None => continue,
        };
        result.push((counter, name, filename));
        counter += 1;
    }
    Ok(result)
}

pub async fn build_working_profile()
-> Result<WorkingProfile, Box<dyn std::error::Error + Send + Sync>> {
    let profile = read_config().await?;
    let client = Client::new();
    let working_profile = WorkingProfile { profile, client };
    Ok(working_profile)
}
