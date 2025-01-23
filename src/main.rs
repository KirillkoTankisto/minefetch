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
use rand::distributions::Alphanumeric;
use rand::Rng;
use rfd::AsyncFileDialog;
use serde_json::json;
use serde_json::{self, Value};
use sha1::{Digest, Sha1};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufWriter};

// Internal modules
mod consts;
mod mfio;
mod structs;
use consts::*;
use mfio::*;
use structs::{Config, Hash, MFHashMap, Profile, Search, VersionsList};

/// The start of the main async function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("add") => match args.get(2).map(String::as_str) {
            Some(s) => {
                async_println!(":: Starting add...").await;
                let profile: Profile = read_config().await?;
                let params = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[profile.gameversion])?,
                    ),
                ];
                let client = reqwest::Client::new();
                let modversion = fetch_latest_version(s, &client, &params).await?;
                download_file(&profile.modsfolder, &modversion.0, &modversion.1, &client).await?;
                async_println!(":: Downloaded {} ({})", &s, &modversion.0).await;
            }

            _ => async_println!(":: Usage: minefetch add <modname>").await,
        },

        Some("profile") => match args.get(2).map(String::as_str) {
            Some("create") => config_create().await?,
            Some("delete") => match args.get(3).map(String::as_str) {
                Some("all") => profile_delete_all().await?,
                _ => profile_delete().await?,
            },
            Some("switch") => profile_switch().await?,
            Some("list") => profile_list().await?,
            _ => {
                async_println!(":: Usage: minefetch profile <create|delete|delete all|switch|list>")
                    .await
            }
        },

        Some("version") => async_println!(":: {} {}", NAME, PROGRAM_VERSION).await,

        Some("search") => match args.get(2) {
            Some(query) => {
                let profile: Profile = read_config().await?;
                let facets = json!([
                    [format!("categories:{}", profile.loader)],
                    [format!("versions:{}", profile.gameversion)],
                    ["project_type:mod"],
                ]);
                let fetch_params: Vec<(String, String)> = vec![
                    (
                        "loaders".to_string(),
                        serde_json::to_string(&[profile.loader])?,
                    ),
                    (
                        "game_versions".to_string(),
                        serde_json::to_string(&[profile.gameversion])?,
                    ),
                ];
                let client = reqwest::Client::new();
                let files = mod_search(query, facets, &client, &fetch_params).await?;
                download_multiple_files(files, &profile.modsfolder, &client).await?;
            }
            None => async_println!(":: Usage: minefetch search <query>").await,
        },

        Some("upgrade") | Some("update") => {
            let profile: Profile = read_config().await?;
            let files = upgrade(&profile).await?;
            let client = reqwest::Client::new();
            download_multiple_files(files, &profile.modsfolder, &client).await?;
        }

        Some(_) => async_println!(":: There is no such a command!").await,

        None => async_println!(":: No arguments provided").await,
    }
    Ok(())
}

/// Returns filename and url
async fn fetch_latest_version(
    modname: &str,
    client: &reqwest::Client,
    params: &[(String, String)],
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let params: Vec<(String, String)> =
        params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

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

    let version = parsed.get(0).ok_or("No versions available")?;

    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .ok_or("No primary file found")?;

    Ok((file.filename.clone(), file.url.clone()))
}

/// Mod search
async fn mod_search(
    query: &String,
    facets: Value,
    client: &reqwest::Client,
    fetch_params: &[(String, String)],
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let facets_string = facets.to_string();
    let params = [("query", query.to_string()), ("facets", facets_string)];
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let url = reqwest::Url::parse_with_params("https://api.modrinth.com/v2/search", &params)?;

    let res = client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await?
        .text()
        .await?;

    let parsed: Search = serde_json::from_str(&res)?;
    for i in (0..parsed.hits.len()).rev() {
        async_println!("[{}] {}", i + 1, parsed.hits.get(i).unwrap().title).await
    }

    let selected_string = ainput(":: Select mods to install: ").await?;
    let selected_string = selected_string.split(' ');
    let mut numbers: Vec<usize> = Vec::new();
    for i in selected_string {
        numbers.push(i.parse::<usize>().unwrap() - 1);
    }
    let fetch_params: Vec<(String, String)> = fetch_params
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut version: Vec<(String, String)> = Vec::new();
    for i in numbers {
        let v = match parsed.hits.get(i) {
            Some(a) => fetch_latest_version(&a.project_id, &client, &fetch_params).await?,
            None => return Err("Cannot get such mod".into()),
        };
        version.push(v);
    }
    Ok(version)
}

/// Downloads single file
async fn download_file(
    path: &str,
    filename: &str,
    url: &str,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tokio::fs::create_dir_all(path).await?;

    let path = std::path::Path::new(path).join(&filename);

    let mut response = client.get(url).send().await?;

    let mut file = tokio::fs::File::create(path).await?;

    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
    }

    Ok(())
}

/// Downloads multiple files
async fn download_multiple_files(
    files: Vec<(String, String)>,
    path: &str,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut handles = Vec::new();
    let base_path = Path::new(path);

    for (filename, url) in files {
        let client = client.clone();

        let sanitized_path = PathBuf::from(base_path);

        if !sanitized_path.starts_with(base_path) {
            eprintln!(
                "Potential path traversal attack detected: {:?}",
                sanitized_path
            );
            continue;
        }

        let handle = tokio::spawn(async move {
            async_println!(":: Downloading {}", &filename).await;
            download_file(
                sanitized_path.to_str().unwrap_or_default(),
                &filename,
                &url,
                &client,
            )
            .await
        });

        handles.push(handle);
    }

    for handle in handles {
        if let Err(e) = handle.await {
            eprintln!("Task panicked: {:?}", e);
        }
    }

    Ok(())
}

