//! This module contains helper methods for interacting with the HelloWorld program
use anyhow::{ anyhow, Result };
use bitcoin::{
    absolute::LockTime,
    address::Address,
    key::{ TapTweak, TweakedKeypair },
    secp256k1::{ self, Secp256k1 },
    sighash::{ Prevouts, SighashCache },
    transaction::Version,
    Amount,
    OutPoint,
    ScriptBuf,
    Sequence,
    TapSighashType,
    Transaction,
    TxIn,
    Witness,
};
use bitcoincore_rpc::{ Auth, Client, RawTx, RpcApi };
use env_logger;
use log::{ debug, error, info, warn };
use reqwest::blocking::Client as HttpClient;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{ from_str, json, Value };
use std::env;
use std::fs;
use std::process::Child;
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;

use crate::processed_transaction::ProcessedTransaction;

use crate::constants::{
    BITCOIN_NODE_ENDPOINT,
    BITCOIN_NODE_PASSWORD,
    BITCOIN_NODE_USERNAME,
    CALLER_FILE_PATH,
    GET_ACCOUNT_ADDRESS,
    GET_BEST_BLOCK_HASH,
    GET_BLOCK,
    GET_PROCESSED_TRANSACTION,
    GET_PROGRAM,
    NODE1_ADDRESS,
    READ_ACCOUNT_INFO,
    TRANSACTION_NOT_FOUND_CODE,
};
use crate::models::{ BitcoinRpcInfo, CallerInfo };
use arch_program::message::Message;
use arch_program::pubkey::Pubkey;
use crate::runtime_transaction::RuntimeTransaction;
use crate::signature::Signature;
use arch_program::instruction::Instruction;

fn process_result(response: String) -> Result<Value> {
    let result = from_str::<Value>(&response).expect("result should be Value parseable");

    let result = match result {
        Value::Object(object) => object,
        _ => panic!("unexpected output"),
    };

    if let Some(err) = result.get("error") {
        return Err(anyhow!("{:?}", err));
    }

    Ok(result["result"].clone())
}

fn process_get_transaction_result(response: String) -> Result<Value> {
    let result = from_str::<Value>(&response).expect("result should be string parseable");

    let result = match result {
        Value::Object(object) => object,
        _ => panic!("unexpected output"),
    };

    if let Some(err) = result.get("error") {
        if let Value::Number(code) = result["error"]["code"].clone() {
            if code.as_i64() == Some(TRANSACTION_NOT_FOUND_CODE) {
                return Ok(Value::Null);
            }
        }
        return Err(anyhow!("{:?}", err));
    }

    Ok(result["result"].clone())
}

pub fn is_server_up(address: &str) -> Result<(), String> {
    let client = HttpClient::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    client
        .get(address)
        .send()
        .map_err(|e| format!("Failed to connect to server: {}", e))?;

    Ok(())
}

fn post(url: &str, method: &str) -> String {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(url)
        .header("content-type", "application/json")
        .json(
            &json!({
            "jsonrpc": "2.0",
            "id": "curlycurl",
            "method": method,
        })
        )
        .send()
        .expect("post method should not fail");

    res.text().expect("result should be text decodable")
}

fn post_data<T: Serialize + std::fmt::Debug>(url: &str, method: &str, params: T) -> String {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(url)
        .header("content-type", "application/json")
        .json(
            &json!({
            "jsonrpc": "2.0",
            "id": "curlycurl",
            "method": method,
            "params": params,
        })
        )
        .send();

    res.expect("post method should not fail").text().expect("result should be text decodable")
}

/// Returns a caller information using the secret key file specified
fn get_trader(trader_id: u64) -> Result<CallerInfo> {
    let file_path = &format!("../../.arch/trader{}.json", trader_id);
    Ok(CallerInfo::with_secret_key_file(file_path)?)
}

use crate::helper::secp256k1::SecretKey;
use bitcoin::key::UntweakedKeypair;
use bitcoin::XOnlyPublicKey;
use rand_core::OsRng;

