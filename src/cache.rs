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

impl Cache {
    fn new() -> Self {
        return Self {
            elements: Vec::new(),
        };
    }
}

/// Creates a new cache file based on the changes
pub async fn cache_profile(
    working_profile: &WorkingProfile,
    mut cache: Cache,
    new_mods: Option<Vec<String>>,
    old_mods: Option<Vec<String>>,
) -> Result<(), Box<dyn Error>> {
    // Add new mods
    if let Some(new_mods) = new_mods {
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
    if let Some(old_mods) = old_mods {
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

    write(path, to_string(&cache)?).await?;
    Ok(())
}

/// Reads cache from the selected profile
pub async fn read_cache(working_profile: &WorkingProfile) -> Result<Cache, Box<dyn Error>> {
    let path = Path::new(&working_profile.profile.modsfolder).join("cache.toml");
    if let Ok(file) = read_to_string(path).await {
        let parsed: Cache = from_str(&file)?;
        return Ok(parsed);
    }
    Ok(Cache::new())
}

/// Validates cache in the selected profile and rewrites it if needed
pub async fn validate_cache(working_profile: &WorkingProfile) -> Result<(), Box<dyn Error>> {
    if let Ok(real_hashes) = get_hashes(&working_profile.profile.modsfolder).await {
        if let Ok(cache) = read_cache(working_profile).await {
            // Create a HashSet for these lists
            let real_set: HashSet<String> = real_hashes.into_iter().collect();
            let cache_set: HashSet<String> = cache
                .elements
                .iter()
                .map(|element| element.hash.clone())
                .collect();

            // Find which mods were added and which ones were deleted
            let new_mods: Vec<String> = real_set.difference(&cache_set).cloned().collect();
            let old_mods: Vec<String> = cache_set.difference(&real_set).cloned().collect();

            if !new_mods.is_empty() || !old_mods.is_empty() {
                cache_profile(working_profile, cache, Some(new_mods), Some(old_mods)).await?;
            }
        } else {
            cache_profile(working_profile, Cache::new(), Some(real_hashes), None).await?;
        }
    }
    Ok(())
}

pub async fn list_mods_cached(
    working_profile: &WorkingProfile,
) -> Result<Vec<Anymod>, Box<dyn Error>> {
    validate_cache(working_profile).await?;
    Ok(read_cache(working_profile).await?.elements)
}
