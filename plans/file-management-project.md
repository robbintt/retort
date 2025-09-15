# Project Plan: File Management and Prompt Context

This plan outlines the work required to implement file management, allowing users to add read-write and read-only files to the chat context, based on the design in `plans/file-management.md`. The implementation will be broken down into testable phases. Each task is designed to be self-contained to facilitate execution by an AI assistant.

The core of this system is a persistent "context stage," which holds lists of files to be included in the prompt. Users will manage this stage via a `retort stage` command. When a message is sent, the staged files are read and injected into the prompt. To ensure context persists across a conversation, the file list (along with content hashes) is stored in the message's metadata. This "inherited context" is then automatically loaded into the stage for the next message in the same chat thread, allowing for a cumulative file context.

## Phase 1: Database and Core Data Structures for Context

This phase focuses on setting up the database schema and data structures for managing a persistent "context stage". This stage will hold file paths that are to be included in prompts.

- [ ] **Task 1.1: Update Database Schema.** In `src/db.rs`, modify the `setup` function to create a new `context_stages` table and populate it with a 'default' entry.

  Specifically, add the following SQL to the `conn.execute_batch` call in `src/db.rs`:
  ```sql
        CREATE TABLE IF NOT EXISTS context_stages (
            name TEXT PRIMARY KEY NOT NULL,
            read_write_files TEXT NOT NULL,
            read_only_files TEXT NOT NULL
        );

        INSERT OR IGNORE INTO context_stages (name, read_write_files, read_only_files) VALUES ('default', '[]', '[]');
  ```

- [ ] **Task 1.2: Define ContextStage Struct.** In `src/db.rs`, define a `ContextStage` struct to represent a row in the new table. It needs `serde` support for serializing the file lists into JSON.

  Add this struct definition to `src/db.rs` and the required `use` statements at the top of the file. You will need to add `serde` and `serde_json` to `Cargo.toml` in a later step.
  ```rust
  use serde::{Deserialize, Serialize};
  
  #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
  pub struct ContextStage {
      pub name: String,
      pub read_write_files: Vec<String>,
      pub read_only_files: Vec<String>,
  }
  ```

- [ ] **Task 1.3: Implement Context Stage DB Functions.** In `src/db.rs`, add functions to get, update, and modify the 'default' context stage. These functions will handle reading the JSON, modifying the file lists, and writing back to the database.

  - `get_context_stage(conn: &Connection, name: &str) -> Result<ContextStage>`: Fetches the stage by name and deserializes the file lists.
  - `update_context_stage(conn: &Connection, stage: &ContextStage) -> Result<()>`: Serializes the file lists and writes the entire stage object back to the database.
  - `add_file_to_stage(conn: &Connection, name: &str, file_path: &str, read_only: bool) -> Result<()>`: A helper that gets the stage, adds a file path to the correct list (avoiding duplicates), and calls `update_context_stage`.
  - `remove_file_from_stage(conn: &Connection, name: &str, file_path: &str) -> Result<()>`: A helper that gets the stage, removes a file path from both lists, and calls `update_context_stage`.

- [ ] **Task 1.4: Add Database Unit Tests.** Add a new test file `tests/integration/context.rs` (and `mod context;` in `tests/integration/mod.rs`) to test the new database functions.

  The tests should cover:
    1. Setting up an in-memory database.
    2. Calling `add_file_to_stage` to add a read-write file and a read-only file.
    3. Calling `get_context_stage` and asserting that the retrieved data is correct.
    4. Calling `remove_file_from_stage` and asserting that the file is gone.

## Phase 2: CLI for Staging Files

This phase introduces the `retort stage` command and updates the `send` command to support the new context model.

- [x] **Task 2.1: Create `stage` Subcommand.** In `src/cli.rs`, add a `Stage` subcommand. It will handle adding/removing files when a path is provided, and list the context stage when run with no arguments.

  ```rust
  // In the Command enum
  Stage(StageArgs),

  // New struct for arguments
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
  ```

- [x] **Task 2.2: Update `Send` Command.** In `src/cli.rs`, add an `--ignore-inherited-stage` (`-i`) flag to the `Send` command arguments. This will be used in a later phase to allow starting a chat with a clean context. (Note: This flag was already present).

- [ ] **Task 2.3: Implement `stage` Command Logic.** In `src/lib.rs`, implement the logic for the `Stage` subcommand in the main `run` function.

  The logic will differentiate based on whether `file_path` is provided:
  ```rust
  Command::Stage(args) => {
      if let Some(file_path) = args.file_path {
          // A file path was provided, so we are adding or dropping a file.
          if args.drop {
              // Call db::remove_file_from_stage and print confirmation.
          } else {
              // Call db::add_file_to_stage and print confirmation.
          }
      } else {
          // No file path, so list both inherited and prepared contexts.
          // 1. Determine parent message, fetch, and display Inherited Context.
          // 2. Fetch 'default' stage and display Prepared Context.
      }
  }
  ```

