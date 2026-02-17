//! Project scaffolding: `nssa-cli init <name>`

use std::fs;
use std::path::Path;

pub fn init_project(name: &str) {
    let root = Path::new(name);
    if root.exists() {
        eprintln!("❌ Directory '{}' already exists", name);
        std::process::exit(1);
    }

    println!("🚀 Creating NSSA project '{}'...", name);

    let snake_name = name.replace('-', "_");

    // Create directories
    let dirs = [
        "",
        &format!("{}_core/src", snake_name),
        "methods/src",
        &format!("methods/guest/src/bin"),
        "examples/src/bin",
    ];
    for dir in &dirs {
        let p = root.join(dir);
        fs::create_dir_all(&p).unwrap_or_else(|e| {
            eprintln!("❌ Failed to create {}: {}", p.display(), e);
            std::process::exit(1);
        });
    }

    // Root Cargo.toml (workspace)
    write_file(root, "Cargo.toml", &format!(r#"[workspace]
members = [
    "{snake_name}_core",
    "methods",
    "methods/guest",
    "examples",
]
resolver = "2"
"#));

    // .gitignore
    write_file(root, ".gitignore", r#"target/
methods/guest/target/
*.bin
.{name}-state
.{name}-state.tmp
"#);

    // Makefile
    write_file(root, "Makefile", &format!(r#".PHONY: build idl cli

build:
	cd methods && cargo build

idl:
	cargo run --bin generate_idl > {name}-idl.json

cli:
	cargo run --bin {snake_name}_cli -- -i {name}-idl.json $(ARGS)
"#));

    // README
    write_file(root, "README.md", &format!(r#"# {name}

An NSSA/LEZ program built with [nssa-framework](https://github.com/jimmy-claw/nssa-framework).

## Quick Start

### 1. Build the guest program

```bash
cd methods && cargo build
```

### 2. Generate the IDL

```bash
make idl
# or: cargo run --bin generate_idl > {name}-idl.json
```

### 3. Use the CLI

```bash
# Show available commands (auto-generated from IDL):
make cli ARGS="--help"

# Dry run an instruction:
make cli ARGS="--dry-run -p path/to/{name}.bin <command> --arg1 value1"

# Submit a transaction:
make cli ARGS="-p path/to/{name}.bin <command> --arg1 value1"
```

## Project Structure

- **`{snake_name}_core/`** — Shared types and structs (used by guest + host)
- **`methods/guest/`** — The RISC Zero guest program (runs on-chain)
- **`examples/`** — CLI tools (IDL generator + generic CLI wrapper)
"#));

    // program_core
    write_file(root, &format!("{}_core/Cargo.toml", snake_name), &format!(r#"[package]
name = "{snake_name}_core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
borsh = "1.5"
"#));

    write_file(root, &format!("{}_core/src/lib.rs", snake_name), r#"use serde::{Deserialize, Serialize};

/// Example state struct — customize for your program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramState {
    pub initialized: bool,
    pub owner: [u8; 32],
}
"#);

    // methods/Cargo.toml
    write_file(root, "methods/Cargo.toml", &format!(r#"[package]
name = "{snake_name}-methods"
version = "0.1.0"
edition = "2021"

[build-dependencies]
risc0-build = "3.0"

[dependencies]
risc0-zkvm = {{ version = "3.0.3", features = ["std"] }}
{snake_name}_core = {{ path = "../{snake_name}_core" }}
"#));

    // methods/build.rs
    write_file(root, "methods/build.rs", r#"fn main() {
    risc0_build::embed_methods();
}
"#);

    // methods/src/lib.rs
    write_file(root, "methods/src/lib.rs", r#"include!(concat!(env!("OUT_DIR"), "/methods.rs"));
"#);

    // methods/guest/Cargo.toml
    write_file(root, "methods/guest/Cargo.toml", &format!(r#"[package]
name = "{snake_name}-guest"
version = "0.1.0"
edition = "2021"

[workspace]

[[bin]]
name = "{snake_name}"
path = "src/bin/{snake_name}.rs"

[dependencies]
nssa-framework = {{ git = "https://github.com/jimmy-claw/nssa-framework.git" }}
nssa_core = {{ git = "https://github.com/logos-blockchain/lssa.git", branch = "schouhy/full-bedrock-integration" }}
risc0-zkvm = {{ version = "3.0.3", default-features = false, features = ["guest"] }}
{snake_name}_core = {{ path = "../../{snake_name}_core" }}
serde = {{ version = "1.0", features = ["derive"] }}
borsh = "1.5"
"#));

    // Guest program skeleton
    write_file(root, &format!("methods/guest/src/bin/{}.rs", snake_name), &format!(r#"#![no_main]

use nssa_core::account::{{Account, AccountId, AccountWithMetadata}};
use nssa_core::program::{{AccountPostState, ProgramId}};
use nssa_framework::prelude::*;
use {snake_name}_core::ProgramState;

risc0_zkvm::guest::entry!(main);

#[nssa_program]
mod {snake_name} {{
    #[allow(unused_imports)]
    use super::*;

    /// Initialize the program state.
    #[instruction]
    pub fn initialize(
        #[account(init, pda = literal("state"))]
        state: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
    ) {{
        // TODO: implement initialization logic
    }}

    /// Example instruction — replace with your own.
    #[instruction]
    pub fn do_something(
        #[account(mut, pda = literal("state"))]
        state: AccountWithMetadata,
        #[account(signer)]
        owner: AccountWithMetadata,
        amount: u64,
    ) {{
        // TODO: implement your logic
    }}
}}
"#));

    // examples/Cargo.toml
    write_file(root, "examples/Cargo.toml", &format!(r#"[package]
name = "{snake_name}-examples"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "generate_idl"
path = "src/bin/generate_idl.rs"

[[bin]]
name = "{snake_name}_cli"
path = "src/bin/{snake_name}_cli.rs"

[dependencies]
nssa-framework = {{ git = "https://github.com/jimmy-claw/nssa-framework.git" }}
nssa-framework-core = {{ git = "https://github.com/jimmy-claw/nssa-framework.git" }}
nssa-framework-cli = {{ git = "https://github.com/jimmy-claw/nssa-framework.git" }}
{snake_name}_core = {{ path = "../{snake_name}_core" }}
serde_json = "1.0"
tokio = {{ version = "1.28.2", features = ["net", "rt-multi-thread", "sync", "macros"] }}
"#));

    // generate_idl.rs
    write_file(root, "examples/src/bin/generate_idl.rs", &format!(r#"/// Generate IDL JSON for the {name} program.
///
/// Usage:
///   cargo run --bin generate_idl > {name}-idl.json

nssa_framework::generate_idl!("../../methods/guest/src/bin/{snake_name}.rs");
"#));

    // CLI wrapper
    write_file(root, &format!("examples/src/bin/{}_cli.rs", snake_name), r#"#[tokio::main]
async fn main() {
    nssa_framework_cli::run().await;
}
"#);

    println!();
    println!("✅ Project '{}' created!", name);
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  # Edit methods/guest/src/bin/{}.rs with your program logic", snake_name);
    println!("  # Edit {}_core/src/lib.rs with your types", snake_name);
    println!("  make idl        # Generate the IDL");
    println!("  make cli ARGS=\"--help\"  # See available commands");
}

fn write_file(root: &Path, rel_path: &str, content: &str) {
    let path = root.join(rel_path);
    fs::write(&path, content).unwrap_or_else(|e| {
        eprintln!("❌ Failed to write {}: {}", path.display(), e);
        std::process::exit(1);
    });
}
