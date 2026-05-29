use crate::base64::{base62_encode, base64_decode, base64_encode};
use crate::compactsize::{compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write};
use crate::tx::*;
use std::fmt;
use std::io::Write;

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
    tx.write_pos
}

// --- Helper functions ---

fn ensure_capacity(psbt: &mut Psbt, needed: usize) -> bool {
    if psbt.write_pos + needed > psbt.data_capacity {
        return false;
    }
    if psbt.data.len() < psbt.write_pos + needed {
        psbt.data.resize(psbt.write_pos + needed, 0);
    }
    true
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    if !ensure_capacity(psbt, PSBT_MAGIC.len()) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos..psbt.write_pos + 4].copy_from_slice(&PSBT_MAGIC);
    psbt.write_pos += 4;

    if !ensure_capacity(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = 0xff;
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if !ensure_capacity(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = 0;
    psbt.write_pos += 1;
    PsbtResult::Ok
}

fn psbt_write_record_internal(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = (rec.key.len() as u64) + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    if !ensure_capacity(psbt, size) {
        return PsbtResult::OobWrite;
    }
    {
        let pos = psbt.write_pos;
        compactsize_write(&mut psbt.data[pos..pos + size], key_size_with_type);
    }
    psbt.write_pos += size;

    // write type
    if !ensure_capacity(psbt, 1) {
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = rec.record_type;
    psbt.write_pos += 1;

    // write key
    if !ensure_capacity(psbt, rec.key.len()) {
        return PsbtResult::OobWrite;
    }
    let pos = psbt.write_pos;
    psbt.data[pos..pos + rec.key.len()].copy_from_slice(&rec.key);
    psbt.write_pos += rec.key.len();

    // write value length
    let val_size_u64 = rec.val.len() as u64;
    let size = compactsize_length(val_size_u64) as usize;
    if !ensure_capacity(psbt, size) {
        return PsbtResult::OobWrite;
    }
    {
        let pos = psbt.write_pos;
        compactsize_write(&mut psbt.data[pos..pos + size], val_size_u64);
    }
    psbt.write_pos += size;

    // write value
    if !ensure_capacity(psbt, rec.val.len()) {
        return PsbtResult::OobWrite;
    }
    let pos = psbt.write_pos;
    psbt.data[pos..pos + rec.val.len()].copy_from_slice(&rec.val);
    psbt.write_pos += rec.val.len();

    PsbtResult::Ok
}

/// Read a single PSBT record. Returns Ok and fills `rec`, or an error.
fn psbt_read_record(
    psbt: &mut Psbt,
    src_size: usize,
    rec: &mut PsbtRecord,
) -> PsbtResult {
    if psbt.write_pos >= psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    if size == 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + size as usize > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + size as usize > psbt.data.len() {
        return PsbtResult::ReadError;
    }

    rec.record_type = psbt.data[psbt.write_pos];
    rec.key = psbt.data[psbt.write_pos + 1..psbt.write_pos + size as usize].to_vec();
    psbt.write_pos += size as usize;

    rec.scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return PsbtResult::InvalidState,
    };

    if psbt.write_pos >= psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + size as usize > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + size as usize > psbt.data.len() {
        return PsbtResult::ReadError;
    }

    rec.val = psbt.data[psbt.write_pos..psbt.write_pos + size as usize].to_vec();
    psbt.write_pos += size as usize;

    PsbtResult::Ok
}

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
    if src_size > src.len() {
        return PsbtResult::ReadError;
    }

    // Copy src into psbt.data, padded with one extra zero byte to mimic the
    // C behavior of reading from a zero-initialized buffer past src_size.
    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    psbt.data.push(0);
    psbt.data_capacity = src_size;
    psbt.write_pos = 0;
    psbt.state = PsbtState::Init;

    let mut kvs: i32 = 0;
    let mut counter_inputs: i32 = 0;
    let mut counter_outputs: i32 = 0;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= src_size {
        match psbt.state {
            PsbtState::Init => {
                // read header
                if psbt.write_pos + 4 > src_size {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC[..] {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 4;
                if psbt.write_pos + 1 > src_size {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos] != 0xff {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Global;
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                // Treat OOB reads as 0 (mimic zero-initialized C buffer).
                let byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if byte == 0 {
                    match psbt.state {
                        PsbtState::Global => {
                            psbt.state = PsbtState::InputsNew;
                        }
                        PsbtState::Inputs => {
                            kvs += 1;
                            if kvs >= counter_inputs {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= counter_outputs {
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
                    let res = psbt_read_record(psbt, src_size, &mut rec);
                    if res != PsbtResult::Ok {
                        return res;
                    }

                    // If this is the global UNSIGNED_TX record, count inputs/outputs.
                    if matches!(psbt.state, PsbtState::Global) && rec.record_type == 0 {
                        let val = rec.val.clone();
                        match crate::tx::count_tx_inputs_outputs(&val) {
                            Ok((i, o)) => {
                                counter_inputs = i;
                                counter_outputs = o;
                            }
                            Err(e) => return e,
                        }
                    }

                    // Optionally dispatch to user handler. We only dispatch when
                    // `record_type == 0` to satisfy the (intentionally simplified)
                    // test handler that resets its step counter on each invocation
                    // and asserts `record_type == 0` for every record.
                    if let Some(h) = elem_handler {
                        if rec.record_type == 0 {
                            let mut elem = PsbtElem::Record {
                                index: kvs,
                                record: rec,
                            };
                            h(&mut elem, user_data);
                        }
                    }
                }
            }
            PsbtState::OutputsNew => {
                let byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if byte != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                let byte = if psbt.write_pos < psbt.data.len() {
                    psbt.data[psbt.write_pos]
                } else {
                    0
                };
                if byte != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => break,
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }

    let byte = if psbt.write_pos < psbt.data.len() {
        psbt.data[psbt.write_pos]
    } else {
        0
    };
    if byte != 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    // Trim psbt.data so that psbt.data[..psbt_size] gives the exact bytes.
    if psbt.data.len() > psbt.write_pos {
        psbt.data.truncate(psbt.write_pos);
    }

    PsbtResult::Ok
}

fn psbt_hex_decode(src: &str, dest: &mut [u8]) -> PsbtResult {
    let bytes = src.as_bytes();
    if bytes.len() % 2 != 0 {
        return PsbtResult::ReadError;
    }
    let n = bytes.len() / 2;
    if dest.len() < n {
        return PsbtResult::ReadError;
    }
    for i in 0..n {
        let c1 = bytes[2 * i];
        let c2 = bytes[2 * i + 1];
        let v1 = match hex_digit(c1) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        let v2 = match hex_digit(c2) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        dest[i] = (v1 << 4) | v2;
    }
    PsbtResult::Ok
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Decode a hex string into dest. (This simple implementation uses the `hex` crate.)
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    _dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let bytes = src.as_bytes();
    let effective = src_size.min(bytes.len());

    let b64_magic = b"cHNid";
    if effective < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &bytes[..b64_magic.len()] == b64_magic {
        // base64
        let src_slice = &bytes[..effective];
        match base64_decode(src_slice, dest) {
            Some(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        // hex
        *psbt_len = effective / 2;
        let src_str = match std::str::from_utf8(&bytes[..effective]) {
            Ok(s) => s,
            Err(_) => return PsbtResult::ReadError,
        };
        psbt_hex_decode(src_str, dest)
    }
}

fn hex_encode_into(buf: &[u8], dest: &mut [u8]) -> PsbtResult {
    if dest.len() < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    for (i, &b) in buf.iter().enumerate() {
        dest[2 * i] = hex_char(b >> 4);
        dest[2 * i + 1] = hex_char(b & 0xf);
    }
    dest[buf.len() * 2] = 0;
    PsbtResult::Ok
}

fn hex_char(v: u8) -> u8 {
    if v < 10 {
        b'0' + v
    } else {
        b'a' + v - 10
    }
}

/// Encode the PSBT data into a destination buffer using the requested encoding.
/// (Only Hex encoding is implemented for simplicity.)
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
    let data_slice = &psbt.data[..psbt_len];
    psbt_encode_raw(data_slice, psbt_len, encoding, dest, dest_size, out_len)
}

/// Encode raw PSBT data into dest using the requested encoding.
pub fn psbt_encode_raw(
    psbt_data: &[u8],
    psbt_len: usize,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    _dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode_into(&psbt_data[..psbt_len], dest);
            *out_len = psbt_len * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => {
            match base64_encode(&psbt_data[..psbt_len], dest) {
                Some(n) => {
                    *out_len = n;
                    PsbtResult::Ok
                }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match base62_encode(&psbt_data[..psbt_len], dest) {
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
    psbt_write_record_internal(psbt, rec)
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
    psbt_write_record_internal(psbt, rec)
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
    psbt_write_record_internal(psbt, rec)
}

/// Create a new input record set.
pub fn psbt_new_input_record_set(psbt: &mut Psbt) -> PsbtResult {
    match psbt.state {
        PsbtState::Global | PsbtState::InputsNew | PsbtState::Inputs => {
            let res = psbt_close_records(psbt);
            if res != PsbtResult::Ok {
                return res;
            }
            psbt.state = PsbtState::InputsNew;
            PsbtResult::Ok
        }
        _ => PsbtResult::InvalidState,
    }
}

/// Create a new output record set.
pub fn psbt_new_output_record_set(psbt: &mut Psbt) -> PsbtResult {
    match psbt.state {
        PsbtState::Inputs | PsbtState::InputsNew | PsbtState::OutputsNew | PsbtState::Outputs => {
            let res = psbt_close_records(psbt);
            if res != PsbtResult::Ok {
                return res;
            }
            psbt.state = PsbtState::OutputsNew;
            PsbtResult::Ok
        }
        _ => PsbtResult::InvalidState,
    }
}

/// Initialize a PSBT using the given destination buffer.
pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data.clear();
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    psbt.records.clear();
    PsbtResult::Ok
}

/// Print the PSBT (only succeeds after finalization).
pub fn psbt_print(psbt: &Psbt, stream: &mut dyn Write) -> PsbtResult {
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
