use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

// CLI tests with fences are in cli_fence.rs because they break ai pair programming more often.

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(dst.as_ref())?;
    for entry in fs::read_dir(src.as_ref())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

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
        let user_msg_id = retort::db::add_message(&conn, None, "user", "Hello user", None)?;
        let assistant_msg_id = retort::db::add_message(
            &conn,
            Some(user_msg_id),
            "assistant",
            "Hello assistant",
            None,
        )?;
        retort::db::set_chat_tag(&conn, "test-chat", assistant_msg_id)?;

        // another conversation, no user message. Preview should be the assistant message.
        let assistant_msg_id_2 = retort::db::add_message(
            &conn,
            None,
            "assistant",
            "Standalone assistant message",
            None,
        )?;
        retort::db::set_chat_tag(&conn, "another-chat", assistant_msg_id_2)?;
    }

    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("list").env("HOME", home_dir);

    // Assertions
    // Note: The order is descending by creation date, so message 3 comes first.
    let header1 = "ID    Tag                  Last User Message";
    let header2 = "----- -------------------- ----------------------------------------------------------------------";
    let expected_output1 = "3     another-chat         Standalone assistant message";
    let expected_output2 = "2     test-chat            Hello user";

    cmd.assert().success().stdout(predicate::str::diff(format!(
        "{}\n{}\n{}\n{}\n",
        header1, header2, expected_output1, expected_output2
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
        let u1 = retort::db::add_message(&conn, None, "user", "User message 1", None)?;
        let a1 =
            retort::db::add_message(&conn, Some(u1), "assistant", "Assistant message 1", None)?;
        retort::db::set_chat_tag(&conn, "chat1", a1)?;
    }

    let expected = "[user]\nUser message 1\n---\n[assistant]\nAssistant message 1\n";

    // Test 1: history by implicit tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history").arg("chat1").env("HOME", home_dir);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(expected));

    // Test 2: history by explicit tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history")
        .arg("-t")
        .arg("chat1")
        .env("HOME", home_dir);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(expected));

    // Test 3: history by explicit message ID
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history").arg("-m").arg("2").env("HOME", home_dir);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(expected));

    // Test 4: history with active tag
    Command::cargo_bin("retort")?
        .arg("profile")
        .arg("--active-chat")
        .arg("chat1")
        .env("HOME", home_dir)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history").env("HOME", home_dir);
    cmd.assert()
        .success()
        .stdout(predicate::str::diff(expected));

    // Test 5: nonexistent tag
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history").arg("nonexistent").env("HOME", home_dir);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Tag 'nonexistent' not found."));

    // Test 6: nonexistent ID
    let mut cmd = Command::cargo_bin("retort")?;
    cmd.arg("history")
        .arg("-m")
        .arg("999")
        .env("HOME", home_dir);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Message with ID '999' not found."));

    Ok(())
}

#[test]
fn test_send_command() -> Result<()> {
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

    // Copy prompts directory for the test so that the templates can be found
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    copy_dir_all(
        std::path::Path::new(manifest_dir).join("prompts"),
        temp_dir.path().join("prompts"),
    )?;

    // Setup: create a chat and tag it
    let initial_leaf_id;
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let u1 = retort::db::add_message(&conn, None, "user", "user 1", None)?;
        let a1 = retort::db::add_message(&conn, Some(u1), "assistant", "assistant 1", None)?;
        retort::db::set_chat_tag(&conn, "my-chat", a1)?;
        initial_leaf_id = a1;
    }

    // Test 1: retort send --parent <id> "..."
    // Should create a branch from the original assistant message, and NOT update the tag.
    Command::cargo_bin("retort")?
        .arg("send")
        .arg("--parent")
        .arg(initial_leaf_id.to_string())
        .arg("branch prompt")
        .env("HOME", home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success();

    // Verify tag still points to old message
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let tagged_id = retort::db::get_message_id_by_tag(&conn, "my-chat")?.unwrap();
        assert_eq!(tagged_id, initial_leaf_id);
    }

    // Test 2: retort send "..." (using active tag)
    // First, set active tag
    Command::cargo_bin("retort")?
        .arg("profile")
        .arg("--active-chat")
        .arg("my-chat")
        .env("HOME", home_dir)
        .assert()
        .success();

    Command::cargo_bin("retort")?
        .arg("send")
        .arg("continue prompt")
        .env("HOME", home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success();

    // Verify tag points to new message (id 6, since we added 2 in branch test, 2 here)
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let tagged_id = retort::db::get_message_id_by_tag(&conn, "my-chat")?.unwrap();
        assert_eq!(tagged_id, 6);
    }

    // Test 3: retort send --new "..."
    Command::cargo_bin("retort")?
        .arg("send")
        .arg("--new")
        .arg("new prompt")
        .env("HOME", home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success();

    // Verify a new root was created. There should now be 3 leaves.
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let leaves = retort::db::get_leaf_messages(&conn)?;
        assert_eq!(leaves.len(), 3);
    }

    Ok(())
}

