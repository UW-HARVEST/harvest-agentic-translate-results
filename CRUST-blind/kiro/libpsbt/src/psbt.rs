use crate::tx::*;
use crate::compactsize::*;
use crate::base64 as b64;
use std::fmt;
use std::any::Any;

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

// --- Helper macros and functions ---

macro_rules! assert_space {
    ($psbt:expr, $s:expr) => {
        if $psbt.write_pos + $s > $psbt.data.len() {
            return PsbtResult::OobWrite;
        }
    };
}

fn hexdigit(hex: u8) -> u8 {
    if hex <= b'9' {
        hex - b'0'
    } else {
        (hex & !0x20) - b'A' + 10 // toupper then subtract
    }
}

fn hexchar(val: u8) -> u8 {
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
    for (i, &b) in buf.iter().enumerate() {
        dest[i * 2] = hexchar(b >> 4);
        dest[i * 2 + 1] = hexchar(b & 0x0f);
    }
    dest[buf.len() * 2] = 0;
    PsbtResult::Ok
}

// --- Simple tostr functions ---

pub fn psbt_size(tx: &Psbt) -> usize {
    tx.data.len()
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

pub fn psbt_geterr() -> &'static str {
    PSBT_ERRMSG
}

// --- Init / Finalize / Size ---

pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data = Vec::with_capacity(dest_size);
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    // Need 5 bytes: 4 magic + 1 separator (0xff)
    if psbt.data.len() + 5 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.extend_from_slice(&PSBT_MAGIC);
    psbt.data.push(0xff);
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if psbt.data.len() + 1 > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    psbt.data.push(0x00);
    PsbtResult::Ok
}

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

// --- Write record helpers ---

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() as u64 + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type);
    psbt.data.extend_from_slice(&buf[..size]);

    // write type
    psbt.data.push(rec.record_type);

    // write key
    psbt.data.extend_from_slice(&rec.key);

    // write value length
    let val_size = rec.val.len() as u64;
    let size = compactsize_length(val_size) as usize;
    compactsize_write(&mut buf, val_size);
    psbt.data.extend_from_slice(&buf[..size]);

    // write value
    psbt.data.extend_from_slice(&rec.val);

    PsbtResult::Ok
}

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

pub fn psbt_new_input_record_set(_psbt: &mut Psbt) -> PsbtResult {
    if _psbt.state == PsbtState::Global
        || _psbt.state == PsbtState::InputsNew
        || _psbt.state == PsbtState::Inputs
    {
        let res = psbt_close_records(_psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        _psbt.state = PsbtState::InputsNew;
        return PsbtResult::Ok;
    } else if _psbt.state != PsbtState::Inputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(_psbt)
}

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

pub fn psbt_print(psbt: &Psbt, stream: &mut dyn std::io::Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }
    for &b in &psbt.data {
        let _ = write!(stream, "{:02x}", b);
    }
    let _ = writeln!(stream);
    PsbtResult::Ok
}

