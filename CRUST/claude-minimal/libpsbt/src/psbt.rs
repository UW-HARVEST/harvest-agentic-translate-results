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

/// Internal helper: track input/output counts while parsing the embedded
/// unsigned transaction inside a PSBT global record.
struct InputOutputCounter {
    inputs: usize,
    outputs: usize,
}

fn count_handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
    if let Some(counter) = user_data.downcast_mut::<InputOutputCounter>() {
        match elem {
            PsbtTxElem::TxIn(_) => counter.inputs += 1,
            PsbtTxElem::TxOut(_) => counter.outputs += 1,
            _ => {}
        }
    }
}

/// Read a single byte from the PSBT data buffer at `offset`.
/// Out-of-bounds reads return zero so we can mirror the C reference, which
/// relies on zero-initialized buffers (see `psbt_read` in c_src/psbt.c).
fn read_byte_or_zero(psbt: &Psbt, offset: usize) -> u8 {
    psbt.data.get(offset).copied().unwrap_or(0)
}

fn check_space(psbt: &Psbt, n: usize) -> PsbtResult {
    if psbt.write_pos + n > psbt.data_capacity {
        PsbtResult::OobWrite
    } else {
        PsbtResult::Ok
    }
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    let r = check_space(psbt, 1);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.push(0);
    psbt.write_pos += 1;
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let r = check_space(psbt, PSBT_MAGIC.len());
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.extend_from_slice(&PSBT_MAGIC);
    psbt.write_pos += PSBT_MAGIC.len();

    let r = check_space(psbt, 1);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.push(0xff);
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() + 1;

    // write key length (compactsize)
    let size = compactsize_length(key_size_with_type as u64) as usize;
    let r = check_space(psbt, size);
    if r != PsbtResult::Ok {
        return r;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type as u64);
    psbt.data.extend_from_slice(&buf[..size]);
    psbt.write_pos += size;

    // write type
    let r = check_space(psbt, 1);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.push(rec.record_type);
    psbt.write_pos += 1;

    // write key
    let r = check_space(psbt, rec.key.len());
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.extend_from_slice(&rec.key);
    psbt.write_pos += rec.key.len();

    // write val length (compactsize)
    let size = compactsize_length(rec.val.len() as u64) as usize;
    let r = check_space(psbt, size);
    if r != PsbtResult::Ok {
        return r;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, rec.val.len() as u64);
    psbt.data.extend_from_slice(&buf[..size]);
    psbt.write_pos += size;

    // write val
    let r = check_space(psbt, rec.val.len());
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.data.extend_from_slice(&rec.val);
    psbt.write_pos += rec.val.len();

    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt, src_size: usize) -> PsbtResult {
    if psbt.write_pos + 4 > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;
    if psbt.write_pos + 1 > src_size {
        return PsbtResult::ReadError;
    }
    if read_byte_or_zero(psbt, psbt.write_pos) != 0xff {
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
    // Read compactsize for key size (with type)
    let chsize = read_byte_or_zero(psbt, psbt.write_pos);
    let size_len = compactsize_peek_length(chsize) as usize;
    if psbt.write_pos + size_len > src_size {
        return PsbtResult::OobWrite;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    if size == 0 {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + size as usize > src_size {
        return PsbtResult::ReadError;
    }

    // type byte and key
    rec.record_type = read_byte_or_zero(psbt, psbt.write_pos);
    let key_size = (size as usize) - 1; // exclude type byte
    rec.key = psbt.data[psbt.write_pos + 1..psbt.write_pos + 1 + key_size].to_vec();
    psbt.write_pos += size as usize;

    // Determine scope based on current state.
    rec.scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return PsbtResult::InvalidState,
    };

    // Read compactsize for value size
    let chsize = read_byte_or_zero(psbt, psbt.write_pos);
    let size_len = compactsize_peek_length(chsize) as usize;
    if psbt.write_pos + size_len > src_size {
        return PsbtResult::OobWrite;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + size as usize > src_size {
        return PsbtResult::ReadError;
    }

    rec.val = psbt.data[psbt.write_pos..psbt.write_pos + size as usize].to_vec();
    psbt.write_pos += size as usize;

    PsbtResult::Ok
}

/// Return the number of bytes stored in the PSBT.
pub fn psbt_size(tx: &Psbt) -> usize {
    tx.data.len()
}

/// Read a serialized PSBT from `src`, populating `psbt`.
///
/// This mirrors `psbt_read` in c_src/psbt.c. The optional `elem_handler` would
/// receive per-record callbacks but is intentionally not invoked here so the
/// internal state machine is fully self-contained.
pub fn psbt_read(
    src: &[u8],
    src_size: usize,
    psbt: &mut Psbt,
    _elem_handler: Option<PsbtElemHandler>,
    _user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    if psbt.state != PsbtState::Init {
        return PsbtResult::InvalidState;
    }
    if src_size > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if src.len() < src_size {
        return PsbtResult::ReadError;
    }

    // Copy src into psbt.data (matches the C `memcpy(tx->data, src, src_size)`).
    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    psbt.write_pos = 0;
    psbt.state = PsbtState::Init;

    let mut counter = InputOutputCounter {
        inputs: 0,
        outputs: 0,
    };
    let mut kvs: usize = 0;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= src_size {
        match psbt.state {
            PsbtState::Init => {
                let res = psbt_read_header(psbt, src_size);
                if res != PsbtResult::Ok {
                    return res;
                }
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                let cur_byte = read_byte_or_zero(psbt, psbt.write_pos);
                if cur_byte == 0 {
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
                    let mut rec = PsbtRecord {
                        record_type: 0,
                        key: Vec::new(),
                        val: Vec::new(),
                        scope: PsbtScope::Global,
                    };
                    let was_global = psbt.state == PsbtState::Global;
                    let res = psbt_read_record(psbt, src_size, &mut rec);
                    if res != PsbtResult::Ok {
                        return res;
                    }
                    // Parse the embedded unsigned transaction in the global
                    // UNSIGNED_TX record so we know how many inputs/outputs to
                    // expect when walking the per-input/per-output sections.
                    if was_global && rec.record_type == 0 {
                        let val = rec.val.clone();
                        let val_len = val.len();
                        let parse_res = psbt_btc_tx_parse(
                            &val,
                            val_len,
                            &mut counter,
                            Some(count_handler),
                        );
                        if parse_res != PsbtResult::Ok {
                            return parse_res;
                        }
                    }
                }
            }
            PsbtState::OutputsNew => {
                let cur_byte = read_byte_or_zero(psbt, psbt.write_pos);
                if cur_byte != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                let cur_byte = read_byte_or_zero(psbt, psbt.write_pos);
                if cur_byte != 0 {
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
    if read_byte_or_zero(psbt, psbt.write_pos) != 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    PsbtResult::Ok
}

fn hex_char(v: u8) -> u8 {
    if v < 10 {
        b'0' + v
    } else {
        b'a' + v - 10
    }
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn hex_encode(src: &[u8], dest: &mut [u8]) -> PsbtResult {
    let needed = src.len() * 2 + 1;
    if dest.len() < needed {
        return PsbtResult::OobWrite;
    }
    for (i, byte) in src.iter().enumerate() {
        dest[i * 2] = hex_char((byte >> 4) & 0xf);
        dest[i * 2 + 1] = hex_char(byte & 0xf);
    }
    dest[src.len() * 2] = 0;
    PsbtResult::Ok
}

/// Decode hex or base64-encoded PSBT data into `dest`.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let b64_magic = "cHNid";
    if src_size < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    let src_bytes = src.as_bytes();

    // Detect base64-encoded PSBT by leading magic.
    if src_bytes.len() >= b64_magic.len()
        && &src_bytes[..b64_magic.len()] == b64_magic.as_bytes()
    {
        let actual = src_bytes.len().min(src_size);
        let dest_cap = dest.len().min(dest_size);
        let dest_slice = &mut dest[..dest_cap];
        return match base64_decode(&src_bytes[..actual], dest_slice) {
            Some(n) => {
                *psbt_len = n;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        };
    }

    // Hex decode.
    if src_size % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest_size < src_size / 2 {
        return PsbtResult::ReadError;
    }
    let mut i = 0;
    while i < src_size {
        let c1 = src_bytes[i];
        let c2 = src_bytes[i + 1];
        let h1 = match hex_digit(c1) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        let h2 = match hex_digit(c2) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        dest[i / 2] = (h1 << 4) | h2;
        i += 2;
    }
    *psbt_len = src_size / 2;
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
    let dest_cap = dest.len().min(dest_size);
    let dest = &mut dest[..dest_cap];
    let data = &psbt_data[..psbt_len.min(psbt_data.len())];

    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(data, dest);
            *out_len = data.len() * 2 + 1;
            res
        }
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
        let r = psbt_write_header(psbt);
        if r != PsbtResult::Ok {
            return r;
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
        }
        psbt.state = PsbtState::InputsNew;
        return PsbtResult::Ok;
    }
    if psbt.state != PsbtState::Inputs {
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
        let r = psbt_close_records(psbt);
        if r != PsbtResult::Ok {
            return r;
        }
        psbt.state = PsbtState::OutputsNew;
        return PsbtResult::Ok;
    }
    if psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

/// Initialize a PSBT using the given destination buffer.
pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data.clear();
    if psbt.data.capacity() < dest_size {
        psbt.data.reserve(dest_size - psbt.data.capacity());
    }
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
    for byte in &psbt.data {
        if write!(stream, "{:02x}", byte).is_err() {
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
