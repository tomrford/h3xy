"""Validation runner - orchestrates hex generation, test execution, and reporting."""

import argparse
import os
import json
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import TextIO

from .hexgen import HexGenerator, GeneratorConfig
from .testgen import TestCaseGenerator, TestGeneratorConfig, TestCase


@dataclass
class RunConfig:
    """Configuration for a validation run."""

    seed: int = 42
    base_dir: Path = field(default_factory=lambda: Path.cwd())
    inputs_dir: Path = field(default_factory=lambda: Path("inputs"))
    outputs_dir: Path = field(default_factory=lambda: Path("outputs"))
    project_root: Path = field(default_factory=lambda: Path(".."))

    # Generation options
    generate_inputs: bool = True
    random_input_count: int = 10
    fuzz_tests_per_file: int = 5

    # Execution options
    stop_on_first_failure: bool = False
    max_failures: int = 0  # 0 = unlimited
    max_tests: int = 0  # 0 = unlimited
    verbose: bool = False
    keep_outputs: bool = False

    # Comparison script
    compare_script: Path = field(
        default_factory=lambda: Path("../scripts/compare.sh")
    )
    compare_use_scratchpad: bool = False
    compare_copy_inputs: bool = False


@dataclass
class TestResult:
    """Result of a single test."""

    test: TestCase
    passed: bool
    duration_ms: float
    exit_code: int
    stdout: str = ""
    stderr: str = ""
    error: str | None = None


@dataclass
class RunResult:
    """Result of a full validation run."""

    seed: int
    timestamp: str
    total_tests: int
    passed: int
    failed: int
    skipped: int
    duration_seconds: float
    failures: list[TestResult] = field(default_factory=list)


