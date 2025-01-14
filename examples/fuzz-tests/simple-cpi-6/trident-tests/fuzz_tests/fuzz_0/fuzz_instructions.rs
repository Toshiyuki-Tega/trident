use trident_client::fuzzing::*;

/// FuzzInstruction contains all available Instructions.
/// Below, the instruction arguments (accounts and data) are defined.
#[derive(Arbitrary, DisplayIx, FuzzTestExecutor)]
pub enum FuzzInstruction {
    InitializeCallee(InitializeCallee),
    InitializeCaller(InitializeCaller),
}
#[derive(Arbitrary, Debug)]
pub struct InitializeCallee {
    pub _accounts: InitializeCalleeAccounts,
    pub data: InitializeCalleeData,
}
#[derive(Arbitrary, Debug)]
pub struct InitializeCalleeAccounts {
    pub _signer: AccountId,
}
/// Custom data types must derive `Debug` and `Arbitrary`.
/// To do this, redefine the type in the fuzz test and implement the `From`
/// trait
/// to convert it into the type defined in the program.
/// For more details, see: https://ackee.xyz/trident/docs/dev/features/arbitrary-data/#custom-data-types
#[derive(Arbitrary, Debug)]
pub struct InitializeCalleeData {
    pub input: u8,
}
#[derive(Arbitrary, Debug)]
pub struct InitializeCaller {
    pub accounts: InitializeCallerAccounts,
    pub data: InitializeCallerData,
}
#[derive(Arbitrary, Debug)]
pub struct InitializeCallerAccounts {
    pub signer: AccountId,
    pub _program: AccountId,
}
/// Custom data types must derive `Debug` and `Arbitrary`.
/// To do this, redefine the type in the fuzz test and implement the `From`
/// trait
/// to convert it into the type defined in the program.
/// For more details, see: https://ackee.xyz/trident/docs/dev/features/arbitrary-data/#custom-data-types
#[derive(Arbitrary, Debug)]
pub struct InitializeCallerData {
    pub input: u8,
}
///IxOps implementation for `InitializeCallee` with all required functions.
impl IxOps for InitializeCallee {
    type IxData = callee::instruction::InitializeCallee;
    type IxAccounts = FuzzAccounts;
    /// Definition of the program ID that the Instruction is associated with.
    fn get_program_id(&self) -> solana_sdk::pubkey::Pubkey {
        callee::ID
    }
    /// Definition of the Instruction data.
    /// Use randomly generated data from the fuzzer using `self.data.arg_name`
    /// or customize the data as needed.
    /// For more details, visit: https://ackee.xyz/trident/docs/dev/features/fuzz-instructions/#get-data
    fn get_data(
        &self,
        _client: &mut impl FuzzClient,
        _fuzz_accounts: &mut FuzzAccounts,
    ) -> Result<Self::IxData, FuzzingError> {
        let data = callee::instruction::InitializeCallee {
            input: self.data.input,
        };
        Ok(data)
    }
    /// Definition of of the accounts required by the Instruction.
    /// To utilize accounts stored in `FuzzAccounts`, use
    /// `fuzz_accounts.account_name.get_or_create_account()`.
    /// If no signers are required, leave the vector empty.
    /// For AccountMetas use <program>::accounts::<corresponding_metas>
    /// For more details, see: https://ackee.xyz/trident/docs/dev/features/fuzz-instructions/#get-accounts
    fn get_accounts(
        &self,
        _client: &mut impl FuzzClient,
        _fuzz_accounts: &mut FuzzAccounts,
    ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
        let signers = vec![];
        let acc_meta = todo!();
        Ok((signers, acc_meta))
    }
}
///IxOps implementation for `InitializeCaller` with all required functions.
impl IxOps for InitializeCaller {
    type IxData = caller::instruction::InitializeCaller;
    type IxAccounts = FuzzAccounts;
    /// Definition of the program ID that the Instruction is associated with.
    fn get_program_id(&self) -> solana_sdk::pubkey::Pubkey {
        caller::ID
    }
    /// Definition of the Instruction data.
    /// Use randomly generated data from the fuzzer using `self.data.arg_name`
    /// or customize the data as needed.
    /// For more details, visit: https://ackee.xyz/trident/docs/dev/features/fuzz-instructions/#get-data
    fn get_data(
        &self,
        _client: &mut impl FuzzClient,
        _fuzz_accounts: &mut FuzzAccounts,
    ) -> Result<Self::IxData, FuzzingError> {
        let data = caller::instruction::InitializeCaller {
            input: self.data.input,
        };
        Ok(data)
    }
    /// Definition of of the accounts required by the Instruction.
    /// To utilize accounts stored in `FuzzAccounts`, use
    /// `fuzz_accounts.account_name.get_or_create_account()`.
    /// If no signers are required, leave the vector empty.
    /// For AccountMetas use <program>::accounts::<corresponding_metas>
    /// For more details, see: https://ackee.xyz/trident/docs/dev/features/fuzz-instructions/#get-accounts
    fn get_accounts(
        &self,
        client: &mut impl FuzzClient,
        fuzz_accounts: &mut FuzzAccounts,
    ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
        let signer = fuzz_accounts.signer_caller.get_or_create_account(
            self.accounts.signer,
            client,
            5 * solana_sdk::native_token::LAMPORTS_PER_SOL,
        );
        let signers = vec![signer.clone()];
        let acc_meta = caller::accounts::InitializeCaller {
            signer: signer.pubkey(),
            program: callee::ID,
        }
        .to_account_metas(None);
        Ok((signers, acc_meta))
    }
}
/// Use AccountsStorage<T> where T can be one of:
/// Keypair, PdaStore, TokenStore, MintStore, ProgramStore
#[derive(Default)]
pub struct FuzzAccounts {
    _program: AccountsStorage<ProgramStore>,
    signer_caller: AccountsStorage<KeypairStore>,
}
