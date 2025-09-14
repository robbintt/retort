use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub fn setup(db_path_str: &str) -> Result<Connection> {
    let db_path = Path::new(db_path_str);

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;

    // Conversations are stored as a tree of messages.
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT, -- JSON blob for message-specific data
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
            FOREIGN KEY (parent_id) REFERENCES messages (id)
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            message_id INTEGER NOT NULL,
            path TEXT NOT NULL,
            content TEXT NOT NULL,
            read_only BOOLEAN NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
            FOREIGN KEY (message_id) REFERENCES messages (id)
        );
        ",
    )?;

    Ok(conn)
}
