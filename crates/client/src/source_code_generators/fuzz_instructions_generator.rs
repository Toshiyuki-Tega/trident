use anchor_lang_idl_spec::{Idl, IdlInstructionAccountItem, IdlType};
use convert_case::{Case, Casing};
use quote::{format_ident, quote, ToTokens};
use std::collections::{HashMap, HashSet};
use syn::{parse_quote, parse_str};

// Main function to generate source code from IDLs
pub fn generate_source_code(idls: &[Idl]) -> String {
    // Collections to store generated items
    let mut all_instructions: Vec<syn::Variant> = Vec::new();
    let mut all_instruction_inputs: Vec<syn::ItemStruct> = Vec::new();
    let mut all_instructions_ixops_impls: Vec<syn::ItemImpl> = Vec::new();
    let mut all_fuzz_accounts: Vec<syn::FnArg> = Vec::new();

    // Mappings for instructions and accounts
    let mut instructions_mappings: HashMap<String, u8> = HashMap::new();
    let mut accounts_mappings: HashMap<String, u8> = HashMap::new();

    // Extract unique instructions and accounts across all IDLs
    get_unique_accounts_n_instructions(idls, &mut instructions_mappings, &mut accounts_mappings);

    // Iterate over each IDL to generate various parts of the code
    for idl in idls {
        all_instructions.extend(get_instruction_variants(idl, &instructions_mappings));
        all_instruction_inputs.extend(get_instruction_inputs(idl, &instructions_mappings));
        all_instructions_ixops_impls.extend(get_instruction_ixops(idl, &instructions_mappings));
        all_fuzz_accounts.extend(get_fuzz_accounts(idl, &accounts_mappings));
    }

    // Define the Rust module with all generated code
    let module_definition = quote! {
        use trident_client::fuzzing::*;

        /// FuzzInstruction contains all available Instructions.
        /// Below, the instruction arguments (accounts and data) are defined.
        #[derive(Arbitrary, DisplayIx, FuzzTestExecutor)]
        pub enum FuzzInstruction {
            #(#all_instructions),*
        }

        #(#all_instruction_inputs)*

        #(#all_instructions_ixops_impls)*

        /// Check supported AccountsStorages at
        /// https://ackee.xyz/trident/docs/latest/features/account-storages/
        #[derive(Default)]
        pub struct FuzzAccounts {
            #(#all_fuzz_accounts),*
        }
    };

    // Convert the module definition to a string and return it
    module_definition.into_token_stream().to_string()
}

// Function to get unique accounts and instructions across all IDLs
fn get_unique_accounts_n_instructions(
    idls: &[Idl],
    instructions_mappings: &mut HashMap<String, u8>,
    accounts_mappings: &mut HashMap<String, u8>,
) {
    for idl in idls {
        let mut seen_accounts: HashSet<String> = HashSet::new();

        for instruction in idl.instructions.iter() {
            let instruction_name = instruction.name.to_case(Case::UpperCamel);
            *instructions_mappings.entry(instruction_name).or_insert(0) += 1;

            for account in instruction.accounts.iter() {
                let account_name = match account {
                    IdlInstructionAccountItem::Composite(_) => {
                        panic!("Composite Accounts are not supported yet!")
                    }
                    IdlInstructionAccountItem::Single(single_account) => {
                        let account_name = single_account.name.clone();
                        account_name.to_case(Case::Snake)
                    }
                };
                // Only add the account if it hasn't been seen in this IDL yet
                if !seen_accounts.contains(&account_name) {
                    *accounts_mappings
                        .entry(account_name.to_string())
                        .or_insert(0) += 1;
                    seen_accounts.insert(account_name);
                }
            }
        }
    }
}

