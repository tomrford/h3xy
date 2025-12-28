# h3xy

Hex file processing library & CLI (HexView alternative/replacement).

## Commands

```bash
cargo build           # Build
cargo check           # Typecheck
cargo test            # Run tests
cargo clippy          # Lint
cargo run -- [args]   # Run CLI
```

## Structure

- `src/lib.rs` - Module declarations + public re-exports
- `src/bin/h3xy.rs` - CLI entry point
- `src/` - Core library modules

## Conventions (from mint/tracy)

### Error Handling
- Use `thiserror` for all error types
- Hierarchical errors: module-specific enums that compose into top-level `Error`
- Use `#[error(transparent)]` and `#[from]` for error composition
- Use `#[source]` with `Box<Self>` for recursive context wrapping
- Full `Result<T, SpecificError>` - no type aliases

### Module Organization
- `lib.rs` declares modules only, with `pub use` re-exports
- Each module gets its own `error.rs` if needed
- Private impl files (no `pub mod`)
- Public API via `pub use` in mod.rs

### Function Signatures
- Context/config params first, data params second
- Helper functions private (no `pub`)
- Descriptive names with clear intent

### Structs
- `#[derive(Debug, Clone, PartialEq, Eq)]` for data types
- `#[serde(deny_unknown_fields)]` for strict config validation
- Immutable config structs with references where appropriate

### CLI (clap)
- `#[command(flatten)]` for composing sub-argument structs
- Separate `Args` struct per module if complex
- `ExitCode` wrapper pattern in main.rs for error display

### Testing
- Unit tests inline with `#[cfg(test)] mod tests`
- Integration tests in `tests/` with `common/mod.rs` for utilities
- Use `tempfile` for temp files, `env!("CARGO_MANIFEST_DIR")` for fixtures
- Descriptive test names: `test_<behavior>` or `<subject>_<scenario>`

### General
- Prefer `u32` for addresses (covers most embedded use cases)
- Early returns with `let Some(x) = ... else { return }`
- Use `.map_err()` for adding context to errors
- `rayon` for parallelizable operations
