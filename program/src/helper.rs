use std::str::FromStr;

use bitcoin::{
    absolute::LockTime, transaction::Version, OutPoint, ScriptBuf, Sequence, Transaction, TxIn,
    TxOut, Txid, Witness,
};

use crate::{
    account::AccountInfo,
    program::{get_account_script_pubkey, get_bitcoin_tx},
};

pub fn get_state_trasition_tx(accounts: &[AccountInfo]) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: accounts
            .iter()
            .filter(|account| account.is_writable)
            .map(|account| TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_str(&hex::encode(account.utxo.txid())).unwrap(),
                    vout: account.utxo.vout(),
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect::<Vec<TxIn>>(),
        output: accounts
            .iter()
            .filter(|account| account.is_writable)
            .map(|account| {
                let tx: Transaction = bitcoin::consensus::deserialize(
                    &get_bitcoin_tx(account.utxo.txid().try_into().unwrap()).unwrap(),
                )
                .unwrap();

                TxOut {
                    value: tx.output[account.utxo.vout() as usize].value,
                    script_pubkey: ScriptBuf::from_bytes(
                        get_account_script_pubkey(account.key).to_vec(),
                    ),
                }
            })
            .collect::<Vec<TxOut>>(),
    }
}
