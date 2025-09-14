# Conversation Data Structure Requirements

Based on the need to support branching histories and message editing, the data model is being redesigned away from linear sessions. The following are the core requirements for the new message-graph-based data structure.

## Core Concepts

1.  **Message Graph:** The data model will represent conversations as a directed acyclic graph (DAG) of messages, rather than linear sessions. The `message` is the fundamental unit.
2.  **Branching on Edit/Restore:** Any action that diverges from a linear history, such as editing a past message or restoring from a checkpoint, will create a new branch in the graph. Existing messages are immutable.
3.  **Emergent Conversations:** A "conversation" is not a stored entity but a dynamically constructed linear path. It is reconstructed by traversing the graph from a specific leaf node back to a root.

## Functional Requirements

1.  **Branching History:** The system must allow a single message to have multiple child messages, creating branches in the conversation.
2.  **Checkpoint & Resume:** Users must be able to select any message in the graph and resume the conversation from that point, creating a new branch.
3.  **Immutable History:** Editing a message must not alter the original message. Instead, it should create a new branch starting from the parent of the edited message.
4.  **Conversation Navigation:** The primary user interface for navigating conversations will be through "leaf" messages (messages with no children). Users can select a leaf to view its entire history.
5.  **History Reconstruction:** The application must provide a function to trace a path from any given message back to its root ancestor, presenting it to the user as a linear conversation.

## Data Model Requirements

1.  **Messages Table:**
    *   Stores core message data (role, content, timestamp).
    *   A nullable `parent_id` column establishes a tree structure. A `NULL` `parent_id` indicates a root message.
    *   The `session_id` foreign key is removed.
2.  **Message Linkage:**
    *   Each message has at most one parent, forming a tree rather than a more complex graph. This simplifies traversal while still fully supporting branching (a parent can have multiple children).
    *   Editing a message is handled by creating a new message and thus a new branch, not by creating a convergent graph structure.
3.  **Files Table:**
    *   Files are associated directly with `messages`, not obsolete `sessions`.
