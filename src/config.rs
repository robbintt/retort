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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;
    use tempfile::tempdir;

    // Mutex to serialize tests that modify environment variables, preventing race conditions.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_load_default_config() -> Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempdir()?;
        env::set_var("HOME", temp_dir.path());

        let config = load()?;
        let expected_path = temp_dir.path().join(".retort/data/retort.db");
        assert_eq!(config.database_path, expected_path.to_str().unwrap());

        Ok(())
    }

    #[test]
    fn test_load_from_yaml() -> Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempdir()?;
        env::set_var("HOME", temp_dir.path());
        let config_dir = temp_dir.path().join(".retort");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.yaml");
        std::fs::write(config_path, "database_path: /tmp/custom.db")?;

        let config = load()?;
        assert_eq!(config.database_path, "/tmp/custom.db");

        Ok(())
    }

    #[test]
    fn test_load_with_tilde_expansion_in_config() -> Result<()> {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempdir()?;
        env::set_var("HOME", temp_dir.path());
        let config_dir = temp_dir.path().join(".retort");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.yaml");
        std::fs::write(config_path, "database_path: ~/db/from_config.db")?;

        let config = load()?;
        let expected_path = temp_dir.path().join("db/from_config.db");
        assert_eq!(config.database_path, expected_path.to_str().unwrap());

        Ok(())
    }
}
