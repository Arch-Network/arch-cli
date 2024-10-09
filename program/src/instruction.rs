use std::mem::size_of;

use thiserror::Error;

use crate::pubkey::Pubkey;
use crate::{account::AccountMeta, program_error::ProgramError};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha256::digest;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

impl Instruction {
    pub fn serialize(&self) -> Vec<u8> {
        let mut serilized = vec![];

        serilized.extend(self.program_id.serialize());
        // accounts length should fit in a u8
        serilized.push(self.accounts.len() as u8);
        for meta in self.accounts.iter() {
            serilized.extend(&meta.serialize());
        }
        // data length should fit in a u64
        serilized.extend(self.data.len().to_le_bytes());
        serilized.extend(&self.data);

        serilized
    }

    pub fn from_slice(data: &[u8]) -> Self {
        let mut size = 32;
        let accounts_len = data[size] as usize;
        size += 1;
        let mut accounts = Vec::with_capacity(accounts_len);
        for _ in 0..accounts_len {
            accounts.push(AccountMeta::from_slice(&data[size..(size + 34)]));
            size += size_of::<AccountMeta>();
        }
        let data_len = u64::from_le_bytes(data[size..(size + 8)].try_into().unwrap());
        size += size_of::<u64>();

        Self {
            program_id: Pubkey::from_slice(&data[..32]),
            accounts,
            data: (data[size..(size + data_len as usize)]).to_vec(),
        }
    }

