use crate::base64::{base62_encode, base64_decode, base64_encode};
use crate::compactsize::{compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write};
use crate::tx::*;
use std::fmt;

// Common constant from common.h
pub const MAX_SERIALIZE_SIZE: u32 = 0x02000000;

// --- Enum definitions ---
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtResult {
    Ok,
    CompactReadError,
    ReadError,
    WriteError,
    InvalidState,
    NotImplemented,
    OobWrite,
}
impl fmt::Display for PsbtResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtGlobalType {
    UnsignedTx = 0,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtEncoding {
    Hex,
    Base64,
    Base62,
    Protobuf,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtInputType {
    NonWitnessUtxo = 0,
    WitnessUtxo = 1,
    PartialSig = 2,
    SighashType = 3,
    RedeemScript = 4,
    WitnessScript = 5,
    Bip32Derivation = 6,
    FinalScriptSig = 7,
    FinalScriptWitness = 8,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtOutputType {
    RedeemScript = 0,
    WitnessScript = 1,
    Bip32Derivation = 2,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PsbtScope {
    Global,
    Inputs,
    Outputs,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtState {
    Init = 2,
    Global,
    Inputs,
    InputsNew,
    Outputs,
    OutputsNew,
    Finalized,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtElemType {
    Record,
    TxElem,
}
#[derive(Debug, PartialEq, Eq)]
pub enum PsbtTxElemType {
    TxIn,
    TxOut,
    Tx,
    WitnessItem,
}
// --- Struct definitions ---
/// Translates the C struct psbt.
/// (Here we use a Vec<u8> to hold the PSBT data and a write position index.)
pub struct Psbt {
    pub state: PsbtState,
    pub data: Vec<u8>,
    pub write_pos: usize,
    pub data_capacity: usize,
    // For simulation purposes we keep a list of records.
    pub records: Vec<PsbtRecord>,
}
impl Psbt {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: PsbtState::Init,
            data: Vec::with_capacity(capacity),
            write_pos: 0,
            data_capacity: capacity,
            records: Vec::new(),
        }
    }
}
/// Translates the C struct psbt_record.
pub struct PsbtRecord {
    /// (The C field "type" is renamed to avoid conflict with the Rust keyword.)
    pub record_type: u8,
    pub key: Vec<u8>,
    pub val: Vec<u8>,
    pub scope: PsbtScope,
}
/// Translates the C union (record/txelem) in psbt_elem into an enum.
pub enum PsbtElem {
    Record { index: i32, record: PsbtRecord },
    TxElem { index: i32, txelem: PsbtTxElem },
}
/// The C typedef for a handler function.
pub type PsbtElemHandler = fn(elem: &mut PsbtElem, user_data: &mut dyn std::any::Any);
// External constants
pub const PSBT_MAGIC: [u8; 4] = [0x70, 0x73, 0x62, 0x74]; // "psbt"
pub static PSBT_ERRMSG: &str = "psbt error";

/// Return the number of bytes stored in the PSBT.
pub fn psbt_size(tx: &Psbt) -> usize {
    tx.data.len()
}

// ----------------- Internal helpers for writing -----------------

fn psbt_append(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if psbt.data.len() + bytes.len() > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.extend_from_slice(bytes);
    psbt.write_pos = psbt.data.len();
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let res = psbt_append(psbt, &PSBT_MAGIC);
    if res != PsbtResult::Ok {
        return res;
    }
    let res = psbt_append(psbt, &[0xff]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    psbt_append(psbt, &[0x00])
}

fn psbt_write_record_bytes(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type: u32 = (rec.key.len() as u32).wrapping_add(1);
    // write key length
    let size = compactsize_length(key_size_with_type as u64) as usize;
    let mut tmp = [0u8; 9];
    compactsize_write(&mut tmp, key_size_with_type as u64);
    let res = psbt_append(psbt, &tmp[..size]);
    if res != PsbtResult::Ok {
        return res;
    }
    // write type
    let res = psbt_append(psbt, &[rec.record_type]);
    if res != PsbtResult::Ok {
        return res;
    }
    // write key
    let res = psbt_append(psbt, &rec.key);
    if res != PsbtResult::Ok {
        return res;
    }
    // write value length
    let val_size = rec.val.len() as u64;
    let size = compactsize_length(val_size) as usize;
    let mut tmp = [0u8; 9];
    compactsize_write(&mut tmp, val_size);
    let res = psbt_append(psbt, &tmp[..size]);
    if res != PsbtResult::Ok {
        return res;
    }
    // write value
    let res = psbt_append(psbt, &rec.val);
    if res != PsbtResult::Ok {
        return res;
    }
    PsbtResult::Ok
}

// ----------------- Public functions -----------------

/// For testing, we simulate reading by optionally calling the provided callback twice.
pub fn psbt_read(
    src: &[u8],
    src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    if psbt.state != PsbtState::Init {
        return PsbtResult::InvalidState;
    }
    if src_size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    // Copy the source into data, then pad with zeros up to data_capacity
    // (matches the C behavior of reading from a zero-initialized buffer beyond src_size)
    let cap = psbt.data_capacity;
    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    if psbt.data.len() < cap {
        psbt.data.resize(cap, 0);
    }
    // For asserts to work properly (matches C: tx->data_capacity = src_size)
    psbt.data_capacity = src_size;

    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    psbt.records.clear();

    let mut kvs: i32 = 0;
    let mut inputs: i32 = 0;
    let mut outputs: i32 = 0;

    // We use a local cursor variable for parsing.
    // After parsing we set psbt.write_pos = cursor at end.
    let end = src_size;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= end {
        match psbt.state {
            PsbtState::Init => {
                // psbt_read_header
                if psbt.write_pos + 4 > end {
                    return PsbtResult::OobWrite;
                }
                if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 4;
                if psbt.write_pos + 1 > end {
                    return PsbtResult::OobWrite;
                }
                if psbt.data[psbt.write_pos] != 0xff {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Global;
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                if psbt.write_pos > end || psbt.write_pos >= psbt.data.len() {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos] == 0 {
                    match psbt.state {
                        PsbtState::Global => {
                            psbt.state = PsbtState::InputsNew;
                        }
                        PsbtState::Inputs => {
                            kvs += 1;
                            if kvs >= inputs {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= outputs {
                                psbt.state = PsbtState::Finalized;
                            } else {
                                psbt.state = PsbtState::OutputsNew;
                            }
                        }
                        _ => {
                            return PsbtResult::InvalidState;
                        }
                    }
                } else {
                    // psbt_read_record
                    let rec_result = psbt_read_record(psbt, src_size, kvs);
                    match rec_result {
                        Ok((rec, idx)) => {
                            // After reading, if it's a global unsigned tx, parse it for input/output count
                            if psbt.state == PsbtState::Global
                                && rec.record_type == PsbtGlobalType::UnsignedTx as u8
                            {
                                let mut counter = TxCounter {
                                    inputs: 0,
                                    outputs: 0,
                                };
                                let res = psbt_btc_tx_parse(
                                    &rec.val,
                                    rec.val.len(),
                                    &mut counter,
                                    Some(tx_counter_handler),
                                );
                                if res != PsbtResult::Ok {
                                    return res;
                                }
                                inputs = counter.inputs;
                                outputs = counter.outputs;
                            }

                            // record callback
                            if let Some(h) = elem_handler {
                                let cloned = PsbtRecord {
                                    record_type: rec.record_type,
                                    key: rec.key.clone(),
                                    val: rec.val.clone(),
                                    scope: rec.scope.clone(),
                                };
                                let mut elem = PsbtElem::Record {
                                    index: idx,
                                    record: cloned,
                                };
                                h(&mut elem, user_data);
                            }
                            psbt.records.push(rec);
                        }
                        Err(e) => return e,
                    }
                }
            }
            PsbtState::OutputsNew => {
                if psbt.write_pos >= end {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos] != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                if psbt.write_pos >= end {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos] != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => {
                // unreachable in C
                break;
            }
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    // The C code checks *write_pos against 0, relying on zero-padded buffer.
    // We've zero-padded data up to data.len() above, so just check the byte if in range.
    if psbt.write_pos >= psbt.data.len() {
        return PsbtResult::ReadError;
    }
    if psbt.data[psbt.write_pos] != 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    PsbtResult::Ok
}

struct TxCounter {
    inputs: i32,
    outputs: i32,
}

fn tx_counter_handler(elem: &mut PsbtTxElem, user_data: &mut dyn std::any::Any) {
    let counter = user_data.downcast_mut::<TxCounter>().unwrap();
    match elem {
        PsbtTxElem::TxIn(_) => counter.inputs += 1,
        PsbtTxElem::TxOut(_) => counter.outputs += 1,
        _ => {}
    }
}

fn psbt_read_record(
    psbt: &mut Psbt,
    src_size: usize,
    kvs: i32,
) -> Result<(PsbtRecord, i32), PsbtResult> {
    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::ReadError);
    }

    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > src_size {
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    if size == 0 {
        return Err(PsbtResult::ReadError);
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + size as usize > src_size {
        return Err(PsbtResult::ReadError);
    }
    if psbt.write_pos + size as usize > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }

    let key_size = (size - 1) as usize;
    let record_type = psbt.data[psbt.write_pos];
    let key_start = psbt.write_pos + 1;
    let key = psbt.data[key_start..key_start + key_size].to_vec();
    psbt.write_pos += size as usize;

    let scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return Err(PsbtResult::InvalidState),
    };

    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > src_size {
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;
    if psbt.write_pos + size as usize > src_size {
        return Err(PsbtResult::ReadError);
    }
    let val = psbt.data[psbt.write_pos..psbt.write_pos + size as usize].to_vec();
    psbt.write_pos += size as usize;

    Ok((
        PsbtRecord {
            record_type,
            key,
            val,
            scope,
        },
        kvs,
    ))
}

/// Decode a hex/base64 string into dest. Returns the decoded length in `psbt_len`.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let b64_magic = b"cHNid";
    let b64_magic_size = b64_magic.len();

    if src_size < b64_magic_size {
        return PsbtResult::ReadError;
    }

    let src_bytes = src.as_bytes();
    if &src_bytes[..b64_magic_size] == b64_magic {
        match base64_decode(&src_bytes[..src_size], &mut dest[..dest_size]) {
            Some(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        if src_size % 2 != 0 {
            return PsbtResult::ReadError;
        }
        if dest_size < src_size / 2 {
            return PsbtResult::ReadError;
        }
        for i in 0..src_size / 2 {
            let c1 = src_bytes[i * 2];
            let c2 = src_bytes[i * 2 + 1];
            if !is_hex_digit(c1) || !is_hex_digit(c2) {
                return PsbtResult::ReadError;
            }
            dest[i] = (hex_digit(c1) << 4) | hex_digit(c2);
        }
        *psbt_len = src_size / 2;
        PsbtResult::Ok
    }
}

fn is_hex_digit(c: u8) -> bool {
    (c.is_ascii_digit()) || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

fn hex_digit(c: u8) -> u8 {
    if c <= b'9' {
        c - b'0'
    } else {
        c.to_ascii_uppercase() - b'A' + 10
    }
}

fn hex_encode(buf: &[u8], dest: &mut [u8]) -> PsbtResult {
    if dest.len() < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    let hex_chars = b"0123456789abcdef";
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hex_chars[(b >> 4) as usize];
        dest[i * 2 + 1] = hex_chars[(b & 0xf) as usize];
    }
    dest[buf.len() * 2] = 0;
    PsbtResult::Ok
}

/// Encode the PSBT data into a destination buffer using the requested encoding.
pub fn psbt_encode(
    psbt: &Psbt,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        return PsbtResult::WriteError;
    }
    psbt_encode_raw(&psbt.data, psbt.data.len(), encoding, dest, dest_size, out_len)
}

/// Encode raw PSBT data into dest using the requested encoding.
pub fn psbt_encode_raw(
    psbt_data: &[u8],
    psbt_len: usize,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(&psbt_data[..psbt_len], &mut dest[..dest_size]);
            *out_len = psbt_len * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => match base64_encode(&psbt_data[..psbt_len], &mut dest[..dest_size])
        {
            Some(n) => {
                *out_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::WriteError,
        },
        PsbtEncoding::Base62 => match base62_encode(&psbt_data[..psbt_len], &mut dest[..dest_size])
        {
            Some(n) => {
                *out_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::WriteError,
        },
        PsbtEncoding::Protobuf => PsbtResult::NotImplemented,
    }
}

/// Return the last error message.
pub fn psbt_geterr() -> &'static str {
    PSBT_ERRMSG
}

/// Convert a PSBT state to a human-readable string.
pub fn psbt_state_tostr(state: PsbtState) -> &'static str {
    match state {
        PsbtState::Init => "INIT",
        PsbtState::Global => "GLOBAL",
        PsbtState::Inputs => "INPUTS",
        PsbtState::InputsNew => "INPUTS_NEW",
        PsbtState::Outputs => "OUTPUTS",
        PsbtState::OutputsNew => "OUTPUTS_NEW",
        PsbtState::Finalized => "FINALIZED",
    }
}

/// Return a string for a record type and scope.
pub fn psbt_type_tostr(record_type: u8, scope: PsbtScope) -> &'static str {
    match scope {
        PsbtScope::Global => match record_type {
            0 => psbt_global_type_tostr(PsbtGlobalType::UnsignedTx),
            _ => "UNKNOWN_GLOBAL_TYPE",
        },
        PsbtScope::Inputs => match record_type {
            0 => psbt_input_type_tostr(PsbtInputType::NonWitnessUtxo),
            1 => psbt_input_type_tostr(PsbtInputType::WitnessUtxo),
            2 => psbt_input_type_tostr(PsbtInputType::PartialSig),
            3 => psbt_input_type_tostr(PsbtInputType::SighashType),
            4 => psbt_input_type_tostr(PsbtInputType::RedeemScript),
            5 => psbt_input_type_tostr(PsbtInputType::WitnessScript),
            6 => psbt_input_type_tostr(PsbtInputType::Bip32Derivation),
            7 => psbt_input_type_tostr(PsbtInputType::FinalScriptSig),
            8 => psbt_input_type_tostr(PsbtInputType::FinalScriptWitness),
            _ => "UNKNOWN_INPUT_TYPE",
        },
        PsbtScope::Outputs => match record_type {
            0 => psbt_output_type_tostr(PsbtOutputType::RedeemScript),
            1 => psbt_output_type_tostr(PsbtOutputType::WitnessScript),
            2 => psbt_output_type_tostr(PsbtOutputType::Bip32Derivation),
            _ => "UNKNOWN_OUTPUT_TYPE",
        },
    }
}

/// Return a string for a psbt_txelem type.
pub fn psbt_txelem_type_tostr(txelem_type: PsbtTxElemType) -> &'static str {
    match txelem_type {
        PsbtTxElemType::Tx => "TX",
        PsbtTxElemType::TxIn => "TXIN",
        PsbtTxElemType::TxOut => "TXOUT",
        PsbtTxElemType::WitnessItem => "WITNESS_ITEM",
    }
}

pub fn psbt_global_type_tostr(gt: PsbtGlobalType) -> &'static str {
    match gt {
        PsbtGlobalType::UnsignedTx => "GLOBAL_UNSIGNED_TX",
    }
}

pub fn psbt_output_type_tostr(ot: PsbtOutputType) -> &'static str {
    match ot {
        PsbtOutputType::RedeemScript => "OUT_REDEEM_SCRIPT",
        PsbtOutputType::WitnessScript => "OUT_WITNESS_SCRIPT",
        PsbtOutputType::Bip32Derivation => "OUT_BIP32_DERIVATION",
    }
}

pub fn psbt_input_type_tostr(it: PsbtInputType) -> &'static str {
    match it {
        PsbtInputType::NonWitnessUtxo => "IN_NON_WITNESS_UTXO",
        PsbtInputType::WitnessUtxo => "IN_WITNESS_UTXO",
        PsbtInputType::PartialSig => "IN_PARTIAL_SIG",
        PsbtInputType::SighashType => "IN_SIGHASH_TYPE",
        PsbtInputType::RedeemScript => "IN_REDEEM_SCRIPT",
        PsbtInputType::WitnessScript => "IN_WITNESS_SCRIPT",
        PsbtInputType::Bip32Derivation => "IN_BIP32_DERIVATION",
        PsbtInputType::FinalScriptSig => "IN_FINAL_SCRIPTSIG",
        PsbtInputType::FinalScriptWitness => "IN_FINAL_SCRIPTWITNESS",
    }
}

/// Write a global record into the PSBT.
pub fn psbt_write_global_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Init {
        let res = psbt_write_header(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::Global;
    } else if psbt.state != PsbtState::Global {
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record_bytes(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Global,
        });
    }
    res
}

/// Write an input record into the PSBT.
pub fn psbt_write_input_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Global {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::Inputs;
    } else if psbt.state != PsbtState::Inputs && psbt.state != PsbtState::InputsNew {
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record_bytes(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Inputs,
        });
    }
    res
}

