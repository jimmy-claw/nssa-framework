//! Generic IDL-driven CLI library for NSSA/LEZ programs.
//!
//! Provides:
//! - IDL parsing and type-aware argument handling
//! - risc0-compatible serialization
//! - Transaction building and submission
//! - PDA computation from IDL seeds
//! - Binary inspection (ProgramId extraction)
//!
//! Use this as a library to build program-specific CLIs, or use the
//! `nssa-cli` binary for a fully generic IDL-driven experience.

pub mod hex;
pub mod parse;
pub mod serialize;
pub mod pda;
pub mod tx;
pub mod inspect;
pub mod cli;