    pub fn hash(&self) -> String {
        digest(digest(self.serialize()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{account::AccountMeta, pubkey::Pubkey};

    use super::Instruction;

    #[test]
    fn test_serialize_deserialize() {
        let instruction = Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![],
            data: vec![],
        };

        assert_eq!(
            instruction,
            Instruction::from_slice(&instruction.serialize())
        );

        let instruction = Instruction {
            program_id: Pubkey::system_program(),
            accounts: vec![AccountMeta {
                pubkey: Pubkey::system_program(),
                is_signer: true,
                is_writable: true,
            }],
            data: vec![10; 364],
        };

        assert_eq!(
            instruction,
            Instruction::from_slice(&instruction.serialize())
        );
    }
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum InstructionError {
    /// Deprecated! Use CustomError instead!
    /// The program instruction returned an error
    #[error("generic instruction error")]
    GenericError,

    /// The arguments provided to a program were invalid
    #[error("invalid program argument")]
    InvalidArgument,

    /// An instruction's data contents were invalid
    #[error("invalid instruction data")]
    InvalidInstructionData,

    /// An account's data contents was invalid
    #[error("invalid account data for instruction")]
    InvalidAccountData,

    /// An account's data was too small
    #[error("account data too small for instruction")]
    AccountDataTooSmall,

    /// An account's balance was too small to complete the instruction
    #[error("insufficient funds for instruction")]
    InsufficientFunds,

    /// The account did not have the expected program id
    #[error("incorrect program id for instruction")]
    IncorrectProgramId,

    /// A signature was required but not found
    #[error("missing required signature for instruction")]
    MissingRequiredSignature,

    /// An initialize instruction was sent to an account that has already been initialized.
    #[error("instruction requires an uninitialized account")]
    AccountAlreadyInitialized,

    /// An attempt to operate on an account that hasn't been initialized.
    #[error("instruction requires an initialized account")]
    UninitializedAccount,

    /// Program's instruction lamport balance does not equal the balance after the instruction
    #[error("sum of account balances before and after instruction do not match")]
    UnbalancedInstruction,

    /// Program illegally modified an account's program id
    #[error("instruction illegally modified the program id of an account")]
    ModifiedProgramId,

    /// Program spent the lamports of an account that doesn't belong to it
    #[error("instruction spent from the balance of an account it does not own")]
    ExternalAccountLamportSpend,

    /// Program modified the data of an account that doesn't belong to it
    #[error("instruction modified data of an account it does not own")]
    ExternalAccountDataModified,

    /// Read-only account's data was modified
    #[error("instruction modified data of a read-only account")]
    ReadonlyDataModified,

    /// An account was referenced more than once in a single instruction
    // Deprecated, instructions can now contain duplicate accounts
    #[error("instruction contains duplicate accounts")]
    DuplicateAccountIndex,

    /// Executable bit on account changed, but shouldn't have
    #[error("instruction changed executable bit of an account")]
    ExecutableModified,

    /// The instruction expected additional account keys
    #[error("insufficient account keys for instruction")]
    NotEnoughAccountKeys,

    /// Program other than the account's owner changed the size of the account data
    #[error("program other than the account's owner changed the size of the account data")]
    AccountDataSizeChanged,

    /// The instruction expected an executable account
    #[error("instruction expected an executable account")]
    AccountNotExecutable,

    /// Failed to borrow a reference to account data, already borrowed
    #[error("instruction tries to borrow reference for an account which is already borrowed")]
    AccountBorrowFailed,

    /// Account data has an outstanding reference after a program's execution
    #[error("instruction left account with an outstanding borrowed reference")]
    AccountBorrowOutstanding,

    /// The same account was multiply passed to an on-chain program's entrypoint, but the program
    /// modified them differently.  A program can only modify one instance of the account because
    /// the runtime cannot determine which changes to pick or how to merge them if both are modified
    #[error("instruction modifications of multiply-passed account differ")]
    DuplicateAccountOutOfSync,

    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    #[error("custom program error: {0:#x}")]
    Custom(u32),

    /// Error caused during a processing of a program
    #[error("program error: {0}")]
    ProgramError(ProgramError),

    /// The return value from the program was invalid.  Valid errors are either a defined builtin
    /// error value or a user-defined error in the lower 32 bits.
    #[error("program returned invalid error code")]
    InvalidError,

    /// Executable account's data was modified
    #[error("instruction changed executable accounts data")]
    ExecutableDataModified,

    /// Unsupported program id
    #[error("Unsupported program id")]
    UnsupportedProgramId,

    /// Cross-program invocation call depth too deep
    #[error("Cross-program invocation call depth too deep")]
    CallDepth,

    /// An account required by the instruction is missing
    #[error("An account required by the instruction is missing")]
    MissingAccount,

    /// Cross-program invocation reentrancy not allowed for this instruction
    #[error("Cross-program invocation reentrancy not allowed for this instruction")]
    ReentrancyNotAllowed,

    /// Length of the seed is too long for address generation
    #[error("Length of the seed is too long for address generation")]
    MaxSeedLengthExceeded,

    /// Provided seeds do not result in a valid address
    #[error("Provided seeds do not result in a valid address")]
    InvalidSeeds,

    /// Failed to reallocate account data of this length
    #[error("Failed to reallocate account data")]
    InvalidRealloc,

    /// Computational budget exceeded
    #[error("Computational budget exceeded")]
    ComputationalBudgetExceeded,

    /// Cross-program invocation with unauthorized signer or writable account
    #[error("Cross-program invocation with unauthorized signer or writable account")]
    PrivilegeEscalation,

    /// Failed to create program execution environment
    #[error("Failed to create program execution environment")]
    ProgramEnvironmentSetupFailure,

    /// Program failed to complete
    #[error("Program failed to complete")]
    ProgramFailedToComplete,

    /// Program failed to compile
    #[error("Program failed to compile")]
    ProgramFailedToCompile,

    /// Program failed to compile
    #[error("Elf failed to parse")]
    ElfFailedToParse,

    /// Account is immutable
    #[error("Account is immutable")]
    Immutable,

    /// Incorrect authority provided
    #[error("Incorrect authority provided")]
    IncorrectAuthority,

    /// Failed to serialize or deserialize account data
    ///
    /// Warning: This error should never be emitted by the runtime.
    ///
    /// This error includes strings from the underlying 3rd party Borsh crate
    /// which can be dangerous because the error strings could change across
    /// Borsh versions. Only programs can use this error because they are
    /// consistent across Solana software versions.
    ///
    #[error("Failed to serialize or deserialize account data: {0}")]
    BorshIoError(String),

    /// Invalid account owner
    #[error("Invalid account owner")]
    InvalidAccountOwner,

    /// Program arithmetic overflowed
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    /// Unsupported sysvar
    #[error("Unsupported sysvar")]
    UnsupportedSysvar,

    /// Illegal account owner
    #[error("Provided owner is not allowed")]
    IllegalOwner,

    /// Accounts data allocations exceeded the maximum allowed per transaction
    #[error("Accounts data allocations exceeded the maximum allowed per transaction")]
    MaxAccountsDataAllocationsExceeded,

    /// Max accounts exceeded
    #[error("Max accounts exceeded")]
    MaxAccountsExceeded,

    /// Max instruction trace length exceeded
    #[error("Max instruction trace length exceeded")]
    MaxInstructionTraceLengthExceeded,

    /// Error Initilising BTC RPC
    #[error("unable to connect to bitcoin rpc")]
    RPCError,

    /// Builtin programs must consume compute units
    #[error("Builtin programs must consume compute units")]
    BuiltinProgramsMustConsumeComputeUnits,

    /// Vm execution failed
    #[error("Vm failed while executing ebpf ncode {0}")]
    EbpfError(String),

    /// Builtin programs must consume compute units
    #[error("Builtin programs must consume compute units")]
    InvalidTxToSign,
    // Note: For any new error added here an equivalent ProgramError and its
    // conversions must also be added
}
