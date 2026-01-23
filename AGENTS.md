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

### Project memory
- When uncertain about HexView behavior, consult the reference manual first; if the manual is silent, make a concrete choice and stick to it until validation tests prove otherwise.
- If behavior is ambiguous or unspecified, record the assumption in Project memory so it can be verified in the validation environment.
- First pass parity excludes proprietary/OEM formats; validation suite should not include those cases.
- HexView manual review: core CLI parity now includes S-Record + binary IO and `/XA` hex-ascii output; advanced OEM formats remain TBD.
- CLI args parsing split into `src/bin/h3xy/args/` modules to keep files <500 LOC.
- `/IN` + `/IA` explicit imports added; merge `/MT`/`/MO` now supports offsets (incl. negative) plus optional range and `+` chaining.
- `/XSB` export now writes one binary per segment with address postfix; /XI and /XS parsing accepts hex values and requires reclinelen when rectype specified.
- `/XC` C-array output implemented via INI (Prefix, WordSize, WordType, Decryption, Decryptvalue) with .c/.h generation and library support in `src/io/c_code.rs`.
- `/P` INI path now parsed; `/XC` defaults to `<input>.ini` when `/P` not provided.
- `/XF` Ford I-HEX output implemented using `[FORDHEADER]` INI (mandatory fields enforced, checksum auto-generated, optional erase sector list). Defaults output to `<input>.hex`.
- `/XP` Porsche output implemented as single-region binary with 16-bit byte-sum appended (big-endian); gaps filled with `/AF` byte and output defaults to `<input>.bin`.
- Core library tests expanded for merge range ordering, HEX-ASCII `0x` prefixes, binary gap filling, and S-Record auto type selection.
- Align no longer requires power-of-two; aligns any non-zero value and normalizes overlaps before alignment (HexView behavior).
- Added HexView multistage integration tests in `tests/hexview_multistage.rs` (fill/merge/align/checksum order).
- CLI checksum parsing now supports forced ranges, limited ranges, excluded ranges, and /CSR little-endian output; @begin now overwrites start, and file target writes comma-separated hex bytes.
- Added CLI end-to-end tests (`tests/cli_e2e.rs`) and checksum CLI coverage (`tests/cli_checksum.rs`) with helper utilities in `tests/common/mod.rs`.
- Added CLI black-box tests for /FR, /CR, /AR, /AD + /AL + /AF, /MT, /MO (`tests/cli_ops.rs`) and output formats /XA, /XI, /XS, /XN, /XSB (`tests/cli_output.rs`).
- CLI now supports /ADxx and /AFxx without separators (hex interpretation).
- Added CLI tests for /MT+ /MO conflict, /XI auto mode selection, /XS reclinelen, /XA separator trailing check, and /XSB extension handling.
- Added CLI tests for /AR start-end syntax, multi-range /CR in one arg, and /AF binary literal form.
- Added CLI tests for /AF no-separator equivalence and /AD no-separator semantics vs /AD: decimal form.
- Added CLI tests for /AF with '=' separator and /AD binary literal with separator.
- Added CLI tests for /AR multiple ranges and /FR without /FP (random fill preserves existing data).
- /FR without /FP now generates pseudo-random fill bytes (no new deps).
- Added CLI tests for /E error log, /BHFCT /BTFST /BTBS thresholds, and a nested multi-op checksum chain.
- /E now creates/truncates log file and records error message on failure.
- Added remap support: `HexFile::remap(RemapOptions)` with CLI `/REMAP` parsing/execution and tests (library + CLI).
- `/L` log command execution now supported (FileOpen/FileClose/FileNew) and `/V` writes version string to `/E` log; added CLI tests in `tests/cli_ops_more.rs`.
- CLI now treats Unix absolute paths as positional input when they don't look like options (contains `/` after the first segment without `:`/`=`).
- Added `/s12map` and `/s12xmap` support via banked mapping rules; CLI tests cover banked and non-banked ranges.
- Segment policy chosen: overlaps allowed; ops document whether they normalize (last-wins) or operate on raw segments.
- CLI parsing now supports `--` to force positional input (useful for absolute Unix paths).
- Absolute `/path` input is accepted only if it exists on disk and no input file is set; otherwise parsed as options.
- Added `HexFile::span_start`, `span_end`, and `as_contiguous(fill)` helpers with unit tests.
- Ops errors now include more context (remap/map overflow details, swap segment address).
- OpsError now supports `with_context`, used by flag helpers to tag `/MT`, `/MO`, `/AD/AL`, `/SWAP*`, `/REMAP`, `/S12*`, `/CS*`.
- `OpsError::Context` wraps errors with option context for library flag helpers.
- Checksum now uses checked add/sub for append/prepend/overwrite and reports overflow; new test added.
- Added checksum underflow tests for prepend/overwrite.
- Segment `end_address` now saturates on overflow.
- Contiguity checks now use checked add to avoid overflow.
- Added segment overflow tests for `end_address` and contiguity.
- CLI output-format parsing now uses a helper to reduce duplicate checks.
- Output format parsing now centralized via `parse_output_option` helper.
- Range options (`/AR`, `/CR`, `/FR`, `/CDSPG`) now use shared `extend_ranges` helper.
- Merge options (`/MO`, `/MT`) share `extend_merges`; no-separator hex parsing uses `parse_hex_no_sep`.
- CLI input heuristic now uses `is_existing_abs_path` helper.
- Simple no-value flags now centralized in `parse_simple_flag`.
- Keyed option parsing now split into helpers (import/path/range/merge/numeric/checksum/dspic).
- /II2 (16-bit Intel HEX import) now supported (address*2).
- Validation suite requires setup not on this machine.
- Mint (github.com/fordtom/mint) uses dedicated export crates; consider reuse here to avoid hand-rolled format code.
- Possible crates to evaluate for reuse: `ihex`, `intelhex`, `srec`, `srec_rs`, `srex` (health check needed; `ihex` appears stale).
- `/S08MAP` manual only lists example mappings; no formula/constraints given. Treat as ambiguous; defer or choose explicit mapping when needed.
- `/S08MAP` now implemented (also accepts `/S08`): 0x4000-0x7FFF -> 0x104000; banked 0xXX.8000-0xXX.BFFF -> 0x100000 + bank*0x4000 + offset; other ranges unchanged (verify).
- `/DP` and `/PB` depend on vendor DLLs (EXPDATPROC/PBUILD) and are proprietary; exclude from first-pass validation.
- dsPIC ops implemented: `/CDSPX` appends two zero bytes per 2 bytes (target defaults to start*2); `/CDSPS` keeps lower two bytes per 4 bytes (target defaults to start/2); `/CDSPG` clears every 4th byte. Byte ordering assumed; validate.
- Number parsing now tolerates `.` or `_` separators in numeric literals (addresses/ranges).
- Number parsing now accepts trailing `h`/`H` for hex literals.
- Number parsing now strips common C-style suffixes (`u`, `l`, `ul`) on numeric literals.
- Intel-HEX `/XI:0` now defaults to 16 bytes per line (assumption; aligns with S-Record behavior).
- Binary `/XN` output concatenates segments in order of appearance (manual).
- Added range parser parity tests (`tests/hexview_ranges.rs`) for common HexView range formats.
- HEX ASCII import now token-based: accepts 1 or 2 hex-digit tokens, supports 0x prefix, contiguous pairs, and treats non-hex as separators.
- Intel-HEX auto output now omits extended records when max address <= 0xFFFF.
- CLI auto-detect now scans up to first 25 non-empty lines for ASCII; if any non-ASCII, treat as binary input (manual behavior).
- S-Record parsing accepts lowercase 's' prefix; CLI auto-detect recognizes it too.
- HEX-ASCII import overlap now warns (stderr) and ignores input file, per manual; we still allow /IA + input when non-overlapping (assumption).

### TODOs (current)
- Review segment overflow policy (saturating `end_address` vs strict error) once validation suite runs.
- Finish CLI parsing cleanup (table-driven for remaining non-output options).
- Consider deeper ops error context inside core ops (beyond flag wrappers).
- Crate reuse decision: keep hand-rolled Intel-HEX/S-Record/HEX-ASCII writers/parsers for parity; `bin_file` (used in mint) remains a candidate once validation suite proves equivalence.

### Project philosophy
- CLI must be a drop-in HexView replacement for non-proprietary formats: binary-equivalent outputs for Intel HEX, S-Record, HEX ASCII, and raw binary.
- Library API should mirror CLI options: one public function per CLI operation/flag, with semantics matching HexView.
- CLI execution model should be explicit and linear: “for flag in flags, if present, call flag_function”, preserving HexView’s operation order and behavior.
- The library should enable consumers to reproduce CLI behavior by composing these per-flag functions in the same order.