// Generate instruction variants for the enum
fn get_instruction_variants(
    idl: &Idl,
    instruction_mappings: &HashMap<String, u8>,
) -> Vec<syn::Variant> {
    let program_name = idl.metadata.name.to_case(Case::UpperCamel);

    idl.instructions
        .iter()
        .fold(Vec::new(), |mut variants, instruction| {
            let mut instruction_name = instruction.name.to_case(Case::UpperCamel);
            let count = instruction_mappings.get(&instruction_name).unwrap_or(&1);

            // Append the program name if the instruction name is not unique
            if *count > 1 {
                instruction_name.push_str(&program_name);
            }

            let instruction_struct_name: syn::Ident = parse_str(&instruction_name).unwrap();
            let variant: syn::Variant = parse_quote! {
                #instruction_struct_name(#instruction_struct_name)
            };

            variants.push(variant);
            variants
        })
}

// Generate input structures for each instruction
fn get_instruction_inputs(
    idl: &Idl,
    instruction_mappings: &HashMap<String, u8>,
) -> Vec<syn::ItemStruct> {
    let program_name = idl.metadata.name.to_case(Case::UpperCamel);

    idl.instructions
        .iter()
        .fold(Vec::new(), |mut instructions_data, instruction| {
            let mut instruction_name = instruction.name.to_case(Case::UpperCamel);
            let count = instruction_mappings.get(&instruction_name).unwrap_or(&1);

            // Append the program name if the instruction name is not unique
            if *count > 1 {
                instruction_name.push_str(&program_name);
            }

            let instruction_name_ident: syn::Ident = format_ident!("{}", &instruction_name);
            let instruction_data_name: syn::Ident = format_ident!("{}Data", &instruction_name);
            let instruction_accounts_name: syn::Ident =
                format_ident!("{}Accounts", &instruction_name);

            // Generate accounts and parameters
            let accounts = instruction
                .accounts
                .iter()
                .map(|account| match account {
                    IdlInstructionAccountItem::Composite(_composite) => {
                        panic!("Composite Accounts are not supported yet!")
                    }
                    IdlInstructionAccountItem::Single(single) => {
                        let name = format_ident!("{}", single.name);
                        let account: syn::FnArg = parse_quote!(#name: AccountId);
                        account
                    }
                })
                .collect::<Vec<_>>();

            let parameters = instruction
                .args
                .iter()
                .map(|arg| {
                    let arg_name = format_ident!("{}", arg.name);
                    let (arg_type, _is_custom) = idl_type_to_syn_type(&arg.ty, 0);
                    let parameter: syn::FnArg = parse_quote!(#arg_name: #arg_type);
                    parameter
                })
                .collect::<Vec<_>>();

            // Define the input structures
            let instructions_inputs: syn::ItemStruct = parse_quote! {
                #[derive(Arbitrary, Debug)]
                pub struct #instruction_name_ident {
                     pub accounts: #instruction_accounts_name,
                     pub data: #instruction_data_name
                }
            };

            let instructions_input_accounts: syn::ItemStruct = parse_quote! {
                #[derive(Arbitrary, Debug)]
                pub struct #instruction_accounts_name {
                     #(pub #accounts),*
                }
            };

            let instructions_input_data: syn::ItemStruct = parse_quote! {
                /// Custom data types must derive `Debug` and `Arbitrary`.
                /// To do this, redefine the type in the fuzz test and implement the `From` trait
                /// to convert it into the type defined in the program.
                /// For more details, see: https://ackee.xyz/trident/docs/latest/features/fuzz-instructions/#custom-data-types
                #[derive(Arbitrary, Debug)]
                pub struct #instruction_data_name {
                     #(pub #parameters),*
                }
            };

            instructions_data.push(instructions_inputs);
            instructions_data.push(instructions_input_accounts);
            instructions_data.push(instructions_input_data);
            instructions_data
        })
}

