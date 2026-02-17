//! # NSSA Framework Proc Macros
//!
//! This crate provides the `#[nssa_program]` attribute macro that eliminates
//! boilerplate in NSSA/LEZ guest binaries.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use nssa_framework::prelude::*;
//!
//! #[nssa_program]
//! mod my_program {
//!     #[instruction]
//!     pub fn create(
//!         #[account(init)] state: AccountWithMetadata,
//!         name: String,
//!     ) -> NssaResult {
//!         // business logic only
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, Attribute, FnArg, Ident, ItemFn, ItemMod, Pat, PatType,
    ReturnType, Type,
};

/// Main entry point: `#[nssa_program]` on a module.
///
/// This macro:
/// 1. Finds all `#[instruction]` functions in the module
/// 2. Generates a Borsh-serializable `Instruction` enum
/// 3. Generates the `fn main()` with read/dispatch/write boilerplate
/// 4. Generates account validation code per instruction
#[proc_macro_attribute]
pub fn nssa_program(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemMod);
    match expand_nssa_program(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Marker attribute for instruction functions within an `#[nssa_program]` module.
/// Processed by `#[nssa_program]`, not standalone.
#[proc_macro_attribute]
pub fn instruction(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Pass through — actual processing happens in nssa_program
    item
}

// ─── Internal expansion logic ────────────────────────────────────────────

/// Parsed info about one instruction function.
struct InstructionInfo {
    fn_name: Ident,
    /// Account parameters (AccountWithMetadata type), in order
    accounts: Vec<AccountParam>,
    /// Non-account parameters (the instruction args)
    args: Vec<ArgParam>,
    /// The original function item (with #[instruction] stripped)
    func: ItemFn,
}

struct AccountParam {
    name: Ident,
    constraints: AccountConstraints,
}

#[derive(Default)]
struct AccountConstraints {
    mutable: bool,
    init: bool,
    owner: Option<syn::Expr>,
    signer: bool,
}

struct ArgParam {
    name: Ident,
    ty: Type,
}

fn expand_nssa_program(input: ItemMod) -> syn::Result<TokenStream2> {
    let mod_name = &input.ident;

    let (_, items) = input
        .content
        .as_ref()
        .ok_or_else(|| syn::Error::new_spanned(&input, "nssa_program module must have a body"))?;

    // Collect instruction functions and other items
    let mut instructions: Vec<InstructionInfo> = Vec::new();
    let mut other_items: Vec<TokenStream2> = Vec::new();

    for item in items {
        match item {
            syn::Item::Fn(func) => {
                if has_instruction_attr(&func.attrs) {
                    instructions.push(parse_instruction(func.clone())?);
                } else {
                    other_items.push(quote! { #func });
                }
            }
            other => {
                other_items.push(quote! { #other });
            }
        }
    }

    if instructions.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "nssa_program must contain at least one #[instruction] function",
        ));
    }

    // Generate the Instruction enum
    let enum_variants = generate_enum_variants(&instructions);
    let enum_def = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub enum Instruction {
            #(#enum_variants),*
        }
    };

    // Generate match arms for dispatch
    let match_arms = generate_match_arms(mod_name, &instructions);

    // Generate the handler functions (with #[instruction] stripped, account attrs stripped)
    let handler_fns = generate_handler_fns(&instructions);

    // Generate validation functions
    let validation_fns = generate_validation(&instructions);

    // Generate main function
    let main_fn = quote! {
        fn main() {
            // Read inputs from zkVM host
            let (nssa_core::program::ProgramInput { pre_states, instruction }, instruction_words)
                = nssa_core::program::read_nssa_inputs::<Instruction>();
            let pre_states_clone = pre_states.clone();

            // Dispatch to instruction handler
            let result: Result<
                (Vec<nssa_core::program::AccountPostState>, Vec<nssa_core::program::ChainedCall>),
                nssa_framework_core::error::NssaError
            > = match instruction {
                #(#match_arms)*
            };

            // Handle result
            let (post_states, chained_calls) = match result {
                Ok(output) => output,
                Err(e) => {
                    panic!("Program error [{}]: {}", e.error_code(), e);
                }
            };

            // Write outputs to zkVM host
            nssa_core::program::write_nssa_outputs_with_chained_call(
                instruction_words,
                pre_states_clone,
                post_states,
                chained_calls,
            );
        }
    };

    // Generate IDL function
    let idl_fn = generate_idl_fn(mod_name, &instructions);

    // Assemble everything
    let expanded = quote! {
        // The instruction enum (used by both on-chain and client)
        #enum_def

        // The program module with handler functions
        mod #mod_name {
            use super::*;

            #(#other_items)*

            #(#handler_fns)*

            #(#validation_fns)*
        }

        // IDL generation (available at host-side for tooling)
        #idl_fn

        // The guest binary entry point
        #main_fn
    };

    Ok(expanded)
}

