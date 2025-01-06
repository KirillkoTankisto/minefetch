mod structs;

use structs::Config;
use structs::Profile;
use structs::VersionsList;

use tokio::io::AsyncWriteExt;

use rand::distributions::Alphanumeric;
use rand::Rng;

use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if args[1] == "add" {
            if args.len() > 2 {
                println!("Starting add...");
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
                let version = fetch_latest_version(&args[2], &client, &params).await?;
                download_file(&profile.modsfolder, version.0, version.1, &client).await?;
            }
        }
    } else {
        println!("No arguments provided");
    }
    Ok(())
}

// Returns filename and url
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
        .ok_or("No version available")?;

    Ok((latest_parsed.filename.clone(), latest_parsed.url.clone()))
}

// Downloads single file
async fn download_file(
    path: &str,
    filename: String,
    url: String,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(path).join(filename);

    let mut response = client.get(url).send().await?;

    let mut file = tokio::fs::File::create(path).await?;

    while let Some(chunk) = response.chunk().await? {
        file.write(&chunk).await?;
    }

    Ok(())
}

// Generates random 64 char string
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

// Returns single active Profile
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
