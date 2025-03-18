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
use crate::structs::{Dependency, Hash, MFHashMap, Object2, Profile, Search, VersionsList};
use crate::utils::{get_hashes, remove_mods_by_hash};
use reqwest::Client;
use serde_json::{self, Value};

/// Returns filename, url, and optional dependencies.
pub async fn fetch_latest_version(
    modname: &String,
    client: &reqwest::Client,
    params: &[(String, String)],
) -> Result<(String, String, Option<Vec<Dependency>>), Box<dyn std::error::Error + Send + Sync>> {
    // Clone params to a new Vec
    let params: Vec<(String, String)> =
        params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    // Construct the URL with parameters.
    let url = reqwest::Url::parse_with_params(
        &format!("https://api.modrinth.com/v2/project/{}/version", modname),
        &params,
    )?;

    // Send the request.
    let res = client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await?
        .text()
        .await?;

    // Parse the response.
    let parsed: VersionsList = serde_json::from_str(&res).map_err(|_| "Cannot find such mod")?;

    // Get the first version.
    let version = parsed.get(0).ok_or("No versions available")?;

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
) -> Result<Vec<(String, String, Option<Vec<Dependency>>)>, Box<dyn std::error::Error + Send + Sync>>
{
    let facets_string = facets.to_string();
    let params = [("query", query.to_string()), ("facets", facets_string)];
    let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let url = reqwest::Url::parse_with_params("https://api.modrinth.com/v2/search", &params)?;

    let res = match client
        .get(url)
        .header("User-Agent", "KirillkoTankisto")
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => return Err("No internet connection".into()),
    }
    .text()
    .await?;

    let parsed: Search = serde_json::from_str(&res)?;

    if parsed.hits.is_empty() {
        return Err("No hits".into());
    }

    for i in (0..parsed.hits.len()).rev() {
        if let Some(hit) = parsed.hits.get(i) {
            async_println!("[{}] {}", i + 1, hit.title).await;
        }
    }

    let selected_string = ainput(":: Select mods to install: ").await?;
    let selected_string = selected_string.split(' ');
    let mut numbers: Vec<usize> = Vec::new();
    for i in selected_string {
        numbers.push(
            match i.parse::<usize>() {
                Ok(n) => n,
                Err(_) => return Err("Failed to parse arguments".into()),
            } - 1,
        );
    }
    let fetch_params: Vec<(String, String)> = fetch_params
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut version: Vec<(String, String, Option<Vec<Dependency>>)> = Vec::new();
    for i in numbers {
        let v = match parsed.hits.get(i) {
            Some(a) => fetch_latest_version(&a.project_id, &client, &fetch_params).await?,
            None => return Err("Cannot get such mod".into()),
        };
        version.push(v);
    }
    Ok(version)
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
    let url = "https://api.modrinth.com/v2/version_files/update";
    let res = match client
        .post(url)
        .header("User-Agent", "KirillkoTankisto")
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => return Err("No internet connection".into()),
    }
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

    let mut version: Vec<(String, String, _)> = Vec::new();

    let mut hashes_to_remove = Vec::new();

    for (s, v) in &versions {
        let files = v
            .files
            .iter()
            .find(|v| v.primary)
            .ok_or("No primary file found")?;
        let file: (String, String, _) = (files.filename.clone(), files.url.clone(), None);
        version.push(file);
        hashes_to_remove.push(s.clone())
    }

    remove_mods_by_hash(&profile.modsfolder, &hashes_to_remove).await?;

    Ok(version)
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
    let res = match client
        .post(url)
        .header("User-Agent", "KirillkoTankisto")
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await
    {
        Ok(res) => res,
        Err(_) => {
            return Err("No internet connection".into());
        }
    }
    .text()
    .await?;

    let versions: MFHashMap = serde_json::from_str(&res)?;
    Ok((versions.len(), versions))
}

/// Returns dependencies
pub async fn get_dependencies(
    dependencies: &Vec<Dependency>,
    client: &Client,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut list: Vec<(String, String)> = Vec::new();

    for i in dependencies {
        let url = format!("https://api.modrinth.com/v2/project/{}", i.project_id);
        let res = client
            .get(url)
            .header("User-Agent", "KirillkoTankisto")
            .send()
            .await?
            .text()
            .await?;
        let parsed: Object2 = serde_json::from_str(&res)?;
        list.push((parsed.title, i.dependency_type.clone()))
    }
    Ok(list)
}
