# HexKit: Rust Hex File Processing Library & CLI

A cross-platform replacement for HexView's core processing features, designed as both a library crate and CLI tool.

---

## Core Data Model

### Segment

A contiguous block of data at a specific address:

```rust
pub struct Segment {
    pub start_address: u32,
    pub data: Vec<u8>,
}

impl Segment {
    pub fn end_address(&self) -> u32;  // start_address + data.len() - 1
    pub fn len(&self) -> usize;
}
```

### HexFile

A collection of non-overlapping segments:

```rust
pub struct HexFile {
    segments: Vec<Segment>,  // sorted by start_address, non-overlapping
}
```

---

## Range Specification

Ranges are specified in two equivalent formats, used throughout the API:

```rust
pub enum Range {
    /// Start address and length: "0x1000,0x200"
    StartLength { start: u32, length: u32 },
    /// Start and end address (inclusive): "0x1000-0x11FF"  
    StartEnd { start: u32, end: u32 },
}

impl Range {
    pub fn start(&self) -> u32;
    pub fn end(&self) -> u32;      // inclusive
    pub fn length(&self) -> u32;
    pub fn contains(&self, addr: u32) -> bool;
    pub fn overlaps(&self, other: &Range) -> bool;
}
```

**Parsing rules:**
- Values accept: decimal (`255`), hex (`0x1000`), binary (`0b1010` or `1010b`)
- Multiple ranges separated by `:` → `Vec<Range>`

---

## File Format Support

### Input Formats

```rust
pub enum InputFormat {
    Auto,       // Detect from content
    IntelHex,   // Lines starting with ':'
    SRecord,    // Lines starting with 'S'
    Binary,     // Raw bytes, requires base address
}

pub fn parse(input: &[u8], format: InputFormat, base_address: Option<u32>) -> Result<HexFile>;
pub fn parse_file(path: &Path, format: InputFormat, base_address: Option<u32>) -> Result<HexFile>;
```

**Auto-detection logic:**
1. If first non-empty line starts with `:` → Intel HEX
2. If first non-empty line starts with `S` → S-Record
3. Otherwise → Binary (requires base address or defaults to 0x0)

### Output Formats

```rust
pub enum OutputFormat {
    IntelHex { 
        bytes_per_line: u8,           // default 16, max typically 255
        address_mode: IntelHexMode,   // Auto, ExtendedLinear, ExtendedSegment
    },
    SRecord {
        bytes_per_line: u8,           // default 16
        record_type: SRecordType,     // Auto, S1, S2, S3
    },
    Binary,                           // Raw concatenated data (gaps filled or error)
}

pub enum IntelHexMode {
    Auto,             // Choose based on max address
    ExtendedLinear,   // 32-bit addressing (rectype=1)
    ExtendedSegment,  // 20-bit addressing (rectype=2)
}

pub enum SRecordType {
    Auto,  // S1 for 16-bit, S2 for 24-bit, S3 for 32-bit
    S1,    // 16-bit addresses
    S2,    // 24-bit addresses
    S3,    // 32-bit addresses
}

impl HexFile {
    pub fn to_bytes(&self, format: OutputFormat) -> Result<Vec<u8>>;
    pub fn write_file(&self, path: &Path, format: OutputFormat) -> Result<()>;
}
```

---

## Core Operations

### 1. Address Range Filtering

**Load only specific range(s):**

```rust
impl HexFile {
    /// Keep only data within the specified range
    /// Clips segments that partially overlap
    pub fn filter_range(&mut self, range: Range);
    
    /// Keep only data within any of the specified ranges
    pub fn filter_ranges(&mut self, ranges: &[Range]);
}
```

**CLI:** `--range 0x1000,0x200` or `--range 0x1000-0x11FF`

**Behavior:**
- Data outside range is discarded
- Segments partially inside are clipped to fit
- Multiple ranges: data kept if in ANY range

---

### 2. Cut (Remove) Data

**Remove data within specified range(s):**

```rust
impl HexFile {
    /// Remove all data within the specified range
    /// Splits segments if the cut is in the middle
    pub fn cut(&mut self, range: Range);
    
    /// Remove data within multiple ranges
    pub fn cut_ranges(&mut self, ranges: &[Range]);
}
```

**CLI:** `--cut 0x1000,0x200` or `--cut 0x7000-0x7FFF`

**Behavior:**
- Data in range is removed
- If cut is in middle of segment, segment splits into two
- Multiple ranges: each cut applied

**Example:**
```
Before: Segment 0x1000-0x2FFF
Cut:    0x1800-0x1FFF  
After:  Segment 0x1000-0x17FF, Segment 0x2000-0x2FFF
```

