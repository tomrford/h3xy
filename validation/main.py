#!/usr/bin/env python3
"""Main entry point for h3xy validation.

Usage:
    # Run from validation directory
    cd validation
    uv run python main.py

    # Or with options
    uv run python main.py --seed 12345 --verbose

    # Quick smoke test
    uv run python main.py --random-inputs 2 --fuzz-per-file 2 --stop-on-fail
"""

import sys
from pathlib import Path

# Add src to path for local development
sys.path.insert(0, str(Path(__file__).parent / "src"))

from h3xy_validation.runner import main

if __name__ == "__main__":
    sys.exit(main())
