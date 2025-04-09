/*
 __  __ _            _____    _       _          _    ____ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__      / \  |  _ \_ _|
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \    / _ \ | |_) | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  / ___ \|  __/| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| /_/   \_\_|  |___|

*/

// Imports
use crate::async_println;
use crate::mfio::ainput;
use crate::profile::{get_locks, remove_locked_ones};
use crate::structs::{Dependency, Hash, MFHashMap, Object2, Profile, Search, VersionsList};
use crate::utils::{get_hashes, remove_mods_by_hash};
use reqwest::Client;
use serde_json::{self, Value};

/// Returns filename, url, and optional dependencies.
pub async fn fetch_latest_version(
    modname: &String,
    client: &reqwest::Client,
    params: &[(String, String)],
    profile: &Profile,
) -> Result<(String, String, Option<Vec<Dependency>>), Box<dyn std::error::Error + Send + Sync>> {
    // Clone params to a new Vec
    let params: Vec<(String, String)> = params
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    // Construct the URL with parameters.
    let url = reqwest::Url::parse_with_params(
        &format!("https://api.modrinth.com/v2/project/{}/version", modname),
        &params,
    )?;

    // Send the request.
    let response = client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await?
        .text()
        .await?;

    // Parse the response.
    let parsed: VersionsList =
        serde_json::from_str(&response).map_err(|_| "Cannot find such mod")?;

    // Get the first version.
    let version = parsed.get(0).ok_or("No versions available")?;

    let locks = get_locks(&profile).await?;

    for lock in locks {
        if version
            .files
            .iter()
            .find(|file| file.primary)
            .unwrap()
            .hashes
            .sha1
            == lock
        {
            return Err("This mod is locked".into());
        }
    }

    // Get the primary file.
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .ok_or("No primary file found")?;

    Ok((
        file.filename.clone(),
        file.url.clone(),
        version.dependencies.clone(),
    ))
}

/// Mod search
pub async fn search_mods(
    query: &String,
    facets: Value,
    client: &reqwest::Client,
    fetch_params: &[(String, String)],
    profile: &Profile,
) -> Result<Vec<(String, String, Option<Vec<Dependency>>)>, Box<dyn std::error::Error + Send + Sync>>
{
    let facets_string = facets.to_string();
    let params = [("query", query.to_string()), ("facets", facets_string)];
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let url = reqwest::Url::parse_with_params("https://api.modrinth.com/v2/search", &params)?;

    let response = client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await?
        .text()
        .await?;

    let parsed: Search = serde_json::from_str(&response)?;

    if parsed.hits.is_empty() {
        return Err("No hits".into());
    }

    for number in (0..parsed.hits.len()).rev() {
        if let Some(hit) = parsed.hits.get(number) {
            async_println!("[{}] {}", number + 1, hit.title).await;
        }
    }

    let selected_string = ainput(":: Select mods to install: ").await?;
    let selected_string = selected_string.split(' ');
    let mut numbers: Vec<usize> = Vec::new();
    for object in selected_string {
        numbers.push(
            match object.parse::<usize>() {
                Ok(number) => number,
                Err(_) => return Err("Failed to parse arguments".into()),
            } - 1,
        );
    }
    let fetch_params: Vec<(String, String)> = fetch_params
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut versions: Vec<(String, String, Option<Vec<Dependency>>)> = Vec::new();
    for number in numbers {
        let version: (String, String, Option<Vec<Dependency>>) = match parsed.hits.get(number) {
            Some(object) => {
                fetch_latest_version(&object.project_id, &client, &fetch_params, &profile).await?
            }
            None => return Err("Cannot get such mod".into()),
        };
        versions.push(version);
    }
    Ok(versions)
}

/// Updates mods to the latest version
pub async fn upgrade_mods(
    profile: &Profile,
) -> Result<Vec<(String, String, Option<Vec<Dependency>>)>, Box<dyn std::error::Error + Send + Sync>>
{
    let hashes = get_hashes(&profile.modsfolder).await?;
    let hashes = Hash {
        hashes,
        algorithm: "sha1".to_string(),
        loaders: Some(vec![profile.loader.to_string()]),
        game_versions: Some(vec![profile.gameversion.to_string()]),
    };
    let hashes_send = serde_json::to_string(&hashes)?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.modrinth.com/v2/version_files/update")
        .header("User-Agent", "KirillkoTankisto")
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await
        .map_err(|error| error)?
        .text()
        .await?;

    let mut versions: MFHashMap = serde_json::from_str(&response)?;

    let versions = remove_locked_ones(&mut versions, get_locks(profile).await?).await?;

    let keys_to_remove: Vec<_> = versions
        .iter()
        .filter_map(|(_, version)| {
            version
                .files
                .iter()
                .find(|file| file.primary)
                .and_then(|file| {
                    hashes
                        .hashes
                        .contains(&file.hashes.sha1)
                        .then(|| file.hashes.sha1.clone())
                })
        })
        .collect();

    for key in &keys_to_remove {
        versions.remove(key);
    }

    let mut new_versions = Vec::new();
    let mut hashes_to_remove = Vec::new();
    for (hash, version) in versions {
        if let Some(files) = version.files.iter().find(|file| file.primary) {
            new_versions.push((files.filename.clone(), files.url.clone(), None));
            hashes_to_remove.push(hash);
        }
    }

    remove_mods_by_hash(&profile.modsfolder, &hashes_to_remove).await?;
    Ok(new_versions)
}

/// Lists mods in selected profile
pub async fn list_mods(
    profile: &Profile,
    client: &reqwest::Client,
) -> Result<(usize, MFHashMap), Box<dyn std::error::Error + Send + Sync>> {
    let hashes = Hash {
        hashes: match get_hashes(&profile.modsfolder).await {
            Ok(hashes) => hashes,
            Err(_) => return Err("There are no mods yet".into()),
        },
        algorithm: "sha1".to_string(),
        loaders: None,
        game_versions: None,
    };
    let hashes_send = serde_json::to_string(&hashes)?;

    let url = "https://api.modrinth.com/v2/version_files";
    let response = client
        .post(url)
        .header("User-Agent", "KirillkoTankisto")
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await?
        .text()
        .await?;

    let versions: MFHashMap = serde_json::from_str(&response)?;
    Ok((versions.len(), versions))
}

/// Returns dependencies
pub async fn get_dependencies(
    dependencies: &Vec<Dependency>,
    client: &Client,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut list: Vec<(String, String)> = Vec::new();

    for dependency in dependencies {
        let url = format!(
            "https://api.modrinth.com/v2/project/{}",
            dependency.project_id
        );
        let response = client
            .get(url)
            .header("User-Agent", "KirillkoTankisto")
            .send()
            .await?
            .text()
            .await?;
        let parsed: Object2 = serde_json::from_str(&response)?;
        list.push((parsed.title, dependency.dependency_type.clone()))
    }
    Ok(list)
}
