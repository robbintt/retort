use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// Internal struct for serialization to avoid breaking changes to the public API
// and to handle DB data format migration gracefully.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
struct PreparedContext {
    read_write_files: Vec<String>,
    read_only_files: Vec<String>,
    dropped_files: Vec<String>,
}

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


        CREATE TABLE IF NOT EXISTS chat_tags (
            tag TEXT PRIMARY KEY NOT NULL,
            message_id INTEGER NOT NULL,
            FOREIGN KEY (message_id) REFERENCES messages (id)
        );

        CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            active_chat_tag TEXT,
            project_root TEXT
        );

        INSERT OR IGNORE INTO profiles (name) VALUES ('default');

        CREATE TABLE IF NOT EXISTS context_stages (
            name TEXT PRIMARY KEY NOT NULL,
            read_write_files TEXT NOT NULL,
            read_only_files TEXT NOT NULL
        );

        INSERT OR IGNORE INTO context_stages (name, read_write_files, read_only_files) VALUES ('default', '[]', '[]');
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
    pub project_root: Option<String>,
}

pub fn get_profile_by_name(conn: &Connection, name: &str) -> Result<Profile> {
    conn.query_row(
        "SELECT name, active_chat_tag, project_root FROM profiles WHERE name = ?1",
        [name],
        |row| {
            Ok(Profile {
                name: row.get(0)?,
                active_chat_tag: row.get(1)?,
                project_root: row.get(2)?,
            })
        },
    )
    .map_err(Into::into)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContextStage {
    pub name: String,
    pub read_write_files: Vec<String>,
    pub read_only_files: Vec<String>,
    pub dropped_files: Vec<String>,
}

pub fn get_context_stage(conn: &Connection, name: &str) -> Result<ContextStage> {
    conn.query_row(
        "SELECT read_write_files, read_only_files FROM context_stages WHERE name = ?1",
        [name],
        |row| {
            let prepared_json: String = row.get(0)?;
            // Try to parse as new format (a JSON object with all file lists)
            if let Ok(prepared) = serde_json::from_str::<PreparedContext>(&prepared_json) {
                return Ok(ContextStage {
                    name: name.to_string(),
                    read_write_files: prepared.read_write_files,
                    read_only_files: prepared.read_only_files,
                    dropped_files: prepared.dropped_files,
                });
            }

            // Fallback to old format (two separate JSON arrays)
            let read_write_files = serde_json::from_str(&prepared_json).unwrap_or_default();
            let ro_json: String = row.get(1)?;
            let read_only_files = serde_json::from_str(&ro_json).unwrap_or_default();

            Ok(ContextStage {
                name: name.to_string(),
                read_write_files,
                read_only_files,
                dropped_files: Vec::new(),
            })
        },
    )
    .map_err(Into::into)
}

pub fn update_context_stage(conn: &Connection, stage: &ContextStage) -> Result<()> {
    let prepared = PreparedContext {
        read_write_files: stage.read_write_files.clone(),
        read_only_files: stage.read_only_files.clone(),
        dropped_files: stage.dropped_files.clone(),
    };
    let prepared_json = serde_json::to_string(&prepared)?;

    // On update, we migrate to the new format by storing everything in the first column
    // and clearing the second, ensuring future reads will use the new format.
    conn.execute(
        "UPDATE context_stages SET read_write_files = ?1, read_only_files = '[]' WHERE name = ?2",
        (prepared_json, &stage.name),
    )?;
    Ok(())
}

pub fn add_file_to_stage(
    conn: &Connection,
    name: &str,
    file_path: &str,
    read_only: bool,
) -> Result<()> {
    let mut stage = get_context_stage(conn, name)?;
    let file_path_string = file_path.to_string();

    // When adding a file, it should be removed from the dropped list.
    stage.dropped_files.retain(|f| f != &file_path_string);

    if read_only {
        // Ensure it's not in the read-write list
        stage.read_write_files.retain(|f| f != &file_path_string);
        // Add to read-only list if not present
        if !stage.read_only_files.contains(&file_path_string) {
            stage.read_only_files.push(file_path_string);
        }
    } else {
        // Ensure it's not in the read-only list
        stage.read_only_files.retain(|f| f != &file_path_string);
        // Add to read-write list if not present
        if !stage.read_write_files.contains(&file_path_string) {
            stage.read_write_files.push(file_path_string);
        }
    }

    update_context_stage(conn, &stage)
}

pub fn get_message_metadata(conn: &Connection, message_id: i64) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT metadata FROM messages WHERE id = ?1")?;
    let mut rows = stmt.query_map([message_id], |row| row.get(0))?;
    if let Some(metadata_result) = rows.next() {
        Ok(metadata_result?)
    } else {
        Ok(None)
    }
}

pub fn get_parent_id(conn: &Connection, message_id: i64) -> Result<Option<i64>> {
    let mut stmt = conn.prepare("SELECT parent_id FROM messages WHERE id = ?1")?;
    let mut rows = stmt.query_map([message_id], |row| row.get(0))?;
    if let Some(parent_id_result) = rows.next() {
        Ok(parent_id_result?)
    } else {
        Ok(None)
    }
}

pub fn set_project_root(conn: &Connection, name: &str, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE profiles SET project_root = ?1 WHERE name = ?2",
        (path, name),
    )?;
    Ok(())
}

pub fn clear_context_stage(conn: &Connection, name: &str) -> Result<()> {
    let stage = ContextStage {
        name: name.to_string(),
        ..Default::default()
    };
    update_context_stage(conn, &stage)
}

pub fn remove_file_from_stage(conn: &Connection, name: &str, file_path: &str) -> Result<()> {
    let mut stage = get_context_stage(conn, name)?;
    let file_path_string = file_path.to_string();

    // Remove from any addition lists.
    stage.read_write_files.retain(|f| f != &file_path_string);
    stage.read_only_files.retain(|f| f != &file_path_string);

    // Add to the dropped list to ensure it's removed from inherited context.
    if !stage.dropped_files.contains(&file_path_string) {
        stage.dropped_files.push(file_path_string);
    }

    update_context_stage(conn, &stage)
}
