// Standard imports
use std::path::Path;
use std::result::Result;

// External crates
use inquire::{
    ui::{Color, RenderConfig, Styled},
    Select,
};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rfd::AsyncFileDialog;
use serde_json;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

// Internal modules
mod consts;
mod structs;
use consts::*;
use structs::{Config, Profile, VersionsList};

/// Macro for async std output
macro_rules! async_println {
    ($($arg:tt)*) => {{
        async {
            let mut stdout = BufWriter::new(io::stdout());
            if let Err(e) = stdout.write_all(format!($($arg)*).as_bytes()).await {
                eprintln!("Error writing to stdout: {}", e)
            }

            if let Err(e) = stdout.write_all(format!("\n").as_bytes()).await {
                eprintln!("Error writing to stdout: {}", e)
            }

            if let Err(e) = stdout.flush().await {
                eprintln!("Error flushing stdout: {}", e)
            }
        }
    }}
}

/// Macro for async std output (without \n)
macro_rules! async_print {
    ($($arg:tt)*) => {{
        async {
            let mut stdout = BufWriter::new(io::stdout());
            if let Err(e) = stdout.write_all(format!($($arg)*).as_bytes()).await {
                eprintln!("Error writing to stdout: {}", e)
            }
            if let Err(e) = stdout.flush().await {
                eprintln!("Error flushing stdout: {}", e)
            }
        }
    }}
}

/// The start of main async function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("add") => match args.get(2).map(String::as_str) {
            Some(s) => {
                async_println!(":: Starting add...").await;
                let profile: Profile = read_config().await?;
                let params = vec![
                    ("loaders", serde_json::to_string(&[profile.loader])?),
                    (
                        "game_versions",
                        serde_json::to_string(&[profile.gameversion])?,
                    ),
                ];
                let client = reqwest::Client::new();
                let modversion = fetch_latest_version(s, &client, &params).await?;
                download_file(&profile.modsfolder, modversion.0, modversion.1, &client).await?
            }

            _ => async_println!(":: Usage: minefetch add <modname>").await,
        },

        Some("profile") => match args.get(2).map(String::as_str) {
            Some("create") => config_create().await?,
            _ => async_println!(":: Usage: minefetch profile <create>").await,
        },

        Some("version") => async_println!(":: {} {}", NAME, PROGRAM_VERSION).await,

        _ => async_println!(":: No arguments provided").await,
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
    let home_dir = home::home_dir().ok_or(":wtf: Couldn't find the home dir")?;
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
    async_print!(":: Press enter to choose mods directory").await;

    press_enter().await?;

    let folder_path = match AsyncFileDialog::new().pick_folder().await {
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
                async_print!(":err: No folder with such name").await;
                return Ok(());
            }
            buffer.trim().to_string()
        }
    };

    let modversion = ainput(":: Type a Minecraft version: ").await?;

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

    let selected_value = match Select::new(":: Choose a loader\n", choices)
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
        Err(_) => {
            async_println!(":err: Why did you do that?").await;
            std::process::exit(0)
        }
    };

    let name = ainput(":: What should this profile be called? ").await?;

    let mut current_config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => Config::default(),
    };

    let new_profile = Profile {
        active: true,
        name: name,
        modsfolder: folder_path,
        gameversion: modversion,
        loader: selected_value,
        hash: generate_hash().await?,
    };

    for obj in current_config.profile.iter_mut() {
        obj.active = false;
    }

    current_config.profile.push(new_profile);

    let string_toml = toml::to_string(&current_config)?;

    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Press enter to continue functionality
async fn press_enter() -> Result<(), tokio::io::Error> {
    let mut stdin = io::stdin();

    let mut buffer = [0u8; 1];

    stdin.read_exact(&mut buffer).await?;

    Ok(())
}

/// Reads user input and returns String
async fn ainput(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());

    async_print!("{}", prompt).await;
    reader.read_line(&mut buffer).await?;

    Ok(buffer.trim().to_string())
}
