use crate::tx::*;
use crate::{base64, compactsize};
use std::fmt;
use std::sync::{Mutex, OnceLock};
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

fn errmsg_cell() -> &'static Mutex<&'static str> {
    static CELL: OnceLock<Mutex<&'static str>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(PSBT_ERRMSG))
}

fn set_err(msg: &'static str) {
    if let Ok(mut guard) = errmsg_cell().lock() {
        *guard = msg;
    }
}

fn get_err() -> &'static str {
    errmsg_cell()
        .lock()
        .map(|guard| *guard)
        .unwrap_or(PSBT_ERRMSG)
}

fn ensure_space(psbt: &mut Psbt, additional: usize) -> bool {
    match psbt.write_pos.checked_add(additional) {
        Some(end) if end <= psbt.data_capacity => {
            if psbt.data.len() < end {
                psbt.data.resize(end, 0);
            }
            true
        }
        _ => false,
    }
}

fn write_byte(psbt: &mut Psbt, byte: u8) -> PsbtResult {
    if !ensure_space(psbt, 1) {
        set_err("write out of bounds");
        return PsbtResult::OobWrite;
    }
    psbt.data[psbt.write_pos] = byte;
    psbt.write_pos += 1;
    PsbtResult::Ok
}

fn write_bytes(psbt: &mut Psbt, bytes: &[u8]) -> PsbtResult {
    if !ensure_space(psbt, bytes.len()) {
        set_err("write out of bounds");
        return PsbtResult::OobWrite;
    }
    let end = psbt.write_pos + bytes.len();
    psbt.data[psbt.write_pos..end].copy_from_slice(bytes);
    psbt.write_pos = end;
    PsbtResult::Ok
}

fn write_compactsize(psbt: &mut Psbt, size: u64) -> PsbtResult {
    let len = compactsize::compactsize_length(size) as usize;
    if !ensure_space(psbt, len) {
        set_err("write out of bounds");
        return PsbtResult::OobWrite;
    }
    compactsize::compactsize_write(&mut psbt.data[psbt.write_pos..psbt.write_pos + len], size);
    psbt.write_pos += len;
    PsbtResult::Ok
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

fn psbt_close_records(psbt: &mut Psbt) -> PsbtResult {
    write_byte(psbt, 0)
}

fn psbt_write_record(psbt: &mut Psbt, rec: &PsbtRecord) -> PsbtResult {
    let key_size_with_type = match u64::try_from(rec.key.len()) {
        Ok(v) => v + 1,
        Err(_) => {
            set_err("write out of bounds");
            return PsbtResult::OobWrite;
        }
    };
    let mut res = write_compactsize(psbt, key_size_with_type);
    if res != PsbtResult::Ok {
        return res;
    }

    res = write_byte(psbt, rec.record_type);
    if res != PsbtResult::Ok {
        return res;
    }

    res = write_bytes(psbt, &rec.key);
    if res != PsbtResult::Ok {
        return res;
    }

    res = write_compactsize(psbt, rec.val.len() as u64);
    if res != PsbtResult::Ok {
        return res;
    }

    write_bytes(psbt, &rec.val)
}

struct TxCounter<'a> {
    inputs: i32,
    outputs: i32,
    user_data: &'a mut dyn std::any::Any,
    handler: Option<PsbtElemHandler>,
}

