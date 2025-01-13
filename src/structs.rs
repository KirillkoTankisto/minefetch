use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SiteInfo {
    pub about: String,
    pub documentation: String,
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Deserialize)]
pub struct File {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub file_type: Option<String>,
}

#[derive(Deserialize)]
pub struct Version {
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub id: String,
    pub project_id: String,
    pub author_id: String,
    pub featured: bool,
    pub name: String,
    pub version_number: String,
    pub changelog: String,
    pub changelog_url: Option<String>,
    pub date_published: String,
    pub downloads: u64,
    pub version_type: String,
    pub status: String,
    pub requested_status: Option<String>,
    pub files: Vec<File>,
}

pub type VersionsList = Vec<Version>;

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
    pub active:         bool,
    pub name:           String,
    pub modsfolder:     String,
    pub gameversion:    String,
    pub loader:         String,
    pub hash:           String,
}