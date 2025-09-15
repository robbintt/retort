use ::llm::chat::ChatMessage;
use clap::Parser;
use futures::StreamExt;
use std::fs;
use std::io::{stdout, Write};

pub mod cli;
pub mod config;
pub mod db;
pub mod hooks;
pub mod llm;
pub mod prompt;

use cli::{Cli, Command, TagSubcommand};
use hooks::HookManager;

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
                    // For now, just show prepared context. Inheritance is in a later phase.
                    let stage = db::get_context_stage(&conn, "default")?;
                    println!("Prepared Context (for next message):");
                    if !stage.read_write_files.is_empty() {
                        println!("  Read-Write:");
                        for file in &stage.read_write_files {
                            println!("    - {}", file);
                        }
                    }
                    if !stage.read_only_files.is_empty() {
                        println!("  Read-Only:");
                        for file in &stage.read_only_files {
                            println!("    - {}", file);
                        }
                    }
                    if stage.read_write_files.is_empty() && stage.read_only_files.is_empty() {
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
            Command::Profile { active_chat } => {
                if let Some(tag) = active_chat {
                    db::set_active_chat_tag(&conn, &tag)?;
                    println!("Set active chat tag to: {}", tag);
                } else {
                    let profile = db::get_profile_by_name(&conn, "default")?;
                    println!("Active Profile: {}", profile.name);
                    println!(
                        "  active_chat_tag: {}",
                        profile.active_chat_tag.as_deref().unwrap_or("None")
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
                ignore_inherited_stage: _,
            } => {
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

                const PROMPT_METADATA: &str = r#"{"system_prompt": "aider_default"}"#;

                // Add user message
                let user_message_id =
                    db::add_message(&conn, parent_id, "user", &prompt, Some(PROMPT_METADATA))?;
                println!("Added user message with ID: {}", user_message_id);

                // --- Prompt Assembly ---
                // 1. Load file context
                let prepared_stage = db::get_context_stage(&conn, "default")?;
                let mut read_write_files = Vec::new();
                for path in &prepared_stage.read_write_files {
                    let content = fs::read_to_string(path)?;
                    read_write_files.push((path.clone(), content));
                }

                let mut read_only_files = Vec::new();
                for path in &prepared_stage.read_only_files {
                    let content = fs::read_to_string(path)?;
                    read_only_files.push((path.clone(), content));
                }

                // 2. Print context view for user
                println!("---");
                println!("CONTEXT (for this message):");
                println!("Prepared:");
                if !read_write_files.is_empty() {
                    println!("  Read-Write:");
                    for (path, _) in &read_write_files {
                        println!("    - {}", path);
                    }
                }
                if !read_only_files.is_empty() {
                    println!("  Read-Only:");
                    for (path, _) in &read_only_files {
                        println!("    - {}", path);
                    }
                }
                if read_write_files.is_empty() && read_only_files.is_empty() {
                    println!("  (empty)");
                }
                println!("---");

                // 3. Get conversation history to build prompt
                let history = db::get_conversation_history(&conn, user_message_id)?;

                let (cur_messages, done_messages) = if let Some(last) = history.last() {
                    (vec![last.clone()], history[..history.len() - 1].to_vec())
                } else {
                    (Vec::new(), Vec::new())
                };

                let llm_messages_for_prompt = prompt::build_prompt_messages(
                    done_messages,
                    cur_messages,
                    &read_write_files,
                    &read_only_files,
                )?;

                // Convert to LLM ChatMessage format
                let llm_messages: Vec<ChatMessage> = llm_messages_for_prompt
                    .iter()
                    .map(|msg| {
                        if msg.role == "user" || msg.role == "system" {
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
                    let mut stream = llm::get_response_stream(&llm_messages).await?;
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
                    llm::get_response(&llm_messages).await?
                };

                hook_manager.run_post_send_hooks(&assistant_response)?;

                let assistant_message_id = db::add_message(
                    &conn,
                    Some(user_message_id),
                    "assistant",
                    &assistant_response,
                    Some(PROMPT_METADATA),
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
