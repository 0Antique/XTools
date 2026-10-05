pub mod icon;
pub mod launch;
pub mod pinyin;
pub mod scanner;
pub mod search;

use serde::{Deserialize, Serialize};

/// Persisted application identity is independent of its display name or source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
    pub pinyin: String,
    pub pinyin_initials: String,
    pub source: String,
    pub launch_type: String,
    pub launch_target: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    pub icon_path: Option<String>,
    pub unique_key: String,
    pub launch_count: u64,
    pub last_launched_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub icon_path: Option<String>,
    pub subtitle: Option<String>,
}

impl Application {
    pub(crate) fn new(
        name: String,
        source: &str,
        launch_type: &str,
        target: String,
        unique_key: String,
    ) -> Self {
        use sha2::{Digest, Sha256};
        let id = format!("{:x}", Sha256::digest(unique_key.as_bytes()));
        let (full, initials) = pinyin::precompute(&name);
        Self {
            id,
            normalized_name: pinyin::normalize(&name),
            pinyin: full,
            pinyin_initials: initials,
            name,
            source: source.into(),
            launch_type: launch_type.into(),
            launch_target: target,
            arguments: None,
            working_directory: None,
            icon_path: None,
            unique_key,
            launch_count: 0,
            last_launched_at: None,
        }
    }
}
