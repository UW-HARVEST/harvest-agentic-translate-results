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

fn parse_le32(data: &[u8], pos: usize) -> Option<u32> {
    if pos + 4 > data.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        data[pos],
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
    ]))
}

fn parse_le64(data: &[u8], pos: usize) -> Option<u64> {
    if pos + 8 > data.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        data[pos],
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
        data[pos + 4],
        data[pos + 5],
        data[pos + 6],
        data[pos + 7],
    ]))
}

/// Internal parser that uses a closure-based handler. Used to avoid the
/// `&mut dyn Any` requirement of the public API when the caller needs to
/// pass non-'static borrows through the handler.
pub(crate) fn parse_tx_with_callback<F>(tx: &[u8], tx_size: usize, mut f: F) -> PsbtResult
where
    F: FnMut(&mut PsbtTxElem),
{
    let data_size = tx_size.min(tx.len());
    let data = &tx[..data_size];
    let mut p: usize = 0;

    // Parse version
    let version = match parse_le32(data, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    // Parse input count
    if p >= data_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // Parse inputs
    for _i in 0..count {
        // txid (32 bytes)
        if p + 32 > data_size {
            return PsbtResult::ReadError;
        }
        let txid = data[p..p + 32].to_vec();
        p += 32;

        // index
        let index = match parse_le32(data, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        p += 4;

        // script len
        if p >= data_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > data_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;

        let script_len = script_len as usize;
        if p + script_len > data_size {
            return PsbtResult::ReadError;
        }
        let script = if script_len > 0 {
            data[p..p + script_len].to_vec()
        } else {
            Vec::new()
        };
        p += script_len;

        // sequence
        let sequence_number = match parse_le32(data, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        p += 4;

        let mut elem = PsbtTxElem::TxIn(PsbtTxIn {
            txid,
            index,
            script,
            sequence_number,
        });
        f(&mut elem);
    }

    // Parse output count
    if p >= data_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // Parse outputs
    for _i in 0..count {
        // amount
        let amount = match parse_le64(data, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        p += 8;

        // script length
        if p >= data_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > data_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;

        let script_len = script_len as usize;
        if p + script_len > data_size {
            return PsbtResult::ReadError;
        }
        let script = data[p..p + script_len].to_vec();
        p += script_len;

        let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
        f(&mut elem);
    }

    // Witness flag is 0 in the C version (uninitialized to 0), so witness section is skipped.

    // Parse lock_time
    let lock_time = match parse_le32(data, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    if p != data_size {
        return PsbtResult::ReadError;
    }

    let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
    f(&mut elem);

    PsbtResult::Ok
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    parse_tx_with_callback(tx, tx_size, |elem| {
        if let Some(h) = handler {
            h(elem, user_data);
        }
    })
}
