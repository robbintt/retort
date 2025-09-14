use anyhow::Result;
use retort::db;
use rusqlite::Connection;

fn setup_in_memory_db() -> Result<Connection> {
    // The existing setup function handles schema creation.
    // Passing ":memory:" to rusqlite creates an in-memory database.
    db::setup(":memory:")
}

#[test]
fn test_chat_flow() -> Result<()> {
    let conn = setup_in_memory_db()?;

    // 1. Create a root message for a new chat.
    let root_id = db::add_message(&conn, None, "user", "Hello, world!")?;
    assert_eq!(root_id, 1);

    // Verify it's the only leaf.
    let leaves = db::get_leaf_messages(&conn)?;
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].id, root_id);

    // 2. Tag the message to track the conversation.
    db::set_chat_tag(&conn, "test-chat", root_id)?;
    assert_eq!(
        db::get_message_id_by_tag(&conn, "test-chat")?.unwrap(),
        root_id
    );

    // 3. Continue the conversation from the tag.
    let parent_id = db::get_message_id_by_tag(&conn, "test-chat")?.unwrap();
    let child_id = db::add_message(&conn, Some(parent_id), "user", "Tell me more.")?;
    assert_eq!(child_id, 2);

    // The new message should now be the only leaf.
    let leaves = db::get_leaf_messages(&conn)?;
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].id, child_id);

    // 4. Update the tag to point to the new message.
    db::set_chat_tag(&conn, "test-chat", child_id)?;
    assert_eq!(
        db::get_message_id_by_tag(&conn, "test-chat")?.unwrap(),
        child_id
    );

    Ok(())
}
