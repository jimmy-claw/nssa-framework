//! PDA (Program Derived Address) computation from IDL seed definitions.

use std::collections::HashMap;
use nssa::AccountId;
use nssa_core::program::{PdaSeed, ProgramId};
use nssa_framework_core::idl::IdlSeed;
use crate::parse::ParsedValue;

/// Compute PDA AccountId from IDL seed definitions.
pub fn compute_pda_from_seeds(
    seeds: &[IdlSeed],
    program_id: &ProgramId,
    account_map: &HashMap<String, AccountId>,
    _parsed_args: &HashMap<String, ParsedValue>,
) -> Result<AccountId, String> {
    if seeds.len() != 1 {
        return Err(format!(
            "Multi-seed PDAs not yet supported (got {} seeds)",
            seeds.len()
        ));
    }

    let seed_bytes: [u8; 32] = match &seeds[0] {
        IdlSeed::Const { value } => {
            let mut bytes = [0u8; 32];
            let src = value.as_bytes();
            if src.len() > 32 {
                return Err(format!("Const seed '{}' exceeds 32 bytes", value));
            }
            bytes[..src.len()].copy_from_slice(src);
            bytes
        }
        IdlSeed::Account { path } => {
            let account_id = account_map
                .get(path)
                .ok_or_else(|| {
                    format!(
                        "PDA seed references account '{}' which hasn't been resolved yet",
                        path
                    )
                })?;
            *account_id.value()
        }
        IdlSeed::Arg { path } => {
            return Err(format!("Arg-based PDA seeds not yet supported (arg: {})", path));
        }
    };

    let pda_seed = PdaSeed::new(seed_bytes);
    Ok(AccountId::from((program_id, &pda_seed)))
}
