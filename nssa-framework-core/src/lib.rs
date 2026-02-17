//! # NSSA Framework Core
//!
//! Core types and traits for the NSSA program framework.

pub mod error;
pub mod types;
pub mod idl;
pub mod validation;

pub mod prelude {
    pub use crate::error::{NssaError, NssaResult};
    pub use crate::types::{NssaOutput, AccountConstraint};
    pub use nssa_core::account::{Account, AccountWithMetadata};
    pub use nssa_core::program::{AccountPostState, ChainedCall, PdaSeed, ProgramId};
}