---

### 3. Fill Region

**Create/fill regions with pattern:**

```rust
pub struct FillOptions {
    pub pattern: Vec<u8>,       // Pattern to repeat (default: [0xFF])
    pub overwrite: bool,        // If true, overwrites existing data
}

impl HexFile {
    /// Fill a region with the specified pattern
    /// By default, does NOT overwrite existing data (fills gaps only)
    pub fn fill(&mut self, range: Range, options: FillOptions);
    
    /// Fill multiple regions
    pub fn fill_ranges(&mut self, ranges: &[Range], options: FillOptions);
}
```

**CLI:** `--fill 0x1000,0x200` with `--fill-pattern DEADBEEF`

**Behavior (overwrite=false, default):**
- Creates new segments in gaps
- Existing data preserved
- Pattern repeats to fill entire range

**Behavior (overwrite=true):**
- Overwrites everything in range with pattern

---

### 4. Create Single Region (Fill All Gaps)

**Merge all segments into one contiguous block:**

```rust
impl HexFile {
    /// Fill all gaps between first and last segment with fill byte
    /// Result: single contiguous segment
    pub fn fill_gaps(&mut self, fill_byte: u8);
}
```

**CLI:** `--fill-gaps` with `--fill-byte FF`

**Behavior:**
- Finds min and max addresses
- Fills all gaps between segments with fill byte
- Merges into single segment

---

### 5. Alignment

**Align segment start addresses and/or lengths:**

```rust
pub struct AlignOptions {
    pub alignment: u32,         // Must be power of 2
    pub fill_byte: u8,          // Byte to use for padding (default 0xFF)
    pub align_length: bool,     // Also align segment lengths
}

impl HexFile {
    /// Align all segment start addresses to multiples of alignment
    /// Prepends fill bytes as needed
    pub fn align(&mut self, options: AlignOptions);
}
```

**CLI:** `--align 4` with optional `--align-length`

**Behavior:**
- For each segment with misaligned start:
  - Calculate aligned start (round DOWN to multiple)
  - Prepend fill bytes from aligned start to original start
- If `align_length`:
  - Append fill bytes until length is multiple of alignment

**Example:**
```
Alignment: 4, fill_byte: 0xFF
Before: Segment at 0x1001, length 5
After:  Segment at 0x1000, length 8
        (1 byte prepended, 2 bytes appended)
```

---

### 6. Merge Files

**Combine data from multiple files:**

```rust
pub enum MergeMode {
    /// New data overwrites existing (opaque)
    Overwrite,
    /// Existing data preserved, new fills gaps (transparent)
    Preserve,
}

pub struct MergeOptions {
    pub mode: MergeMode,
    pub offset: i64,              // Address offset to apply (can be negative)
    pub range: Option<Range>,     // Only merge data within this range (before offset)
}

impl HexFile {
    /// Merge another file into this one
    pub fn merge(&mut self, other: &HexFile, options: MergeOptions);
}
```

**CLI:** 
- `--merge-overwrite file.hex` (opaque mode)
- `--merge-preserve file.hex` (transparent mode)  
- With `--merge-offset -0x1000` and/or `--merge-range 0x2000-0x3FFF`

**Behavior - Overwrite mode:**
- Other file's data replaces any existing data at same addresses
- Offset applied to all addresses in other file

**Behavior - Preserve mode:**
- Existing data kept, other file only fills gaps
- Offset applied to all addresses in other file

**Range behavior:**
- Range applies to source file BEFORE offset
- First filter by range, then apply offset

---

### 7. Split Blocks

**Split large segments into smaller chunks:**

```rust
impl HexFile {
    /// Split any segment larger than max_size into multiple segments
    pub fn split(&mut self, max_size: u32);
}
```

**CLI:** `--split-size 0x1000`

**Behavior:**
- Each segment > max_size becomes multiple segments
- Addresses remain contiguous
- Useful for download protocols with block size limits

---

### 8. Byte Swapping

**Swap byte order within words/dwords:**

```rust
pub enum SwapMode {
    /// Swap pairs: AA BB → BB AA
    Word,
    /// Swap quads: AA BB CC DD → DD CC BB AA  
    DWord,
}

impl HexFile {
    /// Swap bytes within all segments
    /// Segments must have lengths that are multiples of swap size
    pub fn swap_bytes(&mut self, mode: SwapMode) -> Result<()>;
}
```

**CLI:** `--swap word` or `--swap dword`

