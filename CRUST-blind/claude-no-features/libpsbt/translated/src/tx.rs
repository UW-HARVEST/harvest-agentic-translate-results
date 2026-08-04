use super::psbt::PsbtResult;
use crate::compactsize::{compactsize_peek_length, compactsize_read};

/// Translates the C struct psbt_txin.
pub struct PsbtTxIn {
    pub txid: Vec<u8>,
    pub index: u32,
    pub script: Vec<u8>,
    pub sequence_number: u32,
}
/// Translates the C struct psbt_txout.
pub struct PsbtTxOut {
    pub amount: u64,
    pub script: Vec<u8>,
}
/// Translates the C struct psbt_witness_item.
pub struct PsbtWitnessItem {
    pub input_index: i32,
    pub item_index: i32,
    pub item: Vec<u8>,
}
/// Translates the C struct psbt_tx.
pub struct PsbtTx {
    pub version: u32,
    pub lock_time: u32,
}
/// Translates the C union inside psbt_txelem.
pub enum PsbtTxElem {
    TxIn(PsbtTxIn),
    TxOut(PsbtTxOut),
    Tx(PsbtTx),
    WitnessItem(PsbtWitnessItem),
}
/// The handler type for psbt_txelem.
pub type PsbtTxElemHandler = fn(elem: &mut PsbtTxElem, user_data: &mut dyn std::any::Any);

const SEGREGATED_WITNESS_FLAG: u8 = 0x1;

fn parse_le32(data: &[u8], cursor: usize) -> Option<u32> {
    if cursor + 4 > data.len() {
        return None;
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[cursor..cursor + 4]);
    Some(u32::from_le_bytes(bytes))
}

fn parse_le64(data: &[u8], cursor: usize) -> Option<u64> {
    if cursor + 8 > data.len() {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[cursor..cursor + 8]);
    Some(u64::from_le_bytes(bytes))
}

fn parse_txin(data: &[u8], cursor: &mut usize) -> Result<PsbtTxIn, PsbtResult> {
    let p = *cursor;
    if p + 32 > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let txid = data[p..p + 32].to_vec();
    let mut p = p + 32;

    let index = match parse_le32(data, p) {
        Some(v) => v,
        None => return Err(PsbtResult::ReadError),
    };
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

    let sequence_number = match parse_le32(data, p) {
        Some(v) => v,
        None => return Err(PsbtResult::ReadError),
    };
    p += 4;

    *cursor = p;

    Ok(PsbtTxIn {
        txid,
        index,
        script,
        sequence_number,
    })
}

fn parse_txout(data: &[u8], cursor: &mut usize) -> Result<PsbtTxOut, PsbtResult> {
    let p = *cursor;
    let amount = match parse_le64(data, p) {
        Some(v) => v,
        None => return Err(PsbtResult::ReadError),
    };
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

fn parse_witness_item(
    data: &[u8],
    cursor: &mut usize,
) -> Result<PsbtWitnessItem, PsbtResult> {
    let mut p = *cursor;
    if p >= data.len() {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let (item_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;

    let item_len = item_len as usize;
    if p + item_len > data.len() {
        return Err(PsbtResult::ReadError);
    }
    let item = data[p..p + item_len].to_vec();
    p += item_len;

    *cursor = p;
    Ok(PsbtWitnessItem {
        input_index: 0,
        item_index: 0,
        item,
    })
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = &tx[..tx_size.min(tx.len())];
    let mut p: usize = 0;

    // version
    let version = match parse_le32(data, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    // input count
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
    let flag: u8 = 0; // matches C: never set

    // parse inputs
    for _ in 0..inputs {
        let txin = match parse_txin(data, &mut p) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxIn(txin);
            h(&mut elem, user_data);
        }
    }

    // output count
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

    // parse outputs
    for _ in 0..outputs {
        let txout = match parse_txout(data, &mut p) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxOut(txout);
            h(&mut elem, user_data);
        }
    }

    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs {
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

            for j in 0..(count as usize) {
                let mut wi = match parse_witness_item(data, &mut p) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                wi.input_index = i as i32;
                wi.item_index = j as i32;
                if let Some(h) = handler {
                    let mut elem = PsbtTxElem::WitnessItem(wi);
                    h(&mut elem, user_data);
                }
            }
        }
    }

    let lock_time = match parse_le32(data, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    if p != data.len() {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx {
            version,
            lock_time,
        });
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}
