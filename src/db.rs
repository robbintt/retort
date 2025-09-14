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

        CREATE TABLE IF NOT EXISTS chat_tags (
            tag TEXT PRIMARY KEY NOT NULL,
            message_id INTEGER NOT NULL,
            FOREIGN KEY (message_id) REFERENCES messages (id)
        );

        CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            active_chat_tag TEXT
        );

        INSERT OR IGNORE INTO profiles (name) VALUES ('default');
        ",
    )?;

    Ok(conn)
}

pub struct Tag {
    pub name: String,
    pub message_id: i64,
}

pub struct Leaf {
    pub id: i64,
    pub created_at: String,
    pub content: String,
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub fn get_leaf_messages(conn: &Connection) -> Result<Vec<Leaf>> {
    let mut stmt = conn.prepare(
        "
        SELECT m1.id, m1.created_at, m1.content, ct.tag
        FROM messages m1
        LEFT JOIN chat_tags ct ON m1.id = ct.message_id
        WHERE NOT EXISTS (SELECT 1 FROM messages m2 WHERE m2.parent_id = m1.id)
        ORDER BY m1.created_at DESC, m1.id DESC;
        ",
    )?;

    let messages_iter = stmt.query_map([], |row| {
        Ok(Leaf {
            id: row.get(0)?,
            created_at: row.get(1)?,
            content: row.get(2)?,
            tag: row.get(3)?,
        })
    })?;

    let mut messages = Vec::new();
    for message in messages_iter {
        messages.push(message?);
    }
    Ok(messages)
}

pub fn get_conversation_history(conn: &Connection, leaf_id: i64) -> Result<Vec<HistoryMessage>> {
    let mut stmt = conn.prepare(
        "
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_id, role, content, created_at
            FROM messages
            WHERE id = ?1
            UNION ALL
            SELECT m.id, m.parent_id, m.role, m.content, m.created_at
            FROM messages m
            JOIN ancestors a ON m.id = a.parent_id
        )
        SELECT role, content, created_at FROM ancestors ORDER BY created_at ASC, id ASC;
        ",
    )?;

    let messages_iter = stmt.query_map([leaf_id], |row| {
        Ok(HistoryMessage {
            role: row.get(0)?,
            content: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;

    let mut messages = Vec::new();
    for message in messages_iter {
        messages.push(message?);
    }
    Ok(messages)
}

pub fn add_message(
    conn: &Connection,
    parent_id: Option<i64>,
    role: &str,
    content: &str,
    metadata: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO messages (parent_id, role, content, metadata) VALUES (?1, ?2, ?3, ?4)",
        (parent_id, role, content, metadata),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_message_id_by_tag(conn: &Connection, tag: &str) -> Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT message_id FROM chat_tags WHERE tag = ?1")?;
    let mut rows = stmt.query_map([tag], |row| row.get(0))?;
    if let Some(id_result) = rows.next() {
        Ok(Some(id_result?))
    } else {
        Ok(None)
    }
}

pub fn set_chat_tag(conn: &Connection, tag: &str, message_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO chat_tags (tag, message_id) VALUES (?1, ?2)",
        (tag, message_id),
    )?;
    Ok(())
}

pub fn delete_chat_tag(conn: &Connection, tag: &str) -> Result<Option<i64>> {
    let message_id = get_message_id_by_tag(conn, tag)?;
    if message_id.is_some() {
        conn.execute("DELETE FROM chat_tags WHERE tag = ?1", [tag])?;
    }
    Ok(message_id)
}

pub fn get_all_tags(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare("SELECT tag, message_id FROM chat_tags ORDER BY tag ASC")?;
    let tags_iter = stmt.query_map([], |row| {
        Ok(Tag {
            name: row.get(0)?,
            message_id: row.get(1)?,
        })
    })?;
    let mut tags = Vec::new();
    for tag in tags_iter {
        tags.push(tag?);
    }
    Ok(tags)
}

pub fn get_active_chat_tag(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT active_chat_tag FROM profiles WHERE name = 'default'")?;
    let mut rows = stmt.query_map([], |row| row.get(0))?;
    if let Some(tag_result) = rows.next() {
        Ok(tag_result?)
    } else {
        Ok(None)
    }
}

pub fn set_active_chat_tag(conn: &Connection, tag: &str) -> Result<()> {
    conn.execute(
        "UPDATE profiles SET active_chat_tag = ?1 WHERE name = 'default'",
        [tag],
    )?;
    Ok(())
}

pub fn message_exists(conn: &Connection, id: i64) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM messages WHERE id = ?1")?;
    Ok(stmt.exists([id])?)
}

#[derive(Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub active_chat_tag: Option<String>,
}

pub fn get_profile_by_name(conn: &Connection, name: &str) -> Result<Profile> {
    conn.query_row(
        "SELECT name, active_chat_tag FROM profiles WHERE name = ?1",
        [name],
        |row| {
            Ok(Profile {
                name: row.get(0)?,
                active_chat_tag: row.get(1)?,
            })
        },
    )
    .map_err(Into::into)
}
