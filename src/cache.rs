// Standard libraries
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

// External imports
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;
use tokio::fs::write;
use toml::from_str;
use toml::to_string;

// Internal modules
use crate::structs::Hash;
use crate::utils::get_hashes;
use crate::{
    api::{Anymod, get_mods_from_hash},
    structs::WorkingProfile,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    elements: Vec<Anymod>,
}

/// Creates a new cache file based on the changes
pub async fn cache_profile(
    working_profile: &WorkingProfile,
    mut cache: Cache,
    new_mods: Vec<String>,
    old_mods: Vec<String>,
) -> Result<(), Box<dyn Error>> {
    // Add new mods
    if !new_mods.is_empty() {
        let hash = Hash {
            hashes: new_mods.iter().map(|hash| hash.clone()).collect(),
            algorithm: "sha1".to_string(),
            loaders: None,
            game_versions: None,
        };

        let new_anymods = get_mods_from_hash(working_profile, hash).await?;

        for anymod in new_anymods {
            cache.elements.push(anymod);
        }
    }

    // Delete old mods
    if !old_mods.is_empty() {
        cache
            .elements
            .retain(|element| !old_mods.contains(&&element.hash));
    }

    // Sort the result
    cache
        .elements
        .sort_by_key(|element| element.title.clone().unwrap_or_default());

    // Write to the file
    write_cache(working_profile, cache).await?;

    Ok(())
}

/// Writes cache into the selected profile
pub async fn write_cache(
    working_profile: &WorkingProfile,
    cache: Cache,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(&working_profile.profile.modsfolder).join("cache.toml");
    let data = to_string(&cache)?;

    write(path, data).await?;

    Ok(())
}

/// Reads cache from the selected profile
pub async fn read_cache(working_profile: &WorkingProfile) -> Result<Cache, Box<dyn Error>> {
    let path = Path::new(&working_profile.profile.modsfolder).join("cache.toml");

    let cache: Cache = from_str(&read_to_string(path).await?)?;

    Ok(cache)
}

/// Validates cache in the selected profile and rewrites it if needed
pub async fn validate_cache(working_profile: &WorkingProfile) -> Result<(), Box<dyn Error>> {
    if let Ok(cache) = read_cache(working_profile).await {
        let real_hashes = get_hashes(&working_profile.profile.modsfolder).await?;

        // Create a HashSet for these lists
        let real_set: HashSet<String> = real_hashes.into_iter().collect();
        let cache_set: HashSet<String> = cache
            .elements
            .iter()
            .map(|element| element.hash.clone())
            .collect();

        let new_mods: Vec<String> = real_set.difference(&cache_set).cloned().collect();
        let old_mods: Vec<String> = cache_set.difference(&real_set).cloned().collect();

        if !new_mods.is_empty() || !old_mods.is_empty() {
            cache_profile(working_profile, cache, new_mods, old_mods).await?;
        }
    };

    Ok(())
}

pub async fn list_mods_cached(
    working_profile: &WorkingProfile,
) -> Result<Vec<Anymod>, Box<dyn Error>> {
    validate_cache(working_profile).await?;

    let mods = read_cache(working_profile).await?.elements;

    Ok(mods)
}
