use super::compactsize::{compactsize_peek_length, compactsize_read};
use super::psbt::PsbtResult;

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

/// Helper to safely read a little-endian u32 at offset `off` from `tx`.
fn parse_le32(tx: &[u8], off: usize) -> Option<u32> {
    if off + 4 > tx.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        tx[off],
        tx[off + 1],
        tx[off + 2],
        tx[off + 3],
    ]))
}

/// Helper to safely read a little-endian u64 at offset `off` from `tx`.
fn parse_le64(tx: &[u8], off: usize) -> Option<u64> {
    if off + 8 > tx.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        tx[off],
        tx[off + 1],
        tx[off + 2],
        tx[off + 3],
        tx[off + 4],
        tx[off + 5],
        tx[off + 6],
        tx[off + 7],
    ]))
}

/// Parse a Bitcoin transaction.
///
/// Walks through the serialized transaction emitting transaction elements
/// (txin, txout, tx) to the provided handler. Mirrors `psbt_btc_tx_parse` in
/// `c_src/tx.c`.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data_size = tx_size.min(tx.len());
    let mut p: usize = 0;

    // version
    let version = match parse_le32(tx, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    // input count
    if p >= data_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(tx[p]) as usize;
    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&tx[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        // txid
        if p + 32 > data_size {
            return PsbtResult::ReadError;
        }
        let txid = tx[p..p + 32].to_vec();
        p += 32;

        // index
        let index = match parse_le32(tx, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        p += 4;

        // script length
        if p >= data_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(tx[p]) as usize;
        if p + size_len > data_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&tx[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;

        if p + script_len > data_size {
            return PsbtResult::ReadError;
        }
        let script = if script_len > 0 {
            tx[p..p + script_len].to_vec()
        } else {
            Vec::new()
        };
        p += script_len;

        // sequence number
        let sequence_number = match parse_le32(tx, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
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
    if p >= data_size {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(tx[p]) as usize;
    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&tx[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // parse outputs
    for _ in 0..count {
        // amount
        let amount = match parse_le64(tx, p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        p += 8;

        // script length
        if p >= data_size {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(tx[p]) as usize;
        if p + size_len > data_size {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&tx[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;

        if p + script_len > data_size {
            return PsbtResult::ReadError;
        }
        let script = tx[p..p + script_len].to_vec();
        p += script_len;

        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
            h(&mut elem, user_data);
        }
    }

    // (Witness data is not parsed in C reference because flag is always 0;
    // skip it here as well.)
    let _ = inputs;

    // lock_time
    let lock_time = match parse_le32(tx, p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };
    p += 4;

    if p != data_size {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}
