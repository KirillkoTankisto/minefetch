/*
 __  __ _            _____    _       _          _    ____ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__      / \  |  _ \_ _|
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \    / _ \ | |_) | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  / ___ \|  __/| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| /_/   \_\_|  |___|

*/

// Internal modules
use crate::async_println;
use crate::consts::USER_AGENT;
use crate::downloader::download_file;
use crate::json;
use crate::mfio::{ainput, select};
use crate::profile::{get_locks, remove_locked_ones};
use crate::structs::{
    Dependency, Hash, MFHashMap, Object2, Search, Version, VersionsList, WorkingProfile,
};
use crate::utils::{get_hashes, remove_mods_by_hash};

// External crates
use reqwest::Client;

/// Returns filename, URL, and optional dependencies.
pub async fn fetch_latest_version(
    modname: &String,
    working_profile: &WorkingProfile,
) -> Result<(String, String, Option<Vec<Dependency>>), Box<dyn std::error::Error + Send + Sync>> {
    // Set the parameters for the URL
    let params = &[
        (
            "loaders",
            &serde_json::to_string(&[&working_profile.profile.loader])?,
        ),
        (
            "game_versions",
            &serde_json::to_string(&[&working_profile.profile.gameversion])?,
        ),
    ];

    // Construct the URL with parameters.
    let url = reqwest::Url::parse_with_params(
        &format!("https://api.modrinth.com/v2/project/{}/version", modname),
        params,
    )?;

    // Send the request.
    let response = working_profile
        .client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .text()
        .await?;

    // Parse the response.
    let parsed: VersionsList =
        serde_json::from_str(&response).map_err(|_| "Cannot find such mod")?;

    // Get the first version.
    let version = parsed.get(0).ok_or("No versions available")?;

    // Search for locks
    let locks = match get_locks(&working_profile.profile).await {
        Ok(locks) => locks,
        Err(_) => Vec::new(),
    };

    // Check if this mod is in locks or not
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

    // Return all info about mod version
    Ok((
        file.filename.clone(),
        file.url.clone(),
        version.dependencies.clone(),
    ))
}

/// Mod search
pub async fn search_mods(
    query: &String,
    working_profile: &WorkingProfile,
) -> Result<Vec<(String, String, Option<Vec<Dependency>>)>, Box<dyn std::error::Error + Send + Sync>>
{
    // Set facets
    let facets = json!([
        [format!("categories:{}", working_profile.profile.loader)],
        [format!("versions:{}", working_profile.profile.gameversion)],
        ["project_type:mod"],
    ]);

    // Set parameters
    let params: &[(&str, String)] = &[("query", query.to_string()), ("facets", facets.to_string())];

    // Parse the URL
    let url = reqwest::Url::parse_with_params("https://api.modrinth.com/v2/search", params)?;

    // Send the request
    let response = working_profile
        .client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .text()
        .await?;

    // Parse the response
    let parsed: Search = serde_json::from_str(&response)?;

    // Check if there's no hits
    if parsed.hits.is_empty() {
        return Err("No hits".into());
    }

    // Print all hits
    for number in (0..parsed.hits.len()).rev() {
        if let Some(hit) = parsed.hits.get(number) {
            async_println!("[{}] {}", number + 1, hit.title).await;
        }
    }

    // Parse the user input
    let selected_string = ainput(":out: Select mods to install: ").await?;
    let selected_string = selected_string.split(' ');

    // Create a selected number list
    let mut numbers: Vec<usize> = Vec::new();

    // Parse the numbers
    for object in selected_string {
        numbers.push(
            match object.parse::<usize>() {
                Ok(number) => number,
                Err(_) => return Err("Failed to parse arguments".into()),
            } - 1,
        );
    }

    // Create a version list
    let mut versions: Vec<(String, String, Option<Vec<Dependency>>)> = Vec::new();

    // Fill up the version list
    for number in numbers {
        let version: (String, String, Option<Vec<Dependency>>) = match parsed.hits.get(number) {
            Some(object) => fetch_latest_version(&object.project_id, working_profile).await?, // Get a mod
            None => return Err("Cannot get such mod".into()), // The number is out of range
        };
        versions.push(version); // Push a version into the list
    }

    // Return a version list
    Ok(versions)
}

