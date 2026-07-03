# Contributing Guidelines

Thank you for your interest in contributing! We welcome bug reports, feature requests, and code contributions.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites

To build and run this project locally, you will need:
- [Rust](https://www.rust-lang.org/) (latest stable release)
- [Trunk](https://trunkrs.dev/) (for compiling the WebAssembly frontend)
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`

### Setup & Run

1. Clone the repository.
2. For frontend development: Run `trunk serve` (within the `frontend` directory if applicable, or project root).
3. For backend/full stack: Run `cargo run` (within the `backend` directory if applicable, or project root).

## Standard Checks

Before submitting a Pull Request, please ensure the following commands pass without errors or warnings:

```bash
# Check formatting
cargo fmt --all -- --check

# Run clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Run unit tests
cargo test --workspace
```

Run these locally before pushing. Maintainers review PRs against the same checks.

## Pull Request Process

1. Create a new branch from `main`/`master`/`dev`.
2. Keep your changes focused and write clear commit messages.
3. Submit a Pull Request and describe the changes you've made.
