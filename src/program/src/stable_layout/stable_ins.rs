use crate::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    stable_layout::stable_vec::StableVec,
};

#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct StableInstruction {
    pub program_id: Pubkey,
    pub accounts: StableVec<AccountMeta>,
    pub data: StableVec<u8>,
}

impl From<Instruction> for StableInstruction {
    fn from(other: Instruction) -> Self {
        Self {
            program_id: other.program_id,
            accounts: other.accounts.into(),
            data: other.data.into(),
        }
    }
}
