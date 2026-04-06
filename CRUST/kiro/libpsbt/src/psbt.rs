use crate::tx::*;
use crate::compactsize::*;
use crate::base64 as b64;
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
pub struct Psbt {
    pub state: PsbtState,
    pub data: Vec<u8>,
    pub write_pos: usize,
    pub data_capacity: usize,
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

pub struct PsbtRecord {
    pub record_type: u8,
    pub key: Vec<u8>,
    pub val: Vec<u8>,
    pub scope: PsbtScope,
}

pub enum PsbtElem {
    Record { index: i32, record: PsbtRecord },
    TxElem { index: i32, txelem: PsbtTxElem },
}

pub type PsbtElemHandler = fn(elem: &mut PsbtElem, user_data: &mut dyn std::any::Any);

// External constants
pub const PSBT_MAGIC: [u8; 4] = [0x70, 0x73, 0x62, 0x74];
pub static PSBT_ERRMSG: &str = "psbt error";

pub fn psbt_size(tx: &Psbt) -> usize {
    tx.data.len()
}

// Helper: ensure we can write `s` more bytes
macro_rules! assert_write_space {
    ($psbt:expr, $s:expr) => {
        if $psbt.data.len() + $s > $psbt.data_capacity {
            return PsbtResult::OobWrite;
        }
    };
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    assert_write_space!(psbt, 4);
    psbt.data.extend_from_slice(&PSBT_MAGIC);
    assert_write_space!(psbt, 1);
    psbt.data.push(0xff);
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    assert_write_space!(psbt, 1);
    psbt.data.push(0x00);
    PsbtResult::Ok
}

fn psbt_write_record_impl(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() as u64 + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    assert_write_space!(psbt, size);
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type);
    psbt.data.extend_from_slice(&buf[..size]);

    // write type
    assert_write_space!(psbt, 1);
    psbt.data.push(rec.record_type);

    // write key
    assert_write_space!(psbt, rec.key.len());
    psbt.data.extend_from_slice(&rec.key);

    // write value length
    let val_size = rec.val.len() as u64;
    let size = compactsize_length(val_size) as usize;
    assert_write_space!(psbt, size);
    compactsize_write(&mut buf, val_size);
    psbt.data.extend_from_slice(&buf[..size]);

    // write value
    assert_write_space!(psbt, rec.val.len());
    psbt.data.extend_from_slice(&rec.val);

    PsbtResult::Ok
}

pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data = Vec::with_capacity(dest_size);
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    PsbtResult::Ok
}

pub fn psbt_write_global_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Init {
        let res = psbt_write_header(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::Global;
    } else if psbt.state != PsbtState::Global {
        return PsbtResult::InvalidState;
    }
    psbt_write_record_impl(psbt, rec)
}

pub fn psbt_write_input_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Global {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::Inputs;
    } else if psbt.state != PsbtState::Inputs && psbt.state != PsbtState::InputsNew {
        return PsbtResult::InvalidState;
    }
    psbt_write_record_impl(psbt, rec)
}

pub fn psbt_write_output_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Inputs {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::Outputs;
    } else if psbt.state != PsbtState::Outputs && psbt.state != PsbtState::OutputsNew {
        return PsbtResult::InvalidState;
    }
    psbt_write_record_impl(psbt, rec)
}