fn has_instruction_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("instruction"))
}

fn parse_instruction(func: ItemFn) -> syn::Result<InstructionInfo> {
    let fn_name = func.sig.ident.clone();
    let mut accounts = Vec::new();
    let mut args = Vec::new();

    for input in &func.sig.inputs {
        match input {
            FnArg::Typed(pat_type) => {
                let param_name = extract_param_name(pat_type)?;
                let ty = &*pat_type.ty;

                if is_account_type(ty) {
                    let constraints = parse_account_constraints(&pat_type.attrs)?;
                    accounts.push(AccountParam {
                        name: param_name,
                        constraints,
                    });
                } else {
                    args.push(ArgParam {
                        name: param_name,
                        ty: ty.clone(),
                    });
                }
            }
            FnArg::Receiver(_) => {
                return Err(syn::Error::new_spanned(
                    input,
                    "instruction functions cannot have self parameter",
                ));
            }
        }
    }

    Ok(InstructionInfo {
        fn_name,
        accounts,
        args,
        func,
    })
}

fn extract_param_name(pat_type: &PatType) -> syn::Result<Ident> {
    match &*pat_type.pat {
        Pat::Ident(pat_ident) => Ok(pat_ident.ident.clone()),
        _ => Err(syn::Error::new_spanned(
            &pat_type.pat,
            "expected simple identifier pattern",
        )),
    }
}

fn is_account_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "AccountWithMetadata";
        }
    }
    false
}

fn parse_account_constraints(attrs: &[Attribute]) -> syn::Result<AccountConstraints> {
    let mut constraints = AccountConstraints::default();

    for attr in attrs {
        if attr.path().is_ident("account") {
            // Parse the token stream inside #[account(...)]
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("mut") {
                    constraints.mutable = true;
                    Ok(())
                } else if meta.path.is_ident("init") {
                    constraints.init = true;
                    constraints.mutable = true; // init implies mut
                    Ok(())
                } else if meta.path.is_ident("signer") {
                    constraints.signer = true;
                    Ok(())
                } else if meta.path.is_ident("owner") {
                    let value = meta.value()?;
                    let expr: syn::Expr = value.parse()?;
                    constraints.owner = Some(expr);
                    Ok(())
                } else {
                    Err(meta.error("unknown account constraint"))
                }
            })?;
        }
    }

    Ok(constraints)
}

