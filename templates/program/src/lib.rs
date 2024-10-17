use arch_program::{
    account::AccountInfo,
    entrypoint, msg,
    helper::add_state_transition,
    input_to_sign::InputToSign,
    program::{
        get_account_script_pubkey, get_bitcoin_block_height,
        next_account_info, set_transaction_to_sign, invoke
    },
    program_error::ProgramError,
    pubkey::Pubkey, utxo::UtxoMeta, 
    transaction_to_sign::TransactionToSign,
    system_instruction::SystemInstruction,
};
use bitcoin::{self, Transaction, transaction::Version, absolute::LockTime};
use borsh::{BorshDeserialize, BorshSerialize};

entrypoint!(process_instruction);
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    if accounts.len() != 2 {
        return Err(ProgramError::Custom(501));
    }

    let bitcoin_block_height = get_bitcoin_block_height();
    msg!("bitcoin_block_height {:?}", bitcoin_block_height);

    let account_iter = &mut accounts.iter();
    let account = next_account_info(account_iter)?;
    let account2 = next_account_info(account_iter)?;

    msg!("account {:?}", account);
    msg!("account2 {:?}", account2);

    if account2.utxo.clone() != UtxoMeta::from_slice(&[0; 36]) {
        msg!("UTXO {:?}", account2.utxo.clone());
        return Err(ProgramError::Custom(502));
    }

    let params: HelloWorldParams = borsh::from_slice(instruction_data).unwrap();
    let fees_tx: Transaction = bitcoin::consensus::deserialize(&params.tx_hex).unwrap();

    let new_data = format!("Hello {}", params.name);

    // Extend the account data to fit the new data
    let data_len = account.data.try_borrow().unwrap().len();
    if new_data.as_bytes().len() > data_len {
        account.realloc(new_data.len(), true)?;
    }

    let script_pubkey = get_account_script_pubkey(account.key);
    msg!("script_pubkey {:?}", script_pubkey);

    account
        .data
        .try_borrow_mut()
        .unwrap()
        .copy_from_slice(new_data.as_bytes());

    if account2.is_writable {

        invoke(
            &SystemInstruction::new_create_account_instruction(
                params.utxo.txid().try_into().unwrap(), 
                params.utxo.vout(), account2.key.clone()
            ), 
            &[account2.clone()]
        ).expect("failed");
    }

    let mut tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![],
        output: vec![],
    };
    add_state_transition(&mut tx, account);
    tx.input.push(fees_tx.input[0].clone());

    let tx_to_sign = TransactionToSign {
        tx_bytes: &bitcoin::consensus::serialize(&tx),
        inputs_to_sign: &[InputToSign {
            index: 0,
            signer: account.key.clone(),
        }],
    };

    msg!("tx_to_sign{:?}", tx_to_sign);

    set_transaction_to_sign(accounts, tx_to_sign)
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    pub name: String,
    pub tx_hex: Vec<u8>,
    pub utxo: UtxoMeta,
}
