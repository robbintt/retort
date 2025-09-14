use clap::{Args as ClapArgs, Parser, Subcommand};

pub mod config;
pub mod db;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[clap(flatten)]
    prompt_args: PromptArgs,
}

#[derive(ClapArgs, Debug)]
struct PromptArgs {
    /// The prompt to send to the model
    #[arg(short, long)]
    prompt: Option<String>,

    /// The parent message ID to continue from
    #[arg(long)]
    parent: Option<i64>,

    /// The chat tag to continue from.
    #[arg(long)]
    chat: Option<String>,

    /// List all chats
    #[arg(short, long)]
    list_chats: bool,

    /// Show the history of a chat by tag or ID.
    /// If no value is provided, it uses the active chat tag.
    #[arg(short, long, value_name = "TAG_OR_ID", num_args = 0..=1, default_missing_value = None)]
    history: Option<Option<String>>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Manage profiles
    Profile {
        /// Set the active chat tag for the default profile
        #[arg(long)]
        active_chat: Option<String>,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = config::load()?;
    let expanded_path = shellexpand::tilde(&config.database_path);
    let conn = db::setup(&expanded_path)?;

    if let Some(command) = cli.command {
        match command {
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
        }
    } else if let Some(history_target) = cli.prompt_args.history {
        let leaf_id = match history_target {
            // Case: retort -h <value>
            Some(value) => {
                // Heuristic: check for tag first, then fall back to ID.
                if let Some(id_from_tag) = db::get_message_id_by_tag(&conn, &value)? {
                    id_from_tag
                } else {
                    match value.parse::<i64>() {
                        Ok(id) => {
                            if !db::message_exists(&conn, id)? {
                                anyhow::bail!("Message with ID '{}' not found.", id);
                            }
                            id
                        }
                        Err(_) => anyhow::bail!("Tag '{}' not found.", value),
                    }
                }
            }
            // Case: retort -h
            None => {
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
        };

        let history = db::get_conversation_history(&conn, leaf_id)?;
        for (i, message) in history.iter().enumerate() {
            println!("[{}]", message.role);
            println!("{}", message.content);
            if i < history.len() - 1 {
                println!("---");
            }
        }
    } else if cli.prompt_args.list_chats {
        let leaves = db::get_leaf_messages(&conn)?;
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
    } else if let Some(prompt) = cli.prompt_args.prompt {
        // Determine chat tag to use for this operation.
        // The user can specify a tag directly, or we can fall back to the active one.
        let chat_tag_for_update = cli.prompt_args.chat.or(db::get_active_chat_tag(&conn)?);

        // Determine parent_id
        // Priority: --parent > --chat > active_chat_tag
        let mut parent_id: Option<i64> = None;
        if let Some(id) = cli.prompt_args.parent {
            parent_id = Some(id);
        } else if let Some(ref tag) = chat_tag_for_update {
            // Look up the message ID from the tag
            parent_id = db::get_message_id_by_tag(&conn, tag)?;
        }

        // Add user message
        let user_message_id = db::add_message(&conn, parent_id, "user", &prompt)?;
        println!("Added user message with ID: {}", user_message_id);

        // Dummy LLM response
        let assistant_message_id =
            db::add_message(&conn, Some(user_message_id), "assistant", "Ok.")?;
        println!("Added assistant message with ID: {}", assistant_message_id);

        // If a chat tag was in play, update it to point to the new assistant message
        if let Some(tag) = chat_tag_for_update {
            db::set_chat_tag(&conn, &tag, assistant_message_id)?;
            println!(
                "Updated tag '{}' to point to message ID {}",
                tag, assistant_message_id
            );
        }
    }

    Ok(())
}
