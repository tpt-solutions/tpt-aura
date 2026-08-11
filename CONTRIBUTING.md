# Contributing to AURA

Thank you for your interest in contributing to AURA! This document describes how
to get started.

## Code of conduct

Be respectful and constructive. We want AURA to be a welcoming project for
everyone working on open media provenance.

## Getting started

1. Fork and clone the repository.
2. Install the latest stable Rust toolchain.
3. Build the workspace: `cargo build --workspace`.
4. Run the tests: `cargo test --workspace`.

## Development workflow

- Keep `cargo fmt --all -- --check` and `cargo clippy --workspace -- -D warnings`
  clean before opening a PR. CI enforces both.
- Add unit tests for new behavior. Public API items must have rustdoc.
- Keep the `onnx` feature optional so the workspace still builds offline.

## Commit / PR guidelines

- Write clear, imperative commit messages ("Add cycle detection to SemanticDAG").
- Open PRs against `main`. Fill in the PR template.
- For spec changes, open an issue first to discuss the RFC update.

## License

By contributing you agree that your contributions will be dual licensed under
Apache-2.0 OR MIT, matching the rest of the project.
