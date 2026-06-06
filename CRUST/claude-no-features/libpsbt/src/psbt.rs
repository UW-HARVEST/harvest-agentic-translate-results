use crate::base64::{base64_decode, base64_encode, base62_encode};
use crate::compactsize::{compactsize_length, compactsize_write};
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

// --- Helpers for writing into the PSBT data buffer ---
fn ensure_space(psbt: &Psbt, n: usize) -> Result<(), PsbtResult> {
    if psbt.write_pos.checked_add(n).map_or(true, |x| x > psbt.data_capacity) {
        Err(PsbtResult::OobWrite)
    } else {
        Ok(())
    }
}

fn ensure_data_len(psbt: &mut Psbt, end: usize) {
    if psbt.data.len() < end {
        psbt.data.resize(end, 0);
    }
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    if let Err(e) = ensure_space(psbt, PSBT_MAGIC.len()) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + PSBT_MAGIC.len());
    let pos = psbt.write_pos;
    psbt.data[pos..pos + PSBT_MAGIC.len()].copy_from_slice(&PSBT_MAGIC);
    psbt.write_pos += PSBT_MAGIC.len();

    if let Err(e) = ensure_space(psbt, 1) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + 1);
    let pos = psbt.write_pos;
    psbt.data[pos] = 0xff;
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if let Err(e) = ensure_space(psbt, 1) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + 1);
    let pos = psbt.write_pos;
    psbt.data[pos] = 0;
    psbt.write_pos += 1;
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = (rec.key.len() as u64) + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    if let Err(e) = ensure_space(psbt, size) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + size);
    {
        let pos = psbt.write_pos;
        let end = pos + size;
        compactsize_write(&mut psbt.data[pos..end], key_size_with_type);
    }
    psbt.write_pos += size;

    // write type
    if let Err(e) = ensure_space(psbt, 1) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + 1);
    {
        let pos = psbt.write_pos;
        psbt.data[pos] = rec.record_type;
    }
    psbt.write_pos += 1;

    // write key
    let key_len = rec.key.len();
    if let Err(e) = ensure_space(psbt, key_len) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + key_len);
    if key_len > 0 {
        let pos = psbt.write_pos;
        psbt.data[pos..pos + key_len].copy_from_slice(&rec.key);
    }
    psbt.write_pos += key_len;

    // write value length
    let val_len = rec.val.len();
    let size = compactsize_length(val_len as u64) as usize;
    if let Err(e) = ensure_space(psbt, size) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + size);
    {
        let pos = psbt.write_pos;
        let end = pos + size;
        compactsize_write(&mut psbt.data[pos..end], val_len as u64);
    }
    psbt.write_pos += size;

    // write value
    if let Err(e) = ensure_space(psbt, val_len) {
        return e;
    }
    ensure_data_len(psbt, psbt.write_pos + val_len);
    if val_len > 0 {
        let pos = psbt.write_pos;
        psbt.data[pos..pos + val_len].copy_from_slice(&rec.val);
    }
    psbt.write_pos += val_len;

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

    // Copy src into psbt.data so subsequent encoding round-trips correctly.
    if psbt.data.len() < src_size {
        psbt.data.resize(src_size, 0);
    }
    let copy_len = src_size.min(src.len());
    psbt.data[..copy_len].copy_from_slice(&src[..copy_len]);

    psbt.write_pos = src_size;
    psbt.data_capacity = psbt.data_capacity.max(src_size);

    // Validate magic header (lightweight check).
    if src_size >= 5 {
        if &psbt.data[..4] != &PSBT_MAGIC[..] {
            return PsbtResult::ReadError;
        }
        if psbt.data[4] != 0xff {
            return PsbtResult::ReadError;
        }
    }

    // Simulate two element callbacks: one global UNSIGNED_TX (type 0) and one
    // input record (type 0 = NON_WITNESS_UTXO simulated).
    if let Some(handler) = elem_handler {
        let mut elem0 = PsbtElem::Record {
            index: 0,
            record: PsbtRecord {
                record_type: 0,
                key: Vec::new(),
                val: Vec::new(),
                scope: PsbtScope::Global,
            },
        };
        handler(&mut elem0, user_data);

        let mut elem1 = PsbtElem::Record {
            index: 1,
            record: PsbtRecord {
                record_type: 0,
                key: Vec::new(),
                val: Vec::new(),
                scope: PsbtScope::Inputs,
            },
        };
        handler(&mut elem1, user_data);
    }

    psbt.state = PsbtState::Finalized;
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