fn read_record(psbt: &mut Psbt, src_size: usize) -> Result<PsbtRecord, PsbtResult> {
    let Some(&ch) = psbt.data.get(psbt.write_pos) else {
        set_err("psbt_read: invalid psbt");
        return Err(PsbtResult::ReadError);
    };
    let size_len = compactsize::compactsize_peek_length(ch) as usize;
    if psbt.write_pos + size_len > src_size {
        set_err("psbt_read: invalid psbt");
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize::compactsize_read(&psbt.data[psbt.write_pos..src_size]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    if size == 0 {
        set_err("psbt_read: record key size too large");
        return Err(PsbtResult::ReadError);
    }
    psbt.write_pos += size_len;

    let size = usize::try_from(size).map_err(|_| {
        set_err("psbt_read: record key size too large");
        PsbtResult::ReadError
    })?;
    if psbt.write_pos + size > src_size {
        set_err("psbt_read: record key size too large");
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
            set_err("psbt_read_record: invalid record state");
            return Err(PsbtResult::InvalidState);
        }
    };

    let Some(&ch) = psbt.data.get(psbt.write_pos) else {
        set_err("psbt_read: record value size too large");
        return Err(PsbtResult::ReadError);
    };
    let size_len = compactsize::compactsize_peek_length(ch) as usize;
    if psbt.write_pos + size_len > src_size {
        set_err("psbt_read: invalid psbt");
        return Err(PsbtResult::OobWrite);
    }
    let (size, res) = compactsize::compactsize_read(&psbt.data[psbt.write_pos..src_size]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    psbt.write_pos += size_len;

    let size = usize::try_from(size).map_err(|_| {
        set_err("psbt_read: record value size too large");
        PsbtResult::ReadError
    })?;
    if psbt.write_pos + size > src_size {
        set_err("psbt_read: record value size too large");
        return Err(PsbtResult::ReadError);
    }

    let val = psbt.data[psbt.write_pos..psbt.write_pos + size].to_vec();
    psbt.write_pos += size;

    Ok(PsbtRecord {
        record_type,
        key,
        val,
        scope,
    })
}

fn forward_txelem(counter: &mut TxCounter<'_>, txelem: PsbtTxElem) {
    if let Some(handler) = counter.handler {
        let mut elem = PsbtElem::TxElem {
            index: 0,
            txelem: match &txelem {
                PsbtTxElem::TxIn(v) => PsbtTxElem::TxIn(PsbtTxIn {
                    txid: v.txid.clone(),
                    index: v.index,
                    script: v.script.clone(),
                    sequence_number: v.sequence_number,
                }),
                PsbtTxElem::TxOut(v) => PsbtTxElem::TxOut(PsbtTxOut {
                    amount: v.amount,
                    script: v.script.clone(),
                }),
                PsbtTxElem::Tx(v) => PsbtTxElem::Tx(PsbtTx {
                    version: v.version,
                    lock_time: v.lock_time,
                }),
                PsbtTxElem::WitnessItem(v) => PsbtTxElem::WitnessItem(PsbtWitnessItem {
                    input_index: v.input_index,
                    item_index: v.item_index,
                    item: v.item.clone(),
                }),
            },
        };
        handler(&mut elem, counter.user_data);
    }

    match txelem {
        PsbtTxElem::TxIn(_) => counter.inputs += 1,
        PsbtTxElem::TxOut(_) => counter.outputs += 1,
        _ => {}
    }
}
/// Return the number of bytes stored in the PSBT.
pub fn psbt_size(tx: &Psbt) -> usize {
    tx.write_pos
}
/// For testing, we simulate reading by optionally calling the provided callback twice.
pub fn psbt_read(
    src: &[u8],
    src_size: usize,
    psbt: &mut Psbt,
    elem_handler: Option<PsbtElemHandler>,
    user_data: &mut dyn std::any::Any,
) -> PsbtResult {
    let src_size = src_size.min(src.len());
    if psbt.state != PsbtState::Init {
        set_err("psbt_read: psbt not initialized, use psbt_init first");
        return PsbtResult::InvalidState;
    }
    if src_size > psbt.data_capacity {
        set_err("psbt_read: read buffer is larger than psbt capacity");
        return PsbtResult::OobWrite;
    }

    psbt.data.clear();
    psbt.data.extend_from_slice(&src[..src_size]);
    psbt.write_pos = 0;
    psbt.state = PsbtState::Init;
    psbt.data_capacity = src_size;
    psbt.records.clear();

    let end = src_size;
    let mut kvs = 0i32;
    let mut counter = TxCounter {
        inputs: 0,
        outputs: 0,
        user_data,
        handler: elem_handler,
    };

    while psbt.state != PsbtState::Finalized && psbt.write_pos <= end {
        match psbt.state {
            PsbtState::Init => {
                if psbt.write_pos + 5 > end {
                    set_err("psbt_read: invalid magic header");
                    return PsbtResult::ReadError;
                }
                if psbt.data[psbt.write_pos..psbt.write_pos + 4] != PSBT_MAGIC {
                    set_err("psbt_read: invalid magic header");
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 4;
                if psbt.data.get(psbt.write_pos).copied() != Some(0xff) {
                    set_err("psbt_read: no 0xff found after magic");
                    return PsbtResult::ReadError;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Global;
            }
            PsbtState::Global | PsbtState::Inputs | PsbtState::Outputs => {
                let Some(&current) = psbt.data.get(psbt.write_pos) else {
                    break;
                };
                if current == 0 {
                    match psbt.state {
                        PsbtState::Global => psbt.state = PsbtState::InputsNew,
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
                        _ => {}
                    }
                } else {
                    let rec = match read_record(psbt, src_size) {
                        Ok(rec) => rec,
                        Err(err) => return err,
                    };

                    if psbt.state == PsbtState::Global
                        && rec.record_type == PsbtGlobalType::UnsignedTx as u8
                    {
                        let tx_res = parse_tx_with(&rec.val, rec.val.len(), |txelem| {
                            forward_txelem(&mut counter, txelem);
                        });
                        if tx_res != PsbtResult::Ok {
                            return tx_res;
                        }
                    }

                    psbt.records.push(PsbtRecord {
                        record_type: rec.record_type,
                        key: rec.key.clone(),
                        val: rec.val.clone(),
                        scope: match rec.scope {
                            PsbtScope::Global => PsbtScope::Global,
                            PsbtScope::Inputs => PsbtScope::Inputs,
                            PsbtScope::Outputs => PsbtScope::Outputs,
                        },
                    });

                    if let Some(handler) = elem_handler {
                        let mut elem = PsbtElem::Record {
                            index: kvs,
                            record: rec,
                        };
                        handler(&mut elem, counter.user_data);
                    }
                }
            }
            PsbtState::OutputsNew => {
                if psbt.data.get(psbt.write_pos).copied() != Some(0) {
                    set_err("psbt_read: invalid psbt");
                    return PsbtResult::InvalidState;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Outputs;
            }
            PsbtState::InputsNew => {
                if psbt.data.get(psbt.write_pos).copied() != Some(0) {
                    set_err("psbt_read: invalid psbt");
                    return PsbtResult::InvalidState;
                }
                psbt.write_pos += 1;
                psbt.state = PsbtState::Inputs;
            }
            PsbtState::Finalized => break,
        }
    }

    if psbt.state != PsbtState::Finalized {
        set_err("psbt_read: invalid psbt");
        return PsbtResult::InvalidState;
    } else if psbt.data.get(psbt.write_pos).copied() != Some(0) {
        set_err("psbt_read: expected null byte at end of psbt");
        return PsbtResult::ReadError;
    }

    psbt.write_pos += 1;
    PsbtResult::Ok
}
/// Decode a hex string into dest. (This simple implementation uses the `hex` crate.)
pub fn psbt_decode(
    src: &str,
    src_size: usize,
    dest: &mut [u8],
    dest_size: usize,
    psbt_len: &mut usize,
) -> PsbtResult {
    let src_bytes = src.as_bytes();
    let src_size = src_size.min(src_bytes.len());
    let b64_magic = b"cHNid";
    if src_size < b64_magic.len() {
        set_err("psbt_decode: psbt too small");
        return PsbtResult::ReadError;
    }

    if &src_bytes[..b64_magic.len()] == b64_magic {
        let usable = dest_size.min(dest.len());
        let out = match base64::base64_decode(&src_bytes[..src_size], &mut dest[..usable]) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        *psbt_len = out;
        return PsbtResult::Ok;
    }

    if src_size % 2 != 0 {
        set_err("psbt_decode: invalid hex string");
        return PsbtResult::ReadError;
    }
    if dest_size < src_size / 2 || dest.len() < src_size / 2 {
        set_err("psbt_decode: dest_size must be at least half the size of src_size");
        return PsbtResult::ReadError;
    }

    fn hexdigit(hex: u8) -> u8 {
        if hex <= b'9' {
            hex - b'0'
        } else {
            hex.to_ascii_uppercase() - b'A' + 10
        }
    }

    for i in (0..src_size).step_by(2) {
        let c1 = src_bytes[i];
        let c2 = src_bytes[i + 1];
        if !(c1 as char).is_ascii_hexdigit() || !(c2 as char).is_ascii_hexdigit() {
            set_err("psbt_decode: invalid hex string");
            return PsbtResult::ReadError;
        }
        dest[i / 2] = (hexdigit(c1) << 4) | hexdigit(c2);
    }
    *psbt_len = src_size / 2;
    PsbtResult::Ok
}
/// Encode the PSBT data into a destination buffer using the requested encoding.
/// (Only Hex encoding is implemented for simplicity.)
pub fn psbt_encode(
    psbt: &Psbt,
    encoding: PsbtEncoding,
    dest: &mut [u8],
    dest_size: usize,
    out_len: &mut usize,
) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        set_err(
            "psbt_encode: psbt not in finalized state. use psbt_read to parse an existing psbt, or the psbt_write functions to create one.",
        );
        return PsbtResult::WriteError;
    }
    psbt_encode_raw(
        &psbt.data[..psbt_size(psbt)],
        psbt_size(psbt),
        encoding,
        dest,
        dest_size,
        out_len,
    )
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
    let psbt_len = psbt_len.min(psbt_data.len());
    let data = &psbt_data[..psbt_len];
    match encoding {
        PsbtEncoding::Hex => {
            if dest_size < data.len() * 2 + 1 || dest.len() < data.len() * 2 + 1 {
                return PsbtResult::OobWrite;
            }
            const HEX: &[u8; 16] = b"0123456789abcdef";
            for (i, byte) in data.iter().enumerate() {
                dest[i * 2] = HEX[(byte >> 4) as usize];
                dest[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
            }
            dest[data.len() * 2] = 0;
            *out_len = data.len() * 2 + 1;
            PsbtResult::Ok
        }
        PsbtEncoding::Base64 => {
            let usable = dest_size.min(dest.len());
            match base64::base64_encode(data, &mut dest[..usable]) {
                Some(len) => {
                    *out_len = len;
                    PsbtResult::Ok
                }
                None => {
                    set_err("psbt_encode: base64 encode failure");
                    PsbtResult::WriteError
                }
            }
        }
        PsbtEncoding::Base62 => {
            let usable = dest_size.min(dest.len());
            match base64::base62_encode(data, &mut dest[..usable]) {
                Some(len) => {
                    *out_len = len;
                    PsbtResult::Ok
                }
                None => {
                    set_err("psbt_encode: base62 encode failure");
                    PsbtResult::WriteError
                }
            }
        }
        PsbtEncoding::Protobuf => PsbtResult::NotImplemented,
    }
}
/// Return the last error message.
pub fn psbt_geterr() -> &'static str {
    get_err()
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
        PsbtScope::Global => {
            if record_type == PsbtGlobalType::UnsignedTx as u8 {
                psbt_global_type_tostr(PsbtGlobalType::UnsignedTx)
            } else {
                "UNKNOWN_GLOBAL_TYPE"
            }
        }
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
        set_err(
            "psbt_write_global_record: you can only write a global record after psbt_init and before psbt_write_input_record",
        );
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Global,
        });
    }
    res
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
        set_err(
            "psbt_write_input_record: attempting to write an input record before any global records have been written. use psbt_write_global_record first",
        );
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Inputs,
        });
    }
    res
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
        set_err(
            "psbt_write_input_record: attempting to write an input record before any global records have been written. use psbt_write_global_record first",
        );
        return PsbtResult::InvalidState;
    }
    let res = psbt_write_record(psbt, rec);
    if res == PsbtResult::Ok {
        psbt.records.push(PsbtRecord {
            record_type: rec.record_type,
            key: rec.key.clone(),
            val: rec.val.clone(),
            scope: PsbtScope::Outputs,
        });
    }
    res
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
    }
    set_err(
        "psbt_new_input_record_set: this can only be called after psbt_write_global_record, psbt_new_input_record_set, or psbt_write_input_record",
    );
    PsbtResult::InvalidState
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
    }
    set_err("psbt_new_output_record_set: this can only be called after writing input records");
    PsbtResult::InvalidState
}
/// Initialize a PSBT using the given destination buffer.
pub fn psbt_init(psbt: &mut Psbt, _dest: &mut [u8], dest_size: usize) -> PsbtResult {
    psbt.write_pos = 0;
    psbt.data.clear();
    if psbt.data.capacity() < dest_size {
        psbt.data.reserve(dest_size - psbt.data.capacity());
    }
    psbt.data_capacity = dest_size;
    psbt.state = PsbtState::Init;
    psbt.records.clear();
    PsbtResult::Ok
}
/// Print the PSBT (only succeeds after finalization).
pub fn psbt_print(psbt: &Psbt, stream: &mut dyn std::io::Write) -> PsbtResult {
    if psbt.state != PsbtState::Finalized {
        set_err("psbt_print: transaction is not finished");
        return PsbtResult::InvalidState;
    }
    for byte in &psbt.data[..psbt_size(psbt)] {
        if write!(stream, "{byte:02x}").is_err() {
            return PsbtResult::WriteError;
        }
    }
    if stream.write_all(b"\n").is_err() {
        return PsbtResult::WriteError;
    }
    PsbtResult::Ok
}
/// Finalize the PSBT.
pub fn psbt_finalize(psbt: &mut Psbt) -> PsbtResult {
    if psbt.state != PsbtState::OutputsNew && psbt.state != PsbtState::Outputs {
        set_err("psbt_finalize: no output records found");
        return PsbtResult::InvalidState;
    }
    let res = psbt_close_records(psbt);
    if res != PsbtResult::Ok {
        return res;
    }
    psbt.state = PsbtState::Finalized;
    PsbtResult::Ok
}
