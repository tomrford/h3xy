# h3xy Validation Framework

Automated validation of h3xy against HexView.exe reference implementation.

## Quick Start

```bash
cd validation

# Full validation run (generates inputs, tests, compares)
uv run python main.py

# Quick smoke test
uv run python main.py --random-inputs 2 --fuzz-per-file 2 --stop-on-fail

# Reproducible run with specific seed
uv run python main.py --seed 12345

# Verbose output
uv run python main.py --verbose --keep-outputs
```

## Prerequisites

This framework requires:
- **HexView.exe** symlinked at `../reference/HexView.exe`
- **Rust project** built (`cargo build` in parent dir)
- **compare.sh** script at `../scripts/compare.sh`

For WSL environments:
- Set up `../scratchpad/` symlink to Windows-accessible directory
- Use `-s` flag in compare.sh for path translation

## Architecture

```
validation/
├── main.py                     # Entry point
├── src/h3xy_validation/
│   ├── intel_hex.py           # Intel HEX format utilities
│   ├── hexgen.py              # Input file generator
│   ├── testgen.py             # Test case generator
│   └── runner.py              # Test orchestration
├── inputs/                     # Generated input .hex files
├── outputs/                    # Test output files
└── failures.json              # Last run's failures
```

## Test Generation

### Input Files (hexgen.py)

Generates varied hex files:

| Category | Files | Purpose |
|----------|-------|---------|
| Basic shapes | single_small, multi_basic, scattered | Standard segment configurations |
| Edge cases | tiny, max_addr, boundary_64k, misaligned | Boundary conditions |
| Patterns | sequential, const_ff, alternating, words | Recognizable data |
| Merge pairs | overlap, adjacent, disjoint | Merge operation testing |
| Random | rand_000...rand_N | Fuzz testing |

### Test Cases (testgen.py)

Operations tested (HexView CLI syntax):

| Operation | CLI | Description |
|-----------|-----|-------------|
| Address Range | `/AR:'start-end'` | Filter to range |
| Cut | `/CR:'start-end'` | Remove range |
| Fill Range | `/FR:'start,len'` + `/FP:pattern` | Fill with pattern |
| Fill All | `/FA` | Fill all gaps |
| Align | `/AD:N`, `/AL` | Address/length alignment |
| Split | `/SB:size` | Split into blocks |
| Swap | `/SWAPWORD`, `/SWAPLONG` | Byte order swap |
| CRC | `/CS0:@addr`, `/CS1:@addr` | Checksum insertion |
| Merge | `/MO:file`, `/MT:file` | Overwrite/preserve merge |

Each input file gets:
- Passthrough test (baseline)
- Operation variants with different parameters
- Combined operation tests
- Random fuzz tests

## CLI Options

```
--seed N              Random seed for reproducibility (default: 42)
--random-inputs N     Random input files to generate (default: 10)
--fuzz-per-file N     Fuzz tests per input file (default: 5)
--stop-on-fail        Stop on first failure
--max-failures N      Stop after N failures
-v, --verbose         Verbose comparison output
-k, --keep-outputs    Keep output files for inspection
--no-generate         Use existing inputs (don't regenerate)
--project-root PATH   Path to h3xy project (default: ..)
```

## Output

After a run:
- **Console**: Progress dots, summary, first 20 failures
- **failures.json**: Full failure details for automated processing

Example failure entry:
```json
{
  "name": "ar_single_small_0_ff",
  "input_file": "single_small.hex",
  "args": ["/AR:'0x0-0xFF'"],
  "output_name": "ar_single_small_0_ff.hex",
  "exit_code": 1,
  "error": null,
  "duration_ms": 234.5
}
```

## Doom Loop Integration

For continuous fixing:

```bash
# Run validation
uv run python main.py --seed 42 --max-failures 5

# Check failures.json
cat failures.json | jq '.failures[:3]'

# Fix issues in h3xy, then re-run with same seed
uv run python main.py --seed 42 --max-failures 5
```

The same seed ensures identical test cases for reproducible debugging.

## Extending

### Add new input patterns

Edit `hexgen.py`:
```python
def gen_my_pattern(self, name: str = "my_pattern") -> Path:
    return self.gen_single_segment(
        name, start_address=0x1000, size=512,
        pattern_fn=self.pattern_sequential
    )
```

### Add new test operations

Edit `testgen.py`:
```python
def gen_my_operation(self, input_file: str) -> TestCase:
    return TestCase(
        name=f"myop_{Path(input_file).stem}",
        input_file=input_file,
        args=["/MYOP:value"],
        output_name="output.hex",
    )
```