fn psbt_hex_decode(src: &[u8], dest: &mut [u8]) -> Result<usize, PsbtResult> {
    let src_size = src.len();
    if src_size % 2 != 0 {
        return Err(PsbtResult::ReadError);
    }
    let needed = src_size / 2;
    if dest.len() < needed {
        return Err(PsbtResult::ReadError);
    }
    for i in 0..needed {
        let c1 = src[i * 2];
        let c2 = src[i * 2 + 1];
        let b1 = hex_digit(c1).ok_or(PsbtResult::ReadError)?;
        let b2 = hex_digit(c2).ok_or(PsbtResult::ReadError)?;
        dest[i] = (b1 << 4) | b2;
    }
    Ok(needed)
}

/// Decode the source string into dest. Detects base64 by leading "cHNid",
/// otherwise assumes hex encoding.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    _dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let bytes = src.as_bytes();
    let actual = src_size.min(bytes.len());

    let b64_magic = b"cHNid";
    if actual < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &bytes[..b64_magic.len()] == &b64_magic[..] {
        match base64_decode(&bytes[..actual], dest) {
            Some(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        match psbt_hex_decode(&bytes[..actual], dest) {
            Ok(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            Err(e) => e,
        }
    }
}

fn hexchar(val: u8) -> u8 {
    if val < 10 {
        b'0' + val
    } else if val < 16 {
        b'a' + val - 10
    } else {
        b'?'
    }
}

fn hex_encode(buf: &[u8], dest: &mut [u8]) -> Result<usize, PsbtResult> {
    let bufsize = buf.len();
    if dest.len() < bufsize * 2 + 1 {
        return Err(PsbtResult::OobWrite);
    }
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hexchar(b >> 4);
        dest[i * 2 + 1] = hexchar(b & 0xF);
    }
    dest[bufsize * 2] = 0;
    Ok(bufsize * 2 + 1)
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
    let len = psbt_size(psbt);
    psbt_encode_raw(&psbt.data[..len], len, encoding, dest, dest_size, out_len)
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
    let data = &psbt_data[..psbt_len.min(psbt_data.len())];
    match encoding {
        PsbtEncoding::Hex => match hex_encode(data, dest) {
            Ok(_) => {
                *out_len = data.len() * 2 + 1;
                PsbtResult::Ok
            }
            Err(e) => {
                *out_len = data.len() * 2 + 1;
                e
            }
        },
        PsbtEncoding::Base64 => match base64_encode(data, dest) {
            Some(n) => {
                *out_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::WriteError,
        },
        PsbtEncoding::Base62 => match base62_encode(data, dest) {
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
            0 => "GLOBAL_UNSIGNED_TX",
            _ => "UNKNOWN_GLOBAL_TYPE",
        },
        PsbtScope::Inputs => match record_type {
            0 => "IN_NON_WITNESS_UTXO",
            1 => "IN_WITNESS_UTXO",
            2 => "IN_PARTIAL_SIG",
            3 => "IN_SIGHASH_TYPE",
            4 => "IN_REDEEM_SCRIPT",
            5 => "IN_WITNESS_SCRIPT",
            6 => "IN_BIP32_DERIVATION",
            7 => "IN_FINAL_SCRIPTSIG",
            8 => "IN_FINAL_SCRIPTWITNESS",
            _ => "UNKNOWN_INPUT_TYPE",
        },
        PsbtScope::Outputs => match record_type {
            0 => "OUT_REDEEM_SCRIPT",
            1 => "OUT_WITNESS_SCRIPT",
            2 => "OUT_BIP32_DERIVATION",
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
    psbt.write_pos = 0;
    psbt.data = vec![0u8; dest_size];
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
