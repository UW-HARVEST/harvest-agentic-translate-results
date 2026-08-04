use crate::tx::*;
use crate::compactsize::*;
use crate::base64::{base64_encode, base64_decode, base62_encode};
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
pub const PSBT_MAGIC: [u8; 4] = [0x70, 0x73, 0x62, 0x74]; // "psbt"
pub static PSBT_ERRMSG: &str = "psbt error";

pub fn psbt_size(tx: &Psbt) -> usize {
    tx.data.len()
}

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

// --- Helper: check space for writes ---
fn assert_space(psbt: &Psbt, needed: usize) -> PsbtResult {
    if psbt.data.len() + needed > psbt.data_capacity {
        return PsbtResult::OobWrite;
    }
    PsbtResult::Ok
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    if assert_space(psbt, 4) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.extend_from_slice(&PSBT_MAGIC);
    if assert_space(psbt, 1) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.push(0xff);
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if assert_space(psbt, 1) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.push(0x00);
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() as u64 + 1;

    // write key length
    let size = compactsize_length(key_size_with_type) as usize;
    if assert_space(psbt, size) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, key_size_with_type);
    psbt.data.extend_from_slice(&buf[..size]);

    // write type
    if assert_space(psbt, 1) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.push(rec.record_type);

    // write key
    if assert_space(psbt, rec.key.len()) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.extend_from_slice(&rec.key);

    // write value length
    let val_size = rec.val.len() as u64;
    let size = compactsize_length(val_size) as usize;
    if assert_space(psbt, size) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    compactsize_write(&mut buf, val_size);
    psbt.data.extend_from_slice(&buf[..size]);

    // write value
    if assert_space(psbt, rec.val.len()) != PsbtResult::Ok { return PsbtResult::OobWrite; }
    psbt.data.extend_from_slice(&rec.val);

    PsbtResult::Ok
}

pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.data.clear();
    psbt.write_pos = 0;
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    PsbtResult::Ok
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

pub fn psbt_write_global_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Init {
        psbt_write_header(psbt);
        psbt.state = PsbtState::Global;
    } else if psbt.state != PsbtState::Global {
        return PsbtResult::InvalidState;
    }
    psbt_write_record(psbt, rec)
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
    } else if psbt.state != PsbtState::Inputs {
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
    } else if psbt.state != PsbtState::Outputs {
        return PsbtResult::InvalidState;
    }
    psbt_close_records(psbt)
}

pub fn psbt_write_input_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Global {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::Inputs;
    } else if psbt.state != PsbtState::Inputs && psbt.state != PsbtState::InputsNew {
        return PsbtResult::InvalidState;
    }
    psbt_write_record(psbt, rec)
}

pub fn psbt_write_output_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Inputs {
        let res = psbt_close_records(psbt);
        if res != PsbtResult::Ok { return res; }
        psbt.state = PsbtState::Outputs;
    } else if psbt.state != PsbtState::Outputs && psbt.state != PsbtState::OutputsNew {
        return PsbtResult::InvalidState;
    }
    psbt_write_record(psbt, rec)
}

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

// --- Hex helpers ---
fn hexdigit(hex: u8) -> u8 {
    if hex <= b'9' { hex - b'0' }
    else { hex.to_ascii_uppercase() - b'A' + 10 }
}

