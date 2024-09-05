use arch_program::{
    account::AccountInfo,
    entrypoint,
    instruction::Instruction,
    msg,
    program::next_account_info,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use borsh::{ BorshDeserialize, BorshSerialize };

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> Result<(), ProgramError> {
    msg!("Hello, Arch Network!");
    msg!("Program ID: {:?}", program_id);
    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct AppInstruction {
    // Define your instruction structure here
}
