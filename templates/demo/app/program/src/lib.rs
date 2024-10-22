use arch_program::{
    account::AccountInfo,
    entrypoint,
    msg,
    program::next_account_info,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use arch_program::program::get_clock;

use borsh::{BorshSerialize, BorshDeserialize};

entrypoint!(process_instruction);

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GraffitiMessage {
    pub timestamp: i64,
    pub name: [u8; 16],
    pub message: [u8; 64],
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GraffitiWall {
    pub messages: Vec<GraffitiMessage>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GraffitiWallParams {
    pub name: [u8; 16],
    pub message: [u8; 64],
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    msg!("Graffiti Wall: Processing instruction");

    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    // Print out account owner and program id
    msg!("Graffiti Wall: Account owner: {:?}", account.owner);
    msg!("Graffiti Wall: Program id: {:?}", program_id);
    
    if account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let params = GraffitiWallParams::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let clock = get_clock();
    let timestamp = clock.unix_timestamp;

    let new_message = GraffitiMessage {
        timestamp,
        name: params.name,
        message: params.message,
    };

    msg!("Graffiti Wall: New message: {:?}", new_message);

    let mut wall = if account.data_len() > 0 {
        GraffitiWall::try_from_slice(&account.data.borrow())
            .map_err(|_| ProgramError::InvalidAccountData)?
    } else {
        GraffitiWall { messages: vec![] }
    };

    wall.messages.push(new_message);

    let serialized_data = borsh::to_vec(&wall)
        .map_err(|_| ProgramError::AccountDataTooSmall)?;

    // Ensure data fits within 10MB limit
    if serialized_data.len() > 10 * 1024 * 1024 {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let required_len = serialized_data.len();
    if account.data_len() < required_len {
        account.realloc(required_len, false)?;
    }

    account.data.borrow_mut()[..required_len].copy_from_slice(&serialized_data);

    msg!("Graffiti Wall: Message added successfully");
    Ok(())
}