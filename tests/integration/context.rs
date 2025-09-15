use anyhow::Result;
use retort::db;
use rusqlite::Connection;

fn setup_in_memory_db() -> Result<Connection> {
    // The existing setup function handles schema creation.
    // Passing ":memory:" to rusqlite creates an in-memory database.
    db::setup(":memory:")
}

#[test]
fn test_context_stage_flow() -> Result<()> {
    let conn = setup_in_memory_db()?;

    // 1. Initial default stage should be empty.
    let stage = db::get_context_stage(&conn, "default")?;
    assert_eq!(stage.name, "default");
    assert!(stage.read_write_files.is_empty());
    assert!(stage.read_only_files.is_empty());

    // 2. Add a read-write file.
    db::add_file_to_stage(&conn, "default", "src/main.rs", false)?;
    let stage = db::get_context_stage(&conn, "default")?;
    assert_eq!(stage.read_write_files, vec!["src/main.rs"]);
    assert!(stage.read_only_files.is_empty());

    // 3. Add a read-only file.
    db::add_file_to_stage(&conn, "default", "README.md", true)?;
    let stage = db::get_context_stage(&conn, "default")?;
    assert_eq!(stage.read_write_files, vec!["src/main.rs"]);
    assert_eq!(stage.read_only_files, vec!["README.md"]);

    // 4. Add a duplicate file - should be ignored.
    db::add_file_to_stage(&conn, "default", "src/main.rs", false)?;
    let stage = db::get_context_stage(&conn, "default")?;
    assert_eq!(stage.read_write_files.len(), 1);

    // 5. Remove a file.
    db::remove_file_from_stage(&conn, "default", "src/main.rs")?;
    let stage = db::get_context_stage(&conn, "default")?;
    assert!(stage.read_write_files.is_empty());
    assert_eq!(stage.read_only_files, vec!["README.md"]);

    // 6. Remove a non-existent file - should do nothing.
    db::remove_file_from_stage(&conn, "default", "src/other.rs")?;
    let stage = db::get_context_stage(&conn, "default")?;
    assert_eq!(stage.read_only_files.len(), 1);

    Ok(())
}
