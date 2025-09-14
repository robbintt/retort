# retort

A Rust CLI pair programmer.

## Naming

> **Retort**: A glass vessel used for distillation or decomposition by heat. Similar to Alembic, it implies a focused, transformative process.

The name reflects the goal of this tool: to apply a focused, transformative process to a codebase through interaction with an AI.

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

You can run the application using `cargo run --`. All arguments passed after the `--` are sent to the `retort` binary.

### List Chats

To see a list of the current conversations (the latest message in each branch), use the `--show-chats` flag:

```bash
cargo run -- --show-chats
```

### Start a New Chat

To start a new conversation, use the `-p` or `--prompt` flag:

```bash
cargo run -- -p "your prompt here"
```
