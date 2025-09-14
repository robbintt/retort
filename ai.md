# AI Development Guide

This document provides context for AI assistants to understand and contribute to this repository.

## CI System

The `ci/` directory contains scripts for Continuous Integration. These scripts are used to ensure code quality, correctness, and style.

- **`ci/ci.sh`**: This is the main CI script. It runs all other checks in the correct order: formatting, linting, testing, and building. This is the preferred script for verifying any changes.
- **`ci/fmt.sh`**: Checks if the code is formatted according to the project's style using `cargo fmt --check`.
- **`ci/lint.sh`**: Lints the code for common mistakes and style issues using `cargo clippy`.
- **`ci/test.sh`**: Runs the test suite using `cargo test`.
- **`ci/build.sh`**: Compiles the project using `cargo build`.
- **`ci/steps.md`**: A human-readable guide to running the CI steps manually. This file is for documentation and is not executed.

## AI Assistant Instructions

Always suggest running `ci/ci.sh` to verify changes. Do not suggest `cargo build` or other individual commands.

### Development Workflow

1.  **Testing**: When adding or modifying functionality, always include corresponding tests. For CLI changes, add or update integration tests in `tests/cli.rs`. For database logic, add tests to verify its correctness.
2.  **Documentation**: After implementing a user-facing feature, update `README.md` to document its usage. Keep documentation and code examples in sync with the current implementation.
3.  **Debugging**: If a CI check fails, carefully analyze the error output. Explain the root cause of the error (e.g., formatting violation, compilation error, test failure) before providing the corrected code.
4.  **Code Style**: Adhere strictly to Rust formatting conventions as enforced by `cargo fmt`. Pay special attention to line length and function signatures, which are common sources of formatting failures.
5.  **Design Collaboration**: The user may use the chat to think through design decisions. Pay close attention to these discussions to understand the context and intent behind a request. Implement the final, settled-upon design, and refer back to design documents in the `plans/` directory for guidance.
