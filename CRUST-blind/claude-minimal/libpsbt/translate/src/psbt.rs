use crate::base64::{base62_encode, base64_encode, base64_decode};
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

// --- Helpers ---

fn ensure_space(psbt: &Psbt, needed: usize) -> bool {
    psbt.write_pos.checked_add(needed).map_or(false, |end| end <= psbt.data_capacity)
}

fn write_bytes(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if !ensure_space(psbt, bytes.len()) {
        return PsbtResult::OobWrite;
    }
    let end = psbt.write_pos + bytes.len();
    if psbt.data.len() < end {
        psbt.data.resize(end, 0);
    }
    psbt.data[psbt.write_pos..end].copy_from_slice(bytes);
    psbt.write_pos = end;
    PsbtResult::Ok
}

fn write_byte(psbt: &mut Psbt, b: u8) -> PsbtResult {
    write_bytes(psbt, &[b])
}

fn write_compactsize(psbt: &mut Psbt, val: u64) -> PsbtResult {
    let size = compactsize_length(val) as usize;
    if !ensure_space(psbt, size) {
        return PsbtResult::OobWrite;
    }
    let end = psbt.write_pos + size;
    if psbt.data.len() < end {
        psbt.data.resize(end, 0);
    }
    compactsize_write(&mut psbt.data[psbt.write_pos..end], val);
    psbt.write_pos = end;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    write_byte(psbt, 0)
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = (rec.key.len() as u64) + 1;

    // write key length
    let res = write_compactsize(psbt, key_size_with_type);
    if res != PsbtResult::Ok {
        return res;
    }

    // write type
    let res = write_byte(psbt, rec.record_type);
    if res != PsbtResult::Ok {
        return res;
    }

    // write key
    let res = write_bytes(psbt, &rec.key);
    if res != PsbtResult::Ok {
        return res;
    }

    // write value length
    let res = write_compactsize(psbt, rec.val.len() as u64);
    if res != PsbtResult::Ok {
        return res;
    }

    // write value
    write_bytes(psbt, &rec.val)
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let res = write_bytes(psbt, &PSBT_MAGIC);
    if res != PsbtResult::Ok {
        return res;
    }
    let res = write_byte(psbt, 0xff);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data.len() {
        return PsbtResult::OobWrite;
    }
    if &psbt.data[psbt.write_pos..psbt.write_pos + 4] != &PSBT_MAGIC[..] {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;

    if psbt.write_pos >= psbt.data.len() {
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
) -> Result<PsbtRecord, PsbtResult> {
    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::OobWrite);
    }

    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }
    let (key_size_with_type, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;

    let key_size_with_type = key_size_with_type as usize;

    if psbt.write_pos + key_size_with_type > src_size {
        return Err(PsbtResult::ReadError);
    }
    if psbt.write_pos + key_size_with_type > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }

    let record_type = psbt.data[psbt.write_pos];
    let key_size = key_size_with_type - 1;
    let key = psbt.data[psbt.write_pos + 1..psbt.write_pos + 1 + key_size].to_vec();
    psbt.write_pos += key_size_with_type;

    let scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return Err(PsbtResult::InvalidState),
    };

    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::OobWrite);
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }
    let (val_size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;

    let val_size = val_size as usize;
    if psbt.write_pos + val_size > src_size {
        return Err(PsbtResult::ReadError);
    }
    if psbt.write_pos + val_size > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }

    let val = psbt.data[psbt.write_pos..psbt.write_pos + val_size].to_vec();
    psbt.write_pos += val_size;

    Ok(PsbtRecord {
        record_type,
        key,
        val,
        scope,
    })
}