fn hexchar(val: u8) -> u8 {
    if val < 10 { b'0' + val }
    else { b'a' + val - 10 }
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

fn psbt_hex_decode(src: &str, src_size: usize, dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if src_size % 2 != 0 { return PsbtResult::ReadError; }
    if dest_size < src_size / 2 { return PsbtResult::ReadError; }
    let bytes = src.as_bytes();
    for i in (0..src_size).step_by(2) {
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
    let src_size = src.len();
    let b64_magic = b"cHNid";
    if src_size < b64_magic.len() { return PsbtResult::ReadError; }

    if src.as_bytes()[..b64_magic.len()] == b64_magic[..] {
        match base64_decode(src.as_bytes(), dest) {
            Some(n) => { *psbt_len = n; return PsbtResult::Ok; }
            None => return PsbtResult::ReadError,
        }
    }

    *psbt_len = src_size / 2;
    psbt_hex_decode(src, src_size, dest, dest_size)
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
            match base64_encode(psbt_data, dest) {
                Some(n) => { *out_len = n; PsbtResult::Ok }
                None => PsbtResult::WriteError,
            }
        }
        PsbtEncoding::Base62 => {
            match base62_encode(psbt_data, dest) {
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
    psbt_encode_raw(&psbt.data, psbt_size(psbt), encoding, dest, dest_size, out_len)
}

// --- psbt_read ---

struct ReadContext {
    counter_inputs: i32,
    counter_outputs: i32,
    handler: Option<PsbtElemHandler>,
    // We store a raw pointer to user_data so the tx_counter fn can forward events
    user_data_ptr: *mut dyn std::any::Any,
}

fn tx_counter_fn(elem: &mut PsbtTxElem, user_data: &mut dyn std::any::Any) {
    let ctx = user_data.downcast_mut::<ReadContext>().unwrap();

    // Forward txelem events to user handler if present
    if let Some(h) = ctx.handler {
        let ud = unsafe { &mut *ctx.user_data_ptr };
        let mut psbt_elem = PsbtElem::TxElem {
            index: 0,
            txelem: take_txelem(elem),
        };
        h(&mut psbt_elem, ud);
        // put it back
        if let PsbtElem::TxElem { txelem, .. } = psbt_elem {
            *elem = txelem;
        }
    }

    match elem {
        PsbtTxElem::TxIn(_) => ctx.counter_inputs += 1,
        PsbtTxElem::TxOut(_) => ctx.counter_outputs += 1,
        _ => {}
    }
}

// Helper to temporarily take a PsbtTxElem out
fn take_txelem(elem: &mut PsbtTxElem) -> PsbtTxElem {
    std::mem::replace(elem, PsbtTxElem::Tx(PsbtTx { version: 0, lock_time: 0 }))
}

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

    psbt.data.clear();
    psbt.data.extend_from_slice(src);
    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    psbt.data_capacity = src_size;

    let mut kvs: i32 = 0;
    let mut counter_inputs: i32 = 0;
    let mut counter_outputs: i32 = 0;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= src_size {
        match psbt.state {
            PsbtState::Init => {
                if psbt.write_pos + 4 > psbt.data_capacity {
                    return PsbtResult::OobWrite;
                }
                if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 4;
                if psbt.data[psbt.write_pos] != 0xff {
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Global;
            }

            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                if psbt.write_pos >= src_size {
                    // At end of data: if we've seen all output record sets, finalize
                    if matches!(psbt.state, PsbtState::Outputs) && kvs >= counter_outputs {
                        psbt.state = PsbtState::Finalized;
                        continue;
                    }
                    break;
                }
                if psbt.data[psbt.write_pos] == 0 {
                    match psbt.state {
                        PsbtState::Global => {
                            if counter_inputs == 0 {
                                psbt.state = PsbtState::OutputsNew;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
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
                        _ => {}
                    }
                } else {
                    // read record
                    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
                    if psbt.write_pos + size_len > psbt.data_capacity {
                        return PsbtResult::OobWrite;
                    }
                    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
                    psbt.write_pos += size_len;
                    if res != PsbtResult::Ok { return res; }

                    let size = size as usize;
                    if psbt.write_pos + size > src_size { return PsbtResult::ReadError; }
                    if psbt.write_pos + size > psbt.data_capacity { return PsbtResult::OobWrite; }

                    let key_size = size - 1;
                    let rec_type = psbt.data[psbt.write_pos];
                    let key = psbt.data[psbt.write_pos + 1..psbt.write_pos + 1 + key_size].to_vec();
                    psbt.write_pos += size;

                    let scope = match psbt.state {
                        PsbtState::Global => PsbtScope::Global,
                        PsbtState::Inputs => PsbtScope::Inputs,
                        PsbtState::Outputs => PsbtScope::Outputs,
                        _ => return PsbtResult::InvalidState,
                    };

                    let size_len = compactsize_peek_length(psbt.data[psbt.write_pos]) as usize;
                    if psbt.write_pos + size_len > psbt.data_capacity {
                        return PsbtResult::OobWrite;
                    }
                    let (val_size, res) = compactsize_read(&psbt.data[psbt.write_pos..]);
                    if res != PsbtResult::Ok { return res; }
                    psbt.write_pos += size_len;

                    let val_size = val_size as usize;
                    if psbt.write_pos + val_size > src_size { return PsbtResult::ReadError; }
                    if psbt.write_pos + val_size > psbt.data_capacity { return PsbtResult::OobWrite; }

                    let val = psbt.data[psbt.write_pos..psbt.write_pos + val_size].to_vec();
                    psbt.write_pos += val_size;

                    // If global unsigned tx, parse for input/output counts
                    if matches!(psbt.state, PsbtState::Global) && rec_type == 0 {
                        let mut ctx = ReadContext {
                            counter_inputs: 0,
                            counter_outputs: 0,
                            handler: elem_handler,
                            user_data_ptr: user_data as *mut dyn std::any::Any,
                        };
                        let tx_res = psbt_btc_tx_parse(
                            &val, val.len(), &mut ctx, Some(tx_counter_fn),
                        );
                        if tx_res != PsbtResult::Ok { return tx_res; }
                        counter_inputs = ctx.counter_inputs;
                        counter_outputs = ctx.counter_outputs;
                    }

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

            PsbtState::OutputsNew => {
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }

            PsbtState::InputsNew => {
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }

            PsbtState::Finalized => break,
        }
    }

    if psbt.state != PsbtState::Finalized {
        return PsbtResult::InvalidState;
    }

    if psbt.write_pos < src_size && psbt.data[psbt.write_pos] != 0 {
        return PsbtResult::ReadError;
    }

    psbt.write_pos += 1;
    PsbtResult::Ok
}