#[test]
fn test_tag_command() -> Result<()> {
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

    // Setup: create messages
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        retort::db::add_message(&conn, None, "user", "user 1", None)?;
        retort::db::add_message(&conn, None, "user", "user 2", None)?;
    }

    // Test 1: retort tag set my-tag -m 1
    Command::cargo_bin("retort")?
        .args(["tag", "set", "my-tag", "-m", "1"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Tagged message 1 with 'my-tag'"));

    // Verify tag was set
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let tagged_id = retort::db::get_message_id_by_tag(&conn, "my-tag")?.unwrap();
        assert_eq!(tagged_id, 1);
    }

    // Test 2: Move tag to another message
    Command::cargo_bin("retort")?
        .args(["tag", "set", "my-tag", "-m", "2"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Moved tag 'my-tag' from message 1 to 2.",
        ));

    // Verify tag was moved
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let tagged_id = retort::db::get_message_id_by_tag(&conn, "my-tag")?.unwrap();
        assert_eq!(tagged_id, 2);
    }

    // Test re-tagging the same message
    Command::cargo_bin("retort")?
        .args(["tag", "set", "my-tag", "-m", "2"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Tag 'my-tag' already points to message 2.",
        ));

    // Test `retort tag list`
    let expected_list = "Tag                            Message ID\n------------------------------ ----------\nmy-tag                         2\n";
    Command::cargo_bin("retort")?
        .args(["tag", "list"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_list));

    // Test 3: Tag a non-existent message
    Command::cargo_bin("retort")?
        .args(["tag", "set", "my-tag", "-m", "99"])
        .env("HOME", home_dir)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Message with ID '99' not found."));

    // Test 4: Delete tag
    Command::cargo_bin("retort")?
        .args(["tag", "delete", "my-tag"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted tag 'my-tag' which pointed to message ID 2",
        ));

    // Verify tag was deleted
    {
        let conn = retort::db::setup(db_path.to_str().unwrap())?;
        let tagged_id = retort::db::get_message_id_by_tag(&conn, "my-tag")?;
        assert!(tagged_id.is_none());
    }

    // Test that list is empty
    Command::cargo_bin("retort")?
        .args(["tag", "list"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("No tags found."));

    // Test 5: Delete non-existent tag
    Command::cargo_bin("retort")?
        .args(["tag", "delete", "my-tag"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Tag 'my-tag' not found."));

    Ok(())
}

#[test]
fn test_profile_project_root() -> Result<()> {
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
    let _conn = retort::db::setup(db_path.to_str().unwrap())?;

    let project_dir = tempdir()?;
    let project_path = project_dir.path().canonicalize()?;
    let project_path_str = project_path.to_str().unwrap();

    // Set project root
    Command::cargo_bin("retort")?
        .args(["profile", "--set-project-root", project_path_str])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Set project root to: {}",
            project_path_str
        )));

    // Verify it was set
    Command::cargo_bin("retort")?
        .arg("profile")
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "project_root: {}",
            project_path_str
        )));

    Ok(())
}

