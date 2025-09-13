# CI Steps for Retort

These are the steps to build, test, and check the `retort` application. They should be run from the root of the repository.

## 1. Install Rust Toolchain

If the `cargo` command is not found, you first need to install the Rust toolchain using `rustup`.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
# Add cargo to the current shell's PATH
source "$HOME/.cargo/env"
```

## 2. Build the application

This command compiles the source code.

```bash
cargo build
```

## 3. Run tests

This command executes the test suite.

```bash
cargo test
```

## 4. Lint the code

This command runs Clippy to check for common mistakes and style issues.

```bash
cargo clippy
```

## 5. Check formatting

This command checks if the code is formatted correctly without modifying any files.

```bash
cargo fmt --check
```
