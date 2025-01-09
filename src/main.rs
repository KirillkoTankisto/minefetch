mod structs;
mod consts;

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

use std::io::Write;
use std::result::Result;

use structs::Config;
use structs::Profile;
use structs::VersionsList;

use consts::*;

use rand::distributions::Alphanumeric;
use rand::Rng;

use serde_json;

use rfd::AsyncFileDialog;

use std::path::Path;

use inquire::Select;

use inquire::ui::{Color, RenderConfig, Styled};

/// The start of main async function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "add" {
            if args.len() > 2 {
                println!(":: Starting add...");
                let profile: Profile = read_config().await?;
                let params = vec![
                    (
                        "loaders",
                        serde_json::to_string(&[profile.loader])
                            .expect("Failed to serialize loaders"),
                    ),
                    (
                        "game_versions",
                        serde_json::to_string(&[profile.gameversion])
                            .expect("Failed to serialize game_versions"),
                    ),
                ];
                let client = reqwest::Client::new();
                let modversion = fetch_latest_version(&args[2], &client, &params).await?;
                download_file(&profile.modsfolder, modversion.0, modversion.1, &client).await?;
            } else {
                println!(":: Usage: minefetch add <modname>");
            }
        } else if args[1] == "test" {
            println!(":: test...");
            config_create().await?;
        }
    } else {
        println!(":: No arguments provided");
    }
    Ok(())
}

/// Returns filename and url
async fn fetch_latest_version(
    modname: &str,
    client: &reqwest::Client,
    params: &[(&str, String)],
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let url = reqwest::Url::parse_with_params(
        &format!("https://api.modrinth.com/v2/project/{}/version", modname),
        &params,
    )?;

    let res = client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await?
        .text()
        .await?;

    let parsed: VersionsList = serde_json::from_str(&res)?;

    let latest_parsed = parsed
        .get(0)
        .and_then(|v| v.files.get(0))
        .ok_or(":: No version available")?;

    Ok((latest_parsed.filename.clone(), latest_parsed.url.clone()))
}

/// Downloads single file
async fn download_file(
    path: &str,
    filename: String,
    url: String,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(path).await?;

    let path = std::path::Path::new(path).join(filename);

    let mut response = client.get(url).send().await?;

    let mut file = tokio::fs::File::create(path).await?;

    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
    }

    Ok(())
}

/// Generates random 64 char string
async fn generate_hash() -> Result<String, Box<dyn std::error::Error>> {
    let random_hash = tokio::task::spawn_blocking(|| {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect::<String>()
    })
    .await?;
    Ok(random_hash)
}

/// Returns single active Profile
async fn read_config() -> Result<Profile, Box<dyn std::error::Error>> {
    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&contents)?;

    config
        .profile
        .into_iter()
        .find(|p| p.active)
        .ok_or_else(|| "No active profile found".into())
}

/// Returns full Config
async fn read_full_config() -> Result<Config, Box<dyn std::error::Error>> {
    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

/// Config creation dialog
async fn config_create() -> Result<(), Box<dyn std::error::Error>> {
    print!(":: Press enter to choose mods directory");

    std::io::stdout().flush().unwrap();

    press_enter().await?;

    let mut folder_path = String::new();

    match AsyncFileDialog::new().pick_folder().await {
        Some(file) => {
            folder_path = file
                .path()
                .to_str()
                .ok_or_else(|| "Invalid UTF-8")
                .unwrap()
                .to_string();
        }
        None => {
            print!(":: Cannot launch the gui folder picker\n:: type mods folder path: ");
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer)?;
            let folder_path_in_path = Path::new(buffer.trim());
            if !std::path::Path::exists(folder_path_in_path) {
                println!("No folder with such name");
                return Ok(());
            }
            folder_path = buffer.trim().to_string()
        }
    }

    println!("{}", folder_path);

    let mut buffer = String::new();

    print!(":: Type a Minecraft version: ");

    std::io::stdout().flush().unwrap();

    std::io::stdin().read_line(&mut buffer)?;

    let modversion = buffer.trim();

    let loaders = vec![
        ("Quilt", "quilt"),
        ("Fabric", "fabric"),
        ("Forge", "forge"),
        ("NeoForge", "neoforge"),
    ];

    let choices: Vec<_> = loaders.iter().map(|(label, _value)| label).collect();

    let prompt_prefix = Styled::new("$").with_fg(Color::DarkRed);
    let render_cfg: RenderConfig = RenderConfig::default().with_prompt_prefix(prompt_prefix);

    let mut selected_value = String::new();

    match Select::new("Choose a loader", choices)
        .with_render_config(render_cfg)
        .prompt()
    {
        Ok(selection) => {
            selected_value = loaders
                .iter()
                .find(|(label, _)| label == selection)
                .map(|(_, value)| value)
                .unwrap_or(&"Unknown")
                .to_string();
        }
        Err(_) => println!("Error"),
    }
    println!("{}", selected_value);

 
    let mut current_config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            println!("There's no config yet ((((");
            return Ok(());
        }
    };

    let new_profile = Profile {
        active: true,
        name: "name".to_string(),
        modsfolder: folder_path,
        gameversion: modversion.to_string(),
        loader: selected_value,
        hash: generate_hash().await?
    };

    current_config.profile.push(new_profile);

    let new_config = Config {
        title: NAME.to_string(),
        version: PROGRAM_VERSION.to_string(),
        profile: current_config.profile,
    };

    Ok(())
}

/// Press enter to continue functionality
async fn press_enter() -> Result<(), tokio::io::Error> {
    let mut stdinc = io::stdin();

    let mut buffer = [0u8; 1];

    stdinc.read_exact(&mut buffer).await?;

    Ok(())
}
