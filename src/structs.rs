/*
 ____  _                   _
/ ___|| |_ _ __ _   _  ___| |_ _   _ _ __ ___  ___
\___ \| __| '__| | | |/ __| __| | | | '__/ _ \/ __|
 ___) | |_| |  | |_| | (__| |_| |_| | | |  __/\__ \
|____/ \__|_|   \__,_|\___|\__|\__,_|_|  \___||___/

*/

// External crates
use reqwest::Client;
use serde::{Deserialize, Serialize};

// Standard imports
use std::collections::HashMap;

/// The structure that contains the hashes
/// (The program uses only sha1 so there's only one value)
#[derive(Deserialize, Clone)]
pub struct Hashes {
    pub sha1: String,
}

/// File info
/// (hashes, url, filename, primary or not)
#[derive(Deserialize, Clone)]
pub struct File {
    pub hashes: Hashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

/// Version info
/// (name, files, dependencies, project id, version id)
#[derive(Deserialize, Clone)]
pub struct Version {
    pub name: String,
    pub files: Vec<File>,
    pub dependencies: Option<Vec<Dependency>>,
    pub project_id: String,
    pub id: String,
}

/// Version List type
pub type VersionsList = Vec<Version>;

/// Hashmap type
pub type MFHashMap = HashMap<String, Version>;

/// Config structure
#[derive(Deserialize, Serialize)]
pub struct Config {
    pub profile: Vec<Profile>,
}

/// Default implementation for Config
/// (Creates a new empty config)
impl Default for Config {
    fn default() -> Self {
        Config {
            profile: Vec::new(),
        }
    }
}

/// Profile structure
#[derive(Deserialize, Serialize)]
pub struct Profile {
    pub active: bool,
    pub name: String,
    pub modsfolder: String,
    pub gameversion: String,
    pub loader: String,
    pub hash: String,
}

/// Structure of the search response
#[derive(Deserialize)]
pub struct Search {
    pub hits: Vec<Object>,
}

/// The project structure from the search response
#[derive(Deserialize)]
pub struct Object {
    pub project_id: String,
    pub title: String,
}

/// Project structure
#[derive(Deserialize)]
pub struct Project {
    pub title: String,
}

/// Hash structure
/// (hashes, algorithm, loaders, game versions)
#[derive(Serialize)]
pub struct Hash {
    pub hashes: Vec<String>,
    pub algorithm: String,
    pub loaders: Option<Vec<String>>,
    pub game_versions: Option<Vec<String>>,
}

/// Dependency structure (project id and dependency type)
#[derive(Deserialize, Clone)]
pub struct Dependency {
    pub project_id: String,
    pub dependency_type: String,
}

/// Locks structure
#[derive(Deserialize, Serialize)]
pub struct Locks {
    pub lock: Vec<String>,
}

/// Working profile structure
/// (Profile and Client)
pub struct WorkingProfile {
    pub profile: Profile,
    pub client: Client,
}