**Behavior:**
- Operates on raw data in each segment
- Error if segment length not multiple of 2 (word) or 4 (dword)

---

### 9. Address Scaling (Word Addressing)

**Multiply or divide addresses for word-addressed architectures:**

```rust
impl HexFile {
    /// Multiply all addresses by factor (for word→byte addressing)
    /// e.g., factor=2 for 16-bit word machines
    pub fn scale_addresses(&mut self, factor: u32);
    
    /// Divide all addresses by factor (for byte→word addressing)
    /// Errors if any address not evenly divisible
    pub fn unscale_addresses(&mut self, divisor: u32) -> Result<()>;
}
```

**CLI:** `--address-scale 2` or `--address-unscale 2`

**Use cases:**
- dsPIC: addresses in hex file are 2x actual (doubled)
- Some DSPs use word addressing, linker outputs byte addresses

**Note:** This is simpler than HexView's dsPIC-specific ghost byte handling. If you need ghost byte support, implement separately.

---

## Checksum Operations

### CRC Calculation

```rust
pub enum ChecksumAlgorithm {
    /// CRC-32 IEEE (polynomial 0x04C11DB7, init 0xFFFFFFFF, final XOR)
    Crc32,
    /// CRC-16 CCITT (polynomial 0x1021, init 0xFFFF)
    Crc16Ccitt,
    /// Simple byte sum into 16-bit value
    ByteSum16,
    /// 16-bit word sum (specify endianness)
    WordSum16 { big_endian: bool },
    /// SHA-256 hash
    Sha256,
}

pub struct ChecksumOptions {
    pub algorithm: ChecksumAlgorithm,
    pub range: Option<Range>,           // Limit calculation to this range
    pub exclude_ranges: Vec<Range>,     // Exclude these ranges from calculation
    pub big_endian_output: bool,        // Output byte order (default: true)
}

pub enum ChecksumTarget {
    /// Write to file (returns value, doesn't modify HexFile)
    None,
    /// Insert at specific address
    Address(u32),
    /// Append after last byte of last segment
    Append,
    /// Insert before first byte of first segment  
    Prepend,
    /// Overwrite last N bytes of last segment
    OverwriteEnd,
}

impl HexFile {
    /// Calculate checksum and optionally insert into data
    pub fn checksum(&mut self, options: ChecksumOptions, target: ChecksumTarget) -> Vec<u8>;
}
```

**CLI:**
```bash
--crc32 @0xFFFC              # CRC-32, insert at address
--crc32 @append              # CRC-32, append after data
--crc32 csum.txt             # CRC-32, write to file
--crc16 @0xFFFE --range 0x1000-0x7FFF    # CRC-16 over range, insert at address
--crc32 @0xFFFC --exclude 0x2000-0x20FF  # Exclude range from calculation
```

### Checksum Algorithm Details

**CRC-32 (IEEE 802.3):**
```
Polynomial: 0x04C11DB7 (or reflected: 0xEDB88320)
Init:       0xFFFFFFFF
Final XOR:  0xFFFFFFFF (invert result)
```

**CRC-16 CCITT:**
```
Polynomial: 0x1021
Init:       0xFFFF  
Final XOR:  0xFFFF (invert result)
```

**Byte Sum:**
- Sum all bytes, result modulo 0x10000
- Some variants use 2's complement of sum

**Word Sum:**
- Combine byte pairs into 16-bit words (respecting endianness)
- Sum all words, result modulo 0x10000
- Some variants use 2's complement

---

## Signing Operations

### HMAC

```rust
pub struct HmacOptions {
    pub algorithm: HmacAlgorithm,
    pub key: Vec<u8>,
    pub include_metadata: bool,  // Include address+length in hash
}

pub enum HmacAlgorithm {
    Sha256,
    Sha1,    // Legacy, not recommended
}

impl HexFile {
    /// Calculate HMAC over all segment data
    /// If include_metadata: for each segment, hash [addr(4 bytes) | len(4 bytes) | data]
    pub fn hmac(&self, options: HmacOptions) -> Vec<u8>;
}
```

**CLI:**
```bash
--hmac-sha256 keyfile.bin            # HMAC-SHA256, key from file
--hmac-sha256 keyfile.bin --with-metadata  # Include addr+len
```

### RSA Signatures

```rust
pub struct RsaSignOptions {
    pub hash_algorithm: HashAlgorithm,
    pub private_key_path: PathBuf,      // PEM or DER format
    pub include_metadata: bool,          // Include address+length in hash
    pub output_path: Option<PathBuf>,    // Where to write signature
}

pub enum HashAlgorithm {
    Sha256,
    Sha1,      // Legacy
}

impl HexFile {
    /// Create RSA signature (PKCS#1 v1.5 padding)
    /// Returns signature bytes
    pub fn sign_rsa(&self, options: RsaSignOptions) -> Result<Vec<u8>>;
}
```

