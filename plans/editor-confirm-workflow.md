# Project Plan: Editor-Based Confirmation Workflow

## 1. Objective

This plan outlines the implementation of a new workflow for the `retort send --confirm` flag. Instead of printing a preview to `stdout` and asking for a `[Y/n]` confirmation, the command will open the user's default shell editor (`$EDITOR`) with the full prompt content. The user can then review and edit the entire prompt before sending it. Saving and closing the editor will proceed with sending the message.

## 2. Implementation Details

The implementation will follow a simplified approach that allows the user full control over the prompt content within the editor.

### 2.1. Add a New Dependency

-   The `edit` crate will be added to `Cargo.toml` to handle the editor interaction. This crate provides a simple, cross-platform way to launch an editor with given text content and retrieve the edited result.

### 2.2. Modify the `send` Command Flow

The core logic for the `send` command (likely handled in `src/lib.rs` and `src/cli.rs`) will be modified:

1.  **Isolate `--confirm` logic**: When the `--confirm` flag is present, the program will divert to the new editor-based workflow instead of the standard `stdout` preview.

2.  **Format Messages for Editor**: A new function, `prompt_to_string`, will be created in `src/prompt.rs`.
    -   **Input**: `&[prompt::Message]`
    -   **Output**: `String`
    -   **Format**: The function will serialize the messages into a single string using a format similar to `retort history` for easy parsing.

    ```
    [user]
    Content of message 1.
    ---
    [assistant]
    Content of message 2.
    ---
    [user]
    The new message being composed.
    ```

3.  **Invoke the Editor**: The formatted string will be passed to `edit::edit()`. This function call will block until the user saves and closes the editor, and it will return the file's final content as a `String`.

4.  **Parse Edited Content**: A corresponding parser function, `string_to_prompt`, will be created in `src/prompt.rs`.
    -   **Input**: `&str` (the string from the editor)
    -   **Output**: `Result<Vec<prompt::Message>>`
    -   **Logic**: This function will parse the string, splitting it by the `---` separator and reconstructing the `Vec<prompt::Message>`.

5.  **Send to LLM**: The resulting `Vec<prompt::Message>` will be sent to the language model. If the user deletes all content from the editor, an empty prompt will be sent, which is considered valid user action.

## 3. Testing Strategy

-   An integration test will be added in `tests/cli.rs` to cover the `send --confirm` flow.
-   The test will need to mock or simulate the editor interaction. This could involve setting the `EDITOR` environment variable to a script that writes predefined content to the file.
-   The test will assert that the `send` command correctly processes the "edited" content from the mocked editor.
-   An edge case to test is the user clearing the editor buffer, which should result in an empty prompt being sent.
