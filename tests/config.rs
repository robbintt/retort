use anyhow::Result;
use retort::config::load;
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
