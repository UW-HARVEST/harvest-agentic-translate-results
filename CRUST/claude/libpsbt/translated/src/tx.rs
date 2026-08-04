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

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    if tx_size > tx.len() {
        return PsbtResult::ReadError;
    }
    let data = &tx[..tx_size];
    let mut p = 0usize;

    // version
    if p + 4 > tx_size {
        return PsbtResult::ReadError;
    }
    let version = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
    p += 4;

    // input count
    if p + 1 > tx_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > tx_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // parse inputs
    for _ in 0..count {
        if p + 32 > tx_size {
            return PsbtResult::ReadError;
        }
        let txid = data[p..p + 32].to_vec();
        p += 32;
        if p + 4 > tx_size {
            return PsbtResult::ReadError;
        }
        let index = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
        p += 4;
        if p + 1 > tx_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > tx_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;
        if p + script_len > tx_size {
            return PsbtResult::ReadError;
        }
        let script = data[p..p + script_len].to_vec();
        p += script_len;
        if p + 4 > tx_size {
            return PsbtResult::ReadError;
        }
        let sequence_number =
            u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
        p += 4;
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxIn(PsbtTxIn {
                txid,
                index,
                script,
                sequence_number,
            });
            h(&mut elem, user_data);
        }
    }

    // output count
    if p + 1 > tx_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > tx_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    for _ in 0..count {
        if p + 8 > tx_size {
            return PsbtResult::ReadError;
        }
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[p..p + 8]);
        let amount = u64::from_le_bytes(amount_bytes);
        p += 8;
        if p + 1 > tx_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > tx_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;
        if p + script_len > tx_size {
            return PsbtResult::ReadError;
        }
        let script = data[p..p + script_len].to_vec();
        p += script_len;
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
            h(&mut elem, user_data);
        }
    }

    // locktime
    if p + 4 > tx_size {
        return PsbtResult::ReadError;
    }
    let lock_time = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
    p += 4;

    if p != tx_size {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}

/// Helper: count the number of inputs and outputs in a Bitcoin transaction without
/// invoking handlers. Returns (inputs, outputs) on success.
pub(crate) fn count_tx_inputs_outputs(tx_data: &[u8]) -> Result<(i32, i32), PsbtResult> {
    let tx_size = tx_data.len();
    let mut p = 0usize;

    // version
    if p + 4 > tx_size {
        return Err(PsbtResult::ReadError);
    }
    p += 4;

    // input count
    if p + 1 > tx_size {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(tx_data[p]) as usize;
    if p + size_len > tx_size {
        return Err(PsbtResult::ReadError);
    }
    let (in_count, res) = compactsize_read(&tx_data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    p += size_len;
    let inputs = in_count as i32;

    for _ in 0..in_count {
        if p + 36 > tx_size {
            return Err(PsbtResult::ReadError);
        }
        p += 36;
        if p + 1 > tx_size {
            return Err(PsbtResult::ReadError);
        }
        let size_len = compactsize_peek_length(tx_data[p]) as usize;
        if p + size_len > tx_size {
            return Err(PsbtResult::ReadError);
        }
        let (script_len, res) = compactsize_read(&tx_data[p..]);
        if res != PsbtResult::Ok {
            return Err(res);
        }
        p += size_len + script_len as usize + 4;
        if p > tx_size {
            return Err(PsbtResult::ReadError);
        }
    }

    // output count
    if p + 1 > tx_size {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(tx_data[p]) as usize;
    if p + size_len > tx_size {
        return Err(PsbtResult::ReadError);
    }
    let (out_count, res) = compactsize_read(&tx_data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    let outputs = out_count as i32;

    Ok((inputs, outputs))
}
