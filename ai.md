# AI Development Guide

This document provides context for AI assistants to understand and contribute to this repository.

## CI System

The `ci/` directory contains scripts for Continuous Integration. These scripts are used to ensure code quality, correctness, and style.

- **`ci/ci.sh`**: This is the main CI script. It runs all other checks in the correct order: formatting, linting, testing, and building.
- **`ci/fmt.sh`**: Checks if the code is formatted according to the project's style using `cargo fmt --check`.
- **`ci/lint.sh`**: Lints the code for common mistakes and style issues using `cargo clippy`.
- **`ci/test.sh`**: Runs the test suite using `cargo test`.
- **`ci/build.sh`**: Compiles the project using `cargo build`.
- **`ci/steps.md`**: A human-readable guide to running the CI steps manually. This file is for documentation and is not executed.