/// Generates random 64 char string
async fn generate_hash() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
async fn read_config() -> Result<Profile, Box<dyn std::error::Error + Send + Sync>> {
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
        .ok_or_else(|| ":: No active profile found".into())
}

/// Returns full Config
async fn read_full_config() -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = home::home_dir().ok_or(":wtf: Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    let contents = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

/// Creates config file
async fn config_create() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                async_print!(":err: No folder with such name").await;
                return Ok(());
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
        Err(_) => {
            async_println!(":err: Why did you do that?").await;
            exit(0)
        }
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

    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes one selected profile
async fn profile_delete() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            async_println!(":: There's no config yet").await;
            return Ok(());
        }
    };

    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|i| (i.name.clone(), i.hash.clone()))
        .collect();

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
        Ok(selection) => profiles
            .iter()
            .find(|(label, _)| &*label == selection) // Notice the dereference here
            .map(|(_, value)| value.clone())
            .ok_or_else(|| ":err: Cannot translate pretty text to system one")
            .unwrap(),
        Err(_) => {
            async_println!(":err: Why did you do that?").await;
            exit(0)
        }
    };

    async_println!("{}", selected_value).await;

    config
        .profile
        .retain(|profile| profile.hash != selected_value);

    let string_toml = toml::to_string(&config)?;
    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Deletes config file completely
async fn profile_delete_all() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");
    tokio::fs::remove_file(config_path).await?;
    Ok(())
}

/// Switches profile to selected one
async fn profile_switch() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = match read_full_config().await {
        Ok(cfg) => cfg,
        Err(_) => {
            async_println!(":: There's no config yet").await;
            return Ok(());
        }
    };

    let profiles: Vec<(String, String)> = config
        .profile
        .iter()
        .map(|i| (i.name.clone(), i.hash.clone()))
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
        Ok(selection) => profiles
            .iter()
            .find(|(label, _)| &*label == selection) // Notice the dereference here
            .map(|(_, value)| value.clone())
            .ok_or_else(|| ":err: Cannot translate pretty text to system one")
            .unwrap(),
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
    let home_dir = home::home_dir().ok_or("Couldn't find the home dir")?;
    let config_path = home_dir
        .join(".config")
        .join("minefetch")
        .join("config.toml");

    tokio::fs::write(config_path, string_toml).await?;

    Ok(())
}

/// Lists all profiles
async fn profile_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = read_full_config().await?;
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

/// Updates mods to the latest version
async fn upgrade(
    profile: &Profile,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let hashes = get_hashes(&profile.modsfolder).await?;
    let hashes = Hash {
        hashes,
        algorithm: "sha1".to_string(),
        loaders: vec![profile.loader.to_string()],
        game_versions: vec![profile.gameversion.to_string()],
    };
    let hashes_send = serde_json::to_string(&hashes)?;

    let client = reqwest::Client::new();
    let url = "https://api.modrinth.com/v2/version_files/update";
    let res = client
        .post(url)
        .header("User-Agent", "KirillkoTankisto")
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await?
        .text()
        .await?;

    let mut versions: MFHashMap = serde_json::from_str(&res)?;
    let mut keys_to_remove = Vec::new();
    for (_, i) in &versions {
        let file = i
            .files
            .iter()
            .find(|v| v.primary)
            .ok_or("No primary file found")?;
        if hashes.hashes.contains(&file.hashes.sha1) {
            keys_to_remove.push(file.hashes.sha1.clone());
        }
    }

    for key in keys_to_remove {
        versions.remove(&key);
    }

    for (_, i) in &versions {
        async_println!("{}", i.name).await;
    }

    let mut version: Vec<(String, String)> = Vec::new();

    let mut hashes_to_remove = Vec::new();

    for (s, v) in &versions {
        let files = v
            .files
            .iter()
            .find(|v| v.primary)
            .ok_or("No primary file found")?;
        let file: (String, String) = (files.filename.clone(), files.url.clone());
        version.push(file);
        hashes_to_remove.push(s.clone())
    }

    remove_mods_by_hash(&profile.modsfolder, &hashes_to_remove).await?;

    Ok(version)
}

/// Returns Vec<String> of hashes in given path
async fn get_hashes(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let mut entries = tokio::fs::read_dir(path).await?;

    let mut hashes: Vec<String> = Vec::new();

    let mut tasks = vec![];

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            tasks.push(tokio::task::spawn(async move {
                let hash = calculate_sha1(&path).await;
                hash
            }));
        }
    }

    for task in tasks {
        match task.await {
            Ok(Ok(hash)) => hashes.push(hash),
            Ok(Err(e)) => eprintln!("Error processing hash: {e}"),
            Err(e) => eprintln!("Task error: {e}"),
        }
    }
    Ok(hashes)
}

/// Calculates hash of a file
async fn calculate_sha1<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = tokio::fs::File::open(&path).await?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Deletes files in folder with same hash
async fn remove_mods_by_hash(
    modsfolder: &str,
    hashes_to_remove: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut entries = tokio::fs::read_dir(modsfolder).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let file_hash = calculate_sha1(&path).await?;
            if hashes_to_remove.contains(&file_hash) {
                tokio::fs::remove_file(&path).await?;
            }
        }
    }

    Ok(())
}
