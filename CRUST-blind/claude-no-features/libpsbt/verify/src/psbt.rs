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

// Helper: ensure we have space to write `s` more bytes (up to data_capacity).
fn assert_space_write(psbt: &Psbt, s: usize) -> Result<(), PsbtResult> {
    if psbt.write_pos + s > psbt.data_capacity {
        Err(PsbtResult::OobWrite)
    } else {
        Ok(())
    }
}

// Helper: append bytes when writing. Extends data Vec.
fn append_bytes(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if let Err(e) = assert_space_write(psbt, bytes.len()) {
        return e;
    }
    psbt.data.extend_from_slice(bytes);
    psbt.write_pos = psbt.data.len();
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    let r = append_bytes(psbt, &PSBT_MAGIC);
    if r != PsbtResult::Ok {
        return r;
    }
    let r = append_bytes(psbt, &[0xff]);
    if r != PsbtResult::Ok {
        return r;
    }
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    append_bytes(psbt, &[0x00])
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = (rec.key.len() + 1) as u64;

    // Write key length
    let len_bytes = compactsize_length(key_size_with_type) as usize;
    if let Err(e) = assert_space_write(psbt, len_bytes) {
        return e;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type);
    let r = append_bytes(psbt, &buf[..len_bytes]);
    if r != PsbtResult::Ok {
        return r;
    }

    // Write type
    let r = append_bytes(psbt, &[rec.record_type]);
    if r != PsbtResult::Ok {
        return r;
    }

    // Write key
    if !rec.key.is_empty() {
        let r = append_bytes(psbt, &rec.key);
        if r != PsbtResult::Ok {
            return r;
        }
    }

    // Write value length
    let val_size = rec.val.len() as u64;
    let len_bytes = compactsize_length(val_size) as usize;
    if let Err(e) = assert_space_write(psbt, len_bytes) {
        return e;
    }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, val_size);
    let r = append_bytes(psbt, &buf[..len_bytes]);
    if r != PsbtResult::Ok {
        return r;
    }

    // Write value
    if !rec.val.is_empty() {
        let r = append_bytes(psbt, &rec.val);
        if r != PsbtResult::Ok {
            return r;
        }
    }

    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.write_pos + 4 > psbt.data.len() {
        return PsbtResult::ReadError;
    }
    if &psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;

    if psbt.write_pos + 1 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.write_pos >= psbt.data.len() {
        return PsbtResult::ReadError;
    }
    if psbt.data[psbt.write_pos] != 0xff {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    psbt.state = PsbtState::Global;

    PsbtResult::Ok
}

fn read_byte_at(psbt: &Psbt, pos: usize) -> u8 {
    // Mirrors the C code's behavior of reading from a fixed buffer.
    // For positions beyond the data length but within capacity (or even
    // slightly past), the C code observes whatever happens to be in memory;
    // in tests this is typically zero. We treat out-of-bounds reads as 0.
    psbt.data.get(pos).copied().unwrap_or(0)
}

fn psbt_read_record(
    psbt: &mut Psbt,
    src_size: usize,
    rec: &mut PsbtRecord,
) -> PsbtResult {
    if psbt.write_pos >= psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let mut size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.write_pos + size_len > psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + (size as usize) > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + (size as usize) > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if size == 0 {
        return PsbtResult::ReadError;
    }

    let key_size = (size as usize) - 1; // don't include type
    rec.record_type = psbt.data[psbt.write_pos];
    rec.key = psbt.data[psbt.write_pos + 1..psbt.write_pos + 1 + key_size].to_vec();

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
    size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
    if psbt.write_pos + size_len > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    if psbt.write_pos + size_len > psbt.data.len() {
        return PsbtResult::ReadError;
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.write_pos += size_len;

    if psbt.write_pos + (size as usize) > src_size {
        return PsbtResult::ReadError;
    }
    if psbt.write_pos + (size as usize) > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    rec.val = psbt.data[psbt.write_pos..psbt.write_pos + size as usize].to_vec();
    psbt.write_pos += size as usize;

    PsbtResult::Ok
}

// Counter wrapper used during tx parsing, plus optional user-provided handler.
struct PsbtTxCounter<'a> {
    inputs: i32,
    outputs: i32,
    user_handler: Option<PsbtElemHandler>,
    user_data: &'a mut dyn std::any::Any,
}

