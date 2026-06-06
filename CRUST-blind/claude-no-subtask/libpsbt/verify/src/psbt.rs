use crate::base64::{base62_encode, base64_decode, base64_encode};
use crate::compactsize::{
    compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write,
};
use crate::tx::*;
use std::any::Any;
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
    tx.data.len()
}

// --- Internal helpers ---

/// Internal counter used during psbt_read to count inputs/outputs of the
/// embedded unsigned transaction.
struct TxCounter {
    inputs: i32,
    outputs: i32,
}

fn tx_count_handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
    if let Some(counter) = user_data.downcast_mut::<TxCounter>() {
        match elem {
            PsbtTxElem::TxIn(_) => counter.inputs += 1,
            PsbtTxElem::TxOut(_) => counter.outputs += 1,
            _ => {}
        }
    }
}

fn write_byte(psbt: &mut Psbt, byte: u8) -> PsbtResult {
    if psbt.data.len() + 1 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.push(byte);
    psbt.write_pos = psbt.data.len();
    PsbtResult::Ok
}

fn write_bytes(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if psbt.data.len() + bytes.len() > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.extend_from_slice(bytes);
    psbt.write_pos = psbt.data.len();
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let r = write_bytes(psbt, &PSBT_MAGIC);
    if r != PsbtResult::Ok {
        return r;
    }
    let r = write_byte(psbt, 0xff);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    write_byte(psbt, 0)
}

fn psbt_write_record_internal(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type: u64 = rec.key.len() as u64 + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    if psbt.data.len() + size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type);
    let r = write_bytes(psbt, &buf[..size]);
    if r != PsbtResult::Ok {
        return r;
    }

    // write type
    let r = write_byte(psbt, rec.record_type);
    if r != PsbtResult::Ok {
        return r;
    }

    // write key
    let r = write_bytes(psbt, &rec.key);
    if r != PsbtResult::Ok {
        return r;
    }

    // write value length
    let val_size = rec.val.len() as u64;
    let size = compactsize_length(val_size) as usize;
    if psbt.data.len() + size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, val_size);
    let r = write_bytes(psbt, &buf[..size]);
    if r != PsbtResult::Ok {
        return r;
    }

    // write value
    let r = write_bytes(psbt, &rec.val);
    if r != PsbtResult::Ok {
        return r;
    }

    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.data.len() < psbt.write_pos + 4 {
        return PsbtResult::ReadError;
    }
    if &psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;

    if psbt.write_pos + 1 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.data.len() <= psbt.write_pos {
        return PsbtResult::ReadError;
    }
    let b = psbt.data[psbt.write_pos];
    psbt.write_pos += 1;
    if b != 0xff {
        return PsbtResult::ReadError;
    }

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

