# Development Guide

## Setup
1. Clone the repository.
2. Install Rust stable (`rustup default stable`). The project supports MSRV 1.78.
3. Install required OS packages (e.g. `libasound2-dev` on Linux).

## Formatting and Linting
Before submitting a PR, ensure your code is formatted and lint-free:
```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

## Testing
Run all tests with:
```sh
cargo test --all
```

## Debugging
You can set `RUST_LOG=spotifust=debug,librespot=info` for verbose output.

## Release Builds
When compiling for end-users, use the release profile:
```sh
cargo build --release
```

## Branch Protection Recommendations
When configuring this repository on GitHub, ensure the following branch protection rules are applied to the main branch:
- **Require a pull request before merging**
- **Require status checks to pass before merging** (CI, CodeQL, etc.)
- **Dismiss stale pull request approvals when new commits are pushed**
- **Require linear history**
- **Restrict force pushes**

