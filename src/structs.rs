// External crates
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub struct FileHashes {
    pub sha1: String,
}

#[derive(Deserialize, Clone)]
pub struct File {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[derive(Deserialize)]
pub struct Version {
    pub name: String,
    pub files: Vec<File>,
    pub dependencies: Option<Vec<Dependency>>,
}

pub type VersionsList = Vec<Version>;

pub type MFHashMap = HashMap<String, Version>;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub profile: Vec<Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            profile: Vec::new(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Profile {
    pub active: bool,
    pub name: String,
    pub modsfolder: String,
    pub gameversion: String,
    pub loader: String,
    pub hash: String,
}
#[derive(Deserialize)]
pub struct Search {
    pub hits: Vec<Object>,
}

#[derive(Deserialize)]
pub struct Object {
    pub project_id: String,
    pub title: String,
}

#[derive(Deserialize)]
pub struct Object2 {
    pub title: String,
}

#[derive(Serialize)]
pub struct Hash {
    pub hashes: Vec<String>,
    pub algorithm: String,
    pub loaders: Option<Vec<String>>,
    pub game_versions: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
pub struct Dependency {
    pub project_id: String,
    pub dependency_type: String,
}
