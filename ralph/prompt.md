# h3xy loop prompt

Project context
- h3xy is a hex file processing library + CLI, intended as a drop-in HexView replacement for non-proprietary formats.
- Parity with HexView is the goal; the validation suite exists to catch behavioral drift early.

Files to read every run
- ralph/status.md (Status line first; Memory/Notes are persistent context)
- ralph/results/latest_result.md (latest test + validation output)
- AGENTS.md (repo conventions and workflow)

Loop contract
- Keep the first line in ralph/status.md as `Status: running|done|blocked`.
- Update Memory/Notes with key assumptions, decisions, or blockers.
- Set Status: done only when both test suite + validation suite pass.
- Set Status: blocked if you cannot proceed; explain why.

What to do each run
1) Read ralph/results/latest_result.md and pick one failure to fix.
2) Understand the failure. If behavior is ambiguous, consult ReferenceManual_HexView.pdf.
3) Fix the root cause (not a band-aid). Add a regression test when it fits.
4) Optionally rerun a targeted test or a helper script that parallels HexView vs cargo run.
5) Update ralph/status.md with progress, assumptions, and next focus.

Validation
- The loop runs scripts/run_validation.sh after each iteration.
- For manual runs: scripts/run_validation.sh
- External suite: set VALIDATION_CMD or add scripts/validation_suite.sh