pub fn with_secret_key_file(file_path: &str) -> Result<(UntweakedKeypair, Pubkey)> {
    let secp = Secp256k1::new();
    let secret_key = match fs::read_to_string(file_path) {
        Ok(key) => SecretKey::from_str(&key).unwrap(),
        Err(_) => {
            let (key, _) = secp.generate_keypair(&mut OsRng);
            fs
                ::write(file_path, &key.display_secret().to_string())
                .map_err(|_| anyhow!("Unable to write file"))?;
            key
        }
    };
    let keypair = UntweakedKeypair::from_secret_key(&secp, &secret_key);
    let pubkey = Pubkey::from_slice(&XOnlyPublicKey::from_keypair(&keypair).0.serialize());
    Ok((keypair, pubkey))
}

use arch_program::system_instruction::SystemInstruction;
use crate::runtime_transaction::RUNTIME_TX_SIZE_LIMIT;

fn extend_bytes_max_len() -> usize {
    let message = Message {
        signers: vec![Pubkey::system_program()],
        instructions: vec![
            SystemInstruction::new_extend_bytes_instruction(vec![0_u8; 8], Pubkey::system_program())
        ],
    };

    RUNTIME_TX_SIZE_LIMIT -
        (RuntimeTransaction {
            version: 0,
            signatures: vec![Signature([0_u8; 64].to_vec())],
            message,
        })
            .serialize()
            .len()
}

