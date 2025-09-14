use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database_path: String,
    #[serde(default)]
    pub stream: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_path: "~/.retort/data/retort.db".to_string(),
            stream: None,
        }
    }
}

pub fn load() -> Result<Config> {
    let config_path_str = "~/.retort/config.yaml";
    let expanded_config_path = shellexpand::tilde(config_path_str);
    let config_path = Path::new(expanded_config_path.as_ref());

    let mut config: Config = if config_path.exists() {
        let file_contents = fs::read_to_string(config_path)?;
        serde_yaml::from_str(&file_contents)?
    } else {
        Config::default()
    };

    config.database_path = shellexpand::tilde(&config.database_path).to_string();

    Ok(config)
}
