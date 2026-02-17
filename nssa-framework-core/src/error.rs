//! Structured error types for NSSA programs.
//!
//! Replaces the current pattern of `panic!` and `.expect()` with
//! proper Result-based error handling.

use borsh::{BorshDeserialize, BorshSerialize};
use thiserror::Error;

/// Result type alias for NSSA program operations.
/// All instruction handlers should return this type.
pub type NssaResult = Result<NssaOutput, NssaError>;

/// Re-export for convenience in result type
pub use crate::types::NssaOutput;

/// Structured error type for NSSA programs.
///
/// Programs can use the built-in variants for common errors,
/// or use `Custom` for program-specific error codes.
///
/// # Example
/// ```rust
/// use nssa_framework_core::error::NssaError;
///
/// fn check_balance(balance: u128, amount: u128) -> Result<(), NssaError> {
///     if balance < amount {
///         return Err(NssaError::InsufficientBalance {
///             available: balance,
///             requested: amount,
///         });
///     }
///     Ok(())
/// }
/// ```
#[derive(Error, Debug, BorshSerialize, BorshDeserialize)]
pub enum NssaError {
    /// Wrong number of accounts provided for this instruction
    #[error("Expected {expected} accounts, got {actual}")]
    AccountCountMismatch {
        expected: usize,
        actual: usize,
    },

    /// Account is not owned by the expected program
    #[error("Account {account_index} has wrong owner: expected {expected_owner}")]
    InvalidAccountOwner {
        account_index: usize,
        expected_owner: String,
    },

    /// Account should be uninitialized but contains data
    #[error("Account {account_index} is already initialized")]
    AccountAlreadyInitialized {
        account_index: usize,
    },

    /// Account should be initialized but is empty/default
    #[error("Account {account_index} is not initialized")]
    AccountNotInitialized {
        account_index: usize,
    },

    /// Insufficient balance for transfer or burn
    #[error("Insufficient balance: have {available}, need {requested}")]
    InsufficientBalance {
        available: u128,
        requested: u128,
    },

    /// Failed to deserialize account data
    #[error("Failed to deserialize account data at index {account_index}: {message}")]
    DeserializationError {
        account_index: usize,
        message: String,
    },

    /// Failed to serialize account data  
    #[error("Failed to serialize data: {message}")]
    SerializationError {
        message: String,
    },

    /// Arithmetic overflow
    #[error("Arithmetic overflow: {operation}")]
    Overflow {
        operation: String,
    },

    /// Authorization failure
    #[error("Unauthorized: {message}")]
    Unauthorized {
        message: String,
    },

    /// PDA derivation mismatch
    #[error("PDA mismatch for account {account_index}")]
    PdaMismatch {
        account_index: usize,
    },

    /// Custom program-specific error with code and message
    #[error("Program error {code}: {message}")]
    Custom {
        code: u32,
        message: String,
    },
}

impl NssaError {
    /// Create a custom error with a code and message.
    pub fn custom(code: u32, message: impl Into<String>) -> Self {
        NssaError::Custom {
            code,
            message: message.into(),
        }
    }

    /// Get a numeric error code for client-side handling.
    pub fn error_code(&self) -> u32 {
        match self {
            NssaError::AccountCountMismatch { .. } => 1000,
            NssaError::InvalidAccountOwner { .. } => 1001,
            NssaError::AccountAlreadyInitialized { .. } => 1002,
            NssaError::AccountNotInitialized { .. } => 1003,
            NssaError::InsufficientBalance { .. } => 1004,
            NssaError::DeserializationError { .. } => 1005,
            NssaError::SerializationError { .. } => 1006,
            NssaError::Overflow { .. } => 1007,
            NssaError::Unauthorized { .. } => 1008,
            NssaError::PdaMismatch { .. } => 1009,
            NssaError::Custom { code, .. } => 6000 + code,
        }
    }
}