/// Updates mods to the latest version
pub async fn upgrade_mods(
    working_profile: &WorkingProfile,
) -> Result<Vec<(String, String, Option<Vec<Dependency>>)>, Box<dyn std::error::Error + Send + Sync>>
{
    // Get hashes from mods' directory
    let hashes = get_hashes(&working_profile.profile.modsfolder).await?;

    // Create a Hash structure to send to the API server
    let hashes = Hash {
        hashes,
        algorithm: "sha1".to_string(),
        loaders: Some(vec![working_profile.profile.loader.to_string()]),
        game_versions: Some(vec![working_profile.profile.gameversion.to_string()]),
    };

    // Transform into json string
    let hashes_send = serde_json::to_string(&hashes)?;

    // Send a request
    let response = working_profile
        .client
        .post("https://api.modrinth.com/v2/version_files/update")
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await
        .map_err(|error| error)?
        .text()
        .await?;

    /*
        Parse the response.
        This is a list of mods which
        includes both updated mods and
        those that have not been changed.
    */

    let mut versions: MFHashMap = serde_json::from_str(&response)?;

    // Search for locks
    let locks: Vec<String> = match get_locks(&working_profile.profile).await {
        Ok(locks) => locks,
        Err(_) => Vec::new(),
    };

    /*
        If locks are empty then do nothing.
        If there're locks then remove them from 'versions' and then return modified version list
    */

    let versions = if locks.is_empty() {
        &mut versions
    } else {
        remove_locked_ones(&mut versions, locks).await?
    };

    /*
        Get the hashes (keys in HashMap) that must be removed from version HashMap.
        It's needed to filter out the mods that weren't updated.
    */

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

    // Remove the hashes that were found above
    for key in &keys_to_remove {
        versions.remove(key);
    }

    // Create a list of updated versions
    let mut new_versions = Vec::new();

    // Create a list of hashes to remove (outdated mods)
    let mut hashes_to_remove = Vec::new();

    // Fill the 'new_versions' and 'hashes_to_remove' lists
    for (hash, version) in versions {
        if let Some(files) = version.files.iter().find(|file| file.primary) {
            new_versions.push((files.filename.clone(), files.url.clone(), None));
            hashes_to_remove.push(hash);
        }
    }

    // If there're mods that have been changed / updated
    if hashes_to_remove.len() != 0 {
        remove_mods_by_hash(&working_profile.profile.modsfolder, &hashes_to_remove).await?;
    };

    // Return the list (it can be empty)
    Ok(new_versions)
}

/// Lists mods in selected profile
pub async fn list_mods(
    working_profile: &WorkingProfile,
) -> Result<(usize, MFHashMap), Box<dyn std::error::Error + Send + Sync>> {
    // Get the hashes
    let hashes = Hash {
        hashes: match get_hashes(&working_profile.profile.modsfolder).await {
            Ok(hashes) => hashes,
            Err(_) => return Err("There are no mods yet".into()),
        },
        algorithm: "sha1".to_string(),
        loaders: None,
        game_versions: None,
    };

    // Parse into json string
    let hashes_send = serde_json::to_string(&hashes)?;

    // Define the URL
    let url = "https://api.modrinth.com/v2/version_files";

    // Send the post request with json string
    let response = working_profile
        .client
        .post(url)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .body(hashes_send)
        .send()
        .await?
        .text()
        .await?;

    // Parse the response
    let versions: MFHashMap = serde_json::from_str(&response)?;

    // Return the list and its length
    Ok((versions.len(), versions))
}