**CLI:**
```bash
--sign-rsa private.pem --hash sha256 -o signature.bin
--sign-rsa private.pem --hash sha256 --with-metadata
```

**Signature format:** EMSA-PKCS1-v1_5 (standard RSA signature padding)

---

## CLI Interface Design

### Command Structure

```bash
hexkit <INPUT> [OPTIONS] -o <OUTPUT>
```

### Global Options

```
-o, --output <FILE>         Output file path
-f, --format <FORMAT>       Output format: intel-hex, srec, binary [default: auto]
    --bytes-per-line <N>    Bytes per line in hex output [default: 16]
-q, --quiet                 Suppress non-error output
-v, --verbose               Verbose output
```

### Input Options

```
<INPUT>                     Input file (format auto-detected)
    --input-format <FMT>    Force input format: intel-hex, srec, binary
    --base-address <ADDR>   Base address for binary input [default: 0]
```

### Range Operations

```
    --range <RANGE>         Only load data in range (can repeat)
    --cut <RANGE>           Remove data in range (can repeat)
```

### Fill Operations

```
    --fill <RANGE>          Fill range with pattern (can repeat)
    --fill-pattern <HEX>    Pattern for fill [default: FF]
    --fill-gaps             Fill all gaps between segments
    --fill-byte <HEX>       Byte for gap filling [default: FF]
```

### Alignment

```
    --align <N>             Align addresses to multiple of N
    --align-length          Also align segment lengths
```

### Merge

```
    --merge <FILE>          Merge file (overwrite mode)
    --merge-preserve <FILE> Merge file (preserve existing)
    --merge-offset <OFF>    Offset for merged file
    --merge-range <RANGE>   Range filter for merged file
```

### Transform

```
    --swap <MODE>           Swap bytes: word, dword
    --address-scale <N>     Multiply all addresses by N
    --address-unscale <N>   Divide all addresses by N
    --split <SIZE>          Split segments larger than SIZE
```

### Checksum

```
    --crc32 <TARGET>        Calculate CRC-32, target: @ADDR, @append, @prepend, or filename
    --crc16 <TARGET>        Calculate CRC-16-CCITT
    --checksum-range <RANGE>    Range for checksum calculation
    --checksum-exclude <RANGE>  Exclude range from checksum (can repeat)
    --checksum-little-endian    Output checksum in little-endian
```

### Signing

```
    --hmac-sha256 <KEYFILE>     Calculate HMAC-SHA256
    --sign-rsa <KEYFILE>        RSA signature with SHA-256
    --with-metadata             Include address+length in signature
    --signature-output <FILE>   Where to write signature
```

---

## Library API Summary

```rust
// Core types
pub struct HexFile { ... }
pub struct Segment { ... }
pub enum Range { ... }

// Parsing
pub fn parse(input: &[u8], format: InputFormat, base: Option<u32>) -> Result<HexFile>;
pub fn parse_file(path: &Path, format: InputFormat, base: Option<u32>) -> Result<HexFile>;

// Output  
impl HexFile {
    pub fn to_bytes(&self, format: OutputFormat) -> Result<Vec<u8>>;
    pub fn write_file(&self, path: &Path, format: OutputFormat) -> Result<()>;
}

// Inspection
impl HexFile {
    pub fn segments(&self) -> &[Segment];
    pub fn min_address(&self) -> Option<u32>;
    pub fn max_address(&self) -> Option<u32>;
    pub fn total_bytes(&self) -> usize;
    pub fn gap_count(&self) -> usize;
}

// Mutation - Range operations
impl HexFile {
    pub fn filter_range(&mut self, range: Range);
    pub fn filter_ranges(&mut self, ranges: &[Range]);
    pub fn cut(&mut self, range: Range);
    pub fn cut_ranges(&mut self, ranges: &[Range]);
}

// Mutation - Fill operations
impl HexFile {
    pub fn fill(&mut self, range: Range, options: FillOptions);
    pub fn fill_gaps(&mut self, fill_byte: u8);
}

// Mutation - Alignment
impl HexFile {
    pub fn align(&mut self, options: AlignOptions);
}

// Mutation - Merge
impl HexFile {
    pub fn merge(&mut self, other: &HexFile, options: MergeOptions);
}

// Mutation - Transform
impl HexFile {
    pub fn split(&mut self, max_size: u32);
    pub fn swap_bytes(&mut self, mode: SwapMode) -> Result<()>;
    pub fn scale_addresses(&mut self, factor: u32);
    pub fn unscale_addresses(&mut self, divisor: u32) -> Result<()>;
}

// Checksums (may mutate if target is Address/Append/etc)
impl HexFile {
    pub fn checksum(&mut self, options: ChecksumOptions, target: ChecksumTarget) -> Vec<u8>;
}

// Signing (read-only, returns signature)
impl HexFile {
    pub fn hmac(&self, options: HmacOptions) -> Vec<u8>;
    pub fn sign_rsa(&self, options: RsaSignOptions) -> Result<Vec<u8>>;
}
```

