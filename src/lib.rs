use clap::Parser;

pub mod config;
pub mod db;

/// Retort: An AI pair programmer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The prompt to send to the model, bypassing the prompt-builder
    #[arg(short, long)]
    prompt: Option<String>,

    /// Show current chats
    #[arg(long)]
    show_chats: bool,
}

pub fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let config = config::load()?;
    let expanded_path = shellexpand::tilde(&config.database_path);
    let conn = db::setup(&expanded_path)?;

    if args.show_chats {
        let messages = db::get_leaf_messages(&conn)?;
        for message in messages {
            let truncated_content: String = message.content.chars().take(100).collect();
            println!(
                "[{}] {} (ID: {})",
                message.created_at,
                truncated_content.replace('\n', " "),
                message.id
            );
        }
    } else if let Some(_prompt) = args.prompt {
        // TODO: Handle prompt
    }

    Ok(())
}
