# h3xy Development Plan

## Overview

Three stages to a working HexView replacement:

1. **Format I/O** - Read/write Intel HEX, S-Record, binary with word-addressing support
2. **Processing Library** - All the mutation/transform operations
3. **CLI/Lib Separation** - Clean public API + CLI tool

---

## Stage 1: Format I/O

**Problem:** Existing crates (e.g., `ihex`, `srec`) choke on word-addressed files or non-standard record types.

### Core Types

```rust
pub struct Segment {
    pub start_address: u32,
    pub data: Vec<u8>,
}

pub struct HexFile {
    segments: Vec<Segment>,
}
```

### Intel HEX Parser

| Record Type | Description | Must Support |
|-------------|-------------|--------------|
| 00 | Data | ✓ |
| 01 | EOF | ✓ |
| 02 | Extended Segment Address (20-bit) | ✓ |
| 03 | Start Segment Address | ignore |
| 04 | Extended Linear Address (32-bit) | ✓ |
| 05 | Start Linear Address | ignore |

**Word-addressing handling:**
- Parser reads addresses as-is (no scaling)
- Caller applies `--address-scale` or `--address-unscale` post-parse
- This keeps parser simple and explicit

**Tasks:**
- [ ] Parse single record line → `(record_type, address, data)`
- [ ] Track extended address state machine
- [ ] Accumulate into `Vec<Segment>`, merging contiguous
- [ ] Validate checksums (warn or error on mismatch)
- [ ] Handle non-standard line endings (CR, LF, CRLF)

**Writer:**
- [ ] Emit records with configurable bytes-per-line (default 16)
- [ ] Auto-insert Extended Address records when needed
- [ ] Option to force Extended Linear vs Extended Segment mode
- [ ] Correct checksum calculation

### S-Record Parser

| Record | Address Bytes | Description |
|--------|---------------|-------------|
| S0 | 2 | Header (ignore content) |
| S1 | 2 | Data (16-bit addr) |
| S2 | 3 | Data (24-bit addr) |
| S3 | 4 | Data (32-bit addr) |
| S5/S6 | - | Record count (validate or ignore) |
| S7/S8/S9 | - | Start address / EOF |

**Tasks:**
- [ ] Parse S0-S9 records
- [ ] Auto-detect S1/S2/S3 (can be mixed in one file)
- [ ] Merge contiguous data into segments
- [ ] Validate checksums

**Writer:**
- [ ] Auto-select S1/S2/S3 based on max address (or force via option)
- [ ] Configurable bytes-per-line
- [ ] Emit S0 header, S5/S6 count, S7/S8/S9 terminator

### Binary Format

**Reader:**
- [ ] Just wrap raw bytes in single Segment at base address
- [ ] Base address from CLI arg (default 0)

**Writer:**
- [ ] Concatenate all segments
- [ ] Option: error if gaps exist, OR fill gaps with byte

### Testing Strategy

```
tests/
  fixtures/
    simple.hex          # basic Intel HEX
    extended_linear.hex # uses type 04 records
    extended_segment.hex # uses type 02 records  
    word_addressed.hex  # addresses are word-scaled
    simple.s19          # S1 records only
    mixed.s37           # S2 and S3 mixed
    gaps.hex            # non-contiguous segments
```

**Tests:**
- Round-trip: parse → write → parse → compare
- Known-good files: parse and assert segment addresses/data
- Malformed input: checksum errors, truncated records, garbage
- Edge cases: empty file, single byte, max address (0xFFFFFFFF)

---

## Stage 2: Processing Library

Operations from hexkit_spec, prioritized:

### Must Have (P0)

| Operation | Description |
|-----------|-------------|
| `filter_range` | Keep only data in range(s) |
| `cut` | Remove data in range(s), may split segments |
| `fill` | Fill range with pattern (gaps only or overwrite) |
| `fill_gaps` | Make all segments contiguous |
| `merge` | Combine two HexFiles (overwrite or preserve mode) |
| `offset` | Shift all addresses by delta |

### Should Have (P1)

| Operation | Description |
|-----------|-------------|
| `align` | Align segment starts/lengths to boundary |
| `scale_addresses` | Multiply addresses (for word-addressed output) |
| `unscale_addresses` | Divide addresses (for word-addressed input) |
| `swap_bytes` | Word/dword byte swap |
| `split` | Split large segments into max-size chunks |

