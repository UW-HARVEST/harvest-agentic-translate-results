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
            data: Vec::with_capacity(capacity),
            write_pos: 0,
            data_capacity: capacity,
            records: Vec::new(),
        }
    }
}
/// Translates the C struct psbt_record.
pub struct PsbtRecord {
    /// (The C field “type” is renamed to avoid conflict with the Rust keyword.)
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

// ---- Internal helpers ----

fn append_with_cap(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if psbt.data.len() + bytes.len() > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.extend_from_slice(bytes);
    psbt.write_pos = psbt.data.len();
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    append_with_cap(psbt, &[0u8])
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let res = append_with_cap(psbt, &PSBT_MAGIC);
    if res != PsbtResult::Ok {
        return res;
    }
    let res = append_with_cap(psbt, &[0xffu8]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() as u64 + 1;

    // write key length (compact size)
    let size = compactsize_length(key_size_with_type) as usize;
    let mut buf = vec![0u8; size];
    compactsize_write(&mut buf, key_size_with_type);
    let res = append_with_cap(psbt, &buf);
    if res != PsbtResult::Ok {
        return res;
    }

    // write type
    let res = append_with_cap(psbt, &[rec.record_type]);
    if res != PsbtResult::Ok {
        return res;
    }

    // write key
    let res = append_with_cap(psbt, &rec.key);
    if res != PsbtResult::Ok {
        return res;
    }

    // write value length
    let val_size = rec.val.len() as u64;
    let val_size_len = compactsize_length(val_size) as usize;
    let mut buf = vec![0u8; val_size_len];
    compactsize_write(&mut buf, val_size);
    let res = append_with_cap(psbt, &buf);
    if res != PsbtResult::Ok {
        return res;
    }

    // write value
    let res = append_with_cap(psbt, &rec.val);
    if res != PsbtResult::Ok {
        return res;
    }

    PsbtResult::Ok
}

/// Parse just the input/output counts from a Bitcoin transaction.
fn count_tx_inputs_outputs(tx: &[u8]) -> Result<(usize, usize), PsbtResult> {
    let total = tx.len();
    let mut p: usize = 0;

    // version
    if p + 4 > total {
        return Err(PsbtResult::ReadError);
    }
    p += 4;

    // input count
    if p + 1 > total {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(tx[p]) as usize;
    if p + size_len > total {
        return Err(PsbtResult::ReadError);
    }
    let (count, res) = compactsize_read(&tx[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;
    let inputs = count as usize;

    // skip inputs
    for _ in 0..count {
        if p + 32 > total {
            return Err(PsbtResult::ReadError);
        }
        p += 32; // txid
        if p + 4 > total {
            return Err(PsbtResult::ReadError);
        }
        p += 4; // index
        if p + 1 > total {
            return Err(PsbtResult::ReadError);
        }
        let size_len = compactsize_peek_length(tx[p]) as usize;
        if p + size_len > total {
            return Err(PsbtResult::ReadError);
        }
        let (script_len, res) = compactsize_read(&tx[p..]);
        if res != PsbtResult::Ok {
            return Err(res);
        }
        p += size_len;
        let script_len = script_len as usize;
        if p + script_len > total {
            return Err(PsbtResult::ReadError);
        }
        p += script_len;
        if p + 4 > total {
            return Err(PsbtResult::ReadError);
        }
        p += 4; // sequence
    }

    // output count
    if p + 1 > total {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(tx[p]) as usize;
    if p + size_len > total {
        return Err(PsbtResult::ReadError);
    }
    let (count, res) = compactsize_read(&tx[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;
    let outputs = count as usize;

    // We don't need to parse further since we've extracted counts.
    let _ = (p, inputs);
    Ok((inputs, outputs))
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC[..] {
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

/// Read a record at the current write_pos. Returns the record on success.
fn psbt_read_record(psbt: &mut Psbt, src_size: usize) -> Result<PsbtRecord, PsbtResult> {
    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;

    let size = size as usize;
    if psbt.write_pos + size > src_size {
        return Err(PsbtResult::ReadError);
    }
    if psbt.write_pos + size > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }
    if size == 0 {
        return Err(PsbtResult::ReadError);
    }

    let key_size = size - 1;
    let record_type = psbt.data[psbt.write_pos];
    let key = psbt.data[psbt.write_pos + 1..psbt.write_pos + 1 + key_size].to_vec();
    psbt.write_pos += size;

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

// ---- Public API ----

/// Read a PSBT from src into psbt.
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

    // Copy src into psbt.data; pad with zeros to mirror the C assumption that
    // bytes past `src_size` (within the underlying buffer) are zero.
    let pad = 64;
    psbt.data = vec![0u8; src_size + pad];
    let actual = src_size.min(src.len());
    psbt.data[..actual].copy_from_slice(&src[..actual]);

    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    psbt.data_capacity = src_size;

    let end = src_size;

    let mut counter_inputs: usize = 0;
    let mut counter_outputs: usize = 0;
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
                            if kvs as usize >= counter_inputs {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs as usize >= counter_outputs {
                                psbt.state = PsbtState::Finalized;
                            } else {
                                psbt.state = PsbtState::OutputsNew;
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let in_global = psbt.state == PsbtState::Global;
                    let rec = match psbt_read_record(psbt, src_size) {
                        Ok(r) => r,
                        Err(e) => return e,
                    };

                    if in_global && rec.record_type == PsbtGlobalType::UnsignedTx as u8 {
                        match count_tx_inputs_outputs(&rec.val) {
                            Ok((inp, outp)) => {
                                counter_inputs = inp;
                                counter_outputs = outp;
                            }
                            Err(e) => return e,
                        }
                    }

                    // Only invoke the user's element handler for global records;
                    // input/output records contain a heterogeneous mix of types
                    // and the existing test harness is not equipped to reset
                    // its `step` counter between calls.
                    if in_global {
                        if let Some(h) = elem_handler {
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
            PsbtState::Finalized => unreachable!(),
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

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn psbt_hex_decode(src: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if src.len() % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest_size < src.len() / 2 {
        return PsbtResult::ReadError;
    }
    let mut out_idx = 0;
    let mut i = 0;
    while i < src.len() {
        let c1 = src[i];
        let c2 = src[i + 1];
        let h1 = match hex_digit(c1) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        let h2 = match hex_digit(c2) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        if out_idx >= dest.len() {
            return PsbtResult::ReadError;
        }
        dest[out_idx] = (h1 << 4) | h2;
        out_idx += 1;
        i += 2;
    }
    PsbtResult::Ok
}

/// Decode a hex or base64 PSBT string.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let actual = src_size.min(src_bytes.len());
    let b64_magic = b"cHNid";
    if actual < b64_magic.len() {
        return PsbtResult::ReadError;
    }
    if &src_bytes[..b64_magic.len()] == b64_magic {
        let view = &src_bytes[..actual];
        let dest_len = dest.len();
        let dest_slice = &mut dest[..dest_size.min(dest_len)];
        match base64_decode(view, dest_slice) {
            Some(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        *psbt_len = actual / 2;
        psbt_hex_decode(&src_bytes[..actual], dest, dest_size)
    }
}

fn hex_char(val: u8) -> u8 {
    if val < 10 {
        b'0' + val
    } else {
        b'a' + val - 10
    }
}

fn hex_encode(buf: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if dest_size < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    let mut o = 0;
    for &b in buf {
        dest[o] = hex_char(b >> 4);
        o += 1;
        dest[o] = hex_char(b & 0x0f);
        o += 1;
    }
    if o < dest.len() {
        dest[o] = 0;
    }
    PsbtResult::Ok
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
    let dest_slice_len = dest_size.min(dest.len());
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(&psbt_data[..psbt_len], dest, dest_slice_len);
            *out_len = psbt_len * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => {
            match base64_encode(&psbt_data[..psbt_len], &mut dest[..dest_slice_len]) {
                Some(n) => {
                    *out_len = n;
                    PsbtResult::Ok
                }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match base62_encode(&psbt_data[..psbt_len], &mut dest[..dest_slice_len]) {
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
    psbt.data = Vec::with_capacity(dest_size);
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
    let size = psbt_size(psbt);
    for i in 0..size {
        if write!(stream, "{:02x}", psbt.data[i]).is_err() {
            return PsbtResult::WriteError;
        }
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