/// Read PSBT bytes, dispatching records and txelems through the optional handler.
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

    // Copy src into psbt.data
    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    psbt.write_pos = 0;
    // mirror the C behavior of constraining capacity to src_size
    psbt.data_capacity = src_size;

    let end = src_size;

    // Parse the embedded transaction (if present) to know the input/output counts.
    let mut input_count: i32 = 0;
    let mut output_count: i32 = 0;
    let mut kvs: i32 = 0;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= end {
        match psbt.state {
            PsbtState::Init => {
                let res = psbt_read_header(psbt);
                if res != PsbtResult::Ok {
                    return res;
                }
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                if psbt.write_pos >= psbt.data.len() {
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos] == 0 {
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
                    let was_global = psbt.state == PsbtState::Global;
                    let rec = match psbt_read_record(psbt, src_size) {
                        Ok(r) => r,
                        Err(e) => return e,
                    };

                    if was_global && rec.record_type == PsbtGlobalType::UnsignedTx as u8 {
                        // Parse the transaction to discover input/output counts.
                        let tx_bytes = rec.val.clone();
                        let mut counts = TxCounts {
                            inputs: 0,
                            outputs: 0,
                        };
                        let res = psbt_btc_tx_parse(
                            &tx_bytes,
                            tx_bytes.len(),
                            &mut counts,
                            Some(tx_counter),
                        );
                        if res != PsbtResult::Ok {
                            return res;
                        }
                        input_count = counts.inputs;
                        output_count = counts.outputs;
                    }

                    if let Some(h) = elem_handler {
                        let mut elem = PsbtElem::Record {
                            index: kvs,
                            record: PsbtRecord {
                                record_type: rec.record_type,
                                key: rec.key.clone(),
                                val: rec.val.clone(),
                                scope: rec.scope.clone(),
                            },
                        };
                        h(&mut elem, user_data);
                    }
                    psbt.records.push(rec);
                    continue;
                }
                // We landed on a 0-byte separator; advance past it now.
                // (Actually in C the byte advance happens in *_NEW states.)
            }
            PsbtState::OutputsNew => {
                if psbt.write_pos >= psbt.data.len() || psbt.data[psbt.write_pos] != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                if psbt.write_pos >= psbt.data.len() || psbt.data[psbt.write_pos] != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => {
                // unreachable while loop guard catches this
                break;
            }
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    if psbt.write_pos >= psbt.data.len() || psbt.data[psbt.write_pos] != 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    PsbtResult::Ok
}

// Helpers for forwarding tx parse callbacks.
struct TxCounts {
    inputs: i32,
    outputs: i32,
}

fn tx_counter(elem: &mut PsbtTxElem, user_data: &mut dyn std::any::Any) {
    if let Some(counts) = user_data.downcast_mut::<TxCounts>() {
        match elem {
            PsbtTxElem::TxIn(_) => counts.inputs += 1,
            PsbtTxElem::TxOut(_) => counts.outputs += 1,
            _ => {}
        }
    }
}

/// Decode a PSBT from a hex (or base64) string into `dest`.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let len = src_size.min(src_bytes.len());

    let b64_magic = b"cHNid";
    if len < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &src_bytes[..b64_magic.len()] == b64_magic {
        match base64_decode(&src_bytes[..len], &mut dest[..dest_size]) {
            Some(decoded) => {
                *psbt_len = decoded;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        if len % 2 != 0 {
            return PsbtResult::ReadError;
        }
        if dest_size < len / 2 {
            return PsbtResult::ReadError;
        }
        for i in (0..len).step_by(2) {
            let c1 = src_bytes[i];
            let c2 = src_bytes[i + 1];
            if !c1.is_ascii_hexdigit() || !c2.is_ascii_hexdigit() {
                return PsbtResult::ReadError;
            }
            dest[i / 2] = (hex_digit(c1) << 4) | hex_digit(c2);
        }
        *psbt_len = len / 2;
        PsbtResult::Ok
    }
}

fn hex_digit(c: u8) -> u8 {
    if c <= b'9' {
        c - b'0'
    } else {
        (c.to_ascii_uppercase()) - b'A' + 10
    }
}

fn hex_char(v: u32) -> u8 {
    if v < 10 {
        b'0' + v as u8
    } else {
        b'a' + (v - 10) as u8
    }
}

fn hex_encode_bytes(buf: &[u8], dest: &mut [u8]) -> PsbtResult {
    if dest.len() < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hex_char((b >> 4) as u32);
        dest[i * 2 + 1] = hex_char((b & 0xf) as u32);
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
    psbt_encode_raw(&psbt.data, psbt_size(psbt), encoding, dest, dest_size, out_len)
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
    let cap = dest_size.min(dest.len());
    let dest = &mut dest[..cap];
    let psbt_data = &psbt_data[..psbt_len.min(psbt_data.len())];

    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode_bytes(psbt_data, dest);
            *out_len = psbt_data.len() * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => match base64_encode(psbt_data, dest) {
            Some(n) => {
                *out_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::WriteError,
        },
        PsbtEncoding::Base62 => match base62_encode(psbt_data, dest) {
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
    psbt.data.clear();
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

    let size = psbt_size(psbt);
    for i in 0..size {
        if write!(stream, "{:02x}", psbt.data[i]).is_err() {
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