/// Creates an instruction, signs it as a message
/// and sends the signed message as a transaction
pub fn sign_and_send_instruction(
    instruction: Instruction,
    signers: Vec<UntweakedKeypair>
) -> Result<(String, String)> {
    let pubkeys = signers
        .iter()
        .map(|signer| Pubkey::from_slice(&XOnlyPublicKey::from_keypair(signer).0.serialize()))
        .collect::<Vec<Pubkey>>();

    let message = Message {
        signers: pubkeys,
        instructions: vec![instruction.clone()],
    };
    let digest_slice = hex::decode(message.hash()).expect("hashed message should be decodable");
    let sig_message = secp256k1::Message
        ::from_digest_slice(&digest_slice)
        .expect("signed message should be gotten from digest slice");

    let secp = Secp256k1::new();
    let signatures = signers
        .iter()
        .map(|signer| { Signature(secp.sign_schnorr(&sig_message, &signer).serialize().to_vec()) })
        .collect::<Vec<Signature>>();

    let params = RuntimeTransaction {
        version: 0,
        signatures,
        message,
    };

    debug!("RuntimeTransaction Params: {:?}", params);

    let result = process_result(post_data(NODE1_ADDRESS, "send_transaction", params))
        .expect("send_transaction should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string();
    let hashed_instruction = instruction.hash();

    Ok((result, hashed_instruction))
}

pub fn sign_and_send_transaction(
    instructions: Vec<Instruction>,
    signers: Vec<UntweakedKeypair>
) -> Result<String> {
    let pubkeys = signers
        .iter()
        .map(|signer| Pubkey::from_slice(&XOnlyPublicKey::from_keypair(signer).0.serialize()))
        .collect::<Vec<Pubkey>>();

    let message = Message {
        signers: pubkeys,
        instructions,
    };
    let digest_slice = hex::decode(message.hash()).expect("hashed message should be decodable");
    let sig_message = secp256k1::Message
        ::from_digest_slice(&digest_slice)
        .expect("signed message should be gotten from digest slice");

    let secp = Secp256k1::new();
    let signatures = signers
        .iter()
        .map(|signer| Signature(secp.sign_schnorr(&sig_message, signer).serialize().to_vec()))
        .collect::<Vec<Signature>>();

    let params = RuntimeTransaction {
        version: 0,
        signatures,
        message,
    };
    let result = process_result(post_data(NODE1_ADDRESS, "send_transaction", params))
        .expect("send_transaction should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string();

    Ok(result)
}

/// Deploys the HelloWorld program using the compiled ELF
pub fn deploy_program_txs(program_keypair: UntweakedKeypair, elf_path: &str) -> Vec<String> {
    info!("Starting program deployment");
    let program_pubkey = Pubkey::from_slice(
        &XOnlyPublicKey::from_keypair(&program_keypair).0.serialize()
    );
    let elf = fs::read(elf_path).expect("Failed to read ELF file");
    info!("ELF file size: {} bytes", elf.len());
    let txs = elf
        .chunks(extend_bytes_max_len())
        .enumerate()
        .map(|(i, chunk)| {
            let mut bytes = vec![];
            let offset: u32 = (i * extend_bytes_max_len()) as u32;
            let len: u32 = chunk.len() as u32;
            bytes.extend(offset.to_le_bytes());
            bytes.extend(len.to_le_bytes());
            bytes.extend(chunk);
            let message = Message {
                signers: vec![program_pubkey.clone()],
                instructions: vec![
                    SystemInstruction::new_extend_bytes_instruction(bytes, program_pubkey.clone())
                ],
            };
            let digest_slice = hex
                ::decode(message.hash())
                .expect("hashed message should be decodable");
            let sig_message = secp256k1::Message
                ::from_digest_slice(&digest_slice)
                .expect("signed message should be gotten from digest slice");
            let secp = Secp256k1::new();
            RuntimeTransaction {
                version: 0,
                signatures: vec![
                    Signature(
                        secp.sign_schnorr(&sig_message, &program_keypair).serialize().to_vec()
                    )
                ],
                message,
            }
        })
        .collect::<Vec<RuntimeTransaction>>();

    info!("Deploying program with {} transactions", txs.len());

    let txids = process_result(post_data(NODE1_ADDRESS, "send_transactions", txs))
        .expect("send_transaction should not fail")
        .as_array()
        .expect("cannot convert result to array")
        .iter()
        .map(|r| { r.as_str().expect("cannot convert object to string").to_string() })
        .collect::<Vec<String>>();

    info!("Successfully sent {} transactions for program deployment", txids.len());

    for (i, txid) in txids.iter().enumerate() {
        match get_processed_transaction(NODE1_ADDRESS, txid.clone()) {
            Ok(_) => debug!("Transaction {} (ID: {}) processed successfully", i + 1, txid),
            Err(e) => warn!("Failed to process transaction {} (ID: {}): {:?}", i + 1, txid, e),
        }
    }

    txids
}

/// Starts Key Exchange by calling the RPC method
pub fn start_key_exchange() {
    match process_result(post(NODE1_ADDRESS, "start_key_exchange")) {
        Err(err) => error!("Failed to initiate Key Exchange: {:?}", err),
        Ok(val) => {
            if val.as_bool().unwrap_or(false) {
                info!("Key Exchange initiated successfully");
            } else {
                warn!("Key Exchange initiation returned unexpected result");
            }
        }
    };
}

/// Starts a Distributed Key Generation round by calling the RPC method
pub fn start_dkg() {
    match process_result(post(NODE1_ADDRESS, "start_dkg")) {
        Ok(_) => info!("Distributed Key Generation (DKG) initiated successfully"),
        Err(err) => error!("Failed to initiate Distributed Key Generation: {:?}", err),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfoResult {
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub utxo: String,
    pub is_executable: bool,
}

/// Read Utxo given the utxo ID
pub fn read_account_info(url: &str, pubkey: Pubkey) -> Result<AccountInfoResult> {
    // Perform the POST request and get the raw response
    let raw_response = post_data(url, READ_ACCOUNT_INFO, pubkey);

    // Process the result
    let result = process_result(raw_response.clone()).map_err(|e|
        anyhow!("Error processing result: {:?}", e)
    )?;

    // Attempt to deserialize into AccountInfoResult
    let account_info: AccountInfoResult = serde_json
        ::from_value(result)
        .map_err(|e| anyhow!("Unable to decode read_account_info result: {:?}", e))?;

    info!("Retrieved account info for pubkey: {:?}", pubkey);
    debug!(
        "Account info details: Owner: {:?}, Data length: {} bytes, Executable: {}",
        account_info.owner,
        account_info.data.len(),
        account_info.is_executable
    );

    Ok(account_info)
}

/// Returns a program given the program ID
pub fn get_program(url: &str, program_id: String) -> String {
    process_result(post_data(url, GET_PROGRAM, program_id))
        .expect("get_program should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string()
}

/// Returns the best block
fn get_best_block() -> String {
    let best_block_hash = process_result(post(NODE1_ADDRESS, GET_BEST_BLOCK_HASH))
        .expect("best_block_hash should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string();
    process_result(post_data(NODE1_ADDRESS, GET_BLOCK, best_block_hash))
        .expect("get_block should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string()
}

/// Returns a processed transaction given the txid
/// Keeps trying for a maximum of 60 seconds if the processed transaction is not available
pub fn get_processed_transaction(url: &str, tx_id: String) -> Result<ProcessedTransaction> {
    let mut processed_tx = process_get_transaction_result(
        post_data(url, GET_PROCESSED_TRANSACTION, tx_id.clone())
    );
    if let Err(e) = processed_tx {
        return Err(anyhow!("{}", e));
    }

    let mut wait_time = 10;
    while let Ok(Value::Null) = processed_tx {
        debug!("Transaction not yet processed. Retrying in {} seconds...", wait_time);
        std::thread::sleep(std::time::Duration::from_secs(wait_time));
        processed_tx = process_get_transaction_result(
            post_data(url, GET_PROCESSED_TRANSACTION, tx_id.clone())
        );
        wait_time += 10;
        if wait_time >= 60 {
            error!("Failed to retrieve processed transaction after 60 seconds");
            return Err(anyhow!("Timeout: Failed to retrieve processed transaction"));
        }
    }

    info!("Successfully retrieved and processed transaction: {}", tx_id);
    Ok(serde_json::from_value(processed_tx?).unwrap())
}

pub fn prepare_fees() -> String {
    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string()
    );
    let rpc = Client::new(BITCOIN_NODE_ENDPOINT, userpass).expect(
        "rpc shouldn not fail to be initiated"
    );

    let caller = CallerInfo::with_secret_key_file(CALLER_FILE_PATH).expect(
        "getting caller info should not fail"
    );

    let txid = rpc
        .send_to_address(
            &caller.address,
            Amount::from_sat(3000),
            None,
            None,
            None,
            None,
            None,
            None
        )
        .expect("SATs should be sent to address");

    let sent_tx = rpc.get_raw_transaction(&txid, None).expect("should get raw transaction");
    let mut vout = 0;

    for (index, output) in sent_tx.output.iter().enumerate() {
        if output.script_pubkey == caller.address.script_pubkey() {
            vout = index as u32;
        }
    }

    let mut tx = Transaction {
        version: Version::TWO,
        input: vec![TxIn {
            previous_output: OutPoint { txid, vout },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![],
        lock_time: LockTime::ZERO,
    };

    let sighash_type = TapSighashType::NonePlusAnyoneCanPay;
    let raw_tx = rpc.get_raw_transaction(&txid, None).expect("raw transaction should not fail");
    let prevouts = vec![raw_tx.output[vout as usize].clone()];
    let prevouts = Prevouts::All(&prevouts);

    let mut sighasher = SighashCache::new(&mut tx);
    let sighash = sighasher
        .taproot_key_spend_signature_hash(0, &prevouts, sighash_type)
        .expect("should not fail to construct sighash");

    // Sign the sighash using the secp256k1 library (exported by rust-bitcoin).
    let secp = Secp256k1::new();
    let tweaked: TweakedKeypair = caller.key_pair.tap_tweak(&secp, None);
    let msg = secp256k1::Message::from(sighash);
    let signature = secp.sign_schnorr(&msg, &tweaked.to_inner());

    // Update the witness stack.
    let signature = bitcoin::taproot::Signature {
        sig: signature,
        hash_ty: sighash_type,
    };
    tx.input[0].witness.push(signature.to_vec());

    tx.raw_hex()
}

pub fn send_utxo(pubkey: Pubkey) -> (String, u32) {
    let userpass = Auth::UserPass(
        BITCOIN_NODE_USERNAME.to_string(),
        BITCOIN_NODE_PASSWORD.to_string()
    );
    let rpc = Client::new(BITCOIN_NODE_ENDPOINT, userpass).expect(
        "rpc shouldn not fail to be initiated"
    );

    let caller = CallerInfo::with_secret_key_file(CALLER_FILE_PATH).expect(
        "getting caller info should not fail"
    );

    let address = get_account_address(pubkey);

    let account_address = Address::from_str(&address)
        .unwrap()
        .require_network(bitcoin::Network::Regtest)
        .unwrap();

    info!("Sending UTXO to account address: {}", address);

    let txid = rpc
        .send_to_address(
            &account_address,
            Amount::from_sat(3000),
            None,
            None,
            None,
            None,
            None,
            None
        )
        .expect("Failed to send SATs to address");

    let sent_tx = rpc.get_raw_transaction(&txid, None).expect("should get raw transaction");
    let mut vout = 0;

    for (index, output) in sent_tx.output.iter().enumerate() {
        if output.script_pubkey == account_address.script_pubkey() {
            vout = index as u32;
            println!("Found a matching UTXO");
        }
    }

    info!("UTXO sent successfully. Transaction ID: {}, Output Index: {}", txid, vout);
    (txid.to_string(), vout)
}

pub async fn get_account_address_async(pubkey: Pubkey) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(NODE1_ADDRESS)
        .json(
            &serde_json::json!({
            "jsonrpc": "2.0",
            "id": "curlycurl",
            "method": GET_ACCOUNT_ADDRESS,
            "params": pubkey,
        })
        )
        .send().await?;

    let result: Value = response.json().await?;
    process_result(result.to_string()).map(|value|
        value.as_str().expect("cannot convert result to string").to_string()
    )
}

pub fn get_account_address(pubkey: Pubkey) -> String {
    process_result(post_data(NODE1_ADDRESS, GET_ACCOUNT_ADDRESS, pubkey))
        .expect("get_account_address should not fail")
        .as_str()
        .expect("cannot convert result to string")
        .to_string()
}

fn get_address_utxos(rpc: &Client, address: String) -> Vec<Value> {
    let client = reqwest::blocking::Client::new();

    let res = client
        .get(format!("https://mempool.dev.aws.archnetwork.xyz/api/address/{}/utxo", address))
        .header("Accept", "application/json")
        .send()
        .unwrap();

    let utxos = from_str::<Value>(&res.text().unwrap()).unwrap();

    utxos
        .as_array()
        .unwrap()
        .iter()
        .filter(|utxo| {
            utxo["status"]["block_height"].as_u64().unwrap() <= rpc.get_block_count().unwrap() - 100
        })
        .map(|utxo| utxo.clone())
        .collect()
}

pub fn start_boot_node(port: u16, arch_nodes: &str, bitcoin_rpc_info: &BitcoinRpcInfo) -> Child {
    std::env::set_var("RISC0_DEV_MODE", "1");

    let mut command = Command::new("cargo");
    command.current_dir(env::current_dir().unwrap().parent().unwrap());

    command.args([
        "run",
        "-p",
        "zkvm",
        "--",
        "--is-boot-node",
        "--arch-nodes",
        arch_nodes,
        "--rpc-bind-port",
        &port.to_string(),
        "--bitcoin-rpc-endpoint",
        &bitcoin_rpc_info.endpoint,
        "--bitcoin-rpc-port",
        &bitcoin_rpc_info.port.to_string(),
        "--bitcoin-rpc-username",
        &bitcoin_rpc_info.username,
        "--bitcoin-rpc-password",
        &bitcoin_rpc_info.password,
    ]);

    info!("Starting boot node on port {}", port);
    command.spawn().expect("Failed to start boot node process")
}

pub fn start_node(port: u16, bitcoin_rpc_info: &BitcoinRpcInfo) -> Child {
    env::set_var("RISC0_DEV_MODE", "1");

    let mut command = Command::new("cargo");
    command.current_dir(env::current_dir().unwrap().parent().unwrap());

    command.args([
        "run",
        "-p",
        "arch-node",
        "--",
        "--rpc-bind-port",
        &port.to_string(),
        "--bitcoin-rpc-endpoint",
        &bitcoin_rpc_info.endpoint,
        "--bitcoin-rpc-port",
        &bitcoin_rpc_info.port.to_string(),
        "--bitcoin-rpc-username",
        &bitcoin_rpc_info.username,
        "--bitcoin-rpc-password",
        &bitcoin_rpc_info.password,
        "--data-dir",
        &format!(".participant{}", port),
    ]);

    info!("Starting node on port {}", port);
    command.spawn().expect("Failed to start node process")
}

async fn stop_node(mut child: Child) {
    match child.kill() {
        Ok(_) => info!("Node stopped successfully"),
        Err(e) => error!("Failed to stop node: {}", e),
    }

    let _ = child.wait();
}
