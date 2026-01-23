# Refactor Suggestions (Library + CLI)

## Critical findings (architecture & correctness)
- **Keep CLI and library pipeline in lockstep**: per‑flag helpers and a pipeline exist, but any remaining CLI‑only behaviors (output handling, argument parsing quirks) risk drift from library semantics.
- **Path parsing ambiguity**: CLI accepts Unix absolute paths that start with `/` by heuristic; this is brittle and can misclassify options if they contain `/` in unusual positions.
- **Drop-in parity mandate**: outputs must be binary‑equivalent to HexView for non‑proprietary formats; any divergence in op ordering or per‑flag semantics is a correctness bug.

## Status (current)
- Output options parsing centralized; ranges + merges share helpers.
- Ops errors wrapped with context via `OpsError::Context`.
- Checksum append/prepend/overwrite now checked for overflow/underflow (tests added).

## Medium refactors (behavior clarity)

## Small refactors (cleanup + ergonomics)
- **Normalize path handling**: parse arguments with an explicit “positional vs option” rule rather than heuristic `/` detection (now supports `--`; absolute `/path` treated as input only if it exists).
- **Eliminate repetitive option parsing**: create a small macro or table‑driven parser for `/X*` and `/C*` variants to reduce branching. (output options centralized; ranges + merges share helpers; other options still branching)
- **Eliminate repetitive option parsing**: keyed options now split into helpers; remaining duplication minimal.
- **Eliminate repetitive option parsing**: simple flags now centralized; remaining options still branching.
- **Better error context**: in ops, include option names (e.g., `OpsError::InvalidRemapParams("/REMAP: size must be non-zero")`) so CLI can write actionable messages. (addressed via `OpsError::Context` + flag wrappers; core ops still generic)

## Behavior notes
- Checksum append/prepend/overwrite now uses checked arithmetic; overflow returns `OpsError::AddressOverflow`.
- `Segment::end_address` now saturates on overflow.
- Contiguity checks use checked add to avoid overflow.

## Testing gaps to address after refactor
- **Invariant tests**: add tests that verify the chosen segment invariant across operations (merge+fill+align sequences).
- **Library parity**: add tests that compare CLI pipeline output to library pipeline output for the same operation set.

## TODOs (near-term)
- **Crate reuse audit**: complete. Decision: keep hand‑rolled parsers/writers for HexView parity; bin_file from mint is a candidate but remains optional until validation suite confirms behavior.
- **Validation suite setup**: not runnable on this machine; document required environment and add a runbook.
- **Proprietary formats**: explicitly exclude from first‑pass validation scope.
- **/S08MAP**: manual lacks formula; implementation matches examples but needs validation.
