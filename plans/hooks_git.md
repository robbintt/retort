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

## Phase 6: Test the Post-Processor Hook

- [x] **Modify `src/llm.rs`:**
    - [x] Update the `get_response` and `get_response_stream` functions to support customizable mock responses for testing.
    - [x] If a `MOCK_LLM_CONTENT` environment variable is set, its value should be used as the mock LLM response. This will take precedence over the existing `MOCK_LLM` flag, which can remain for simpler tests.

- [x] **Add a new integration test in `tests/cli.rs`:**
    - [x] Create a new test function, for example, `test_send_with_postprocessor_hook`.
    - [x] In the test, set up a temporary directory and initialize a Git repository within it.
    - [x] Create and commit an initial version of a file (e.g., `test-file.txt`).
    - [x] Define a mock LLM response string containing a commit message and a `diff` block that modifies the test file.
    - [x] Run the `retort send` command, using the `MOCK_LLM_CONTENT` environment variable to inject the mock response.

- [x] **Add Assertions to the new test:**
    - [x] Verify that the content of `test-file.txt` on disk has been updated according to the diff.
    - [x] Use `git` commands within the test to confirm that a new commit has been created.
    - [x] Verify that the commit message of the new commit matches the message from the mock LLM response.

## Manual Testing Protocol

This protocol describes how to manually test the Git post-processor hook.

1.  **Build the Project:**
    Ensure you have the latest version of the `retort` binary.
    ```bash
    cargo build
    ```

2.  **Set Up a Test Environment:**
    Create a temporary directory for the test. This will contain both the test git repository and the `retort` home directory, keeping the test self-contained.

    ```bash
    # Create a temporary directory and navigate into it
    export TEST_DIR=/tmp/retort-manual-test
    mkdir -p $TEST_DIR
    cd $TEST_DIR

    # Initialize a Git repository
    git init
    git config user.name "Test User"
    git config user.email "test@example.com"

    # Create and commit a file to modify
    echo "Initial content" > test-file.txt
    git add test-file.txt
    git commit -m "Initial commit"
    ```

3.  **Prepare the Mock LLM Response:**
    While inside `$TEST_DIR`, create a file named `mock_response.txt` with the following content. This simulates the output from the LLM that contains a diff.

    ````text
    feat: update test file via manual test

    This commit is generated from a manual test.

    test-file.txt
    ```diff
    --- a/test-file.txt
    +++ b/test-file.txt
    @@ -1 +1 @@
    -Initial content
    +Updated content

    ```
    ````

4.  **Run `retort`:**
    Execute the `retort send` command from within `$TEST_DIR`. You will need to provide the full path to your `retort` binary. We will also override the `HOME` environment variable to prevent `retort` from using your default config/database.

    ```bash
    # Set the path to your retort project root.
    # Replace this with the actual path on your machine.
    export RETORT_PROJECT_PATH=path/to/your/retort/project

    # Run the command with a mocked response and an isolated HOME directory
    MOCK_LLM_CONTENT=$(cat mock_response.txt) \
    HOME=$TEST_DIR \
    $RETORT_PROJECT_PATH/target/debug/retort send --new "make a change"
    ```
    You should see output indicating that a patch was applied and a commit was made.

5.  **Verify the Results:**
    Check that the file content has been updated and a new Git commit has been created.

    ```bash
    # Check the file content
    cat test-file.txt
    # Expected output: Updated content

    # Check the latest git commit message
    git log -1 --pretty=%B
    # Expected output should start with "feat: update test file via manual test"
    ```

6.  **Clean Up:**
    ```bash
    # When you are done, you can remove the test directory
    cd ~
    rm -rf $TEST_DIR
    ```