/// Returns mod's dependencies
pub async fn get_dependencies(
    dependencies: &Vec<Dependency>,
    client: &Client,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    /*
        Create a list of dependencies:
        its name and type (optional or required)
    */
    let mut dependency_list: Vec<(String, String)> = Vec::new();

    // Start fetching info for each dependency
    for dependency in dependencies {
        // This is a URL for current dependency
        let url = format!(
            "https://api.modrinth.com/v2/project/{}",
            dependency.project_id
        );

        // Send a request
        let response = client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?
            .text()
            .await?;

        // Parse the response (extracts the project name)
        let parsed: Object2 = serde_json::from_str(&response)?;

        // Push the name and dependency type into the list
        dependency_list.push((parsed.title, dependency.dependency_type.clone()))
    }

    // Return the list
    Ok(dependency_list)
}

/// Edits a mod
pub async fn edit_mod(
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get the current mod list
    let modlist = list_mods(working_profile).await?;

    /*
        Create a list for a select() function
        where the user chooses what mod to edit
    */
    let mut menu: Vec<(String, Version)> = Vec::new();

    // Push mods into the 'menu' list
    for (_, modification) in modlist.1 {
        menu.push((
            format!(
                "{} ({})",
                modification.name, // Version name
                modification
                    .files
                    .iter()
                    .find(|file| file.primary)
                    .unwrap()
                    .filename  // Version filename
            ),
            modification, // What the program sees
        ));
    }

    // Prompt the user to choose a mod to edit
    let mod_to_edit = select("Select a mod to edit", menu).await?;

    // Parameters for the request
    let params = &[
        (
            "loaders",
            &serde_json::to_string(&[&working_profile.profile.loader])?,
        ),
        (
            "game_versions",
            &serde_json::to_string(&[&working_profile.profile.gameversion])?,
        ),
    ];

    // Create a URL which includes the parameters above
    let url = reqwest::Url::parse_with_params(
        format!(
            "https://api.modrinth.com/v2/project/{}/version",
            mod_to_edit.project_id
        )
        .as_str(),
        params,
    )?;

    // Send the request
    let response = working_profile
        .client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .text()
        .await?;

    // Parse the response
    let parsed: VersionsList = serde_json::from_str(&response)?;

    /*
        Create a list of available versions.
        Then, user chooses which of them to install
    */
    let mut versions_to_install: Vec<(String, &Version)> = Vec::new();

    // Fill the list with all available versions
    for version in &parsed {
        let version_name = if version.id == mod_to_edit.id {
            &format!("{} (Installed)", version.name) // If mod is installed
        } else {
            &version.name // if not
        };

        // Push the version name and a version struct
        versions_to_install.push((version_name.clone(), version));
    }

    // Prompt the user
    let version_to_install = select("Choose a version to install", versions_to_install).await?;

    // Check if the selected mod version equals to already installed one
    if version_to_install.name == mod_to_edit.name {
        return Err("This mod is already installed".into());
    }

    // Get the filename of selected mod version
    let filename = &version_to_install
        .files
        .iter()
        .find(|file| file.primary)
        .unwrap()
        .filename;

    // Download the selected mod version
    download_file(
        &working_profile.profile.modsfolder,
        filename,
        &version_to_install
            .files
            .iter()
            .find(|file| file.primary)
            .unwrap()
            .url,
        &working_profile.client,
    )
    .await?;

    // Print the text
    async_println!(":out: Downloaded {filename}").await;

    // Delete the old mod
    remove_mods_by_hash(
        &working_profile.profile.modsfolder,
        &vec![
            &mod_to_edit
                .files
                .iter()
                .find(|file| file.primary)
                .unwrap()
                .hashes
                .sha1,
        ],
    )
    .await?;

    // Print the text
    async_println!(
        ":out: Deleted {}",
        &mod_to_edit
            .files
            .iter()
            .find(|file| file.primary)
            .unwrap()
            .filename
    )
    .await;

    // Success
    Ok(())
}