/// Read a single record from the PSBT. Returns the record and whether read succeeded.
fn psbt_read_record(psbt: &mut Psbt, src_size: usize) -> Result<PsbtRecord, PsbtResult> {
    if psbt.write_pos >= psbt.data.len() {
        return Err(PsbtResult::ReadError);
    }

    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return Err(PsbtResult::OobWrite);
    }
    if psbt.write_pos + size_len > psbt.data.len() {
        return Err(PsbtResult::ReadError);
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
    if psbt.write_pos + size_len > psbt.data.len() {
        return Err(PsbtResult::ReadError);
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

/// Parse a PSBT from `src`. The `elem_handler` is invoked for each record found.
pub fn psbt_read(
    src: &[u8],
    _src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    let src_size = src.len();

    if psbt.state != PsbtState::Init {
        return PsbtResult::InvalidState;
    }

    if src_size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    // Copy src into psbt data.
    psbt.data.clear();
    psbt.data.extend_from_slice(src);

    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;

    // C does: tx->data_capacity = src_size; (so asserts work)
    psbt.data_capacity = src_size;

    let mut kvs: i32 = 0;
    let mut counter = TxCounter {
        inputs: 0,
        outputs: 0,
    };

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= src_size {
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
                            if kvs >= counter.inputs {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= counter.outputs {
                                psbt.state = PsbtState::Finalized;
                            } else {
                                psbt.state = PsbtState::OutputsNew;
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let rec = match psbt_read_record(psbt, src_size) {
                        Ok(r) => r,
                        Err(e) => return e,
                    };

                    if psbt.state == PsbtState::Global
                        && rec.record_type == PsbtGlobalType::UnsignedTx as u8
                    {
                        let r = psbt_btc_tx_parse(
                            &rec.val,
                            rec.val.len(),
                            &mut counter,
                            Some(tx_count_handler),
                        );
                        if r != PsbtResult::Ok {
                            return r;
                        }
                    }

                    if let Some(handler) = elem_handler {
                        let mut elem = PsbtElem::Record {
                            index: kvs,
                            record: rec,
                        };
                        handler(&mut elem, user_data);
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
            PsbtState::Finalized => {
                break;
            }
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }

    if psbt.write_pos < psbt.data.len() && psbt.data[psbt.write_pos] != 0 {
        return PsbtResult::ReadError;
    }

    psbt.write_pos += 1;

    PsbtResult::Ok
}

fn hexdigit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn psbt_hex_decode(src: &[u8], dest: &mut [u8]) -> PsbtResult {
    if src.len() % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest.len() < src.len() / 2 {
        return PsbtResult::ReadError;
    }
    let mut i = 0;
    let mut o = 0;
    while i < src.len() {
        let c1 = src[i];
        let c2 = src[i + 1];
        let h1 = match hexdigit(c1) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        let h2 = match hexdigit(c2) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        dest[o] = (h1 << 4) | h2;
        o += 1;
        i += 2;
    }
    PsbtResult::Ok
}

/// Decode a hex/base64 string into dest.
pub fn psbt_decode(
    src: &str,
    _src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let src_size = src_bytes.len();
    let dest_size = dest_size.min(dest.len());

    let b64_magic_size = b"cHNid".len();

    if src_size < b64_magic_size {
        return PsbtResult::ReadError;
    }

    if &src_bytes[..b64_magic_size] == b"cHNid" {
        match base64_decode(src_bytes, &mut dest[..dest_size]) {
            Some(n) => {
                *psbt_len = n;
                return PsbtResult::Ok;
            }
            None => return PsbtResult::ReadError,
        }
    }

    *psbt_len = src_size / 2;
    psbt_hex_decode(src_bytes, &mut dest[..dest_size])
}

fn hexchar(val: u32) -> u8 {
    if val < 10 {
        b'0' + val as u8
    } else {
        b'a' + (val as u8) - 10
    }
}

fn hex_encode(buf: &[u8], dest: &mut [u8]) -> PsbtResult {
    if dest.len() < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    let mut o = 0;
    for &c in buf {
        dest[o] = hexchar((c >> 4) as u32);
        o += 1;
        dest[o] = hexchar((c & 0xF) as u32);
        o += 1;
    }
    dest[o] = 0;
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
    let size = psbt_size(psbt);
    psbt_encode_raw(&psbt.data, size, encoding, dest, dest_size, out_len)
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
    let psbt_len = psbt_data.len().min(_psbt_len);
    let dest_size = dest_size.min(dest.len());

    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(&psbt_data[..psbt_len], &mut dest[..dest_size]);
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
        let r = psbt_write_header(psbt);
        if r != PsbtResult::Ok {
            return r;
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
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
            let r = psbt_close_records(psbt);
            if r != PsbtResult::Ok {
                return r;
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
        PsbtState::Inputs
        | PsbtState::InputsNew
        | PsbtState::OutputsNew
        | PsbtState::Outputs => {
            let r = psbt_close_records(psbt);
            if r != PsbtResult::Ok {
                return r;
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
    let r = psbt_close_records(psbt);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}
