# nssa-framework

Developer experience framework for building NSSA/LEZ programs — inspired by [Anchor](https://www.anchor-lang.com/) for Solana.

## Crates

| Crate | Description |
|-------|-------------|
| `nssa-framework-core` | IDL types, error types, `NssaOutput`, validation |
| `nssa-framework-macros` | Proc macros: `#[nssa_program]`, `#[instruction]` |
| `nssa-framework` | Umbrella crate with prelude re-exports |
| `nssa-framework-cli` | Generic IDL-driven CLI with TX submission |

## Quick Start

### Define a program

```rust
use nssa_framework::prelude::*;

#[nssa_program]
pub mod my_program {
    use super::*;

    #[instruction]
    pub fn initialize(ctx: Context, name: String, amount: u64) -> NssaResult {
        // Your program logic here
        Ok(NssaOutput::default())
    }
}
```

### Generate IDL

```bash
cargo run --bin generate_idl > my-program-idl.json
```

### Use the CLI

```bash
# Inspect a compiled program binary
nssa-cli --idl my-program-idl.json inspect artifacts/my-program.bin

# Dry run an instruction
nssa-cli --idl my-program-idl.json --program artifacts/my-program.bin --dry-run \
  initialize --name "hello" --amount 1000

# Submit a transaction
nssa-cli --idl my-program-idl.json --program artifacts/my-program.bin \
  initialize --name "hello" --amount 1000

# Get help for a specific instruction
nssa-cli --idl my-program-idl.json initialize --help
```

### Type formats

| IDL Type | CLI Format |
|----------|------------|
| `u8`, `u32`, `u64`, `u128` | Decimal number |
| `[u8; N]` | Hex string (2×N chars) or UTF-8 string (≤N chars, right-padded) |
| `[u32; 8]` / `program_id` | Comma-separated u32s: `"0,0,0,0,0,0,0,0"` |
| `Vec<[u8; 32]>` | Comma-separated hex or base58: `"aabb...00,ccdd...00"` |
| `Option<T>` | Value or `"none"` |

## License

MIT
