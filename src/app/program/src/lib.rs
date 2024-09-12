use arch_program::{
    account::AccountInfo,
    entrypoint,
    helper::get_state_transition_tx,
    input_to_sign::InputToSign,
    instruction::Instruction,
    msg,
    program::{
        get_account_script_pubkey, get_bitcoin_tx, get_network_xonly_pubkey, invoke,
        next_account_info, set_return_data, set_transaction_to_sign, validate_utxo_ownership,
    },
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::SystemInstruction,
    transaction_to_sign::TransactionToSign,
    utxo::UtxoMeta,
};
use bitcoin::{self, Transaction};
use borsh::{BorshDeserialize, BorshSerialize};

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {

    msg!("Hello World!");

    // Get mutable reference to account data
    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;

    // Deserialize the instruction data
    let params = GraffitiWallParams::try_from_slice(instruction_data).unwrap();

    // Prepare the new entry
    let new_entry = format!("{}|{}|", params.name, params.message);
    
    // Extend the account data to fit the new entry
    let data_len = account.data.try_borrow().unwrap().len();
    if new_entry.as_bytes().len() + data_len > data_len {
        account.realloc(data_len + new_entry.as_bytes().len(), true)?;
    }

    // Get mutable reference to account data after realloc
    let mut account_data = account.data.borrow_mut();
    let current_length = account_data.iter().position(|&x| x == 0).unwrap_or(account_data.len());

    // Check if the new entry fits within the 10MB limit
    let new_entry_length = new_entry.len(); // Updated to use new_entry length
    if current_length + new_entry_length > 10 * 1024 * 1024 { // 10MB limit
        msg!("Graffiti wall is full. Cannot add more entries.");
        return Err(ProgramError::InvalidArgument);
    }

    // Append the new entry to the existing data
    account_data[current_length..current_length + new_entry_length]
        .copy_from_slice(new_entry.as_bytes());

    Ok(())
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct GraffitiWallParams {
    pub name: String,
    pub message: String,
}