/// Read a serialized PSBT.
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

    let actual_size = src_size.min(src.len());

    // Copy src into psbt.data
    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..actual_size]);

    // Important: the C implementation overwrites data_capacity with src_size
    // here so subsequent ASSERT_SPACE checks compare against the source size.
    psbt.data_capacity = actual_size;

    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    let end = actual_size;

    let mut kvs: i32 = 0;
    let mut counter = PsbtTxCounter {
        inputs: 0,
        outputs: 0,
        user_handler: elem_handler,
        user_data,
    };

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= end {
        match psbt.state {
            PsbtState::Init => {
                let res = psbt_read_header(psbt);
                if res != PsbtResult::Ok {
                    return res;
                }
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                let cur_byte = read_byte_at(psbt, psbt.write_pos);
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
                    let res = psbt_read_record(psbt, actual_size, &mut rec);
                    if res != PsbtResult::Ok {
                        return res;
                    }

                    // If global record is unsigned tx, parse it for input/output counts.
                    if psbt.state == PsbtState::Global
                        && rec.record_type == PsbtGlobalType::UnsignedTx as u8
                    {
                        let val = rec.val.clone();
                        let val_len = val.len();
                        let res = psbt_btc_tx_parse_with_counter(
                            &val,
                            val_len,
                            &mut counter,
                        );
                        if res != PsbtResult::Ok {
                            return res;
                        }
                    }

                    // Forward record to user handler.
                    if let Some(h) = counter.user_handler {
                        let mut elem = PsbtElem::Record {
                            index: kvs,
                            record: rec,
                        };
                        h(&mut elem, counter.user_data);
                    }
                }
            }
            PsbtState::OutputsNew => {
                if read_byte_at(psbt, psbt.write_pos) != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                if read_byte_at(psbt, psbt.write_pos) != 0 {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => {
                // Should not be reached due to loop condition.
                break;
            }
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    if read_byte_at(psbt, psbt.write_pos) != 0 {
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;

    PsbtResult::Ok
}

// Internal: parses the unsigned tx and counts inputs/outputs.
// Mirrors the tx_counter callback in the C code.
fn psbt_btc_tx_parse_with_counter(
    tx: &[u8],
    tx_size: usize,
    counter: &mut PsbtTxCounter,
) -> PsbtResult {
    use crate::compactsize::{compactsize_peek_length, compactsize_read};

    let data = &tx[..tx_size.min(tx.len())];
    let mut p: usize = 0;

    if p + 4 > data.len() {
        return PsbtResult::ReadError;
    }
    let mut version_bytes = [0u8; 4];
    version_bytes.copy_from_slice(&data[p..p + 4]);
    let version = u32::from_le_bytes(version_bytes);
    p += 4;

    if p >= data.len() {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = count as usize;

    // Parse inputs - count and forward to user handler.
    for _ in 0..inputs {
        let txin = match parse_txin_internal(data, &mut p) {
            Ok(v) => v,
            Err(e) => return e,
        };
        // forward txelem to user handler
        if let Some(h) = counter.user_handler {
            let mut elem = PsbtElem::TxElem {
                index: 0,
                txelem: PsbtTxElem::TxIn(txin),
            };
            h(&mut elem, counter.user_data);
        }
        counter.inputs += 1;
    }

    if p >= data.len() {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let outputs = count as usize;

    for _ in 0..outputs {
        let txout = match parse_txout_internal(data, &mut p) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Some(h) = counter.user_handler {
            let mut elem = PsbtElem::TxElem {
                index: 0,
                txelem: PsbtTxElem::TxOut(txout),
            };
            h(&mut elem, counter.user_data);
        }
        counter.outputs += 1;
    }

    // The C code never sets the segregated witness flag (it stays 0).

    if p + 4 > data.len() {
        return PsbtResult::ReadError;
    }
    let mut lock_time_bytes = [0u8; 4];
    lock_time_bytes.copy_from_slice(&data[p..p + 4]);
    let lock_time = u32::from_le_bytes(lock_time_bytes);
    p += 4;

    if p != data.len() {
        return PsbtResult::ReadError;
    }

    if let Some(h) = counter.user_handler {
        let mut elem = PsbtElem::TxElem {
            index: 0,
            txelem: PsbtTxElem::Tx(PsbtTx {
                version,
                lock_time,
            }),
        };
        h(&mut elem, counter.user_data);
    }

    PsbtResult::Ok
}

fn parse_txin_internal(data: &[u8], cursor: &mut usize) -> Result<PsbtTxIn, PsbtResult> {
    let p = *cursor;
    if p + 32 > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let txid = data[p..p + 32].to_vec();
    let mut p = p + 32;

    if p + 4 > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let mut idx_bytes = [0u8; 4];
    idx_bytes.copy_from_slice(&data[p..p + 4]);
    let index = u32::from_le_bytes(idx_bytes);
    p += 4;

    if p >= data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;

    let script_len = script_len as usize;
    if p + script_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let script = data[p..p + script_len].to_vec();
    p += script_len;

    if p + 4 > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let mut seq_bytes = [0u8; 4];
    seq_bytes.copy_from_slice(&data[p..p + 4]);
    let sequence_number = u32::from_le_bytes(seq_bytes);
    p += 4;

    *cursor = p;

    Ok(PsbtTxIn {
        txid,
        index,
        script,
        sequence_number,
    })
}

fn parse_txout_internal(data: &[u8], cursor: &mut usize) -> Result<PsbtTxOut, PsbtResult> {
    let p = *cursor;
    if p + 8 > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let mut amt_bytes = [0u8; 8];
    amt_bytes.copy_from_slice(&data[p..p + 8]);
    let amount = u64::from_le_bytes(amt_bytes);
    let mut p = p + 8;

    if p >= data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;

    let script_len = script_len as usize;
    if p + script_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let script = data[p..p + script_len].to_vec();
    p += script_len;

    *cursor = p;
    Ok(PsbtTxOut { amount, script })
}

/// Decode a PSBT-encoded source string into raw bytes.
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let bytes = src.as_bytes();
    let n = src_size.min(bytes.len());
    let dest_size = dest_size.min(dest.len());

    let b64_magic = b"cHNid";
    if n < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &bytes[..b64_magic.len()] == b64_magic {
        match base64_decode(&bytes[..n], &mut dest[..dest_size]) {
            Some(out_size) => {
                *psbt_len = out_size;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        }
    } else {
        *psbt_len = n / 2;
        psbt_hex_decode(&bytes[..n], &mut dest[..dest_size])
    }
}

fn hexdigit(c: u8) -> Option<u8> {
    if c.is_ascii_digit() {
        Some(c - b'0')
    } else if (b'a'..=b'f').contains(&c) {
        Some(c - b'a' + 10)
    } else if (b'A'..=b'F').contains(&c) {
        Some(c - b'A' + 10)
    } else {
        None
    }
}

fn psbt_hex_decode(src: &[u8], dest: &mut [u8]) -> PsbtResult {
    if src.len() % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest.len() < src.len() / 2 {
        return PsbtResult::ReadError;
    }
    for i in 0..(src.len() / 2) {
        let c1 = match hexdigit(src[i * 2]) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        let c2 = match hexdigit(src[i * 2 + 1]) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        dest[i] = (c1 << 4) | c2;
    }
    PsbtResult::Ok
}

fn hex_encode_bytes(buf: &[u8], dest: &mut [u8]) -> PsbtResult {
    if dest.len() < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }
    fn hexchar(v: u32) -> u8 {
        if v < 10 {
            b'0' + v as u8
        } else {
            b'a' + (v as u8) - 10
        }
    }
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hexchar((b >> 4) as u32);
        dest[i * 2 + 1] = hexchar((b & 0xF) as u32);
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
    let psbt_len = psbt_size(psbt);
    psbt_encode_raw(&psbt.data, psbt_len, encoding, dest, dest_size, out_len)
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
    let dest_size = dest_size.min(dest.len());
    let psbt_len = psbt_len.min(psbt_data.len());

    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode_bytes(&psbt_data[..psbt_len], &mut dest[..dest_size]);
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
    let res = psbt_close_records(psbt);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}
