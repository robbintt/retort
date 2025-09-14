use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_list_chats_format_and_logic() -> Result<()> {
    let temp_dir = tempdir()?;
    let home_dir = temp_dir.path();
    let db_path = home_dir.join("test.db");

    // Create a config file to point to our test DB
    let config_dir = home_dir.join(".retort");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    fs::write(
        config_path,
        format!("database_path: {}", db_path.to_str().unwrap()),
    )?;

    // Setup: Create a database with a known conversation
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        // user asks, assistant responds. Preview should be "Hello user".
        let user_msg_id = retort::db::add_message(&conn, None, "user", "Hello user")?;
        let assistant_msg_id =
            retort::db::add_message(&conn, Some(user_msg_id), "assistant", "Hello assistant")?;
        retort::db::set_chat_tag(&conn, "test-chat", assistant_msg_id)?;

        // another conversation, no user message. Preview should be the assistant message.
        let assistant_msg_id_2 =
            retort::db::add_message(&conn, None, "assistant", "Standalone assistant message")?;
        retort::db::set_chat_tag(&conn, "another-chat", assistant_msg_id_2)?;
    }

    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-l").env("HOME", home_dir);

    // Assertions
    // Note: The order is descending by creation date, so message 3 comes first.
    let expected_output1 = "3     another-chat         Standalone assistant message";
    let expected_output2 = "2     test-chat            Hello user";

    cmd.assert().success().stdout(predicate::str::diff(format!(
        "{}\n{}\n",
        expected_output1, expected_output2
    )));

    Ok(())
}

#[test]
fn test_history_command() -> Result<()> {
    let temp_dir = tempdir()?;
    let home_dir = temp_dir.path();
    let db_path = home_dir.join("test.db");

    let config_dir = home_dir.join(".retort");
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");
    fs::write(
        config_path,
        format!("database_path: {}", db_path.to_str().unwrap()),
    )?;

    // Setup DB
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        // user -> assistant. Tagged 'chat1'
        let u1 = retort::db::add_message(&conn, None, "user", "User message 1")?;
        let a1 = retort::db::add_message(&conn, Some(u1), "assistant", "Assistant message 1")?;
        retort::db::set_chat_tag(&conn, "chat1", a1)?;
    }

    // Test 1: history by tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-h").arg("chat1").env("HOME", home_dir);
    let expected = "[user]\nUser message 1\n---\n[assistant]\nAssistant message 1\n";
    cmd.assert().success().stdout(predicate::str::diff(expected));

    // Test 2: history by ID (for a message that is a leaf)
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-h").arg("2").env("HOME", home_dir);
    let expected = "[user]\nUser message 1\n---\n[assistant]\nAssistant message 1\n";
    cmd.assert().success().stdout(predicate::str::diff(expected));

    // Test 3: history with active tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("profile")
        .arg("--active-chat")
        .arg("chat1")
        .env("HOME", home_dir);
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-h").env("HOME", home_dir);
    let expected = "[user]\nUser message 1\n---\n[assistant]\nAssistant message 1\n";
    cmd.assert().success().stdout(predicate::str::diff(expected));

    // Test 4: nonexistent tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-h").arg("nonexistent").env("HOME", home_dir);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Tag 'nonexistent' not found."));

    // Test 5: nonexistent ID
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("-h").arg("999").env("HOME", home_dir);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Message with ID '999' not found."));

    Ok(())
}
