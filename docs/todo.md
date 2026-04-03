# TODO

Post-API-simplification active issues.

The big surface cleanup is done:
- no public `h3xy::cli`
- no public pipeline/flag layer
- no in-memory CLI execution path
- public core model is `HexFile`, `Segment`, `AddressRange`

## Verify On Reference Machine

These are contract questions, not just implementation bugs.

### `/XN` ordering

Question: should binary export preserve raw insertion order or sorted address order?

Current state:
- `HexFile` documents raw segment operations as preserving insertion order
- [`write_binary`](/Users/tomford/code/projects/h3xy/src/io/binary.rs) sorts by address before concatenation

Need:
- verify HexView behavior against the reference manual / binary
- then either document current behavior as correct or change it

### `/L` `FileOpen` path base

Question: should paths in log files resolve relative to cwd or relative to the log file location?

Current state:
- [`parse_log_commands`](/Users/tomford/code/projects/h3xy/src/ops/log.rs) preserves the literal path
- [`execute_log_commands`](/Users/tomford/code/projects/h3xy/src/ops/log.rs) later resolves through the caller/load closure, which currently makes CLI behavior cwd-relative

Need:
- verify HexView behavior
- then either keep cwd-relative resolution or make log-file-relative resolution explicit

## Low-Risk Fixes

These look fixable without needing the manual first.

### Fixed-address checksum overflow

[`src/ops/checksum.rs`](/Users/tomford/code/projects/h3xy/src/ops/checksum.rs)

For `ChecksumTarget::Address`, target exclusion only happens if `AddressRange::from_start_length` succeeds, but the checksum bytes are still written afterward. Near the top of address space this can silently create an overflowed write target.

Likely fix:
- reject overflowing fixed targets before checksum calculation and before write

### Address transforms validate only segment starts

[`src/ops/filter.rs`](/Users/tomford/code/projects/h3xy/src/ops/filter.rs)
[`src/ops/transform.rs`](/Users/tomford/code/projects/h3xy/src/ops/transform.rs)

`offset_addresses` and `scale_addresses` validate only the transformed start address, not the full span. A segment can end past `u32::MAX` and become impossible raw state.

Likely fix:
- validate transformed end address as well as start
- add tests near `u32::MAX`

### `/II2` missing from default output / INI resolution

[`src/bin/h3xy/args/io.rs`](/Users/tomford/code/projects/h3xy/src/bin/h3xy/args/io.rs)

`/II2` is treated as a real primary input for execution, but the `/XC`, `/XF`, `/XP`, and INI path resolvers only consider positional input, `/IN`, and `/IA`.

Reproduced:
- `/II2=<tmp>/in.hex /XP` still errors with `output file required for /XP (use -o <file>)`

Likely fix:
- centralize “primary input path” resolution
- reuse it across default output and INI path helpers

### Singular parsers silently take only the first range

[`src/bin/h3xy/args/parse_util.rs`](/Users/tomford/code/projects/h3xy/src/bin/h3xy/args/parse_util.rs)

Forced checksum range parsing, checksum range parsing, and dsPIC parsing still call `parse_hexview_ranges(...)` and keep only `.next()`.

Likely fix:
- reject `>1` ranges in singular contexts
- add parser tests for multi-range rejection

## Needs Design / More Consideration

These are real issues, but the right fix is less mechanical.

### Ambiguous key / signature source parsing

[`src/bin/h3xy/args/signature.rs`](/Users/tomford/code/projects/h3xy/src/bin/h3xy/args/signature.rs)

`load_signature_bytes` and `load_key_material` still use `path.exists()` to choose between filesystem input and inline material.

Problem:
- cwd files can shadow intended inline input
- missing file-looking strings can be accepted as inline bytes or literal key material

Needs a decision on explicit source syntax or stricter rejection rules.

### `/DP` placement plus `/SV` hashes different images

[`src/bin/h3xy/args/signature.rs`](/Users/tomford/code/projects/h3xy/src/bin/h3xy/args/signature.rs)

`/DP` signs pre-placement bytes, then mutates the image by placing the signature. `/SV` verifies against the post-placement image.

Needs a decision:
- reject same-invocation placed `/DP` + `/SV`
- or define and implement exclusion rules for the placed signature range

### `normalized_lossy()` performance / cloning

[`src/hexfile.rs`](/Users/tomford/code/projects/h3xy/src/hexfile.rs)

Overlap-heavy normalization is still quadratic and clone-heavy. It sits under checksum collection, alignment, gap filling, contiguous reads, and some writers.

Needs a design pass, not a local patch.

### Full-span materialization in sparse images

[`src/ops/filter.rs`](/Users/tomford/code/projects/h3xy/src/ops/filter.rs)
[`src/hexfile.rs`](/Users/tomford/code/projects/h3xy/src/hexfile.rs)
[`src/io/binary.rs`](/Users/tomford/code/projects/h3xy/src/io/binary.rs)
[`src/bin/h3xy/args/io.rs`](/Users/tomford/code/projects/h3xy/src/bin/h3xy/args/io.rs)

`fill_gaps`, `as_contiguous`, gap-filled binary output, and Porsche output still allocate the whole `min..=max` span.

Needs:
- explicit bounds / error behavior
- or streaming output paths for sparse images

### Forced-range checksum builds full synthetic data first

[`src/ops/checksum.rs`](/Users/tomford/code/projects/h3xy/src/ops/checksum.rs)

Forced-range checksum still constructs a whole pattern-backed segment before exclusions are applied.

Needs a more careful checksum-collection refactor.

## API Ergonomics

Keep the canonical public model as `HexFile` + `Segment` + `AddressRange`.

Possible thin ergonomic additions without introducing a second public data model:
- `HexFile::from_bytes(base, data)`
- `HexFile::extend_segments(iter)`
- `impl From<(u32, Vec<u8>)> for Segment`
- examples for parse -> mutate -> write
- examples for single-blob callers and sparse-patch callers
