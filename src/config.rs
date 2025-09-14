use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: "~/.retort/data/retort.db".to_string(),
        }
    }
}

pub fn load() -> Result<Config> {
    let config_path_str = "~/.retort/config.yaml";
    let expanded_config_path = shellexpand::tilde(config_path_str);
    let config_path = Path::new(expanded_config_path.as_ref());

    let config = if config_path.exists() {
        let file_contents = fs::read_to_string(config_path)?;
        serde_yaml::from_str(&file_contents)?
    } else {
        Config::default()
    };

    let expanded_db_path = shellexpand::tilde(&config.database_path).to_string();

    Ok(Config {
        database_path: expanded_db_path,
    })
}
