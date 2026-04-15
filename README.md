# h3xy

Drop-in HexView replacement CLI and Rust library for non-proprietary firmware hex-file workflows.

Supports Intel HEX, Motorola S-Record, HEX ASCII, and raw binary — with the same slash-style flag syntax HexView users already know.

## Install

```bash
cargo install h3xy
```

## Quick examples

```bash
# Filter a range and export Intel HEX
h3xy input.hex /AR:'0x1000-0x1FFF' /XI -o output.hex

# Fill, cut, checksum, then export S-Record
h3xy app.hex /FR:'0x0-0xFFF' /FP:FF /CR:'0x800-0x8FF' /CS0 /XS -o app.s19

# Merge a calibration overlay and export binary
h3xy base.hex /MO:cal.hex /XN -o combined.bin

# Export one binary per segment
h3xy multi.hex /XSB -o segments.bin
```

## Operation order

Flags execute in a fixed pipeline order — not the order they appear on the command line:

1. Input (positional file, `/IN`, `/IA`, `/II2`)
2. Address mapping (`/S08MAP`, `/S12MAP`, `/S12XMAP`, `/REMAP`)
3. dsPIC transforms (`/CDSPX`, `/CDSPS`, `/CDSPG`)
4. Fill (`/FR` + `/FP`)
5. Cut (`/CR`)
6. Merge (`/MT` or `/MO`)
7. Address range filter (`/AR`)
8. Collapse (`/FA`), align (`/AD`, `/AL`, `/AF`), split (`/SB`), swap (`/SWAPWORD`, `/SWAPLONG`)
9. Checksum (`/CS`, `/CSR`, `/CSM`, `/CSMR`)
10. Signing (`/DP`) and verification (`/SV`)
11. Export (`/XI`, `/XS`, `/XN`, `/XSB`, `/XA`, `/XC`, `/XF`, `/XP`) via `-o`

## Library

The core library exposes `HexFile`, `Segment`, and `AddressRange` with typed per-operation methods, so you can compose the same transformations programmatically:

```rust
use h3xy::{HexFile, AddressRange};

let mut hf = HexFile::from_ihex(data)?;
hf.cut(&[AddressRange::new(0x800, 0x8FF)]);
let out = hf.to_ihex(None, None);
```

## Out of scope

Proprietary HexView features — `/PB`, `/expdat`, OEM containers, GM/VBF/FIAT exports, and DLL-backed signing — are currently excluded.

## License

MIT

