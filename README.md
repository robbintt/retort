# retort

A Rust CLI pair programmer.

![A retort in a medieval lab](assets/retort.png)

## Naming

> **Retort**: A glass vessel used for distillation or decomposition by heat. It implies a focused, transformative process.

Retort is manually distilled from [Aider](https://deepwiki.com/Aider-AI/aider), using Aider.

A copy of retort will continue distilling itself, once bootstrapped.


## Building

You can build the binary using Cargo:

```bash
# For a development build (unoptimized)
cargo build

# For a release build (optimized)
cargo build --release
```

The executable will be located at `target/debug/retort` for development builds or `target/release/retort` for release builds.

## Usage

After building with `cargo build`, you can run the application directly.

### Submitting Prompts

To submit a prompt, use the `send` subcommand.

```bash
retort send "your prompt here"
```

By default, this continues the conversation from the active chat tag. This command supports streaming output with the `--stream` flag.

#### Starting a New Chat

Use the `--new` flag to start a completely new conversation, creating a new root message.

```bash
retort send "a totally new idea" --new
```

You can also start a new, named chat by providing a new tag with the `--chat` flag.

```bash
retort send "let's talk about Rust" --chat rust-stuff
```

#### Continuing a Chat

You can explicitly continue an existing conversation by providing a chat tag or create a new branch from a parent message ID.

```bash
# Continue from the chat tagged 'my-chat'
retort send "next question" --chat my-chat

# Create a new branch from message ID 1. This does not update any tags.
retort send "let's try something different" --parent 1
```

By default, Retort will use the active chat tag set on your profile.

### Listing Chats

To see a list of all current conversations (the latest message in each branch), use the `list` subcommand.

```bash
retort list
```

### Managing Tags

You can manually tag messages, which is useful for creating bookmarks or giving meaningful names to important conversation points.

#### Setting a Tag

To create a tag or move an existing tag to a new message, use `tag set`.

```bash
# Tag message ID 1 with 'my-feature'
retort tag set my-feature -m 1
```

Tags are unique. If you set a tag that already exists, it will be moved to the new message ID, and the command will notify you which message it was moved from.

#### Deleting a Tag

To delete a tag, use `tag delete`.

```bash
retort tag delete my-feature
```

The command will output the message ID that the tag was pointing to, so you can re-tag it if you made a mistake.

#### Listing Tags

To see all tags and which message ID they point to, use `tag list`.

```bash
retort tag list
```

### Managing File Context

You can stage files to be included in the context for your next prompt. This allows the model to see the content of your local files.

#### Staging a File

To add a file to the context, use `retort stage`. By default, files are added as "read-write", meaning the model can propose changes to them.

```bash
# Stage a file as read-write
retort stage src/main.rs
```

To stage a file as "read-only", use the `-r` flag. The model will use this file as a reference but will not propose changes to it.

```bash
# Stage a file as read-only
retort stage -r important_logic.rs
```

#### Removing a File from the Stage

To remove a file from the context stage, use the `-d` or `--drop` flag.

```bash
retort stage -d src/main.rs
```

#### Viewing the Staged Context

Running `retort stage` with no arguments shows the current context that will be used for the next message. This is split into two parts:

-   **Inherited Context**: Files that were part of the previous message in the conversation. This context is carried over automatically.
-   **Prepared Context**: Files you have explicitly staged for the *next* message. This is cleared after each message is sent.

### Viewing Chat History

To view the full history of a conversation, use the `history` subcommand.

```bash
# Show history for the tag 'my-chat'
retort history my-chat

# Show history for message ID 2
retort history -m 2

# Show history for the active chat
retort history
```

### Managing Profiles

Retort uses a profile to manage settings, like the active chat and project root.

```bash
# View the default profile
retort profile

# Set the active chat tag to 'my-chat'
retort profile --active-chat my-chat

# Set the project root to the current directory
retort profile --set-project-root .
```

Setting a project root is a safety feature. Retort will not modify any files outside of the specified project root directory.

### Viewing Output

A TUI is useful and possibly in the future.

For now try glow (cli markdown renderer).

```
brew install glow

# view your active chat history, paginated, markdown-rendered, with 80 width.
target/debug/retort history | glow -p -w80
```


