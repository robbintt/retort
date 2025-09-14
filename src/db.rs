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

pub struct Message {
    pub id: i64,
    pub created_at: String,
    pub content: String,
}

pub fn get_leaf_messages(conn: &Connection) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "
        SELECT id, created_at, content
        FROM messages m1
        WHERE NOT EXISTS (SELECT 1 FROM messages m2 WHERE m2.parent_id = m1.id)
        ORDER BY m1.created_at DESC;
        ",
    )?;

    let messages_iter = stmt.query_map([], |row| {
        Ok(Message {
            id: row.get(0)?,
            created_at: row.get(1)?,
            content: row.get(2)?,
        })
    })?;

    let mut messages = Vec::new();
    for message in messages_iter {
        messages.push(message?);
    }
    Ok(messages)
}
