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

To submit a prompt, use the `-p` or `--prompt` flag.

#### Start a New Chat

To start a new conversation, provide a prompt. This creates a new root message.

```bash
retort -p "your prompt here"
```

#### Continue a Chat

You can continue an existing conversation by providing a chat tag or a parent message ID.

```bash
# Continue from the chat tagged 'my-chat'
retort -p "next question" --chat my-chat

# Continue by creating a new branch from message ID 1
retort -p "let's try something different" --parent 1
```

By default, Retort will use the active chat tag set on your profile.

### Listing Chats

To see a list of all current conversations (the latest message in each branch), use the `-l` or `--list-chats` flag.

```bash
retort -l
```

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

Retort uses a profile to manage settings. Currently, this is used for setting the active chat.

```bash
# View the default profile
retort profile

# Set the active chat tag to 'my-chat'
retort profile --active-chat my-chat
```
