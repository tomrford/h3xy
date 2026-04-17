# h3xy

`h3xy` is deprecated and superseded by [`hexy`](https://github.com/tomrford/hexy).

This `1.1.3` release is a final pointer release so existing users on the old crate/repo get a clear migration path. New work continues in the new repo and packages:

- Repo: [`tomrford/hexy`](https://github.com/tomrford/hexy)
- Library crate: `hexy-core`
- HexView-compatible CLI package: `hexy-compat`
- Installed compat binary: `hexy`

## Install the replacement

```bash
cargo install hexy-compat
```

## Library migration

```rust
use hexy_core::{AddressRange, HexFile};

let mut hf = HexFile::from_ihex(data)?;
hf.cut(&[AddressRange::new(0x800, 0x8FF)]);
let out = hf.to_ihex(None, None);
```

## CLI migration

The slash-style HexView-compatible workflow continues in `hexy-compat`, but the installed binary is now `hexy`:

```bash
hexy input.hex /AR:'0x1000-0x1FFF' /XI -o output.hex
```

## Status

This repository is no longer the active development location and is intended to be archived after this final release.

## License

MIT
