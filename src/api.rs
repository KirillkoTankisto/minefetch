/*
 __  __ _            _____    _       _          _    ____ ___
|  \/  (_)_ __   ___|  ___|__| |_ ___| |__      / \  |  _ \_ _|
| |\/| | | '_ \ / _ \ |_ / _ \ __/ __| '_ \    / _ \ | |_) | |
| |  | | | | | |  __/  _|  __/ || (__| | | |  / ___ \|  __/| |
|_|  |_|_|_| |_|\___|_|  \___|\__\___|_| |_| /_/   \_\_|  |___|

*/

use crate::cache::list_mods_cached;
// Internal modules
use crate::consts::USER_AGENT;
use crate::downloader::download_multiple_mods;
use crate::mfio::select;
use crate::profile::{get_locks, remove_locked_ones, write_lock};
use crate::structs::{
    Dependency, File, Hash, Hit, MFHashMap, Project, ProjectList, Search, VersionsList,
    WorkingProfile,
};
use crate::utils::{get_hashes, remove_mods_by_hash};

// External crates
use reqwest::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use serde_json::json;

// Standard libraries
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub struct Anymod {
    pub title: Option<String>,
    pub project_id: String,
    pub version_name: String,
    pub version_id: String,
    pub filename: String,
    pub hash: String,
    pub url: String,
    pub depends: Option<Vec<Dependency>>,
}

pub fn get_primary(files: &Vec<File>) -> Result<File, Box<dyn Error>> {
    let file = files
        .iter()
        .find(|file| file.primary)
        .ok_or("Couldn't get the primary file")?;

    Ok(file.clone())
}

/// Gets the latest version of the mod by slug or id
pub async fn get_latest_version(
    modname: &String,
    working_profile: &WorkingProfile,
) -> Result<Anymod, Box<dyn std::error::Error>> {
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
    let url = Url::parse_with_params(
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
    let parsed: VersionsList = from_str(&response).map_err(|_| "Cannot find such mod")?;

    // Get the first version.
    let version = parsed.get(0).ok_or("No versions available")?;

    // Search for locks
    let locks = get_locks(&working_profile.profile)
        .await
        .unwrap_or_default();

    let file = get_primary(&version.files)?;

    // Check if this mod is in locks or not
    for lock in locks {
        if file.hashes.sha1 == lock {
            return Err("This mod is locked".into());
        }
    }

    let title = get_projects_name(&working_profile.client, vec![&version.project_id])
        .await?
        .first()
        .ok_or("The project list is empty")?
        .title
        .clone();

    let anymod = Anymod {
        title: Some(title.clone()),
        project_id: version.project_id.clone(),
        version_name: version.name.clone(),
        version_id: version.id.clone(),
        filename: file.filename.clone(),
        hash: file.hashes.sha1.clone(),
        url: file.url.clone(),
        depends: version.dependencies.clone(),
    };

    Ok(anymod)
}

/// Mod search
pub async fn search_mods(
    query: &str,
    working_profile: &WorkingProfile,
) -> Result<Vec<Hit>, Box<dyn Error>> {
    // Set facets
    let facets = json!([
        [format!("categories:{}", working_profile.profile.loader)],
        [format!("versions:{}", working_profile.profile.gameversion)],
        ["project_type:mod"],
    ]);

    // Set parameters
    let params: &[(&str, &str)] = &[("query", &query), ("facets", &facets.to_string())];

    // Parse the URL
    let url = Url::parse_with_params("https://api.modrinth.com/v2/search", params)?;

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
    let parsed: Search = from_str(&response)?;

    // Check if there's no hits
    if parsed.hits.is_empty() {
        return Err("No hits".into());
    }

    Ok(parsed.hits)
}

/// Updates mods to the latest version
pub async fn upgrade_mods(
    working_profile: &WorkingProfile,
) -> Result<(Vec<String>, Vec<Anymod>), Box<dyn Error>> {
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

    let keys_to_remove: Vec<String> = versions
        .iter()
        .map(|(_, b)| get_primary(&b.files).unwrap().hashes.sha1)
        .collect();

    // Remove the hashes that were found above
    for key in &keys_to_remove {
        versions.remove(key);
    }

    let mut new_versions: Vec<Anymod> = Vec::new();

    let mut old_versions: Vec<String> = Vec::new();

    // Fill the 'new_versions' and 'old_versions' lists
    for (hash, version) in versions {
        if let Some(files) = Some(get_primary(&version.files)?) {
            let anymod = Anymod {
                title: None,
                project_id: version.project_id.clone(),
                version_name: version.name.clone(),
                version_id: version.id.clone(),
                filename: files.filename.clone(),
                hash: files.hashes.sha1.clone(),
                url: files.url.clone(),
                depends: version.dependencies.clone(),
            };
            new_versions.push(anymod);
            old_versions.push(hash.clone());
        }
    }

    // Return the list (it can be empty)
    Ok((old_versions, new_versions))
}

/// Lists mods in selected profile
pub async fn list_mods(
    working_profile: &WorkingProfile,
) -> Result<Vec<Anymod>, Box<dyn std::error::Error>> {
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

    let pairs: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<Vec<Dependency>>,
    )> = versions
        .iter()
        .map(|(_, v)| {
            let name = v.name.clone();
            let id = v.project_id.clone();
            let file = v.files.iter().find(|f| f.primary).expect("no primary file");

            (
                name,
                id,
                file.filename.clone(),
                v.id.clone(),
                file.hashes.sha1.clone(),
                file.url.clone(),
                v.dependencies.clone(),
            )
        })
        .collect();

    let project_ids: Vec<&String> = pairs.iter().map(|(_, id, _, _, _, _, _)| id).collect();

    let projects = get_projects_name(&working_profile.client, project_ids).await?;

    let projects_map: HashMap<String, String> = projects
        .into_iter()
        .map(|p| (p.id.clone(), p.title))
        .collect();

    let mut end: Vec<Anymod> = Vec::new();

    for (name, pid, filename, vid, hash, url, depends) in pairs.iter() {
        let anymod = Anymod {
            title: Some(projects_map[pid].clone()),
            version_name: name.clone(),
            version_id: vid.clone(),
            project_id: pid.clone(),
            filename: filename.clone(),
            hash: hash.clone(),
            url: url.clone(),
            depends: depends.clone(),
        };
        end.push(anymod);
    }

    end.sort_by_key(|e| e.title.clone().unwrap_or_default());

    // Return the list and its length
    Ok(end)
}