- [ ] **Task 2.4: Add CLI Integration Tests.** In `tests/cli.rs`, add new tests for `retort stage`.
    - Test adding and dropping files, verifying the `context_stages` table is updated.
    - Test `retort stage` (with no arguments) and verify that the output correctly displays both inherited (mocked) and prepared contexts.

## Phase 3: Prompt Integration

This phase connects the file context stage to the prompt generation process.

- [ ] **Task 3.1: Load Files in `send` Command.** In `src/lib.rs` (in `Command::Send`), assemble the file context before building the prompt:
    1.  **Inherited Context**: If `ignore_inherited_stage` flag is `false`, find the parent message, deserialize its metadata (from Task 4.1), and load the files listed there.
    2.  **Prepared Context**: Fetch the 'default' context stage from the database and load its files.
    3.  **Merge Contexts**: Combine both contexts. The prepared context's changes (additions/removals) take precedence.
    4.  Read the content of all files in the final merged context from disk.
    5.  Pass the file contents to `prompt::build_prompt_messages`.
    6.  Print a view of the final context (split by Inherited/Prepared) for the user.

- [ ] **Task 3.2: Update `build_prompt_messages`.** In `src/prompt.rs`, update the signature of `build_prompt_messages` to accept the file content vectors.
  ```rust
  pub fn build_prompt_messages(
      done_messages: Vec<HistoryMessage>,
      cur_messages: Vec<HistoryMessage>,
      read_write_files: &[(String, String)], // (path, content)
      read_only_files: &[(String, String)],  // (path, content)
  ) -> Result<Vec<Message>>
  ```

- [ ] **Task 3.3: Inject File Context into Prompt.** In `src/prompt.rs`, inside `build_prompt_messages`, programmatically construct and insert user/assistant message pairs for the files. This mimics the conversational file-providing pattern shown in `prompts/diff_fenced.j2`.
    - If `read_only_files` is not empty, create a `user` message with the content from `READ_ONLY_FILES_PREFIX`, followed by each file's path and content in a fenced block. Then, add an `assistant` message with the fixed response "Ok, I will use these files as references.".
    - Do the same for `read_write_files` using `CHAT_FILES_PREFIX` and the response "Ok, any changes I propose will be to those files.".
    - Insert these new `Message` objects into `result_messages` after the system prompt but before the main conversation history (`done_messages`).

## Phase 4: Metadata, Project Root, and Context Inheritance

This phase adds safety via a project root, snapshots the file context in message metadata, and makes the context persist across turns.

- [ ] **Task 4.1: Store Prompt Metadata.** In `src/lib.rs` (in `send`), create a serializable struct to hold file paths and their content hashes (e.g., SHA256). Populate it and serialize it to JSON. Store this JSON in the `metadata` column for the new `user` message. Have `sha2` and `serde_json` added to `Cargo.toml`.
  ```rust
  #[derive(Serialize, Deserialize)]
  struct PromptMetadata { /* ... */ }
  #[derive(Serialize, Deserialize)]
  struct FileMetadata { path: String, hash: String }
  ```

- [ ] **Task 4.2: Add Project Root to Profile.** Integrate the project root into user profiles. This involves modifying the `setup` function in `src/db.rs` to add a `project_root TEXT` column to the `profiles` table, updating the `Profile` struct to include `pub project_root: Option<String>`, and updating database functions like `get_profile_by_name`. In `src/cli.rs`, add a `--set-project-root <path>` argument to the `Profile` command. Finally, implement the logic in `src/lib.rs` to store the absolute, canonicalized path in the 'default' profile.

- [ ] **Task 4.3: Enforce Project Root.** The project root will be loaded from the default profile. In `src/hooks/postprocessor.rs`, update `apply_and_commit_changes` to accept an `Option<PathBuf>` for the project root. Before applying changes, it must verify that all file paths are within this directory. Update `HookManager` and `Hook` traits to pass this through.

- [ ] **Task 4.4: Implement Context Inheritance.** In `src/lib.rs` (`send` command), after a message is sent, clear the 'default' (`prepared`) context stage. The full, merged context has already been saved to the new message's metadata in Task 4.1, and will be used as the `inherited` context for the next turn. If a new chat was started (`--new` or no active tag), the `prepared` stage should also be cleared to ensure a clean slate.

- [ ] **Task 4.5: Add Integration Tests.** Add tests in `tests/cli.rs` for project root enforcement (using `retort profile --set-project-root`, then asserting failure when editing outside the root) and context inheritance (asserting that the stage persists between messages and is cleared on `--new`).

## Phase 5: Cleanup and Refinement

This final phase removes obsolete code and improves documentation.

- [ ] **Task 5.1: Remove `files` Table.** In `src/db.rs`, remove the `CREATE TABLE files` statement from the schema in the `setup` function. Also remove any related, now-unused functions that referenced it.

- [ ] **Task 5.2: Update Documentation.** Update `ai.md` and `README.md` to document the new `retort stage` and `retort project` commands and the overall file management workflow.