---

## Processing Order

When multiple operations specified, execute in this order:

1. **Parse** input file(s)
2. **Filter range** (`--range`)
3. **Merge** files (`--merge`, `--merge-preserve`)
4. **Cut** ranges (`--cut`)
5. **Fill** regions (`--fill`)
6. **Fill gaps** (`--fill-gaps`)
7. **Align** (`--align`)
8. **Scale addresses** (`--address-scale`, `--address-unscale`)
9. **Swap bytes** (`--swap`)
10. **Split blocks** (`--split`)
11. **Calculate checksum** (`--crc32`, `--crc16`) - may insert into data
12. **Calculate signature** (`--hmac-*`, `--sign-*`) - writes to separate file
13. **Write** output file

---

## Crate Structure

```
hexkit/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API exports
│   ├── hexfile.rs          # HexFile, Segment types
│   ├── range.rs            # Range type and parsing
│   ├── parse/
│   │   ├── mod.rs
│   │   ├── intel_hex.rs
│   │   ├── srec.rs
│   │   └── binary.rs
│   ├── output/
│   │   ├── mod.rs
│   │   ├── intel_hex.rs
│   │   ├── srec.rs
│   │   └── binary.rs
│   ├── ops/
│   │   ├── mod.rs
│   │   ├── filter.rs       # filter_range, cut
│   │   ├── fill.rs         # fill, fill_gaps
│   │   ├── align.rs
│   │   ├── merge.rs
│   │   ├── transform.rs    # swap, scale, split
│   │   └── checksum.rs
│   ├── sign/
│   │   ├── mod.rs
│   │   ├── hmac.rs
│   │   └── rsa.rs
│   └── error.rs
└── src/bin/
    └── hexkit.rs           # CLI entry point
```

---

## Recommended Dependencies

```toml
[dependencies]
thiserror = "1"       # Error handling
crc = "3"             # CRC algorithms  
sha2 = "0.10"         # SHA-256
hmac = "0.12"         # HMAC
rsa = "0.9"           # RSA signatures
pkcs8 = "0.10"        # Key parsing
clap = { version = "4", features = ["derive"] }  # CLI

[dev-dependencies]
tempfile = "3"
pretty_assertions = "1"
```

---

## Example Usage

### Library

```rust
use hexkit::{HexFile, Range, AlignOptions, ChecksumOptions, ChecksumTarget, Crc32};

// Parse input
let mut hex = HexFile::parse_file("firmware.hex", InputFormat::Auto, None)?;

// Filter to specific range
hex.filter_range(Range::StartEnd { start: 0x8000, end: 0xFFFF });

// Align to 4 bytes
hex.align(AlignOptions {
    alignment: 4,
    fill_byte: 0xFF,
    align_length: true,
});

// Calculate CRC-32 and append
let crc = hex.checksum(
    ChecksumOptions {
        algorithm: ChecksumAlgorithm::Crc32,
        range: None,
        exclude_ranges: vec![],
        big_endian_output: true,
    },
    ChecksumTarget::Append,
);

// Write output
hex.write_file("output.hex", OutputFormat::IntelHex {
    bytes_per_line: 16,
    address_mode: IntelHexMode::Auto,
})?;
```

### CLI

```bash
# Basic conversion
hexkit firmware.s19 -o firmware.hex

# Filter, align, add CRC
hexkit firmware.hex \
    --range 0x8000-0xFFFF \
    --align 4 --align-length \
    --crc32 @append \
    -o output.hex

# Merge calibration, scale addresses for word-addressed MCU
hexkit app.hex \
    --merge-preserve calibration.hex --merge-offset 0x10000 \
    --address-scale 2 \
    --fill-gaps \
    -o combined.hex

# Sign firmware
hexkit firmware.hex \
    --sign-rsa private.pem --with-metadata \
    --signature-output firmware.sig \
    -o firmware.hex
```
