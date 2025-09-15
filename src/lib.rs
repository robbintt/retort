use ::llm::chat::ChatMessage;
use clap::Parser;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;

pub mod cli;
pub mod config;
pub mod db;
pub mod hooks;
pub mod llm;
pub mod prompt;

use cli::{Cli, Command, TagSubcommand};
use hooks::HookManager;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FileMetadata {
    pub path: String,
    pub hash: String, // sha256 hash of content
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MessageMetadata {
    pub read_write_files: Vec<FileMetadata>,
    pub read_only_files: Vec<FileMetadata>,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = config::load()?;
    let expanded_path = shellexpand::tilde(&config.database_path);
    let conn = db::setup(&expanded_path)?;

    let mut hook_manager = HookManager::new();
    hook_manager.register(Box::new(hooks::postprocessor::PostprocessorHook {}));

    if let Some(command) = cli.command {
        match command {
            Command::Tag(tag_command) => match tag_command {
                TagSubcommand::Set { tag, message } => {
                    if !db::message_exists(&conn, message)? {
                        anyhow::bail!("Message with ID '{}' not found.", message);
                    }
                    let old_message_id = db::get_message_id_by_tag(&conn, &tag)?;
                    match old_message_id {
                        Some(old_id) if old_id == message => {
                            println!("Tag '{}' already points to message {}.", tag, message);
                        }
                        Some(old_id) => {
                            db::set_chat_tag(&conn, &tag, message)?;
                            println!(
                                "Moved tag '{}' from message {} to {}.",
                                tag, old_id, message
                            );
                        }
                        None => {
                            db::set_chat_tag(&conn, &tag, message)?;
                            println!("Tagged message {} with '{}'", message, tag);
                        }
                    }
                }
                TagSubcommand::Delete { tag } => {
                    if let Some(message_id) = db::delete_chat_tag(&conn, &tag)? {
                        println!(
                            "Deleted tag '{}' which pointed to message ID {}",
                            tag, message_id
                        );
                    } else {
                        println!("Tag '{}' not found.", tag);
                    }
                }
                TagSubcommand::List => {
                    let tags = db::get_all_tags(&conn)?;
                    if tags.is_empty() {
                        println!("No tags found.");
                    } else {
                        println!("{:<30} Message ID", "Tag");
                        println!("{:-<30} {:-<10}", "", "");
                        for tag in tags {
                            println!("{:<30} {}", tag.name, tag.message_id);
                        }
                    }
                }
            },
            Command::Stage(args) => {
                if let Some(file_path) = args.file_path {
                    if args.drop {
                        db::remove_file_from_stage(&conn, "default", &file_path)?;
                        println!("Removed {} from stage.", file_path);
                    } else {
                        db::add_file_to_stage(&conn, "default", &file_path, args.read_only)?;
                        let file_type = if args.read_only {
                            "read-only"
                        } else {
                            "read-write"
                        };
                        println!("Staged {} as {}.", file_path, file_type);
                    }
                } else {
                    // 1. Get inherited context
                    let mut inherited_stage: MessageMetadata = Default::default();
                    if let Some(tag) = db::get_active_chat_tag(&conn)? {
                        if let Some(assistant_message_id) = db::get_message_id_by_tag(&conn, &tag)?
                        {
                            if let Some(user_message_id) =
                                db::get_parent_id(&conn, assistant_message_id)?
                            {
                                if let Some(metadata_json) =
                                    db::get_message_metadata(&conn, user_message_id)?
                                {
                                    if !metadata_json.is_empty() {
                                        inherited_stage = serde_json::from_str(&metadata_json)?;
                                    }
                                }
                            }
                        }
                    }
                    println!("Inherited Context (from active chat):");
                    if !inherited_stage.read_write_files.is_empty() {
                        println!("  Read-Write:");
                        for file in &inherited_stage.read_write_files {
                            println!("    - {}", file.path);
                        }
                    }
                    if !inherited_stage.read_only_files.is_empty() {
                        println!("  Read-Only:");
                        for file in &inherited_stage.read_only_files {
                            println!("    - {}", file.path);
                        }
                    }
                    if inherited_stage.read_write_files.is_empty()
                        && inherited_stage.read_only_files.is_empty()
                    {
                        println!("  (empty)");
                    }

                    // 2. Get prepared context
                    let prepared_stage = db::get_context_stage(&conn, "default")?;
                    println!("\nPrepared Context (for next message):");
                    if !prepared_stage.read_write_files.is_empty() {
                        println!("  Read-Write:");
                        for file in &prepared_stage.read_write_files {
                            println!("    - {}", file);
                        }
                    }
                    if !prepared_stage.read_only_files.is_empty() {
                        println!("  Read-Only:");
                        for file in &prepared_stage.read_only_files {
                            println!("    - {}", file);
                        }
                    }
                    if prepared_stage.read_write_files.is_empty()
                        && prepared_stage.read_only_files.is_empty()
                    {
                        println!("  (empty)");
                    }
                }
            }
            Command::List => {
                let leaves = db::get_leaf_messages(&conn)?;
                println!("{:<5} {:<20} Last User Message", "ID", "Tag");
                println!("{:-<5} {:-<20} {:-<70}", "", "", "");
                for leaf in leaves {
                    let history = db::get_conversation_history(&conn, leaf.id)?;
                    let last_user_message = history.iter().filter(|m| m.role == "user").next_back();

                    let preview_content = last_user_message
                        .map(|m| m.content.clone())
                        .unwrap_or(leaf.content);

                    let truncated_content: String = preview_content.chars().take(70).collect();
                    let one_line_content = truncated_content.replace('\n', " ");

                    let tag_display = leaf.tag.as_deref().unwrap_or("-");

                    // Produces a clean, column-based output that is easy to parse with standard tools.
                    println!("{:<5} {:<20} {}", leaf.id, tag_display, one_line_content);
                }
            }
            Command::Profile {
                active_chat,
                set_project_root,
            } => {
                let mut modified = false;
                if let Some(tag) = active_chat {
                    db::set_active_chat_tag(&conn, &tag)?;
                    println!("Set active chat tag to: {}", tag);
                    modified = true;
                }

                if let Some(path_str) = set_project_root {
                    let path = PathBuf::from(path_str);
                    let canonical_path = path.canonicalize()?;
                    db::set_project_root(
                        &conn,
                        "default",
                        canonical_path.to_str().ok_or_else(|| {
                            anyhow::anyhow!("Failed to convert project root path to string.")
                        })?,
                    )?;
                    println!("Set project root to: {}", canonical_path.to_string_lossy());
                    modified = true;
                }

                if !modified {
                    let profile = db::get_profile_by_name(&conn, "default")?;
                    println!("Active Profile: {}", profile.name);
                    println!(
                        "  active_chat_tag: {}",
                        profile.active_chat_tag.as_deref().unwrap_or("None")
                    );
                    println!(
                        "  project_root: {}",
                        profile.project_root.as_deref().unwrap_or("None")
                    );
                }
            }
            Command::History {
                target,
                tag,
                message,
            } => {
                let leaf_id = match (target, tag, message) {
                    // `retort history`
                    (None, false, false) => {
                        let active_tag = db::get_active_chat_tag(&conn)?.ok_or_else(|| {
                            anyhow::anyhow!(
                                "No active chat tag set. Use `retort profile --active-chat <tag>`."
                            )
                        })?;
                        db::get_message_id_by_tag(&conn, &active_tag)?.ok_or_else(|| {
                            anyhow::anyhow!(
                                "Active chat tag '{}' does not point to a valid message.",
                                active_tag
                            )
                        })?
                    }
                    // `retort history <value>` or `retort history -t <value>`
                    (Some(value), _, false) => db::get_message_id_by_tag(&conn, &value)?
                        .ok_or_else(|| anyhow::anyhow!("Tag '{}' not found.", value))?,
                    // `retort history -m <value>`
                    (Some(value), false, true) => {
                        let id = value.parse::<i64>()?;
                        if !db::message_exists(&conn, id)? {
                            anyhow::bail!("Message with ID '{}' not found.", id);
                        }
                        id
                    }
                    _ => anyhow::bail!("Invalid combination of arguments for history command."),
                };

                let history = db::get_conversation_history(&conn, leaf_id)?;
                for (i, message) in history.iter().enumerate() {
                    println!("[{}]", message.role);
                    println!("{}", message.content);
                    if i < history.len() - 1 {
                        println!("---");
                    }
                }
            }
            Command::Send {
                prompt,
                parent,
                chat,
                new,
                stream,
                no_stream,
                ignore_inherited_stage,
            } => {
                let profile = db::get_profile_by_name(&conn, "default")?;
                let project_root = profile.project_root.map(PathBuf::from);

                let mut parent_id: Option<i64> = None;
                let mut chat_tag_for_update: Option<String> = None;

                if new {
                    // --new: new root message, no tag update
                } else if let Some(id) = parent {
                    // --parent: new branch from id, no tag update
                    parent_id = Some(id);
                } else if let Some(tag) = chat {
                    // --chat: continue from tag, update tag
                    parent_id = db::get_message_id_by_tag(&conn, &tag)?;
                    chat_tag_for_update = Some(tag);
                } else {
                    // default: continue from active tag, or start a new chat if no active tag
                    if let Some(tag) = db::get_active_chat_tag(&conn)? {
                        parent_id = db::get_message_id_by_tag(&conn, &tag)?;
                        chat_tag_for_update = Some(tag);
                    }
                }

                // --- Prompt Assembly ---
                // 1. Get inherited context
                let mut inherited_stage: MessageMetadata = Default::default();
                if let Some(p_id) = parent_id {
                    if !ignore_inherited_stage {
                        // The parent_id (p_id) is the previous assistant's message.
                        // Its parent is the user message from the same turn, which holds the context metadata.
                        if let Some(user_message_id) = db::get_parent_id(&conn, p_id)? {
                            if let Some(metadata_json) =
                                db::get_message_metadata(&conn, user_message_id)?
                            {
                                if !metadata_json.is_empty() {
                                    inherited_stage = serde_json::from_str(&metadata_json)?;
                                }
                            }
                        }
                    }
                }

                // 2. Get prepared context
                let prepared_stage = db::get_context_stage(&conn, "default")?;

                // 3. Merge contexts. Prepared takes precedence.
                let mut final_context_map: HashMap<String, bool> = HashMap::new(); // path -> is_readonly
                for file in &inherited_stage.read_write_files {
                    final_context_map.insert(file.path.clone(), false);
                }
                for file in &inherited_stage.read_only_files {
                    final_context_map.insert(file.path.clone(), true);
                }
                for path in &prepared_stage.read_write_files {
                    final_context_map.insert(path.clone(), false);
                }
                for path in &prepared_stage.read_only_files {
                    final_context_map.insert(path.clone(), true);
                }

                // 4. Load file contents and prepare for prompt, and build metadata
                let mut read_write_files_prompt = Vec::new();
                let mut read_only_files_prompt = Vec::new();
                let mut metadata = MessageMetadata::default();

                let mut paths: Vec<String> = final_context_map.keys().cloned().collect();
                paths.sort(); // Sort for consistent order in prompt

                for path in paths {
                    let is_readonly = *final_context_map.get(&path).unwrap();
                    let content = fs::read_to_string(&path)?;
                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    let hash = format!("{:x}", hasher.finalize());

                    let file_metadata = FileMetadata {
                        path: path.clone(),
                        hash,
                    };

                    if is_readonly {
                        read_only_files_prompt.push((path, content));
                        metadata.read_only_files.push(file_metadata);
                    } else {
                        read_write_files_prompt.push((path, content));
                        metadata.read_write_files.push(file_metadata);
                    }
                }

                // 5. Print context view for user
                println!("---");
                println!("CONTEXT (for this message):");

                let mut sorted_paths: Vec<String> = final_context_map.keys().cloned().collect();
                sorted_paths.sort();

                let mut final_rw: Vec<String> = Vec::new();
                let mut final_ro: Vec<String> = Vec::new();

                for path in &sorted_paths {
                    if *final_context_map.get(path).unwrap() {
                        final_ro.push(path.clone());
                    } else {
                        final_rw.push(path.clone());
                    }
                }

                if !final_rw.is_empty() {
                    println!("  Read-Write:");
                    for path in &final_rw {
                        println!("    - {}", path);
                    }
                }
                if !final_ro.is_empty() {
                    println!("  Read-Only:");
                    for path in &final_ro {
                        println!("    - {}", path);
                    }
                }
                if final_rw.is_empty() && final_ro.is_empty() {
                    println!("  (empty)");
                }
                println!("---");

                let metadata_json = serde_json::to_string(&metadata)?;

                // Add user message with metadata
                let user_message_id =
                    db::add_message(&conn, parent_id, "user", &prompt, Some(&metadata_json))?;
                println!("Added user message with ID: {}", user_message_id);

                // 6. Get conversation history to build prompt
                let history = db::get_conversation_history(&conn, user_message_id)?;

                let (cur_messages, done_messages) = if let Some(last) = history.last() {
                    (vec![last.clone()], history[..history.len() - 1].to_vec())
                } else {
                    (Vec::new(), Vec::new())
                };

                let mut llm_messages_for_prompt = prompt::build_prompt_messages(
                    done_messages,
                    cur_messages,
                    &read_write_files_prompt,
                    &read_only_files_prompt,
                )?;

                let system_prompt = if !llm_messages_for_prompt.is_empty()
                    && llm_messages_for_prompt[0].role == "system"
                {
                    Some(llm_messages_for_prompt.remove(0).content)
                } else {
                    None
                };

                // Convert to LLM ChatMessage format
                let llm_messages: Vec<ChatMessage> = llm_messages_for_prompt
                    .iter()
                    .map(|msg| {
                        if msg.role == "user" {
                            ChatMessage::user().content(msg.content.clone()).build()
                        } else {
                            ChatMessage::assistant()
                                .content(msg.content.clone())
                                .build()
                        }
                    })
                    .collect();

                // Get LLM response
                let use_stream = if stream {
                    true
                } else if no_stream {
                    false
                } else {
                    config.stream.unwrap_or(false)
                };

                let assistant_response = if use_stream {
                    let mut stream = llm::get_response_stream(&llm_messages, system_prompt).await?;
                    let mut full_response = String::new();
                    while let Some(result) = stream.next().await {
                        let text_chunk = result?;
                        full_response.push_str(&text_chunk);
                        print!("{}", text_chunk);
                        stdout().flush()?;
                    }
                    println!(); // For a newline after the streaming is done
                    full_response
                } else {
                    llm::get_response(&llm_messages, system_prompt).await?
                };

                hook_manager.run_post_send_hooks(&assistant_response, &project_root)?;

                db::clear_context_stage(&conn, "default")?;

                let assistant_message_id = db::add_message(
                    &conn,
                    Some(user_message_id),
                    "assistant",
                    &assistant_response,
                    None, // Assistant messages don't need metadata
                )?;
                println!("Added assistant message with ID: {}", assistant_message_id);

                // If a chat tag was in play for this operation, update it.
                // This happens for --chat or the active profile tag, but not for --parent or --new.
                if let Some(tag) = chat_tag_for_update {
                    if parent_id.is_none() {
                        println!("Creating new chat with tag '{}'", &tag);
                    }
                    db::set_chat_tag(&conn, &tag, assistant_message_id)?;
                    println!(
                        "Updated tag '{}' to point to message ID {}",
                        tag, assistant_message_id
                    );
                }
            }
        }
    }

    Ok(())
}
