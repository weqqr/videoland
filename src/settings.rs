use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub test: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            test: "12345".to_string(),
        }
    }
}

// FIXME: Replace this unwrap-athon with error checking
impl Settings {
    pub fn load_global() -> Self {
        std::fs::read(user_settings_path())
            .map(|data| serde_json::from_slice(&data).unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let data = serde_json::to_string_pretty(self).unwrap();

        let path = user_settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        std::fs::write(user_settings_path(), data).unwrap();
    }
}

fn user_settings_path() -> PathBuf {
    PathBuf::from("videoland.json")
}
