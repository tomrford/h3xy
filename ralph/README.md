# Ralph loop

Purpose
- Run codex in a bounded loop with persistent memory and automated validation.
- Keep behavior aligned with HexView through repeated test + validation runs.

How to run
- scripts/ralph_loop.sh

Key files
- ralph/status.md: top line is the loop exit condition; Memory/Notes persist.
- ralph/prompt.md: instructions given to the agent each run.
- ralph/results/latest_result.md: output from the most recent test + validation run.
- ralph/history/: raw agent logs per iteration.

Environment
- MAX_ITERATIONS (default 200)
- SLEEP_SECONDS (default 2)
- VALIDATION_CMD (optional; overrides validation suite command)