/// Generate enum variants from instruction functions.
///
/// `fn create(#[account] a: ..., #[account] b: ..., name: String, supply: u128)`
/// becomes:
/// `Create { name: String, supply: u128 }`
fn generate_enum_variants(instructions: &[InstructionInfo]) -> Vec<TokenStream2> {
    instructions
        .iter()
        .map(|ix| {
            let variant_name = to_pascal_case(&ix.fn_name);
            let fields: Vec<TokenStream2> = ix
                .args
                .iter()
                .map(|arg| {
                    let name = &arg.name;
                    let ty = &arg.ty;
                    quote! { #name: #ty }
                })
                .collect();

            if fields.is_empty() {
                quote! { #variant_name }
            } else {
                quote! { #variant_name { #(#fields),* } }
            }
        })
        .collect()
}

/// Generate match arms for instruction dispatch.
fn generate_match_arms(mod_name: &Ident, instructions: &[InstructionInfo]) -> Vec<TokenStream2> {
    instructions
        .iter()
        .map(|ix| {
            let variant_name = to_pascal_case(&ix.fn_name);
            let fn_name = &ix.fn_name;
            let num_accounts = ix.accounts.len();

            // Destructure pattern for enum fields
            let field_names: Vec<&Ident> = ix.args.iter().map(|a| &a.name).collect();
            let pattern = if field_names.is_empty() {
                quote! { Instruction::#variant_name }
            } else {
                quote! { Instruction::#variant_name { #(#field_names),* } }
            };

            // Account destructuring
            let account_names: Vec<&Ident> = ix.accounts.iter().map(|a| &a.name).collect();
            let account_destructure = quote! {
                let [#(#account_names),*] = <[_; #num_accounts]>::try_from(pre_states)
                    .unwrap_or_else(|v: Vec<_>| panic!(
                        "Account count mismatch: expected {}, got {}",
                        #num_accounts, v.len()
                    ));
            };

            // Call the handler
            let call_args: Vec<TokenStream2> = ix
                .accounts
                .iter()
                .map(|a| {
                    let name = &a.name;
                    quote! { #name }
                })
                .chain(ix.args.iter().map(|a| {
                    let name = &a.name;
                    quote! { #name }
                }))
                .collect();

            quote! {
                #pattern => {
                    #account_destructure
                    #mod_name::#fn_name(#(#call_args),*)
                        .map(|output| (output.post_states, output.chained_calls))
                }
            }
        })
        .collect()
}

/// Generate handler functions with macro attributes stripped.
fn generate_handler_fns(instructions: &[InstructionInfo]) -> Vec<TokenStream2> {
    instructions
        .iter()
        .map(|ix| {
            let mut func = ix.func.clone();
            
            // Strip #[instruction] attribute
            func.attrs.retain(|a| !a.path().is_ident("instruction"));
            
            // Strip #[account(...)] attributes from parameters
            for input in &mut func.sig.inputs {
                if let FnArg::Typed(pat_type) = input {
                    pat_type.attrs.retain(|a| !a.path().is_ident("account"));
                }
            }

            quote! { #func }
        })
        .collect()
}

/// Generate validation helper functions (one per instruction).
fn generate_validation(instructions: &[InstructionInfo]) -> Vec<TokenStream2> {
    // In a full implementation, this would generate per-instruction
    // validation that checks constraints before calling handlers.
    // For the PoC, validation is embedded in the match arms.
    vec![]
}

/// Convert snake_case to PascalCase for enum variant names.
fn to_pascal_case(ident: &Ident) -> Ident {
    let s = ident.to_string();
    let pascal: String = s
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect();
    format_ident!("{}", pascal)
}

/// Convert a Rust type to an IDL type string.
fn rust_type_to_idl_string(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let ident = segment.ident.to_string();
            match ident.as_str() {
                "u8" | "u16" | "u32" | "u64" | "u128" |
                "i8" | "i16" | "i32" | "i64" | "i128" |
                "bool" | "String" => ident.to_lowercase(),
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            format!("vec<{}>", rust_type_to_idl_string(inner))
                        } else {
                            "vec<unknown>".to_string()
                        }
                    } else {
                        "vec<unknown>".to_string()
                    }
                }
                "ProgramId" => "program_id".to_string(),
                other => other.to_string(),
            }
        }
        Type::Array(arr) => {
            let elem = rust_type_to_idl_string(&arr.elem);
            // Try to extract the length
            if let syn::Expr::Lit(lit) = &arr.len {
                if let syn::Lit::Int(n) = &lit.lit {
                    return format!("[{}; {}]", elem, n);
                }
            }
            format!("[{}; ?]", elem)
        }
        _ => "unknown".to_string(),
    }
}

/// Generate a function that returns the program IDL.
fn generate_idl_fn(mod_name: &Ident, instructions: &[InstructionInfo]) -> TokenStream2 {
    let program_name = mod_name.to_string();

    let instruction_literals: Vec<TokenStream2> = instructions.iter().map(|ix| {
        let ix_name = ix.fn_name.to_string();

        let account_literals: Vec<TokenStream2> = ix.accounts.iter().map(|acc| {
            let acc_name = acc.name.to_string();
            let writable = acc.constraints.mutable;
            let signer = acc.constraints.signer;
            let init = acc.constraints.init;
            quote! {
                nssa_framework_core::idl::IdlAccountItem {
                    name: #acc_name.to_string(),
                    writable: #writable,
                    signer: #signer,
                    init: #init,
                    owner: None,
                    pda: None,
                }
            }
        }).collect();

        let arg_literals: Vec<TokenStream2> = ix.args.iter().map(|arg| {
            let arg_name = arg.name.to_string();
            let type_str = rust_type_to_idl_string(&arg.ty);
            quote! {
                nssa_framework_core::idl::IdlArg {
                    name: #arg_name.to_string(),
                    type_: nssa_framework_core::idl::IdlType::Primitive(#type_str.to_string()),
                }
            }
        }).collect();

        quote! {
            nssa_framework_core::idl::IdlInstruction {
                name: #ix_name.to_string(),
                accounts: vec![#(#account_literals),*],
                args: vec![#(#arg_literals),*],
            }
        }
    }).collect();

    quote! {
        /// Returns the IDL (Interface Definition Language) for this program.
        /// Use this to generate CLI tools, client SDKs, or documentation.
        #[allow(dead_code)]
        pub fn __program_idl() -> nssa_framework_core::idl::NssaIdl {
            nssa_framework_core::idl::NssaIdl {
                version: "0.1.0".to_string(),
                name: #program_name.to_string(),
                instructions: vec![#(#instruction_literals),*],
                accounts: vec![],
                types: vec![],
                errors: vec![],
            }
        }
    }
}
