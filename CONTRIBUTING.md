# Contributing to tessel

We welcome contributions to `tessel`! Please follow these engineering guidelines.

## Development Principles

- **Zero-FFI Pure Rust**: Do not introduce `-sys` crates, C dependencies, or platform-specific FFI.
- **Fail-Closed Verification**: Never silently approximate constraints or skip errors.
- **No Em or En Dashes**: Follow TechTide documentation craft standards (use standard hyphens, colons, or parentheses).
- **Deterministic by Default**: Use in-crate `Rng` with explicit seeds. All outputs must be reproducible across platforms.
- **Quality Gates**: All pull requests must pass strict formatting, clippy pedantic checks, and the full test suite.

## Running Gates Locally

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```