class ValidationRunner:
    """Orchestrates the full validation process."""

    def __init__(self, config: RunConfig | None = None):
        self.config = config or RunConfig()
        self._ensure_dirs()

    def _ensure_dirs(self) -> None:
        """Ensure required directories exist."""
        (self.config.base_dir / self.config.inputs_dir).mkdir(
            parents=True, exist_ok=True
        )
        (self.config.base_dir / self.config.outputs_dir).mkdir(
            parents=True, exist_ok=True
        )

    def _abs_path(self, rel_path: Path) -> Path:
        """Convert relative path to absolute."""
        return (self.config.base_dir / rel_path).resolve()

    # ─────────────────────────────────────────────────────────────────────
    # Input generation
    # ─────────────────────────────────────────────────────────────────────

    def generate_inputs(self) -> dict[str, Path]:
        """Generate all input hex files."""
        print(f"\n{'='*60}")
        print("Generating input files...")
        print(f"{'='*60}\n")

        gen_config = GeneratorConfig(
            seed=self.config.seed,
            output_dir=self._abs_path(self.config.inputs_dir),
        )
        generator = HexGenerator(gen_config)

        files = {}

        # Standard suite
        print("  Standard suite...")
        standard = generator.gen_standard_suite()
        files.update(standard)
        print(f"    Generated {len(standard)} files")

        # Random files
        if self.config.random_input_count > 0:
            print(f"  Random files ({self.config.random_input_count})...")
            random_files = generator.gen_random_suite(
                count=self.config.random_input_count,
                prefix="rand"
            )
            files.update(random_files)
            print(f"    Generated {len(random_files)} files")

        # Merge pairs
        print("  Merge pairs...")
        overlap_a, overlap_b = generator.gen_merge_pair_overlapping()
        adjacent_a, adjacent_b = generator.gen_merge_pair_adjacent()
        disjoint_a, disjoint_b = generator.gen_merge_pair_disjoint()

        files["merge_overlap_a"] = overlap_a
        files["merge_overlap_b"] = overlap_b
        files["merge_adjacent_a"] = adjacent_a
        files["merge_adjacent_b"] = adjacent_b
        files["merge_disjoint_a"] = disjoint_a
        files["merge_disjoint_b"] = disjoint_b
        print("    Generated 6 merge pair files")

        print(f"\n  Total: {len(files)} input files generated\n")
        return files

    # ─────────────────────────────────────────────────────────────────────
    # Test case generation
    # ─────────────────────────────────────────────────────────────────────

    def generate_tests(self, input_files: dict[str, Path]) -> list[TestCase]:
        """Generate test cases for all inputs."""
        print(f"\n{'='*60}")
        print("Generating test cases...")
        print(f"{'='*60}\n")

        gen_config = TestGeneratorConfig(
            seed=self.config.seed,
            inputs_dir=self._abs_path(self.config.inputs_dir),
            outputs_dir=self._abs_path(self.config.outputs_dir),
        )
        generator = TestCaseGenerator(gen_config)

        # Get relative paths for test generation
        rel_files = [f"{name}.hex" for name in input_files.keys()]

        tests = []

        # Standard suite
        print("  Standard test suite...")
        standard_tests = generator.gen_standard_suite(rel_files)
        tests.extend(standard_tests)
        print(f"    Generated {len(standard_tests)} tests")

        # Fuzz tests
        if self.config.fuzz_tests_per_file > 0:
            print(f"  Fuzz tests ({self.config.fuzz_tests_per_file} per file)...")
            fuzz_tests = generator.gen_fuzz_suite(
                rel_files,
                tests_per_file=self.config.fuzz_tests_per_file
            )
            tests.extend(fuzz_tests)
            print(f"    Generated {len(fuzz_tests)} tests")

        # Merge tests
        print("  Merge tests...")
        merge_pairs = [
            ("merge_overlap_a.hex", "merge_overlap_b.hex"),
            ("merge_adjacent_a.hex", "merge_adjacent_b.hex"),
            ("merge_disjoint_a.hex", "merge_disjoint_b.hex"),
        ]
        merge_tests = generator.gen_merge_suite(merge_pairs)
        tests.extend(merge_tests)
        print(f"    Generated {len(merge_tests)} tests")

        print(f"\n  Total: {len(tests)} test cases generated\n")
        return tests

    # ─────────────────────────────────────────────────────────────────────
    # Test execution
    # ─────────────────────────────────────────────────────────────────────

    def run_single_test(self, test: TestCase) -> TestResult:
        """Run a single test case."""
        inputs_dir = self._abs_path(self.config.inputs_dir)
        outputs_dir = self._abs_path(self.config.outputs_dir)
        compare_script = self._abs_path(self.config.compare_script)

        # Build command
        args = test.to_args(inputs_dir, outputs_dir)
        cmd = [str(compare_script)]
        if self.config.compare_use_scratchpad:
            cmd.append("-s")
        if self.config.compare_copy_inputs:
            cmd.append("-c")
        if self.config.verbose:
            cmd.append("-v")
        if self.config.keep_outputs:
            cmd.append("-k")
        cmd.append("--")
        cmd.extend(args)

        start = time.perf_counter()
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=int(os.environ.get("H3XY_COMPARE_TIMEOUT", "300")),
                cwd=str(self._abs_path(self.config.project_root)),
            )
            duration_ms = (time.perf_counter() - start) * 1000

            return TestResult(
                test=test,
                passed=(result.returncode == 0),
                duration_ms=duration_ms,
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr,
            )

        except subprocess.TimeoutExpired:
            duration_ms = (time.perf_counter() - start) * 1000
            return TestResult(
                test=test,
                passed=False,
                duration_ms=duration_ms,
                exit_code=-1,
                error="Test timed out after 60 seconds",
            )

        except Exception as e:
            duration_ms = (time.perf_counter() - start) * 1000
            return TestResult(
                test=test,
                passed=False,
                duration_ms=duration_ms,
                exit_code=-1,
                error=str(e),
            )

    def run_tests(self, tests: list[TestCase]) -> RunResult:
        """Run all tests and collect results."""
        print(f"\n{'='*60}")
        print(f"Running {len(tests)} tests...")
        print(f"{'='*60}\n")

        start_time = time.perf_counter()
        timestamp = datetime.now().isoformat()

        passed = 0
        failed = 0
        skipped = 0
        failures: list[TestResult] = []

        for i, test in enumerate(tests, 1):
            # Progress indicator
            status_char = "."
            result = self.run_single_test(test)

            if result.passed:
                passed += 1
                status_char = "."
            else:
                failed += 1
                status_char = "F"
                failures.append(result)

            # Print progress
            sys.stdout.write(status_char)
            if i % 50 == 0:
                sys.stdout.write(f" [{i}/{len(tests)}]\n")
            sys.stdout.flush()

            # Check stop conditions
            if self.config.stop_on_first_failure and failed > 0:
                print(f"\n\nStopping on first failure.")
                break

            if self.config.max_failures > 0 and failed >= self.config.max_failures:
                print(f"\n\nReached max failures ({self.config.max_failures}).")
                break

        # Final newline if needed
        if len(tests) % 50 != 0:
            print()

        duration = time.perf_counter() - start_time

        return RunResult(
            seed=self.config.seed,
            timestamp=timestamp,
            total_tests=len(tests),
            passed=passed,
            failed=failed,
            skipped=skipped,
            duration_seconds=duration,
            failures=failures,
        )

    # ─────────────────────────────────────────────────────────────────────
    # Reporting
    # ─────────────────────────────────────────────────────────────────────

    def print_summary(self, result: RunResult) -> None:
        """Print test run summary."""
        print(f"\n{'='*60}")
        print("RESULTS")
        print(f"{'='*60}\n")

        print(f"  Seed:     {result.seed}")
        print(f"  Time:     {result.duration_seconds:.2f}s")
        print(f"  Tests:    {result.total_tests}")
        print(f"  Passed:   {result.passed}")
        print(f"  Failed:   {result.failed}")
        print(f"  Skipped:  {result.skipped}")
        print()

        if result.failures:
            print(f"{'─'*60}")
            print("FAILURES:")
            print(f"{'─'*60}\n")

            for i, failure in enumerate(result.failures[:20], 1):
                print(f"  {i}. {failure.test.name}")
                print(f"     Input: {failure.test.input_file}")
                print(f"     Args:  {' '.join(failure.test.args)}")
                if failure.error:
                    print(f"     Error: {failure.error}")
                if failure.stderr and self.config.verbose:
                    print(f"     Stderr: {failure.stderr[:200]}")
                print()

            if len(result.failures) > 20:
                print(f"  ... and {len(result.failures) - 20} more failures\n")

        print(f"{'='*60}\n")

    def write_failures_json(self, result: RunResult, path: Path) -> None:
        """Write failures to JSON for processing."""
        data = {
            "seed": result.seed,
            "timestamp": result.timestamp,
            "summary": {
                "total": result.total_tests,
                "passed": result.passed,
                "failed": result.failed,
                "skipped": result.skipped,
            },
            "failures": [
                {
                    "name": f.test.name,
                    "input_file": f.test.input_file,
                    "args": f.test.args,
                    "output_name": f.test.output_name,
                    "exit_code": f.exit_code,
                    "error": f.error,
                    "duration_ms": f.duration_ms,
                }
                for f in result.failures
            ],
        }

        with open(path, "w") as f:
            json.dump(data, f, indent=2)

    # ─────────────────────────────────────────────────────────────────────
    # Main entry point
    # ─────────────────────────────────────────────────────────────────────

    def run(self) -> RunResult:
        """Run the full validation process."""
        print(f"\n{'#'*60}")
        print("#  h3xy Validation Run")
        print(f"#  Seed: {self.config.seed}")
        print(f"{'#'*60}")

        # Generate inputs
        if self.config.generate_inputs:
            input_files = self.generate_inputs()
        else:
            # List existing input files
            inputs_dir = self._abs_path(self.config.inputs_dir)
            input_files = {
                p.stem: p for p in inputs_dir.glob("*.hex")
            }
            print(f"Using {len(input_files)} existing input files")

        # Generate tests
        tests = self.generate_tests(input_files)
        if self.config.max_tests > 0:
            tests = tests[: self.config.max_tests]

        # Run tests
        result = self.run_tests(tests)

        # Report
        self.print_summary(result)

        # Write failures to JSON
        failures_path = self.config.base_dir / "failures.json"
        self.write_failures_json(result, failures_path)
        print(f"Failures written to: {failures_path}\n")

        return result


