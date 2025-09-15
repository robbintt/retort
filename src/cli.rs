use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Parser, Debug)]
pub struct StageArgs {
    /// Path to a file to add or remove from the context stage.
    pub file_path: Option<String>,

    /// Stage the file as read-only.
    #[arg(short = 'r', long, requires = "file_path")]
    pub read_only: bool,

    /// Remove the file from the context stage.
    #[arg(long, short = 'd', requires = "file_path")]
    pub drop: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List all chats
    List,
    /// Manage chat tags
    #[command(subcommand)]
    Tag(TagSubcommand),
    /// Stage files for chat context
    Stage(StageArgs),
    /// Manage profiles
    Profile {
        /// Set the active chat tag for the default profile
        #[arg(long)]
        active_chat: Option<String>,
    },
    /// Show the history of a chat
    History {
        /// The tag or message ID to show history for. Defaults to the active tag.
        target: Option<String>,

        /// Explicitly treat the target as a tag
        #[arg(short, long)]
        tag: bool,

        /// Explicitly treat the target as a message ID
        #[arg(short, long)]
        message: bool,
    },
    /// Send a prompt to the model
    Send {
        /// The prompt to send
        prompt: String,

        /// The parent message ID to continue from. Creates a new branch and does not update any tags.
        #[arg(long, conflicts_with_all = &["new", "chat"])]
        parent: Option<i64>,

        /// The chat tag to continue from.
        #[arg(long, conflicts_with = "new")]
        chat: Option<String>,

        /// Start a new chat, ignoring the active chat tag.
        #[arg(long)]
        new: bool,

        /// Stream the response (overrides config).
        #[arg(long, conflicts_with = "no_stream")]
        stream: bool,

        /// Do not stream the response (overrides config).
        #[arg(long)]
        no_stream: bool,

        /// Ignore the inherited file context from the parent message.
        #[arg(long, short = 'i', conflicts_with = "new")]
        ignore_inherited_stage: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TagSubcommand {
    /// Create or update a tag for a message
    Set {
        /// The tag name
        tag: String,
        /// The message ID to tag
        #[arg(short, long, required = true)]
        message: i64,
    },
    /// Delete a tag
    Delete {
        /// The tag to delete
        tag: String,
    },
    /// List all tags
    List,
}
