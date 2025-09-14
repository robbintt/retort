use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// List all chats
    #[arg(short, long)]
    pub list_chats: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage profiles
    Profile {
        /// Set the active chat tag for the default profile
        #[arg(long)]
        pub active_chat: Option<String>,
    },
    /// Show the history of a chat
    History {
        /// The tag or message ID to show history for. Defaults to the active tag.
        pub target: Option<String>,

        /// Explicitly treat the target as a tag
        #[arg(short, long)]
        pub tag: bool,

        /// Explicitly treat the target as a message ID
        #[arg(short, long)]
        pub message: bool,
    },
    /// Send a prompt to the model
    Send {
        /// The prompt to send
        pub prompt: String,

        /// The parent message ID to continue from. Creates a new branch and does not update any tags.
        #[arg(long, conflicts_with_all = &["new", "chat"])]
        pub parent: Option<i64>,

        /// The chat tag to continue from.
        #[arg(long, conflicts_with = "new")]
        pub chat: Option<String>,

        /// Start a new chat, ignoring the active chat tag.
        #[arg(long)]
        pub new: bool,

        /// Stream the response (overrides config).
        #[arg(long, conflicts_with = "no_stream")]
        pub stream: bool,

        /// Do not stream the response (overrides config).
        #[arg(long)]
        pub no_stream: bool,
    },
}
