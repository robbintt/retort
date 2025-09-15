use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_send_with_postprocessor_hook() -> Result<()> {
    let temp_dir = tempdir()?;
    // The temp_dir is our project dir, with a git repo.
    // We create a `home` subdir to act as HOME for retort config/db.
    let project_dir = temp_dir.path();
    let home_dir = project_dir.join("home");
    fs::create_dir_all(&home_dir)?;
    let db_path = home_dir.join("test.db");

    // Copy prompts directory for the test so that the templates can be found
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    copy_dir_all(
        std::path::Path::new(manifest_dir).join("prompts"),
        project_dir.join("prompts"),
    )?;

    // Create a config file to point to our test DB
    let config_dir = home_dir.join(".retort");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    fs::write(
        config_path,
        format!("database_path: {}", db_path.to_str().unwrap()),
    )?;

    // Setup DB with a chat to continue from
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let u1 = retort::db::add_message(&conn, None, "user", "start", None)?;
        retort::db::set_chat_tag(&conn, "hook-test", u1)?;
    }

    // Setup git repo in project_dir
    let file_to_change = project_dir.join("test-file.txt");
    fs::write(&file_to_change, "hello world\n")?;

    Command::new("git")
        .current_dir(project_dir)
        .arg("init")
        .assert()
        .success();
    Command::new("git")
        .current_dir(project_dir)
        .args(["config", "user.name", "Test User"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(project_dir)
        .args(["config", "user.email", "test@example.com"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(project_dir)
        .arg("add")
        .arg(".")
        .assert()
        .success();
    Command::new("git")
        .current_dir(project_dir)
        .arg("commit")
        .arg("-m")
        .arg("initial commit")
        .assert()
        .success();

    let mock_response = r#"feat: update test file

This is a commit message.

test-file.txt
<<<<<<< SEARCH
hello world
=======
hello rust
>>>>>>> REPLACE
"#;

    // Run retort send. It should trigger the postprocessor hook.
    Command::cargo_bin("retort")?
        .current_dir(project_dir)
        .arg("send")
        .arg("--chat")
        .arg("hook-test")
        .arg("make a change")
        .env("HOME", &home_dir)
        .env("MOCK_LLM_CONTENT", &mock_response)
        .assert()
        .success();

    // Verify file content change
    let new_content = fs::read_to_string(&file_to_change)?;
    assert_eq!(new_content, "hello rust\n");

    // Verify git commit
    let output = Command::new("git")
        .current_dir(project_dir)
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .output()?;

    let commit_message = String::from_utf8(output.stdout)?;
    assert!(commit_message.starts_with("feat: update test file"));

    Ok(())
}


#[test]
fn test_project_root_enforcement() -> Result<()> {
    // Setup project and home directories
    let project_temp_dir = tempdir()?;
    let project_dir = project_temp_dir.path();
    let home_dir = project_dir.join("home");
    fs::create_dir_all(&home_dir)?;
    let db_path = home_dir.join("test.db");

    // Setup config and db
    let config_dir = home_dir.join(".retort");
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("config.yaml"),
        format!("database_path: {}", db_path.to_str().unwrap()),
    )?;
    let _conn = retort::db::setup(db_path.to_str().unwrap())?;

    // Setup git repo
    Command::new("git")
        .current_dir(project_dir)
        .arg("init")
        .status()?;
    Command::new("git")
        .current_dir(project_dir)
        .args(["config", "user.name", "Test"])
        .status()?;
    Command::new("git")
        .current_dir(project_dir)
        .args(["config", "user.email", "test@example.com"])
        .status()?;

    // Set project root
    let project_root_str = project_dir.to_str().unwrap();
    Command::cargo_bin("retort")?
        .args(["profile", "--set-project-root", project_root_str])
        .env("HOME", &home_dir)
        .assert()
        .success();

    // Test 1: Write inside project root (should succeed)
    let internal_file = project_dir.join("internal.txt");
    fs::write(&internal_file, "original content")?;
    let mock_response_inside = format!(
        r#"feat: write inside

