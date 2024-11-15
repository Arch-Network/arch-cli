use bitcoin::{
    absolute::LockTime,
    hashes::{sha256, Hash},
    key::{Keypair, Secp256k1, TapTweak, UntweakedKeypair},
    opcodes,
    script::{self, PushBytes},
    secp256k1::SecretKey,
    sighash::{self, SighashCache},
    transaction::Version,
    Address, Amount, OutPoint, PrivateKey, Psbt, ScriptBuf, Sequence, TapSighashType, Transaction,
    TxIn, TxOut, Witness, XOnlyPublicKey,
};
use snafu::ResultExt;

mod error;

pub fn sign_message_bip322(
    keypair: &UntweakedKeypair,
    msg: &[u8],
    network: bitcoin::Network,
) -> [u8; 64] {
    let secp = Secp256k1::new();
    let xpubk = XOnlyPublicKey::from_keypair(keypair).0;
    let private_key = PrivateKey::new(SecretKey::from_keypair(keypair), network);

    let address = Address::p2tr(&secp, xpubk, None, network);

    let to_spend = create_to_spend(&address, msg).unwrap();
    let mut to_sign = create_to_sign(&to_spend, None).unwrap();

    let witness = match address.witness_program() {
        Some(witness_program) => {
            let version = witness_program.version().to_num();
            let program_len = witness_program.program().len();

            match version {
                1 => {
                    if program_len != 32 {
                        panic!("not key spend path");
                    }
                    create_message_signature_taproot(&to_spend, &to_sign, private_key)
                }
                _ => {
                    panic!("unsuported address");
                }
            }
        }
        None => {
            panic!("unsuported address");
        }
    };

    to_sign.inputs[0].final_script_witness = Some(witness);

    let signature = to_sign.extract_tx().unwrap().input[0].witness.clone();

    signature.to_vec()[0][..64].try_into().unwrap()
}

pub fn verify_message_bip322(
    msg: &[u8],
    pubkey: [u8; 32],
    signature: [u8; 64],
    uses_sighash_all: bool,
    network: bitcoin::Network,
) -> BIP322Result<()> {
    let mut signature = signature.to_vec();
    if uses_sighash_all {
        signature.push(1);
    }
    let mut witness = Witness::new();
    witness.push(&signature);

    let secp = Secp256k1::new();
    let xpubk = XOnlyPublicKey::from_slice(&pubkey).unwrap();
    let address = Address::p2tr(&secp, xpubk, None, network);

    verify_simple(&address, msg, witness)
}

fn create_message_signature_taproot(
    to_spend_tx: &Transaction,
    to_sign: &Psbt,
    private_key: PrivateKey,
) -> Witness {
    let mut to_sign = to_sign.clone();

    let secp = Secp256k1::new();
    let key_pair = Keypair::from_secret_key(&secp, &private_key.inner);

    let (x_only_public_key, _parity) = XOnlyPublicKey::from_keypair(&key_pair);
    to_sign.inputs[0].tap_internal_key = Some(x_only_public_key);

    let sighash_type = TapSighashType::All;

    let mut sighash_cache = SighashCache::new(to_sign.unsigned_tx.clone());

    let sighash = sighash_cache
        .taproot_key_spend_signature_hash(
            0,
            &sighash::Prevouts::All(&[TxOut {
                value: Amount::from_sat(0),
                script_pubkey: to_spend_tx.output[0].clone().script_pubkey,
            }]),
            sighash_type,
        )
        .expect("signature hash should compute");

    let key_pair = key_pair
        .tap_tweak(&secp, to_sign.inputs[0].tap_merkle_root)
        .to_inner();

    let sig = secp.sign_schnorr(
        &bitcoin::secp256k1::Message::from_digest_slice(sighash.as_ref())
            .expect("should be cryptographically secure hash"),
        &key_pair,
    );

    let witness = sighash_cache
        .witness_mut(0)
        .expect("getting mutable witness reference should work");

    witness.push(
        bitcoin::taproot::Signature {
            signature: sig,
            sighash_type,
        }
        .to_vec(),
    );

    witness.to_owned()
}

type BIP322Result<T = (), E = error::Error> = std::result::Result<T, E>;

const TAG: &str = "BIP0322-signed-message";

/// Create the tagged message hash.
pub fn message_hash(message: &[u8]) -> Vec<u8> {
    let mut tag_hash = sha256::Hash::hash(TAG.as_bytes()).to_byte_array().to_vec();
    tag_hash.extend(tag_hash.clone());
    tag_hash.extend(message);

    sha256::Hash::hash(tag_hash.as_slice())
        .to_byte_array()
        .to_vec()
}

