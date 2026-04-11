---
name: h3xy
description: "Guide for h3xy, a HexView-compatible CLI for non-proprietary firmware workflows. Use when working with h3xy or HexView slash-style pipelines for Intel HEX, S-Record, HEX ASCII, or binary files — range edits, merges, checksums, supported signing/verification subset, remaps, and export."
---

# h3xy

Use `h3xy` as a pipeline tool. Load one input image, apply ordered operations, then export once.

Treat operation order as part of the interface. Do not reorder flags casually. Later flags act on the result of earlier ones, and the command line is designed around that model.

Use slash-style HexView syntax in examples and commands. Keep examples in installed-binary form such as `h3xy input.hex /CR:'0x1000-0x1FFF' /CS0 /XI -o output.hex`.

Keep CLI parsing rules in mind:

- Use `-o <file>` for the output path; most other options use `/FLAG:value` or `/FLAG=value`.
- Use `--` before an absolute Unix path if it might be mistaken for a slash option.
- Expect the first positional argument to be the input file when no explicit import option is used.
- Expect numeric arguments to accept common HexView-style forms such as `0x1234`, `1234h`, `0b1010`, and separators like `_` or `.`.

Read [cli-reference.md](/Users/tomford/code/projects/h3xy/skill/h3xy/references/cli-reference.md) for the actual interface. Scan the execution-order section first, then jump to the specific stage you need.

Use the reference to answer questions like:

- which flag to use for a given transformation
- which order to place fill, cut, merge, checksum, and export flags
- which import or export format matches the current file
- which checksum or signing subset is available in `h3xy`
- which HexView features are intentionally out of scope here

Treat this skill as CLI-only. HexView also supports additional proprietary or OEM-specific formats and DLL-backed features that are not covered here.
