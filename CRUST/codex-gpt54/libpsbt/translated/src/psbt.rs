use crate::base64::{base62_encode, base64_decode, base64_encode};
use crate::compactsize::{
    compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write,
};
use crate::tx::*;
use std::any::Any;
use std::fmt;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

// Common constant from common.h
pub const MAX_SERIALIZE_SIZE: u32 = 0x02000000;

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

#[derive(Debug, PartialEq, Eq)]
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

/// Translates the C struct psbt.
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

/// Translates the C struct psbt_record.
pub struct PsbtRecord {
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
pub type PsbtElemHandler = fn(elem: &mut PsbtElem, user_data: &mut dyn Any);

pub const PSBT_MAGIC: [u8; 4] = [0x70, 0x73, 0x62, 0x74];

static ERRMSG: OnceLock<Mutex<&'static str>> = OnceLock::new();

fn err_cell() -> &'static Mutex<&'static str> {
    ERRMSG.get_or_init(|| Mutex::new("psbt error"))
}

pub(crate) fn set_psbt_errmsg(msg: &'static str) {
    *err_cell().lock().expect("psbt error mutex poisoned") = msg;
}

fn clone_record(record: &PsbtRecord) -> PsbtRecord {
    PsbtRecord {
        record_type: record.record_type,
        key: record.key.clone(),
        val: record.val.clone(),
        scope: match record.scope {
            PsbtScope::Global => PsbtScope::Global,
            PsbtScope::Inputs => PsbtScope::Inputs,
            PsbtScope::Outputs => PsbtScope::Outputs,
        },
    }
}

fn clone_txelem(elem: &PsbtTxElem) -> PsbtTxElem {
    match elem {
        PsbtTxElem::TxIn(txin) => PsbtTxElem::TxIn(PsbtTxIn {
            txid: txin.txid.clone(),
            index: txin.index,
            script: txin.script.clone(),
            sequence_number: txin.sequence_number,
        }),
        PsbtTxElem::TxOut(txout) => PsbtTxElem::TxOut(PsbtTxOut {
            amount: txout.amount,
            script: txout.script.clone(),
        }),
        PsbtTxElem::Tx(tx) => PsbtTxElem::Tx(PsbtTx {
            version: tx.version,
            lock_time: tx.lock_time,
        }),
        PsbtTxElem::WitnessItem(item) => PsbtTxElem::WitnessItem(PsbtWitnessItem {
            input_index: item.input_index,
            item_index: item.item_index,
            item: item.item.clone(),
        }),
    }
}

fn ensure_write_space(psbt: &Psbt, size: usize) -> Result<(), PsbtResult> {
    if psbt
        .write_pos
        .checked_add(size)
        .map_or(true, |end| end > psbt.data_capacity)
    {
        set_psbt_errmsg("write out of bounds");
        Err(PsbtResult::OobWrite)
    } else {
        Ok(())
    }
}

fn push_bytes(psbt: &mut Psbt, bytes: &[u8]) {
    psbt.data.extend_from_slice(bytes);
    psbt.write_pos += bytes.len();
}

fn psbt_write_header(psbt: &mut Psbt) -> PsbtResult {
    if ensure_write_space(psbt, PSBT_MAGIC.len() + 1).is_err() {
        return PsbtResult::OobWrite;
    }
    push_bytes(psbt, &PSBT_MAGIC);
    push_bytes(psbt, &[0xff]);
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    if ensure_write_space(psbt, 1).is_err() {
        return PsbtResult::OobWrite;
    }
    push_bytes(psbt, &[0]);
    PsbtResult::Ok
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = rec.key.len() + 1;
    let key_len_size = compactsize_length(key_size_with_type as u64) as usize;
    if ensure_write_space(psbt, key_len_size).is_err() {
        return PsbtResult::OobWrite;
    }
    let mut encoded_len = vec![0u8; key_len_size];
    compactsize_write(&mut encoded_len, key_size_with_type as u64);
    push_bytes(psbt, &encoded_len);

    if ensure_write_space(psbt, 1 + rec.key.len()).is_err() {
        return PsbtResult::OobWrite;
    }
    push_bytes(psbt, &[rec.record_type]);
    push_bytes(psbt, &rec.key);

    let val_len_size = compactsize_length(rec.val.len() as u64) as usize;
    if ensure_write_space(psbt, val_len_size).is_err() {
        return PsbtResult::OobWrite;
    }
    let mut encoded_len = vec![0u8; val_len_size];
    compactsize_write(&mut encoded_len, rec.val.len() as u64);
    push_bytes(psbt, &encoded_len);

    if ensure_write_space(psbt, rec.val.len()).is_err() {
        return PsbtResult::OobWrite;
    }
    push_bytes(psbt, &rec.val);

    psbt.records.push(clone_record(rec));
    PsbtResult::Ok
}

fn psbt_read_header(psbt: &mut Psbt) -> PsbtResult {
    if psbt.write_pos + 4 > psbt.data.len() {
        set_psbt_errmsg("write out of bounds");
        return PsbtResult::OobWrite;
    }
    if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
        set_psbt_errmsg("psbt_read: invalid magic header");
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 4;

    if psbt.data.get(psbt.write_pos).copied() != Some(0xff) {
        set_psbt_errmsg("psbt_read: no 0xff found after magic");
        return PsbtResult::ReadError;
    }
    psbt.write_pos += 1;
    psbt.state = PsbtState::Global;
    PsbtResult::Ok
}

fn read_record(psbt: &mut Psbt, src_size: usize) -> Result<PsbtRecord, PsbtResult> {
    let Some(&first) = psbt.data.get(psbt.write_pos) else {
        set_psbt_errmsg("write out of bounds");
        return Err(PsbtResult::OobWrite);
    };
    let size_len = compactsize_peek_length(first) as usize;
    if psbt.write_pos + size_len > src_size {
        set_psbt_errmsg("write out of bounds");
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize_read(&psbt.data[psbt.write_pos..src_size]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    if size == 0 {
        set_psbt_errmsg("psbt_read: record key size too large");
        return Err(PsbtResult::ReadError);
    }

    psbt.write_pos += size_len;
    let size = size as usize;
    if psbt.write_pos + size > src_size {
        set_psbt_errmsg("psbt_read: record key size too large");
        return Err(PsbtResult::ReadError);
    }

    let record_type = psbt.data[psbt.write_pos];
    let key = psbt.data[psbt.write_pos + 1..psbt.write_pos + size].to_vec();
    psbt.write_pos += size;

    let scope = match psbt.state {
        PsbtState::Global => PsbtScope::Global,
        PsbtState::Inputs => PsbtScope::Inputs,
        PsbtState::Outputs => PsbtScope::Outputs,
        _ => {
            set_psbt_errmsg("psbt_read_record: invalid record state");
            return Err(PsbtResult::InvalidState);
        }
    };

    let Some(&first) = psbt.data.get(psbt.write_pos) else {
        set_psbt_errmsg("write out of bounds");
        return Err(PsbtResult::OobWrite);
    };
    let size_len = compactsize_peek_length(first) as usize;
    if psbt.write_pos + size_len > src_size {
        set_psbt_errmsg("write out of bounds");
        return Err(PsbtResult::OobWrite);
    }
    let (val_size, res) = compactsize_read(&psbt.data[psbt.write_pos..src_size]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;

    let val_size = val_size as usize;
    if psbt.write_pos + val_size > src_size {
        set_psbt_errmsg("psbt_read: record value size too large");
        return Err(PsbtResult::ReadError);
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

struct TxCollector {
    inputs: usize,
    outputs: usize,
    events: Vec<PsbtTxElem>,
}

fn collect_txelem(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
    let collector = user_data
        .downcast_mut::<TxCollector>()
        .expect("collector type mismatch");
    match elem {
        PsbtTxElem::TxIn(_) => collector.inputs += 1,
        PsbtTxElem::TxOut(_) => collector.outputs += 1,
        _ => {}
    }
    collector.events.push(clone_txelem(elem));
}

fn hex_decode(src: &str, dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if src.len() % 2 != 0 {
        set_psbt_errmsg("psbt_decode: invalid hex string");
        return PsbtResult::ReadError;
    }
    if dest_size < src.len() / 2 {
        set_psbt_errmsg("psbt_decode: dest_size must be at least half the size of src_size");
        return PsbtResult::ReadError;
    }

    for (index, chunk) in src.as_bytes().chunks_exact(2).enumerate() {
        let hi = chunk[0];
        let lo = chunk[1];
        if !hi.is_ascii_hexdigit() || !lo.is_ascii_hexdigit() {
            set_psbt_errmsg("psbt_decode: invalid hex string");
            return PsbtResult::ReadError;
        }
        let hi = (hi as char).to_digit(16).expect("validated hex") as u8;
        let lo = (lo as char).to_digit(16).expect("validated hex") as u8;
        dest[index] = (hi << 4) | lo;
    }

    PsbtResult::Ok
}

fn hex_encode(buf: &[u8], dest: &mut [u8], dest_size: usize) -> PsbtResult {
    if dest_size < buf.len() * 2 + 1 {
        return PsbtResult::OobWrite;
    }

    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, byte) in buf.iter().copied().enumerate() {
        dest[i * 2] = HEX[(byte >> 4) as usize];
        dest[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    dest[buf.len() * 2] = 0;
    PsbtResult::Ok
}

/// Return the number of bytes stored in the PSBT.
pub fn psbt_size(tx: &Psbt) -> usize {
    tx.write_pos
}

pub fn psbt_read(
    src: &[u8],
    src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn Any,
) -> PsbtResult {
    if psbt.state != PsbtState::Init {
        set_psbt_errmsg("psbt_read: psbt not initialized, use psbt_init first");
        return PsbtResult::InvalidState;
    }
    if src_size > psbt.data_capacity {
        set_psbt_errmsg("psbt_read: read buffer is larger than psbt capacity");
        return PsbtResult::OobWrite;
    }
    if src.len() < src_size {
        set_psbt_errmsg("psbt_read: invalid psbt");
        return PsbtResult::ReadError;
    }

    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    psbt.records.clear();
    psbt.state = PsbtState::Init;
    psbt.write_pos = 0;
    psbt.data_capacity = src_size;

    let mut kvs = 0usize;
    let mut inputs = 0usize;
    let mut outputs = 0usize;

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= src_size {
        match psbt.state {
            PsbtState::Init => {
                let res = psbt_read_header(psbt);
                if res != PsbtResult::Ok {
                    return res;
                }
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                let byte = match psbt.data.get(psbt.write_pos).copied() {
                    Some(byte) => byte,
                    None if psbt.state == PsbtState::Outputs && psbt.write_pos == src_size => 0,
                    None => {
                        set_psbt_errmsg("psbt_read: invalid psbt");
                        return PsbtResult::InvalidState;
                    }
                };

                if byte == 0 {
                    match psbt.state {
                        PsbtState::Global => psbt.state = PsbtState::InputsNew,
                        PsbtState::Inputs => {
                            kvs += 1;
                            if kvs >= inputs {
                                psbt.state = PsbtState::OutputsNew;
                                kvs = 0;
                            } else {
                                psbt.state = PsbtState::InputsNew;
                            }
                        }
                        PsbtState::Outputs => {
                            kvs += 1;
                            if kvs >= outputs {
                                psbt.state = PsbtState::Finalized;
                            } else {
                                psbt.state = PsbtState::OutputsNew;
                            }
                        }
                        _ => {}
                    }
                } else {
                    let record = match read_record(psbt, src_size) {
                        Ok(record) => record,
                        Err(err) => return err,
                    };

                    if psbt.state == PsbtState::Global
                        && record.record_type == PsbtGlobalType::UnsignedTx as u8
                    {
                        let mut collector = TxCollector {
                            inputs: 0,
                            outputs: 0,
                            events: Vec::new(),
                        };
                        let res = psbt_btc_tx_parse(
                            &record.val,
                            record.val.len(),
                            &mut collector,
                            Some(collect_txelem),
                        );
                        if res != PsbtResult::Ok {
                            return res;
                        }
                        inputs = collector.inputs;
                        outputs = collector.outputs;

                        if let Some(handler) = elem_handler {
                            for event in collector.events {
                                let mut elem = PsbtElem::TxElem {
                                    index: 0,
                                    txelem: event,
                                };
                                handler(&mut elem, user_data);
                            }
                        }
                    }

                    psbt.records.push(clone_record(&record));
                    if let Some(handler) = elem_handler.filter(|_| matches!(record.scope, PsbtScope::Global)) {
                        let mut elem = PsbtElem::Record {
                            index: kvs as i32,
                            record: clone_record(&record),
                        };
                        handler(&mut elem, user_data);
                    }
                }
            }
            PsbtState::InputsNew => {
                if psbt.data.get(psbt.write_pos).copied() != Some(0) {
                    set_psbt_errmsg("psbt_read: invalid psbt");
                    return PsbtResult::InvalidState;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::OutputsNew => {
                if psbt.data.get(psbt.write_pos).copied() != Some(0) {
                    set_psbt_errmsg("psbt_read: invalid psbt");
                    return PsbtResult::InvalidState;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::Finalized => {}
        }
    }

    if psbt.state != PsbtState::Finalized {
        set_psbt_errmsg("psbt_read: invalid psbt");
        return PsbtResult::InvalidState;
    }
    if psbt.data.get(psbt.write_pos).copied().is_some_and(|byte| byte != 0) {
        set_psbt_errmsg("psbt_read: expected null byte at end of psbt");
        return PsbtResult::ReadError;
    }
    if psbt.write_pos < src_size {
        psbt.write_pos += 1;
    }

    PsbtResult::Ok
}

pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    if src_size < 5 {
        set_psbt_errmsg("psbt_decode: psbt too small");
        return PsbtResult::ReadError;
    }

    let src = &src[..src.len().min(src_size)];
    if src.as_bytes().starts_with(b"cHNid") {
        return match base64_decode(src.as_bytes(), dest) {
            Some(len) => {
                *psbt_len = len;
                PsbtResult::Ok
            }
            None => PsbtResult::ReadError,
        };
    }

    *psbt_len = src.len() / 2;
    hex_decode(src, dest, dest_size)
}

pub fn psbt_encode(
    psbt: &Psbt,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        set_psbt_errmsg(
            "psbt_encode: psbt not in finalized state. use psbt_read to parse an existing psbt, or the psbt_write functions to create one.",
        );
        return PsbtResult::WriteError;
    }

    psbt_encode_raw(&psbt.data[..psbt_size(psbt)], psbt_size(psbt), encoding, dest, dest_size, out_len)
}

pub fn psbt_encode_raw(
    psbt_data: &[u8],
    psbt_len: usize,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    match encoding {
        PsbtEncoding::Hex => {
            let res = hex_encode(&psbt_data[..psbt_len.min(psbt_data.len())], dest, dest_size);
            *out_len = psbt_len * 2 + 1;
            res
        }
        PsbtEncoding::Base64 => match base64_encode(&psbt_data[..psbt_len.min(psbt_data.len())], dest)
        {
            Some(len) => {
                *out_len = len;
                PsbtResult::Ok
            }
            None => {
                set_psbt_errmsg("psbt_encode: base64 encode failure");
                PsbtResult::WriteError
            }
        },
        PsbtEncoding::Base62 => match base62_encode(&psbt_data[..psbt_len.min(psbt_data.len())], dest)
        {
            Some(len) => {
                *out_len = len;
                PsbtResult::Ok
            }
            None => {
                set_psbt_errmsg("psbt_encode: base62 encode failure");
                PsbtResult::WriteError
            }
        },
        PsbtEncoding::Protobuf => PsbtResult::NotImplemented,
    }
}

pub fn psbt_geterr() -> &'static str {
    *err_cell().lock().expect("psbt error mutex poisoned")
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

pub fn psbt_txelem_type_tostr(txelem_type: PsbtTxElemType) -> &'static str {
    match txelem_type {
        PsbtTxElemType::TxIn => "TXIN",
        PsbtTxElemType::TxOut => "TXOUT",
        PsbtTxElemType::Tx => "TX",
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

pub fn psbt_write_global_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    if psbt.state == PsbtState::Init {
        let res = psbt_write_header(psbt);
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::Global;
    } else if psbt.state != PsbtState::Global {
        set_psbt_errmsg(
            "psbt_write_global_record: you can only write a global record after psbt_init and before psbt_write_input_record",
        );
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
        set_psbt_errmsg(
            "psbt_write_input_record: attempting to write an input record before any global records have been written. use psbt_write_global_record first",
        );
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
        set_psbt_errmsg(
            "psbt_write_input_record: attempting to write an input record before any global records have been written. use psbt_write_global_record first",
        );
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
        if res != PsbtResult::Ok {
            return res;
        }
        psbt.state = PsbtState::InputsNew;
        return PsbtResult::Ok;
    }

    set_psbt_errmsg(
        "psbt_new_input_record_set: this can only be called after psbt_write_global_record, psbt_new_input_record_set, or psbt_write_input_record",
    );
    PsbtResult::InvalidState
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
    }

    set_psbt_errmsg("psbt_new_output_record_set: this can only be called after writing input records");
    PsbtResult::InvalidState
}

pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.state = PsbtState::Init;
    psbt.data.clear();
    psbt.data_capacity = dest_size;
    psbt.write_pos = 0;
    psbt.records.clear();
    PsbtResult::Ok
}

pub fn psbt_print(psbt: &Psbt, stream: &mut dyn Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        set_psbt_errmsg("psbt_print: transaction is not finished");
        return PsbtResult::InvalidState;
    }

    let size = psbt_size(psbt);
    for byte in &psbt.data[..size] {
        if write!(stream, "{:02x}", byte).is_err() {
            return PsbtResult::WriteError;
        }
    }
    if writeln!(stream).is_err() {
        return PsbtResult::WriteError;
    }

    PsbtResult::Ok
}

pub fn psbt_finalize(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state != PsbtState::OutputsNew && psbt.state != PsbtState::Outputs {
        set_psbt_errmsg("psbt_finalize: no output records found");
        return PsbtResult::InvalidState;
    }

    let res = psbt_close_records(psbt);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}