pub fn psbt_new_input_record_set(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state == PsbtState::Global
        || psbt.state == PsbtState::InputsNew
        || psbt.state == PsbtState::Inputs
    {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::InputsNew;
        return PsbtResult::Ok;
    }
    if psbt.state != PsbtState::Inputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

pub fn psbt_new_output_record_set(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state == PsbtState::Inputs
        || psbt.state == PsbtState::InputsNew
        || psbt.state == PsbtState::OutputsNew
        || psbt.state == PsbtState::Outputs
    {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::OutputsNew;
        return PsbtResult::Ok;
    }
    if psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

pub fn psbt_finalize(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state != PsbtState::OutputsNew && psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    let res = psbt_close_records(psbt);
    if res != PsbtResult::Ok { return res; }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}

pub fn psbt_print(psbt: &Psbt, stream: &mut dyn std::io::Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    for b in &psbt.data {
        let _ = write!(stream, "{:02x}", b);
    }
    let _ = writeln!(stream);
    PsbtResult::Ok
}

// --- Read / Decode ---

struct CounterData {
    inputs: i32,
    outputs: i32,
}

fn count_handler(elem: &mut PsbtTxElem, ud: &mut dyn std::any::Any) {
    if let Some(cd) = ud.downcast_mut::<CounterData>() {
        match elem {
            PsbtTxElem::TxIn(_) => cd.inputs += 1,
            PsbtTxElem::TxOut(_) => cd.outputs += 1,
            _ => {}
        }
    }
}

fn read_record(data: &[u8], pos: &mut usize, end: usize, state: &PsbtState)
    -> Result<(u8, Vec<u8>, Vec<u8>, PsbtScope), PsbtResult>
{
    let size_len = compactsize_peek_length(data[*pos]) as usize;
    if *pos + size_len > end { return Err(PsbtResult::ReadError); }
    let (size, res) = compactsize_read(&data[*pos..]);
    if res != PsbtResult::Ok { return Err(res); }
    *pos += size_len;

    if size == 0 || *pos + size as usize > end {
        return Err(PsbtResult::ReadError);
    }

    let key_size = (size - 1) as usize;
    let record_type = data[*pos];
    let key = data[*pos + 1..*pos + 1 + key_size].to_vec();
    *pos += size as usize;

    let scope = match state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => return Err(PsbtResult::InvalidState),
    };

    if *pos >= end { return Err(PsbtResult::ReadError); }
    let vsize_len = compactsize_peek_length(data[*pos]) as usize;
    if *pos + vsize_len > end { return Err(PsbtResult::ReadError); }
    let (vsize, res) = compactsize_read(&data[*pos..]);
    if res != PsbtResult::Ok { return Err(res); }
    *pos += vsize_len;

    if *pos + vsize as usize > end { return Err(PsbtResult::ReadError); }
    let val = data[*pos..*pos + vsize as usize].to_vec();
    *pos += vsize as usize;

    Ok((record_type, key, val, scope))
}

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

    // The C code may read one byte past src_size (relying on a larger buffer
    // being zeroed). We append a sentinel 0 byte to replicate that behavior.
    let mut data_vec = src[..src_size].to_vec();
    data_vec.push(0);
    let data = &data_vec;
    let end = src_size;
    let mut pos = 0usize;
    let mut state = PsbtState::Init;
    let mut kvs = 0i32;
    let mut num_inputs = 0i32;
    let mut num_outputs = 0i32;

    while state != PsbtState::Finalized && pos <= end {
        match state {
            PsbtState::Init => {
                if pos + 4 > end { return PsbtResult::ReadError; }
                if data[pos..pos+4] != PSBT_MAGIC { return PsbtResult::ReadError; }
                pos += 4;
                if pos >= end || data[pos] != 0xff { return PsbtResult::ReadError; }
                pos += 1;
                state = PsbtState::Global;
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                if data[pos] == 0 {
                    match state {
                        PsbtState::Global => state = PsbtState::InputsNew,
                        PsbtState::Inputs => {
                            kvs += 1;
                            if kvs >= num_inputs {
                                state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= num_outputs {
                                state = PsbtState::Finalized;
                            } else {
                                state = PsbtState::OutputsNew;
                            }
                        }
                        _ => {}
                    }
                } else {
                    let (record_type, _key, val, _scope) =
                        match read_record(data, &mut pos, end, &state) {
                            Ok(r) => r,
                            Err(e) => return e,
                        };

                    if matches!(state, PsbtState::Global) && record_type == 0 {
                        let mut cd = CounterData { inputs: 0, outputs: 0 };
                        let res = psbt_btc_tx_parse(&val, val.len(),
                            &mut cd as &mut dyn std::any::Any, Some(count_handler));
                        if res != PsbtResult::Ok { return res; }
                        num_inputs = cd.inputs;
                        num_outputs = cd.outputs;
                    }
                }
            }
            PsbtState::InputsNew => {
                pos += 1;
                state = PsbtState::Inputs;
            }
            PsbtState::OutputsNew => {
                pos += 1;
                state = PsbtState::Outputs;
            }
            PsbtState::Finalized => break,
        }
    }

    if state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    if pos < end && data[pos] != 0 {
        return PsbtResult::ReadError;
    }
    pos += 1;

    // Store the data in psbt
    psbt.data = src[..src_size].to_vec();
    psbt.data_capacity = src_size;
    psbt.state = PsbtState::Finalized;
    psbt.write_pos = pos;

    PsbtResult::Ok
}

// --- Decode ---

fn hexdigit(c: u8) -> u8 {
    if c <= b'9' { c - b'0' }
    else { (c | 0x20) - b'a' + 10 }
}

fn psbt_hex_decode(src: &str, dest: &mut [u8]) -> PsbtResult {
    let bytes = src.as_bytes();
    if bytes.len() % 2 != 0 { return PsbtResult::ReadError; }
    if dest.len() < bytes.len() / 2 { return PsbtResult::ReadError; }
    for i in (0..bytes.len()).step_by(2) {
        let c1 = bytes[i];
        let c2 = bytes[i + 1];
        if !c1.is_ascii_hexdigit() || !c2.is_ascii_hexdigit() {
            return PsbtResult::ReadError;
        }
        dest[i / 2] = (hexdigit(c1) << 4) | hexdigit(c2);
    }
    PsbtResult::Ok
}

pub fn psbt_decode(
    src: &str,
    _src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let b64_magic = b"cHNid";

    if src_bytes.len() < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    if &src_bytes[..b64_magic.len()] == &b64_magic[..] {
        match b64::base64_decode(src_bytes, &mut dest[..dest_size]) {
            Some(n) => { *psbt_len = n; return PsbtResult::Ok; }
            None => return PsbtResult::ReadError,
        }
    }

    *psbt_len = src_bytes.len() / 2;
    psbt_hex_decode(src, dest)
}

// --- Encode ---

fn hexchar(val: u8) -> u8 {
    if val < 10 { b'0' + val } else { b'a' + val - 10 }
}

fn hex_encode(buf: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if dest_size < buf.len() * 2 + 1 { return PsbtResult::OobWrite; }
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hexchar(b >> 4);
        dest[i * 2 + 1] = hexchar(b & 0x0f);
    }
    dest[buf.len() * 2] = 0;
    PsbtResult::Ok
}