// --- Encode / Decode ---

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
            match b64::base64_encode(psbt_data, dest) {
                Some(n) => {
                    *out_len = n;
                    PsbtResult::Ok
                }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match b64::base62_encode(psbt_data, dest) {
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

fn psbt_hex_decode(src: &str, dest: &mut [u8]) -> PsbtResult {
    let bytes = src.as_bytes();
    if bytes.len() % 2 != 0 {
        return PsbtResult::ReadError;
    }
    if dest.len() < bytes.len() / 2 {
        return PsbtResult::ReadError;
    }
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
    let b64_magic = b"cHNid";
    let src_bytes = src.as_bytes();

    if src_bytes.len() < b64_magic.len() {
        return PsbtResult::ReadError;
    }

    // base64 detection
    if &src_bytes[..b64_magic.len()] == &b64_magic[..] {
        match b64::base64_decode(src_bytes, dest) {
            Some(n) => {
                *psbt_len = n;
                return PsbtResult::Ok;
            }
            None => return PsbtResult::ReadError,
        }
    }

    *psbt_len = src_bytes.len() / 2;
    psbt_hex_decode(src, dest)
}

// --- psbt_read (the big state machine) ---

struct TxCounter {
    inputs: i32,
    outputs: i32,
}

fn tx_counter_handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
    let counter = user_data.downcast_mut::<TxCounter>().unwrap();
    match elem {
        PsbtTxElem::TxIn(_) => counter.inputs += 1,
        PsbtTxElem::TxOut(_) => counter.outputs += 1,
        _ => {}
    }
}

/// Read a single record from data at position pos, returning (record_type, key, val, new_pos)
fn read_record_at(data: &[u8], pos: usize) -> Result<(u8, Vec<u8>, Vec<u8>, usize), PsbtResult> {
    let mut p = pos;
    if p >= data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return Err(PsbtResult::OobWrite);
    }
    let (key_total_size, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    if key_total_size == 0 {
        return Err(PsbtResult::ReadError);
    }
    p += size_len;

    if p + key_total_size as usize > data.len() {
        return Err(PsbtResult::ReadError);
    }

    let rec_type = data[p];
    let key_size = key_total_size as usize - 1;
    let key = data[p + 1..p + 1 + key_size].to_vec();
    p += key_total_size as usize;

    // read value
    if p >= data.len() {
        return Err(PsbtResult::ReadError);
    }
    let val_size_len = compactsize_peek_length(data[p]) as usize;
    if p + val_size_len > data.len() {
        return Err(PsbtResult::OobWrite);
    }
    let (val_size, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += val_size_len;

    if p + val_size as usize > data.len() {
        return Err(PsbtResult::ReadError);
    }

    let val = data[p..p + val_size as usize].to_vec();
    p += val_size as usize;

    Ok((rec_type, key, val, p))
}

pub fn psbt_read(
    src: &[u8],
    _src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    if psbt.state != PsbtState::Init {
        return PsbtResult::InvalidState;
    }

    if src.len() > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }

    psbt.data = src.to_vec();
    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    psbt.data_capacity = src.len();

    let mut data = psbt.data.clone();
    data.push(0); // C code relies on buffer being larger and zero-filled
    let data_len = data.len();
    let mut pos: usize = 0;
    let mut kvs: i32 = 0;
    let mut state = PsbtState::Init;
    let mut num_inputs: i32 = 0;
    let mut num_outputs: i32 = 0;

    while state != PsbtState::Finalized && pos <= data_len {
        match state {
            PsbtState::Init => {
                if pos + 4 > data_len {
                    return PsbtResult::OobWrite;
                }
                if data[pos..pos + 4] != PSBT_MAGIC {
                    return PsbtResult::ReadError;
                }
                pos += 4;
                if pos >= data_len || data[pos] != 0xff {
                    return PsbtResult::ReadError;
                }
                pos += 1;
                state = PsbtState::Global;
            }

            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                if pos >= data_len {
                    break;
                }
                if data[pos] == 0 {
                    match state {
                        PsbtState::Global => {
                            state = PsbtState::InputsNew;
                        }
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
                    let (rec_type, key, val, new_pos) = match read_record_at(&data, pos) {
                        Ok(r) => r,
                        Err(e) => return e,
                    };
                    pos = new_pos;

                    let scope = match state {
                        PsbtState::Global => PsbtScope::Global,
                        PsbtState::Inputs => PsbtScope::Inputs,
                        PsbtState::Outputs => PsbtScope::Outputs,
                        _ => return PsbtResult::InvalidState,
                    };

                    // If global unsigned tx, parse to count inputs/outputs
                    if matches!(state, PsbtState::Global) && rec_type == 0 {
                        let mut counter = TxCounter { inputs: 0, outputs: 0 };
                        let tx_res = psbt_btc_tx_parse(
                            &val,
                            val.len(),
                            &mut counter as &mut dyn Any,
                            Some(tx_counter_handler),
                        );
                        if tx_res != PsbtResult::Ok {
                            return tx_res;
                        }
                        num_inputs = counter.inputs;
                        num_outputs = counter.outputs;

                        // Also forward txelem events to user handler
                        // (The C code does this via tx_counter callback)
                    }

                    // record callback
                    if let Some(handler) = elem_handler {
                        let rec = PsbtRecord {
                            record_type: rec_type,
                            key,
                            val,
                            scope,
                        };
                        let mut elem = PsbtElem::Record { index: kvs, record: rec };
                        handler(&mut elem, user_data);
                    }
                }
            }

            PsbtState::InputsNew => {
                if pos >= data_len || data[pos] != 0 {
                    return PsbtResult::InvalidState;
                }
                pos += 1;
                state = PsbtState::Inputs;
            }

            PsbtState::OutputsNew => {
                if pos >= data_len || data[pos] != 0 {
                    return PsbtResult::InvalidState;
                }
                pos += 1;
                state = PsbtState::Outputs;
            }

            PsbtState::Finalized => break,
        }
    }

    if state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }

    if pos >= data_len || data[pos] != 0 {
        return PsbtResult::ReadError;
    }
    pos += 1;

    psbt.state = PsbtState::Finalized;
    psbt.write_pos = pos;

    PsbtResult::Ok
}
