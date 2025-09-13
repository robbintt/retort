# Retort Prototype Plan (0 to 1)

This document outlines the initial tasks to get Retort, an AI pair programmer, from conception to a basic working prototype (0 to 1). Our early emphasis will be on intelligent, stateless, session management, while storing session state in a local SQLite database. We will also prioritize prompt building and application of changes to the codebase.

## Core Principles & Design Considerations

*   **Stateless Session Management:** The CLI application will be stateless between invocations. All state required to reconstruct a session (message history, read-only files, read-write files, model sent) will be persisted in and loaded from the SQLite database. This allows users to specify a session as the starting point for any action.
*   **Configuration:** A config file will allow specifying the SQLite database location, with a default of `~/.retort/data/retort.db`.
*   **CLI First:** No UI initially; all interactions will be via the command-line interface (CLI) with flags for messages to the LLM.
*   **Output & Application:** LLM responses will be streamed to stdout, processed for fenced code changes, and applied to the codebase. Initial application will use Git, followed by Jujutsu (jj).
*   **Prompt Building:** High emphasis on understanding the exact prompt sent at every step. The builder should be isolated.
*   **Influences:** Early work heavily influenced by Aider. Aider's prompts will be used during prototyping. Later, prompt optimization techniques (dspy, opro, ape) will be explored.
*   **Repo Map:** Reproduce Aider's repo map feature using tree-sitter for Rust.
*   **Modular Design (Hooks):** Each component will be implemented as a hook. Optional hook configuration will be allowed, with application-supplied defaults. For example, LLM response processing, parsing, and change application (Git/Jj) will be hook points. Parsed data will be stored in the DB, and then change application hooks will query and apply.
*   **Testing:** Good test coverage for permanent elements: client library, database, session manager, CLI, and config. One test for prompt templating is sufficient initially.

## Implementation Phases (E2E Order)

### Phase 1: CLI Foundation & Basic Interaction

The primary goal here is to establish the CLI entry point and reflect user input, allowing for a basic interaction loop.

*   **Select Rust CLI Tooling:** Choose a reliable Rust library for building command-line interfaces (e.g., `clap`).
*   **Initial CLI Command:** Implement a basic command structure, e.g., `rt -m "Your message here"`.
*   **Message Reflection:** For the very first iteration, simply echo the `-m` message back to stdout. This validates the CLI setup.
*   **Config File Setup:** Implement basic reading of a configuration file (e.g., `serde` for TOML/YAML) to define the default SQLite database path (`~/.retort/data/retort.db`) and allow overrides.

### Phase 2: Session Management & Database Integration

Establish the core for state persistence and session handling.

*   **Select Rust SQLite Client:** Choose a robust Rust library for SQLite database interaction (e.g., `rusqlite`).
*   **Database Schema Design:** Define the initial schema for sessions, messages, read-only files, read-write files, and model information.
    *   `sessions` table: To store session metadata.
    *   `messages` table: To store message history, linked to a session.
    *   `files` table: To store file content (read-only/read-write), linked to a session or message.
*   **Session Model:** Create Rust data structures to represent sessions and their associated data.
*   **Database Interaction Layer:** Implement methods for creating new sessions, adding messages to a session, and retrieving session history.
*   **CLI-DB Integration:** Modify the CLI to create a new session (if none specified) or load an existing one, and store the initial user message in the database.

### Phase 3: Prompt Building System

Develop the mechanism for constructing LLM prompts.

*   **Templating System:** Integrate a Rust templating engine (e.g., `tera` - a Jinja2-like engine) for dynamic prompt construction.
*   **Prompt Builder Module:** Create a dedicated module responsible for taking context (session history, files, repo map) and a user message to generate the final LLM prompt using templates.
*   **Initial Prompt Templates:** Create basic templates for common interactions, heavily influenced by Aider's prompt structure during this phase.

### Phase 4: LLM Client Integration & Response Handling

Connect to LLMs, send prompts, and process their responses.

*   **LLM Client Library:** Develop a Rust client library for interacting with Google's Gemini API (Pro, Flash, Flash-lite). This will be our main test LLM.
*   **API Key Management:** Securely handle API keys for LLM access by reading from environment variables. The application should be able to refresh its environment variables without a restart.
*   **Model Configuration & Aliasing:** Implement a system for managing LLM parameters, inspired by Aider's model metadata and settings files. Allow users to define model aliases in the configuration (e.g., `gemini-flash-nothink`, `gemini-flash-autothink`) that map to specific model names and parameter sets (e.g., temperature, thinking tokens).
*   **Streaming Output:** Implement functionality to stream responses from the LLM to stdout as they are received.
*   **Response Parser (Hook):** Develop a parser hook to identify and extract fenced code blocks (` ``` `) from the LLM's streamed response.
*   **Change Storage:** The parser will store the parsed changes (file paths, content) in the database, linked to the current session/message.
*   **Change Application (Hook):**
    *   **Git Integration:** Implement a hook to apply changes stored in the database using Git commands.
    *   **Jujutsu (jj) Integration:** Implement a hook to apply changes using Jujutsu commands, following the Git integration. This hook would query the database for the changes and then apply them.

### Phase 5: Repo Map Integration (Stretch Goal / Follow-up)

*   **Tree-sitter Integration:** Integrate a Rust `tree-sitter` binding to build a repository map.
*   **Repo Map Generation:** Implement logic to generate a condensed, context-rich overview of the codebase using tree-sitter.
*   **Prompt Context:** Integrate the generated repo map into the prompt builder to provide the LLM with relevant code structure context.
