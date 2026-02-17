//! # NSSA Framework
//!
//! Developer framework for building programs on NSSA/LEZ,
//! similar to Anchor for Solana.

// Re-export the proc macros
pub use nssa_framework_macros::{nssa_program, instruction, generate_idl};

// Re-export core types
pub use nssa_framework_core::*;

pub mod prelude {
    pub use crate::nssa_program;
    pub use crate::instruction;
    pub use nssa_framework_core::prelude::*;
    pub use nssa_framework_core::types::NssaOutput;
    pub use nssa_framework_core::error::{NssaError, NssaResult};
    pub use borsh::{BorshSerialize, BorshDeserialize};
}