### Nice to Have (P2)

| Operation | Description |
|-----------|-------------|
| `crc32` / `crc16` | Calculate and optionally embed checksum |
| `compare` | Diff two HexFiles |
| `info` | Print segment map, gaps, stats |

### Testing Strategy

- Unit tests per operation with small synthetic segments
- Property tests: operation preserves total bytes (where applicable)
- Integration: chain multiple operations, verify result

---

## Stage 3: CLI / Lib Separation

### Crate Structure

```
h3xy/
├── Cargo.toml
├── src/
│   ├── lib.rs           # pub use all public API
│   ├── segment.rs       # Segment type
│   ├── hexfile.rs       # HexFile type + operations
│   ├── range.rs         # Range parsing/types
│   ├── parse/
│   │   ├── mod.rs
│   │   ├── intel_hex.rs
│   │   ├── srec.rs
│   │   └── binary.rs
│   ├── write/
│   │   ├── mod.rs
│   │   ├── intel_hex.rs
│   │   ├── srec.rs
│   │   └── binary.rs
│   └── error.rs
└── src/bin/
    └── h3xy.rs          # CLI only - uses lib
```

### Public API Surface

```rust
// Re-export in lib.rs
pub use segment::Segment;
pub use hexfile::HexFile;
pub use range::Range;
pub use error::{Error, Result};

// Format enums
pub enum InputFormat { Auto, IntelHex, SRecord, Binary }
pub enum OutputFormat { IntelHex { ... }, SRecord { ... }, Binary }
```

### CLI Design

```
h3xy [OPTIONS] <INPUT> -o <OUTPUT>

Input/Output:
    <INPUT>                 Input file (- for stdin)
    -o, --output <FILE>     Output file (- for stdout)
    -f, --format <FMT>      Input format: auto, ihex, srec, bin
    -O, --output-format     Output format: ihex, srec, bin
    --base <ADDR>           Base address for binary input

Operations (applied in order):
    --range <RANGE>         Keep only this range
    --cut <RANGE>           Remove this range
    --fill <RANGE>          Fill this range
    --fill-pattern <HEX>    Pattern for fill (default FF)
    --fill-gaps             Fill all gaps between segments
    --offset <DELTA>        Shift all addresses
    --merge <FILE>          Merge another file (overwrites)
    --merge-preserve <FILE> Merge another file (preserves existing)
    --address-scale <N>     Multiply all addresses
    --address-unscale <N>   Divide all addresses
    --align <N>             Align to N bytes
    --swap <MODE>           Byte swap: word, dword

Output options:
    --bytes-per-line <N>    Bytes per record (default 16)
    --intel-mode <MODE>     Intel HEX mode: auto, linear, segment
    --srec-type <TYPE>      S-Record type: auto, s1, s2, s3

Info:
    --info                  Print segment info, don't write output
    --verbose               Print operations as they happen
```

### Testing Strategy

- CLI integration tests using `assert_cmd` crate
- Test each flag combination with fixture files
- Test piping (stdin/stdout)
- Test error messages for bad input

---

## Milestones

### M1: Core I/O (Week 1)
- [ ] Segment + HexFile types
- [ ] Intel HEX read/write
- [ ] Basic CLI: convert between formats
- [ ] Test fixtures + round-trip tests

### M2: S-Record + Operations (Week 2)  
- [ ] S-Record read/write
- [ ] P0 operations: filter, cut, fill, merge, offset
- [ ] CLI flags for operations

### M3: Full Feature (Week 3)
- [ ] P1 operations: align, scale, swap, split
- [ ] Binary format with gap handling
- [ ] CRC operations
- [ ] Polish CLI, error messages

### M4: Release Prep
- [ ] Documentation (README, lib docs)
- [ ] Examples
- [ ] CI setup
- [ ] Publish to crates.io

---

## Decisions

1. **Error handling:** Strict - fail on any checksum error
2. **Segment merging:** Auto-merge adjacent segments at output time
3. **Address overflow:** Error if operation pushes address > u32::MAX
4. **Stdin/stdout:** Defer for now
