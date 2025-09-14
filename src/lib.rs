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

    /// Show current chats
    #[arg(long)]
    show_chats: bool,
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
    } else if cli.prompt_args.show_chats {
        let messages = db::get_leaf_messages(&conn)?;
        for message in messages {
            let truncated_content: String = message.content.chars().take(100).collect();
            let tag_display = message
                .tag
                .map(|t| format!(" (Tag: {})", t))
                .unwrap_or_else(|| "".to_string());
            println!(
                "[{}] {} (ID: {}){}",
                message.created_at,
                truncated_content.replace('\n', " "),
                message.id,
                tag_display
            );
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