#[test]
fn test_context_inheritance() -> Result<()> {
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
    let _conn = retort::db::setup(db_path.to_str().unwrap())?;

    // Copy prompts directory for the test so that the templates can be found
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    copy_dir_all(
        std::path::Path::new(manifest_dir).join("prompts"),
        temp_dir.path().join("prompts"),
    )?;

    // Create some dummy files to stage
    fs::write(temp_dir.path().join("file1.txt"), "content1")?;
    fs::write(temp_dir.path().join("file2.txt"), "content2")?;
    fs::write(temp_dir.path().join("file3.txt"), "content3")?;

    // 1. Stage file1, send msg1. Context should contain file1.
    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args(["stage", "file1.txt"])
        .env("HOME", &home_dir)
        .assert()
        .success();

    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args(["send", "--chat", "inherit-test", "msg1"])
        .env("HOME", &home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Inherited:\n  (empty)\nPrepared:\n  Read-Write:\n    - file1.txt",
        ));

    // After send, prepared stage should be empty.
    Command::cargo_bin("retort")?
        .arg("stage")
        .env("HOME", &home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Prepared Context (for next message):\n  (empty)",
        ));

    // 2. Stage file2, send msg2 continuing chat. Context should have file1 (inherited) and file2 (prepared).
    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args(["stage", "file2.txt"])
        .env("HOME", &home_dir)
        .assert()
        .success();

    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args(["send", "--chat", "inherit-test", "msg2"])
        .env("HOME", &home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Inherited:\n  Read-Write:\n    - file1.txt\nPrepared:\n  Read-Write:\n    - file2.txt",
        ));

    // 3. Stage file3, send msg3 but with --ignore-inherited-stage. Context should only have file3.
    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args(["stage", "file3.txt"])
        .env("HOME", &home_dir)
        .assert()
        .success();

    Command::cargo_bin("retort")?
        .current_dir(temp_dir.path())
        .args([
            "send",
            "--chat",
            "inherit-test",
            "--ignore-inherited-stage",
            "msg3",
        ])
        .env("HOME", &home_dir)
        .env("MOCK_LLM", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Inherited:\n  (empty)\nPrepared:\n  Read-Write:\n    - file3.txt",
        ));

    Ok(())
}

#[test]
fn test_stage_command() -> Result<()> {
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

    // The setup is implicitly called by the command. We just need an empty DB
    // to ensure the context_stages table is created.
    let _conn = retort::db::setup(db_path.to_str().unwrap())?;

    // 1. `retort stage` should be empty initially.
    let expected_empty = "Inherited Context (from active chat):\n  (empty)\n\nPrepared Context (for next message):\n  (empty)\n";
    Command::cargo_bin("retort")?
        .arg("stage")
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_empty));

    // 2. Stage a read-write file.
    Command::cargo_bin("retort")?
        .args(["stage", "file1.txt"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged file1.txt as read-write."));

    // 3. Stage a read-only file.
    Command::cargo_bin("retort")?
        .args(["stage", "file2.txt", "-r"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Staged file2.txt as read-only."));

    // 4. `retort stage` should list both files.
    let expected_list = "Inherited Context (from active chat):\n  (empty)\n\nPrepared Context (for next message):\n  Read-Write:\n    - file1.txt\n  Read-Only:\n    - file2.txt\n";
    Command::cargo_bin("retort")?
        .arg("stage")
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_list));

    // 5. Drop a file.
    Command::cargo_bin("retort")?
        .args(["stage", "file1.txt", "-d"])
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed file1.txt from stage."));

    // 6. `retort stage` should show only the remaining file.
    let expected_final = "Inherited Context (from active chat):\n  (empty)\n\nPrepared Context (for next message):\n  Read-Only:\n    - file2.txt\n";
    Command::cargo_bin("retort")?
        .arg("stage")
        .env("HOME", home_dir)
        .assert()
        .success()
        .stdout(predicate::str::diff(expected_final));

    Ok(())
}
