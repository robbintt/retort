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

    // From prototype.md:
    // sessions table: To store session metadata.
    // messages table: To store message history, linked to a session.
    // files table: To store file content (read-only/read-write), linked to a session or message.
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions (id)
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL,
            path TEXT NOT NULL,
            content TEXT NOT NULL,
            read_only BOOLEAN NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions (id)
        );
        ",
    )?;

    Ok(conn)
}