/// Returns mod's dependencies
pub async fn get_dependencies(
    dependencies: &Vec<Dependency>,
    client: &Client,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
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
        let parsed: Project = serde_json::from_str(&response)?;

        // Push the name and dependency type into the list
        dependency_list.push((parsed.title, dependency.dependency_type.clone()))
    }

    // Return the list
    Ok(dependency_list)
}

pub async fn get_projects_name(
    client: &Client,
    project_id: Vec<&String>,
) -> Result<ProjectList, Box<dyn Error>> {
    // Join all IDs into a comma-separated string
    let ids = json!(project_id);

    // Create a single ("ids", "value1,value2,...") tuple
    let params = &[("ids", ids.to_string())];

    let url = reqwest::Url::parse_with_params("https://api.modrinth.com/v2/projects", params)?;
    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .text()
        .await?;

    let parsed: ProjectList = serde_json::from_str(&response)?;

    Ok(parsed)
}

/// Lists versions for one project
pub async fn list_versions(
    working_profile: &WorkingProfile,
    project: String,
    params: &[(&str, &String)],
) -> Result<Vec<Anymod>, Box<dyn std::error::Error>> {
    let client = working_profile.client.clone();
    let url = reqwest::Url::parse_with_params(
        &format!("https://api.modrinth.com/v2/project/{}/version", &project),
        params,
    )?;

    let response = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let versions: VersionsList = serde_json::from_str(&response)?;

    let mut end: Vec<Anymod> = Vec::new();
    let title = get_projects_name(&client, vec![&versions.first().unwrap().project_id])
        .await?
        .first()
        .unwrap()
        .title
        .clone();
    let project_id = versions.first().unwrap().project_id.clone();
    for version in versions {
        let file = get_primary(&version.files)?;
        let anymod = Anymod {
            title: Some(title.clone()),
            project_id: project_id.clone(),
            version_name: version.name,
            version_id: version.id,
            filename: file.filename.clone(),
            hash: file.hashes.sha1.clone(),
            url: file.url.clone(),
            depends: version.dependencies,
        };
        end.push(anymod);
    }

    Ok(end)
}

/// Edits a mod
pub async fn edit_mod(working_profile: &WorkingProfile) -> Result<(), Box<dyn Error>> {
    // Get the current mod list
    let modlist = list_mods_cached(working_profile).await?;

    /*
        Create a list for a select() function
        where the user chooses what mod to edit
    */
    let mut menu: Vec<(String, Anymod)> = Vec::new();

    // Push mods into the 'menu' list
    for modinfo in modlist {
        menu.push((
            format!(
                "{} ({})",
                modinfo.title.as_ref().unwrap(), // Version name
                modinfo.filename                 // Version filename
            ),
            modinfo, // What the program sees
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

    // Parse the response
    let parsed = list_versions(working_profile, mod_to_edit.project_id.clone(), params).await?;

    /*
        Create a list of available versions.
        Then, user chooses which of them to install
    */
    let mut versions_to_install: Vec<(String, &Anymod)> = Vec::new();

    // Fill the list with all available versions
    for version in &parsed {
        let version_name = if version.version_id == mod_to_edit.version_id {
            &format!("{} (Installed)", version.version_name) // If mod is installed
        } else {
            &version.version_name // if not
        };

        // Push the version name and a version struct
        versions_to_install.push((version_name.clone(), version));
    }

    // Prompt the user
    let version_to_install = select("Choose a version to install", versions_to_install).await?;

    // Check if the selected mod version equals to already installed one
    if version_to_install.version_id == mod_to_edit.version_id {
        return Err("This mod is already installed".into());
    }

    replace_mods(
        vec![&mod_to_edit.hash],
        vec![version_to_install.clone()],
        working_profile,
    )
    .await?;

    // Create a yes / no dialog (lock the mod or not)
    let lock_menu = vec![("Yes".to_string(), true), ("No".to_string(), false)];

    // If user chooses Yes then lock the mod
    if select("Do you want to lock this mod?", lock_menu).await? {
        write_lock(&working_profile.profile, version_to_install.hash.clone()).await?
    }

    // Success
    Ok(())
}

pub async fn replace_mods(
    old_hashes: Vec<&String>,
    new_mods: Vec<Anymod>,
    working_profile: &WorkingProfile,
) -> Result<(), Box<dyn Error>> {
    // Download the new ones
    download_multiple_mods(new_mods, Arc::new(working_profile.clone())).await?;

    // Remove the old mods
    remove_mods_by_hash(&working_profile.profile.modsfolder, &old_hashes).await?;

    Ok(())
}
