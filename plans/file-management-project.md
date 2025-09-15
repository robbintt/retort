# Project Plan: File Management and Prompt Context

This plan outlines the work required to implement file management, allowing users to add read-write and read-only files to the chat context, based on the design in `plans/file-management.md`. The implementation will be broken down into testable phases.

## Phase 1: Database and Core Data Structures for Context

This phase focuses on setting up the database schema and data structures for managing a persistent "context stage".

- [ ] **Task 1.1**: In `src/db.rs`, add a `context_stages` table. It will store the state of the file context between commands. The schema should include: `name` (TEXT, PRIMARY KEY), `project_root` (TEXT, nullable), `read_write_files` (TEXT, as a JSON array of strings), and `read_only_files` (TEXT, as a JSON array of strings). Insert a 'default' stage with empty values.
- [ ] **Task 1.2**: In `src/db.rs`, define a `ContextStage` struct that mirrors the table structure. It should derive `serde::{Serialize, Deserialize}` to handle JSON conversion for file lists.
- [ ] **Task 1.3**: In `src/db.rs`, implement functions to manage the `context_stages` table: `get_context_stage(name)`, `update_context_stage(name, stage)`, `add_file_to_stage(name, file_path, read_only)`, and `remove_file_from_stage(name, file_path)`.
- [ ] **Task 1.4**: In `tests/integration/`, add a new test file or modify an existing one to create unit tests for the new database functions to ensure they correctly manipulate the `context_stages` table and its JSON data.

## Phase 2: CLI for Staging Files

This phase introduces the `retort stage` command, allowing users to interact with the context stage.

- [ ] **Task 2.1**: In `src/cli.rs`, add a `Stage` subcommand to the main `Command` enum. This subcommand should handle adding, removing, and viewing files in the stage (`retort stage <file_path> [--read-only] [--drop]`, `retort stage list`).
- [ ] **Task 2.2**: In `src/lib.rs`, implement the logic for the `Stage` subcommand. This will involve parsing the arguments and calling the appropriate database functions from Phase 1. Provide user feedback, such as "Staged file 'path/to/file.rs' as read-write."
- [ ] **Task 2.3**: In `tests/cli.rs`, add an integration test for the `retort stage` command. The test should execute the CLI command and then query the database to verify that the `context_stages` table is updated correctly.

## Phase 3: Prompt Integration and Metadata

This phase connects the context stage to the prompt generation process and ensures file state is snapshotted in message metadata.

- [ ] **Task 3.1**: In `src/lib.rs` (`send` command), fetch the current context stage from the database. Read the contents of each file specified in the stage.
- [ ] **Task 3.2**: In `src/prompt.rs`, update `build_prompt_messages` to accept the file contents. The function will format the file paths and contents into strings to be injected into the prompt. Replace the hardcoded stub constants (`CHAT_FILES`, `READ_ONLY_FILES`) with this new dynamic data.
- [ ] **Task 3.3**: Update the `prompts/diff_fenced.j2` template to correctly render the lists of read-only and read-write files, including their paths and fenced content blocks.
- [ ] **Task 3.4**: In `src/lib.rs` (`send` command), before saving the user message, calculate a hash (e.g., SHA256) of each staged file's content. Create a serializable struct containing the file lists (path and hash) and store it as a JSON string in the `metadata` column of the `messages` table.

## Phase 4: Project Root and Context Inheritance

This phase introduces the concept of a project root for safety and makes the file context persist across messages in a chat.

- [ ] **Task 4.1**: Create a `project set-root [<path>]` command in `src/cli.rs` and `src/lib.rs` to define a project directory. The absolute, canonicalized path should be stored in the `project_root` field of the 'default' context stage.
- [ ] **Task 4.2**: In `src/hooks/postprocessor.rs`, update the `apply_and_commit_changes` function to enforce the project root. The `send` command will pass the project root from the context to the hook. The hook must verify that all file modifications are within this directory, failing with an error if a path is outside.
- [ ] **Task 4.3**: In `src/lib.rs`'s `send` command, implement context inheritance. If a chat is continued (not `--new`), after the turn is complete, reload the context from the user message just created and write it back to the 'default' row in `context_stages`. If the chat was `--new`, clear the file lists in the 'default' stage.
- [ ] **Task 4.4**: Add an integration test in `tests/cli.rs` to verify project root enforcement. The test should use a mock LLM response that attempts to write a file outside the project root and assert that the operation fails as expected.
- [ ] **Task 4.5**: Add an integration test to verify context inheritance. The test should: 1. Stage a file. 2. Send a message. 3. Check that the file context is still present in the `context_stages` table for the next turn.

## Phase 5: Cleanup and Refinement

This final phase removes obsolete code and improves usability.

- [ ] **Task 5.1**: In `src/db.rs`, remove the `files` table and any related (now unused) functions, as its role is replaced by storing file information in the `messages` table's `metadata` field.
- [ ] **Task 5.2**: Update `ai.md` and other documentation to reflect the new file management workflow and the `retort stage` command.