// Generate implementation of IxOps trait for each instruction
fn get_instruction_ixops(
    idl: &Idl,
    instruction_mappings: &HashMap<String, u8>,
) -> Vec<syn::ItemImpl> {
    let module_name: syn::Ident = parse_str(&idl.metadata.name).unwrap();
    let program_name = idl.metadata.name.to_case(Case::UpperCamel);

    idl.instructions
        .iter()
        .fold(Vec::new(), |mut instructions_ixops_impl, instruction| {
            let mut instruction_name = instruction.name.to_case(Case::UpperCamel);
            let instruction_ident_name: syn::Ident = format_ident!("{}", &instruction_name);
            let count = instruction_mappings.get(&instruction_name).unwrap_or(&1);

            // Append the program name if the instruction name is not unique
            if *count > 1 {
                instruction_name.push_str(&program_name);
            }

            let instruction_ident_name_modified: syn::Ident =
                format_ident!("{}", &instruction_name);

            // Map arguments to their types
            let parameters = instruction
                .args
                .iter()
                .map(|arg| {
                    let arg_name = format_ident!("{}", arg.name);
                    let parameter: syn::FieldValue = match arg.ty {
                        IdlType::Pubkey => parse_quote!(#arg_name: todo!()),
                        IdlType::String => {
                            let arg_value: syn::Expr = parse_quote!(self.data.#arg_name.clone());
                            parse_quote!(#arg_name: #arg_value)
                        }
                        IdlType::Bytes => {
                            let arg_value: syn::Expr = parse_quote!(self.data.#arg_name.clone());
                            parse_quote!(#arg_name: #arg_value)
                        }
                        IdlType::Vec(_) => {
                            let arg_value: syn::Expr = parse_quote!(self.data.#arg_name.clone());
                            parse_quote!(#arg_name: #arg_value)
                        }
                        IdlType::Defined {
                            name: _,
                            generics: _,
                        } => parse_quote!(#arg_name: todo!()),
                        _ => {
                            let arg_value: syn::Expr = parse_quote!(self.data.#arg_name);
                            parse_quote!(#arg_name: #arg_value)
                        }
                    };
                    parameter
                })
                .collect::<Vec<_>>();

            let doc_comment = format!(
                "IxOps implementation for `{}` with all required functions.",
                instruction_ident_name_modified
            );
            let ix_impl: syn::ItemImpl = parse_quote! {
                #[doc = #doc_comment]
                impl IxOps for #instruction_ident_name_modified {
                    type IxData = #module_name::instruction::#instruction_ident_name;
                    type IxAccounts = FuzzAccounts;

                    /// Definition of the program ID that the Instruction is associated with.
                    fn get_program_id(&self) -> solana_sdk::pubkey::Pubkey {
                        #module_name::ID
                    }

                    /// Definition of the Instruction data.
                    /// Use randomly generated data from the fuzzer using `self.data.arg_name`
                    /// or customize the data as needed.
                    /// For more details, visit: https://ackee.xyz/trident/docs/latest/features/fuzz-instructions/#get-data
                    fn get_data(
                        &self,
                        _client: &mut impl FuzzClient,
                        _fuzz_accounts: &mut FuzzAccounts,
                    ) -> Result<Self::IxData, FuzzingError> {
                        let data = #module_name::instruction::#instruction_ident_name {
                            #(#parameters),*
                        };
                        Ok(data)
                    }

                    /// Definition of of the accounts required by the Instruction.
                    /// To utilize accounts stored in `FuzzAccounts`, use
                    /// `fuzz_accounts.account_name.get_or_create_account()`.
                    /// If no signers are required, leave the vector empty.
                    /// For AccountMetas use <program>::accounts::<corresponding_metas>
                    /// For more details, see: https://ackee.xyz/trident/docs/latest/features/fuzz-instructions/#get-accounts
                    fn get_accounts(
                    &self,
                    client: &mut impl FuzzClient,
                    fuzz_accounts: &mut FuzzAccounts,
                    ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {

                        let signers = vec![todo!()];

                        let acc_meta = todo!();

                        Ok((signers, acc_meta))
                    }
                }
            };

            instructions_ixops_impl.push(ix_impl);
            instructions_ixops_impl
        })
}

// Generate accounts for fuzzing
fn get_fuzz_accounts(idl: &Idl, accounts_mappings: &HashMap<String, u8>) -> Vec<syn::FnArg> {
    let program_name = idl.metadata.name.to_case(Case::Snake);

    // Create a HashMap to collect all fuzz accounts
    let fuzz_accounts = idl.instructions.iter().fold(
        HashMap::new(),
        |mut fuzz_accounts: HashMap<syn::Ident, syn::FnArg>, instruction| {
            instruction
                .accounts
                .iter()
                .fold(&mut fuzz_accounts, |fuzz_accounts, account| {
                    match account {
                        IdlInstructionAccountItem::Composite(_composite) => {
                            panic!("Composite Accounts are not supported yet!")
                        }
                        IdlInstructionAccountItem::Single(single) => {
                            let mut account_name = single.name.to_case(Case::Snake);
                            let count = accounts_mappings.get(&account_name).unwrap_or(&1);

                            // Append the program name if the account name is not unique
                            if *count > 1 {
                                account_name.push_str(&format!("_{}", &program_name));
                            }

                            let name: syn::Ident = format_ident!("{}", &account_name);
                            let account = match single.pda {
                                Some(_) => parse_quote! { #name: AccountsStorage<PdaStore> },
                                None => parse_quote! { #name: AccountsStorage<todo!()> },
                            };
                            fuzz_accounts.entry(name).or_insert(account);
                        }
                    };
                    fuzz_accounts
                });
            fuzz_accounts
        },
    );

    // Sort and return the fuzz accounts
    let mut sorted_accounts: Vec<_> = fuzz_accounts.into_iter().collect();
    sorted_accounts.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
    sorted_accounts.into_iter().map(|(_, v)| v).collect()
}

// Converts an `IdlType` to a corresponding Rust `syn::Type`.
fn idl_type_to_syn_type(idl_type: &IdlType, nestings: u8) -> (syn::Type, bool) {
    if nestings >= 5 {
        panic!("No more than 5 nestings allowed");
    }
    match idl_type {
        IdlType::Bool => (parse_quote!(bool), false),
        IdlType::U8 => (parse_quote!(u8), false),
        IdlType::I8 => (parse_quote!(i8), false),
        IdlType::U16 => (parse_quote!(u16), false),
        IdlType::I16 => (parse_quote!(i16), false),
        IdlType::U32 => (parse_quote!(u32), false),
        IdlType::I32 => (parse_quote!(i32), false),
        IdlType::F32 => (parse_quote!(f32), false),
        IdlType::U64 => (parse_quote!(u64), false),
        IdlType::I64 => (parse_quote!(i64), false),
        IdlType::F64 => (parse_quote!(f64), false),
        IdlType::U128 => (parse_quote!(u128), false),
        IdlType::I128 => (parse_quote!(i128), false),
        IdlType::U256 => (parse_quote!(u256), false), // Assuming custom type for u256
        IdlType::I256 => (parse_quote!(i256), false), // Assuming custom type for i256
        IdlType::Bytes => (parse_quote!(Vec<u8>), false),
        IdlType::String => (parse_quote!(String), false),
        IdlType::Pubkey => (parse_quote!(AccountId), false), // Replace with AccountId if needed
        IdlType::Option(inner) => {
            let (inner_type, is_custom) = idl_type_to_syn_type(inner, 0);
            (parse_quote!(Option<#inner_type>), is_custom)
        }
        IdlType::Vec(inner) => {
            let (inner_type, is_custom) = idl_type_to_syn_type(inner, 0);
            (parse_quote!(Vec<#inner_type>), is_custom)
        }
        IdlType::Array(inner, len) => {
            let (inner_type, is_custom) = idl_type_to_syn_type(inner, 0);
            let len = match len {
                anchor_lang_idl_spec::IdlArrayLen::Generic(_generic) => {
                    panic!("Generic within Array len not supported")
                }
                anchor_lang_idl_spec::IdlArrayLen::Value(len) => len,
            };
            (parse_quote!([#inner_type;#len]), is_custom)
        }
        // Handle defined types
        IdlType::Defined { name, generics: _ } => {
            let name_ident: syn::Ident = format_ident!("{}", &name);
            (parse_quote!(#name_ident), true)
        }
        IdlType::Generic(_name) => {
            panic!("Generic currently not supported")
        }
        _ => todo!(),
    }
}