/// Create the `to_spend` transaction.
pub fn create_to_spend(address: &Address, message: &[u8]) -> BIP322Result<Transaction> {
    Ok(Transaction {
        version: Version(0),
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: "0000000000000000000000000000000000000000000000000000000000000000"
                    .parse()
                    .unwrap(),
                vout: 0xFFFFFFFF,
            },
            script_sig: script::Builder::new()
                .push_int(0)
                .push_slice::<&PushBytes>(message_hash(message).as_slice().try_into().unwrap())
                .into_script(),
            sequence: Sequence(0),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(0),
            script_pubkey: address.script_pubkey(),
        }],
    })
}

/// Create the `to_sign` transaction.
pub fn create_to_sign(to_spend: &Transaction, witness: Option<Witness>) -> BIP322Result<Psbt> {
    let inputs = vec![TxIn {
        previous_output: OutPoint {
            txid: to_spend.compute_txid(),
            vout: 0,
        },
        script_sig: ScriptBuf::new(),
        sequence: Sequence(0),
        witness: Witness::new(),
    }];

    let to_sign = Transaction {
        version: Version(0),
        lock_time: LockTime::ZERO,
        input: inputs,
        output: vec![TxOut {
            value: Amount::from_sat(0),
            script_pubkey: script::Builder::new()
                .push_opcode(opcodes::all::OP_RETURN)
                .into_script(),
        }],
    };

    let mut psbt = Psbt::from_unsigned_tx(to_sign).context(error::PsbtExtract)?;

    psbt.inputs[0].witness_utxo = Some(TxOut {
        value: Amount::from_sat(0),
        script_pubkey: to_spend.output[0].script_pubkey.clone(),
    });

    psbt.inputs[0].final_script_witness = witness;

    Ok(psbt)
}

/// Verifies the BIP-322 simple from proper Rust types.
pub fn verify_simple(address: &Address, message: &[u8], signature: Witness) -> BIP322Result<()> {
    verify_full(
        address,
        message,
        create_to_sign(&create_to_spend(address, message)?, Some(signature))?
            .extract_tx()
            .context(error::TransactionExtract)?,
    )
}

/// Verifies the BIP-322 full from proper Rust types.
pub fn verify_full(address: &Address, message: &[u8], to_sign: Transaction) -> BIP322Result<()> {
    match address.witness_program() {
        Some(witness) => {
            if witness.version().to_num() == 1 && witness.program().len() == 32 {
                let pub_key = XOnlyPublicKey::from_slice(witness.program().as_bytes())
                    .map_err(|_| error::Error::InvalidPublicKey)?;

                verify_full_p2tr(address, message, to_sign, pub_key)
            } else {
                Err(error::Error::UnsupportedAddress {
                    address: address.to_string(),
                })
            }
        }
        None => Err(error::Error::UnsupportedAddress {
            address: address.to_string(),
        }),
    }
}

fn verify_full_p2tr(
    address: &Address,
    message: &[u8],
    to_sign: Transaction,
    pub_key: XOnlyPublicKey,
) -> BIP322Result<()> {
    use bitcoin::secp256k1::{schnorr::Signature, Message};

    let to_spend = create_to_spend(address, message)?;
    let to_sign = create_to_sign(&to_spend, Some(to_sign.input[0].witness.clone()))?;

    let to_spend_outpoint = OutPoint {
        txid: to_spend.compute_txid(),
        vout: 0,
    };

    if to_spend_outpoint != to_sign.unsigned_tx.input[0].previous_output {
        return Err(error::Error::ToSignInvalid);
    }

    let Some(witness) = to_sign.inputs[0].final_script_witness.clone() else {
        return Err(error::Error::WitnessEmpty);
    };

    let encoded_signature = witness.to_vec()[0].clone();

    let (signature, sighash_type) = match encoded_signature.len() {
        65 => (
            Signature::from_slice(&encoded_signature.as_slice()[..64])
                .context(error::SignatureInvalid)?,
            TapSighashType::from_consensus_u8(encoded_signature[64])
                .context(error::SigHashTypeInvalid)?,
        ),
        64 => (
            Signature::from_slice(encoded_signature.as_slice()).context(error::SignatureInvalid)?,
            TapSighashType::Default,
        ),
        _ => {
            return Err(error::Error::SignatureLength {
                length: encoded_signature.len(),
                encoded_signature,
            })
        }
    };

    if !(sighash_type == TapSighashType::All || sighash_type == TapSighashType::Default) {
        return Err(error::Error::SigHashTypeUnsupported {
            sighash_type: sighash_type.to_string(),
        });
    }

    let mut sighash_cache = SighashCache::new(to_sign.unsigned_tx);

    let sighash = sighash_cache
        .taproot_key_spend_signature_hash(
            0,
            &sighash::Prevouts::All(&[TxOut {
                value: Amount::from_sat(0),
                script_pubkey: to_spend.output[0].clone().script_pubkey,
            }]),
            sighash_type,
        )
        .expect("signature hash should compute");

    let message = Message::from_digest_slice(sighash.as_ref())
        .expect("should be cryptographically secure hash");

    Secp256k1::verification_only()
        .verify_schnorr(&signature, &message, &pub_key)
        .context(error::SignatureInvalid)
}
