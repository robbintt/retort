use ::llm::chat::ChatMessage;
use clap::Parser;
use futures::StreamExt;
use std::io::{stdout, Write};

pub mod cli;
pub mod config;
pub mod db;
pub mod llm;
pub mod prompt;

use cli::{Cli, Command};

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = config::load()?;
    let expanded_path = shellexpand::tilde(&config.database_path);
    let conn = db::setup(&expanded_path)?;

    if let Some(command) = cli.command {
        match command {
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

                // Add user message
                let user_message_id = db::add_message(&conn, parent_id, "user", &prompt)?;
                println!("Added user message with ID: {}", user_message_id);

                // Get conversation history to build prompt
                let history = db::get_conversation_history(&conn, user_message_id)?;

                let (cur_messages, done_messages) = if let Some(last) = history.last() {
                    (vec![last.clone()], history[..history.len() - 1].to_vec())
                } else {
                    (Vec::new(), Vec::new())
                };

                let prompt_str = prompt::build_prompt(done_messages, cur_messages)?;

                let prompt_messages = prompt::split_chat_history_markdown(&prompt_str);

                // Convert to LLM ChatMessage format
                let llm_messages: Vec<ChatMessage> = prompt_messages
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

                let assistant_message_id = db::add_message(
                    &conn,
                    Some(user_message_id),
                    "assistant",
                    &assistant_response,
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