/// Write an output record into the PSBT.
pub fn psbt_write_output_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Inputs {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::Outputs;
    } else if psbt.state != PsbtState::Outputs && psbt.state != PsbtState::OutputsNew {
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record_bytes(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Outputs,
        });
    }
    res
}

/// Create a new input record set.
pub fn psbt_new_input_record_set(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state == PsbtState::Global
        || psbt.state == PsbtState::InputsNew
        || psbt.state == PsbtState::Inputs
    {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::InputsNew;
        return PsbtResult::Ok;
    } else if psbt.state != PsbtState::Inputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

/// Create a new output record set.
pub fn psbt_new_output_record_set(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state == PsbtState::Inputs
        || psbt.state == PsbtState::InputsNew
        || psbt.state == PsbtState::OutputsNew
        || psbt.state == PsbtState::Outputs
    {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::OutputsNew;
        return PsbtResult::Ok;
    } else if psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

/// Initialize a PSBT using the given destination buffer.
pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data.clear();
    psbt.data.reserve(dest_size);
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    psbt.records.clear();
    PsbtResult::Ok
}

/// Print the PSBT (only succeeds after finalization).
pub fn psbt_print(psbt: &Psbt, stream: &mut dyn std::io::Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    for &b in psbt.data.iter() {
        if write!(stream, "{:02x}", b).is_err() {
            return PsbtResult::WriteError;
        }
    }
    if writeln!(stream).is_err() {
        return PsbtResult::WriteError;
    }
    PsbtResult::Ok
}

/// Finalize the PSBT.
pub fn psbt_finalize(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state != PsbtState::OutputsNew && psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    let res = psbt_close_records(psbt);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}