def main() -> int:
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Run h3xy validation tests against HexView.exe"
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed for reproducibility (default: 42)"
    )
    parser.add_argument(
        "--random-inputs", type=int, default=10,
        help="Number of random input files to generate (default: 10)"
    )
    parser.add_argument(
        "--fuzz-per-file", type=int, default=5,
        help="Number of fuzz tests per input file (default: 5)"
    )
    parser.add_argument(
        "--stop-on-fail", action="store_true",
        help="Stop on first failure"
    )
    parser.add_argument(
        "--max-failures", type=int, default=0,
        help="Stop after N failures (0 = unlimited)"
    )
    parser.add_argument(
        "--max-tests", type=int, default=0,
        help="Run only first N tests (0 = unlimited)"
    )
    parser.add_argument(
        "-v", "--verbose", action="store_true",
        help="Verbose output"
    )
    parser.add_argument(
        "-k", "--keep-outputs", action="store_true",
        help="Keep output files after comparison"
    )
    parser.add_argument(
        "--no-generate", action="store_true",
        help="Don't regenerate inputs, use existing"
    )
    parser.add_argument(
        "--project-root", type=Path, default=Path(".."),
        help="Path to h3xy project root (default: ..)"
    )
    parser.add_argument(
        "--inputs-dir", type=Path, default=None,
        help="Input directory (default: scratchpad/validation_inputs when scratchpad exists)"
    )
    parser.add_argument(
        "--outputs-dir", type=Path, default=None,
        help="Output directory (default: scratchpad/validation_outputs when scratchpad exists)"
    )
    parser.add_argument(
        "--compare-scratchpad", action="store_true", default=None,
        help="Run compare.sh with -s (scratchpad mode)"
    )
    parser.add_argument(
        "--compare-copy-inputs", action="store_true", default=None,
        help="Run compare.sh with -c (copy inputs into scratchpad)"
    )

    args = parser.parse_args()
    scratchpad = Path(
        os.environ.get("SCRATCHPAD", args.project_root / "scratchpad")
    )
    use_scratchpad_defaults = (
        args.inputs_dir is None
        and args.outputs_dir is None
        and scratchpad.exists()
    )
    if use_scratchpad_defaults:
        inputs_dir = scratchpad / "validation_inputs"
        outputs_dir = scratchpad / "validation_outputs"
        compare_use_scratchpad = (
            True if args.compare_scratchpad is None else args.compare_scratchpad
        )
        compare_copy_inputs = (
            True if args.compare_copy_inputs is None else args.compare_copy_inputs
        )
    else:
        inputs_dir = args.inputs_dir or Path("inputs")
        outputs_dir = args.outputs_dir or Path("outputs")
        compare_use_scratchpad = bool(args.compare_scratchpad)
        compare_copy_inputs = bool(args.compare_copy_inputs)

    config = RunConfig(
        seed=args.seed,
        inputs_dir=inputs_dir,
        outputs_dir=outputs_dir,
        random_input_count=args.random_inputs,
        fuzz_tests_per_file=args.fuzz_per_file,
        stop_on_first_failure=args.stop_on_fail,
        max_failures=args.max_failures,
        max_tests=args.max_tests,
        verbose=args.verbose,
        keep_outputs=args.keep_outputs,
        generate_inputs=not args.no_generate,
        project_root=args.project_root,
        compare_use_scratchpad=compare_use_scratchpad,
        compare_copy_inputs=compare_copy_inputs,
    )

    runner = ValidationRunner(config)
    result = runner.run()

    # Exit code: 0 if all passed, 1 if any failed
    return 0 if result.failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
