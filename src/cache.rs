use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs::read_to_string;
use tokio::fs::write;
use toml::from_str;
use toml::to_string;

use crate::utils::get_hashes;
use crate::{
    api::{Anymod, list_mods},
    structs::WorkingProfile,
};

#[derive(Serialize, Deserialize)]
pub struct Cache {
    elements: Vec<Anymod>,
}

pub async fn cache_profile(working_profile: &WorkingProfile) -> Result<(), Box<dyn Error>> {
    let elements = list_mods(working_profile).await?;

    let cache = Cache { elements };

    write_cache(working_profile, cache).await?;

    Ok(())
}

pub async fn write_cache(
    working_profile: &WorkingProfile,
    cache: Cache,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(&working_profile.profile.modsfolder).join("cache.toml");
    let data = to_string(&cache)?;

    write(path, data).await?;

    Ok(())
}

pub async fn read_cache(working_profile: &WorkingProfile) -> Result<Cache, Box<dyn Error>> {
    let path = Path::new(&working_profile.profile.modsfolder).join("cache.toml");
    let data = read_to_string(path).await?;

    let parsed: Cache = from_str(&data)?;

    Ok(parsed)
}

pub async fn validate_cache(working_profile: &WorkingProfile) -> Result<(), Box<dyn Error>> {
    if let Ok(cache) = read_cache(working_profile).await {
        let real_hashes = get_hashes(&working_profile.profile.modsfolder).await?;
        let cached_hashes: Vec<&String> = cache.elements.iter().map(|el| &el.hash).collect();
        let mut invalid_caches: Vec<&String> = real_hashes.iter().map(|h| h).collect();

        invalid_caches.append(cached_hashes.clone().as_mut());

        for cached_hash in cached_hashes {
            if real_hashes.contains(cached_hash) {
                invalid_caches.retain(|h| *h != cached_hash);
            }
        }

        dbg!(&invalid_caches);

        if invalid_caches.is_empty() {
            return Ok(());
        }
    };

    cache_profile(working_profile).await?;

    Ok(())
}

pub async fn list_mods_cached(working_profile: &WorkingProfile) -> Result<Vec<Anymod>, Box<dyn Error>> {
    validate_cache(working_profile).await?;

    let mods = read_cache(working_profile).await?.elements;

    Ok(mods)
}