pub fn psbt_encode_raw(
    psbt_data: &[u8],
    _psbt_len: usize,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(psbt_data, dest, dest_size);
            *out_len = psbt_data.len() * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => {
            match b64::base64_encode(psbt_data, &mut dest[..dest_size]) {
                Some(n) => { *out_len = n; PsbtResult::Ok }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match b64::base62_encode(psbt_data, &mut dest[..dest_size]) {
                Some(n) => { *out_len = n; PsbtResult::Ok }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Protobuf => PsbtResult::NotImplemented,
    }
}

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

// --- String conversion functions ---

pub fn psbt_geterr() -> &'static str {
    PSBT_ERRMSG
}

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

pub fn psbt_output_type_tostr(ot: PsbtOutputType) -> &'static str {
    match ot {
        PsbtOutputType::RedeemScript => "OUT_REDEEM_SCRIPT",
        PsbtOutputType::WitnessScript => "OUT_WITNESS_SCRIPT",
        PsbtOutputType::Bip32Derivation => "OUT_BIP32_DERIVATION",
    }
}

pub fn psbt_global_type_tostr(gt: PsbtGlobalType) -> &'static str {
    match gt {
        PsbtGlobalType::UnsignedTx => "GLOBAL_UNSIGNED_TX",
    }
}

pub fn psbt_txelem_type_tostr(txelem_type: PsbtTxElemType) -> &'static str {
    match txelem_type {
        PsbtTxElemType::Tx => "TX",
        PsbtTxElemType::TxIn => "TXIN",
        PsbtTxElemType::TxOut => "TXOUT",
        PsbtTxElemType::WitnessItem => "WITNESS_ITEM",
    }
}

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
