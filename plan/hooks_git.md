# Git Post-Processor Hook Implementation Plan

This document outlines the step-by-step plan to implement a post-processor hook that parses file diffs from an LLM response, applies them to the local filesystem, and commits them using Git.

## Phase 1: Create the Hook System Infrastructure

- [x] Create a new directory `src/hooks`.
- [x] Create a new file `src/hooks/mod.rs`.
- [x] In `src/hooks/mod.rs`, define a `Hook` trait with a `post_send(&self, llm_response: &str) -> anyhow::Result<()>` method.
- [x] In `src/hooks/mod.rs`, define a `HookManager` struct that holds a `Vec<Box<dyn Hook>>`.
- [x] Implement `new()` and `register()` methods for `HookManager`.
- [x] Implement a `run_post_send_hooks(&self, ll_response: &str)` method on `HookManager` that iterates through registered hooks and calls their `post_send` method.
- [x] Add `pub mod hooks;` to `src/lib.rs` to declare the module.

## Phase 2: Implement the Post-Processor Hook

- [x] Create a new file `src/hooks/postprocessor.rs`.
- [x] In `src/hooks/postprocessor.rs`, define a `FileChange` struct to hold the `path` (String) and `diff_content` (String) of a file modification.
- [x] In `src/hooks/postprocessor.rs`, define an empty `PostprocessorHook` struct.
- [x] Implement the `Hook` trait for `PostprocessorHook`. The `post_send` method will orchestrate parsing, applying, and committing changes.

## Phase 3: Implement Diff Parsing Logic

- [x] Add the `regex` crate to `Cargo.toml`.
- [x] In `src/hooks/postprocessor.rs`, implement a private `parse_changes(response: &str) -> anyhow::Result<(String, Vec<FileChange>)>` function within `PostprocessorHook`.
- [x] Use a regular expression to find all occurrences of file change blocks. A file change block is a file path followed by a fenced `diff` code block.
- [x] This function will parse the LLM response, separating the commit message (text outside of diff blocks) from the `FileChange` blocks.
- [x] It should return a tuple containing the extracted commit message and a vector of `FileChange` structs.

## Phase 4: Implement Change Application and Committing

- [x] In `src/hooks/postprocessor.rs`, implement a private `apply_and_commit_changes(&self, commit_message: &str, changes: &[FileChange]) -> anyhow::Result<()>` function within `PostprocessorHook`.
- [x] This function will first check if there are any changes to apply. If not, it will return early.
- [x] For each `FileChange`, it will execute the `patch -p1` command. It will write the `diff_content` to the standard input of the `patch` process.
- [x] After successfully applying all patches, it will execute `git add <path>` for each modified file.
- [x] Finally, it will execute `git commit -m "..."` using the extracted commit message.

## Phase 5: Integrate the Hook System into the Application

- [x] In `src/lib.rs`, inside the `run` function, create an instance of `HookManager`.
- [x] Register an instance of `PostprocessorHook` with the `HookManager`.
- [x] In the `Command::Send` match arm, after the complete `assistant_response` string is received, call `hook_manager.run_post_send_hooks(&assistant_response)`.
- [x] This call should happen before the assistant's message is saved to the database to ensure a clean separation of concerns.
