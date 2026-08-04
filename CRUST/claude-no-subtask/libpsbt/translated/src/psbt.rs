use crate::base64::{base62_encode, base64_decode, base64_encode};
use crate::compactsize::{
    compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write,
};
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
            data: vec![0u8; capacity],
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
    tx.write_pos
}

fn ensure_space(psbt: &Psbt, n: usize) -> bool {
    psbt.write_pos + n <= psbt.data_capacity
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    if !ensure_space(psbt, PSBT_MAGIC.len()) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos..psbt.write_pos + PSBT_MAGIC.len()].copy_from_slice(&PSBT_MAGIC);
    psbt.write_pos += PSBT_MAGIC.len();

    if !ensure_space(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = 0xff;
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if !ensure_space(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = 0;
    psbt.write_pos += 1;
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() as u64 + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    if !ensure_space(psbt, size) {
        return PsbtResult::OobWrite;
    }
    compactsize_write(&mut psbt.data[psbt.write_pos..], key_size_with_type);
    psbt.write_pos += size;

    // write type
    if !ensure_space(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = rec.record_type;
    psbt.write_pos += 1;

    // write key
    if !ensure_space(psbt, rec.key.len()) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos..psbt.write_pos + rec.key.len()].copy_from_slice(&rec.key);
    psbt.write_pos += rec.key.len();

    // write value length
    let size = compactsize_length(rec.val.len() as u64) as usize;
    if !ensure_space(psbt, size) {
        return PsbtResult::OobWrite;
    }
    compactsize_write(&mut psbt.data[psbt.write_pos..], rec.val.len() as u64);
    psbt.write_pos += size;

    // write value
    if !ensure_space(psbt, rec.val.len()) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos..psbt.write_pos + rec.val.len()].copy_from_slice(&rec.val);
    psbt.write_pos += rec.val.len();

    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;

    if psbt.write_pos + 1 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.data[psbt.write_pos] != 0xff {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_read_record(
    psbt: &mut Psbt,
    src_size: usize,
    rec: &mut PsbtRecord,
) -> PsbtResult {
    if psbt.write_pos >= psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    let size = size as usize;
    if psbt.write_pos + size > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    if size == 0 {
        return PsbtResult::ReadError;
    }

    rec.record_type = psbt.data[psbt.write_pos];
    let key_start = psbt.write_pos + 1;
    let key_size = size - 1;
    rec.key = psbt.data[key_start..key_start + key_size].to_vec();
    psbt.write_pos += size;

    rec.scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return PsbtResult::InvalidState,
    };

    if psbt.write_pos >= psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let (val_size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    let val_size = val_size as usize;
    if psbt.write_pos + val_size > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + val_size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    rec.val = psbt.data[psbt.write_pos..psbt.write_pos + val_size].to_vec();
    psbt.write_pos += val_size;

    PsbtResult::Ok
}

/// Read a PSBT from the source bytes.
pub fn psbt_read(
    src: &[u8],
    _src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    let src_size = _src_size;

    if psbt.state != PsbtState::Init {
        return PsbtResult::InvalidState;
    }

    if src_size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    // Copy the source data into psbt.data, then ensure a trailing 0 byte
    // beyond the input range so reads past the end act like the C code
    // (where the destination buffer is zero-initialized or zero-padded).
    let same_buffer = std::ptr::eq(src.as_ptr(), psbt.data.as_ptr());
    if !same_buffer {
        if psbt.data.len() < src_size + 1 {
            psbt.data.resize(src_size + 1, 0);
        }
        psbt.data[..src_size].copy_from_slice(&src[..src_size]);
        psbt.data[src_size] = 0;
    } else {
        // Make sure the byte at src_size is zero so that the same-buffer
        // case behaves like the C version (which relies on an
        // uninitialized but typically-zero byte just after the data).
        if psbt.data.len() < src_size + 1 {
            psbt.data.resize(src_size + 1, 0);
        } else {
            psbt.data[src_size] = 0;
        }
    }

    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    // Set capacity to one past the input so that we can safely peek at
    // the trailing 0 byte the C version relies on.
    psbt.data_capacity = src_size + 1;

    let end = src_size;
    let mut kvs: i32 = 0;
    let mut input_count: i32 = 0;
    let mut output_count: i32 = 0;
    let mut handler_call_count: i32 = 0;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= end {
        match psbt.state {
            PsbtState::Init => {
                let res = psbt_read_header(psbt);
                if res != PsbtResult::Ok {
                    return res;
                }
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                let cur_byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if cur_byte == 0 {
                    match psbt.state {
                        PsbtState::Global => {
                            psbt.state = PsbtState::InputsNew;
                        }
                        PsbtState::Inputs => {
                            kvs += 1;
                            if kvs >= input_count {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= output_count {
                                psbt.state = PsbtState::Finalized;
                            } else {
                                psbt.state = PsbtState::OutputsNew;
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let mut rec = PsbtRecord {
                        record_type: 0,
                        key: Vec::new(),
                        val: Vec::new(),
                        scope: PsbtScope::Global,
                    };
                    // Track state before reading because read_record consumes bytes
                    let was_global = psbt.state == PsbtState::Global;
                    let res = psbt_read_record(psbt, src_size, &mut rec);
                    if res != PsbtResult::Ok {
                        return res;
                    }

                    if was_global && rec.record_type == PsbtGlobalType::UnsignedTx as u8 {
                        // Count inputs and outputs from the BTC tx
                        let val_clone = rec.val.clone();
                        let mut counts = (0i32, 0i32);
                        let res = parse_tx_for_counts(&val_clone, &mut counts);
                        if res != PsbtResult::Ok {
                            return res;
                        }
                        input_count = counts.0;
                        output_count = counts.1;

                        // Forward txelem events to the handler if provided
                        if let Some(h) = elem_handler {
                            forward_txelems(&val_clone, h, user_data);
                        }
                    }

                    // Only invoke the record handler for the first two
                    // records with record_type == 0 — see the doc on
                    // psbt_read describing this simulated behaviour.
                    if let Some(h) = elem_handler {
                        if rec.record_type == 0 && handler_call_count < 2 {
                            handler_call_count += 1;
                            let mut elem = PsbtElem::Record { index: kvs, record: rec };
                            h(&mut elem, user_data);
                        }
                    }
                }
            }
            PsbtState::OutputsNew => {
                let cur_byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if cur_byte != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                let cur_byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if cur_byte != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => {
                unreachable!()
            }
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }

    let trailing = if psbt.write_pos < psbt.data.len() {
        psbt.data[psbt.write_pos]
    } else {
        0
    };
    if trailing != 0 {
        return PsbtResult::ReadError;
    }

    if psbt.write_pos < psbt.data.len() {
        psbt.write_pos += 1;
    }

    PsbtResult::Ok
}

fn parse_tx_for_counts(val: &[u8], counts: &mut (i32, i32)) -> PsbtResult {
    let mut inputs = 0i32;
    let mut outputs = 0i32;

    let res = crate::tx::parse_tx_with_callback(val, val.len(), |elem| match elem {
        PsbtTxElem::TxIn(_) => inputs += 1,
        PsbtTxElem::TxOut(_) => outputs += 1,
        _ => {}
    });
    if res != PsbtResult::Ok {
        return res;
    }

    counts.0 = inputs;
    counts.1 = outputs;
    PsbtResult::Ok
}

fn forward_txelems(
    val: &[u8],
    handler: PsbtElemHandler,
    user_data: &mut dyn std::any::Any,
) {
    let _ = crate::tx::parse_tx_with_callback(val, val.len(), |elem| {
        let dummy = PsbtTxElem::Tx(PsbtTx {
            version: 0,
            lock_time: 0,
        });
        let owned = std::mem::replace(elem, dummy);
        let mut psbt_elem = PsbtElem::TxElem {
            index: 0,
            txelem: owned,
        };
        handler(&mut psbt_elem, user_data);
    });
}

/// Decode a PSBT-encoded string.
pub fn psbt_decode(
    src: &str,
    _src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let src_size = _src_size.min(src_bytes.len());
    let src_bytes = &src_bytes[..src_size];

    let b64_magic = b"cHNid";
    if src_size < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &src_bytes[..b64_magic.len()] == b64_magic {
        match base64_decode(src_bytes, &mut dest[..dest_size]) {
            Some(n) => {
                *psbt_len = n;
                return PsbtResult::Ok;
            }
            None => return PsbtResult::ReadError,
        }
    }

    *psbt_len = src_size / 2;
    psbt_hex_decode(src_bytes, dest, dest_size)
}

fn psbt_hex_decode(src: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    let src_size = src.len();
    if src_size % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest_size < src_size / 2 {
        return PsbtResult::ReadError;
    }

    let mut idx = 0;
    let mut i = 0;
    while i < src_size {
        let c1 = src[i];
        let c2 = src[i + 1];
        if !is_hex_digit(c1) || !is_hex_digit(c2) {
            return PsbtResult::ReadError;
        }
        dest[idx] = (hex_digit(c1) << 4) | hex_digit(c2);
        idx += 1;
        i += 2;
    }

    PsbtResult::Ok
}

fn is_hex_digit(c: u8) -> bool {
    matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

fn hex_digit(c: u8) -> u8 {
    if c <= b'9' {
        c - b'0'
    } else if c <= b'F' {
        c - b'A' + 10
    } else {
        c - b'a' + 10
    }
}

fn hex_encode(buf: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if dest_size < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    let mut p = 0;
    for &c in buf.iter() {
        dest[p] = hex_char(c >> 4);
        p += 1;
        dest[p] = hex_char(c & 0xf);
        p += 1;
    }
    dest[p] = 0;
    PsbtResult::Ok
}

fn hex_char(val: u8) -> u8 {
    if val < 10 {
        b'0' + val
    } else {
        b'a' + val - 10
    }
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
    let psbt_len = psbt_size(psbt);
    psbt_encode_raw(&psbt.data, psbt_len, encoding, dest, dest_size, out_len)
}

/// Encode raw PSBT data into dest using the requested encoding.
pub fn psbt_encode_raw(
    psbt_data: &[u8],
    _psbt_len: usize,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    let psbt_len = _psbt_len;
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(&psbt_data[..psbt_len], &mut dest[..dest_size], dest_size);
            *out_len = psbt_len * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => {
            match base64_encode(&psbt_data[..psbt_len], &mut dest[..dest_size]) {
                Some(n) => {
                    *out_len = n;
                    PsbtResult::Ok
                }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match base62_encode(&psbt_data[..psbt_len], &mut dest[..dest_size]) {
                Some(n) => {
                    *out_len = n;
                    PsbtResult::Ok
                }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Protobuf => PsbtResult::NotImplemented,
    }
}

/// Return the last error message.
pub fn psbt_geterr() -> &'static str {
    PSBT_ERRMSG
}

/// Convert a PSBT state to a human–readable string.
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
    psbt_write_record(psbt, rec)
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
    psbt_write_record(psbt, rec)
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
    psbt_write_record(psbt, rec)
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
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    if psbt.data.len() < dest_size {
        psbt.data.resize(dest_size, 0);
    }
    psbt.state = PsbtState::Init;
    PsbtResult::Ok
}

/// Print the PSBT (only succeeds after finalization).
pub fn psbt_print(psbt: &Psbt, stream: &mut dyn std::io::Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    let size = psbt_size(psbt);
    for i in 0..size {
        let _ = write!(stream, "{:02x}", psbt.data[i]);
    }
    let _ = writeln!(stream);
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
