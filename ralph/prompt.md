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
2) Understand the failure. If behavior is ambiguous, consult ReferenceManual_HexView.txt.
3) Fix the root cause (not a band-aid). Add a regression test when it fits.
4) Prefer targeted tests (unit/integration or a single compare.sh case). Do not run the full validation suite; the outer loop handles it.
5) Update ralph/status.md with progress, assumptions, and next focus.

Validation
- The loop runs scripts/run_validation.sh before each iteration.
- Full validation requires the WSL HexView environment; do not run it here unless instructed.
- Manual: scripts/run_validation.sh (set TEST_CMD for targeted tests, SKIP_VALIDATION=0 to allow suite).

Exit codes from compare.sh
- 0 = outputs match (pass)
- 1 = outputs differ (real mismatch to fix)
- 2 = execution error (HexView/compare.sh failed; mark blocked for Tom)
