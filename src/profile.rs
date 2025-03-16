/*
 __  __ _            _____    _       _       ____             __ _ _
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__   |  _ \ _ __ ___  / _(_) | ___
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \  | |_) | '__/ _ \| |_| | |/ _ \
| |  | | | | | |  __/  _|  __/ || (__| | | | |  __/| | | (_) |  _| | |  __/
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| |_|   |_|  \___/|_| |_|_|\___|

*/

// Standard imports
use std::path::{Path, PathBuf};
use std::process::exit;
use std::result::Result;
use std::vec;

// External crates
use inquire::{
    ui::{Color, RenderConfig, Styled},
    Select,
};
use rfd::AsyncFileDialog;

// Internal imports
use crate::mfio::{ainput, press_enter};
use crate::structs::{Config, Profile};
use crate::utils::generate_hash;
use crate::{async_print, async_println};

/// Returns single active Profile
pub async fn read_config() -> Result<Profile, Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    let contents = match tokio::fs::read_to_string(&config_path).await {
        Ok(contents) => contents,
        Err(_) => return Err("There's no config yet, type minefetch profile create".into()),
    };
    let config: Config = toml::from_str(&contents)?;

    config
        .profile
        .into_iter()
        .find(|p| p.active) // Searching for only active one
        .ok_or_else(|| ":: No active profile found".into())
}

/// Returns full Config
pub async fn read_full_config() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

/// Creates config file
pub async fn create_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    async_print!(":: Press enter to choose mods directory").await;

    press_enter().await?;

    let modsfolder = match AsyncFileDialog::new().pick_folder().await {
        Some(file) => file
            .path()
            .to_str()
            .ok_or_else(|| "Invalid UTF-8")?
            .to_string(),
        None => {
            let buffer = ainput(
                ":: Cannot launch the gui folder picker\n:: Enter the path to mods folder: ",
            )
            .await?;
            let path = Path::new(&buffer);
            if !path.exists() {
                return Err("No folder with such name".into());
            }
            buffer.trim().to_string()
        }
    };

    let gameversion = ainput(":: Type a Minecraft version: ").await?;

    let loaders = vec![
        ("Quilt", "quilt"),
        ("Fabric", "fabric"),
        ("Forge", "forge"),
        ("NeoForge", "neoforge"),
    ];

    let choices: Vec<_> = loaders.iter().map(|(label, _value)| label).collect();

    let prompt_prefix = Styled::new("");
    let option_prefix = Styled::new(">>").with_fg(Color::DarkGreen);
    let render_cfg: RenderConfig = RenderConfig::empty()
        .with_highlighted_option_prefix(option_prefix)
        .with_prompt_prefix(prompt_prefix);

    let loader = match Select::new(":: Choose a loader\n", choices)
        .without_filtering()
        .without_help_message()
        .with_render_config(render_cfg)
        .prompt()
    {
        Ok(selection) => loaders
            .iter()
            .find(|(label, _)| label == selection)
            .map(|(_, value)| value.to_string())
            .ok_or_else(|| "Cannot translate pretty text to system one")?,
        Err(_) => return Err("Why did you do that?".into()),
    };

    let name = ainput(":: What should this profile be called? ").await?;

    let mut current_config = match read_full_config().await {
        Ok(cfg) => cfg,
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

    for obj in current_config.profile.iter_mut() {
        obj.active = false;
    }

    current_config.profile.push(new_profile);

    let string_toml = toml::to_string(&current_config)?;

    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes one selected profile
pub async fn delete_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|i| (i.name.clone(), i.hash.clone()))
        .collect();

    if profiles.is_empty() {
        return Err("There are no profiles yet".into());
    };

    let choices: Vec<_> = profiles.iter().map(|(label, _)| label).collect();

    let prompt_prefix = Styled::new("");
    let option_prefix = Styled::new(">>").with_fg(Color::DarkGreen);
    let render_cfg: RenderConfig = RenderConfig::empty()
        .with_highlighted_option_prefix(option_prefix)
        .with_prompt_prefix(prompt_prefix);

    let selected_value = match Select::new(":: Which profile to delete?\n", choices.clone())
        .without_filtering()
        .without_help_message()
        .with_render_config(render_cfg)
        .prompt()
    {
        Ok(selection) => match profiles
            .iter()
            .find(|(label, _)| &*label == selection) // Notice the dereference here
            .map(|(_, value)| value.clone())
            .ok_or_else(|| ":err: Cannot translate pretty text to system one")
        {
            Ok(s) => s,
            Err(e) => return Err(e.into()),
        },
        Err(_) => {
            async_println!(":err: Why did you do that?").await;
            exit(0)
        }
    };

    config
        .profile
        .retain(|profile| profile.hash != selected_value);

    let string_toml = toml::to_string(&config)?;
    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes config file completely
pub async fn delete_all_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };
    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");
    tokio::fs::remove_file(config_path).await?;
    Ok(())
}

/// Switches profile to selected one
pub async fn switch_profile() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };

    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|i| {
            let name = if i.active {
                format!(
                    "* {} [{} {}] [{}]",
                    i.name, i.loader, i.gameversion, i.modsfolder
                )
            } else {
                format!(
                    "  {} [{} {}] [{}]",
                    i.name, i.loader, i.gameversion, i.modsfolder
                )
            };
            (name, i.hash.clone())
        })
        .collect();

    let choices: Vec<_> = profiles.iter().map(|(label, _)| label).collect();

    let prompt_prefix = Styled::new("");
    let option_prefix = Styled::new(">>").with_fg(Color::DarkGreen);
    let render_cfg: RenderConfig = RenderConfig::empty()
        .with_highlighted_option_prefix(option_prefix)
        .with_prompt_prefix(prompt_prefix);

    let selected_value = match Select::new(":: Which profile to switch to?\n", choices.clone())
        .without_filtering()
        .without_help_message()
        .with_render_config(render_cfg)
        .prompt()
    {
        Ok(selection) => match profiles
            .iter()
            .find(|(label, _)| &*label == selection) // Notice the dereference here
            .map(|(_, value)| value.clone())
            .ok_or_else(|| ":err: Cannot translate pretty text to system one")
        {
            Ok(s) => s,
            Err(e) => return Err(e.into()),
        },
        Err(_) => {
            async_println!(":err: Why did you do that?").await;
            exit(0)
        }
    };
    for obj in config.profile.iter_mut() {
        if obj.hash == selected_value {
            obj.active = true
        } else {
            obj.active = false;
        }
    }

    let string_toml = toml::to_string(&config)?;
    let home_dir = get_confdir().await?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Lists all profiles
pub async fn list_profiles() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            return Err("There's no config yet, type minefetch profile create".into());
        }
    };
    for i in config.profile {
        if i.active {
            async_println!(
                "* {} [{} {}] [{}]",
                i.name,
                i.loader,
                i.gameversion,
                i.modsfolder
            )
            .await
        } else {
            async_println!(
                "  {} [{} {}] [{}]",
                i.name,
                i.loader,
                i.gameversion,
                i.modsfolder
            )
            .await
        }
    }
    Ok(())
}

/// Returns home dir. Hopefully fixes problems on Windows
pub async fn get_confdir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let system = whoami::platform().to_string();
    let confdir = if system == "Windows" {
        PathBuf::from(format!("C:\\Users\\{}", whoami::username()))
    } else {
        home::home_dir().ok_or(":err: Couldn't find the home dir")?
    };

    Ok(confdir)
}
