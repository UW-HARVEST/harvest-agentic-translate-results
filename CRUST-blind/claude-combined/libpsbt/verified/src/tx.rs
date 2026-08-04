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

fn parse_le32(cursor: &[u8]) -> u32 {
    u32::from_le_bytes([cursor[0], cursor[1], cursor[2], cursor[3]])
}

fn parse_le64(cursor: &[u8]) -> u64 {
    u64::from_le_bytes([
        cursor[0], cursor[1], cursor[2], cursor[3],
        cursor[4], cursor[5], cursor[6], cursor[7],
    ])
}

/// Parse a Bitcoin transaction. Calls the handler for each parsed element.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = &tx[..tx_size];
    let mut p: usize = 0;

    macro_rules! ensure_space {
        ($n:expr) => {
            if p + ($n) > data.len() {
                return PsbtResult::ReadError;
            }
        };
    }

    ensure_space!(4);
    let version = parse_le32(&data[p..]);
    p += 4;

    ensure_space!(1);
    let mut size_len = compactsize_peek_length(data[p]) as usize;

    ensure_space!(size_len);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        // parse_txin
        ensure_space!(32);
        let txid = data[p..p + 32].to_vec();
        p += 32;

        ensure_space!(4);
        let index = parse_le32(&data[p..]);
        p += 4;

        ensure_space!(1);
        let sl = compactsize_peek_length(data[p]) as usize;
        ensure_space!(sl);
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += sl;

        let script_len = script_len as usize;
        if p + script_len > data.len() {
            return PsbtResult::ReadError;
        }
        let script = if script_len > 0 {
            data[p..p + script_len].to_vec()
        } else {
            Vec::new()
        };
        p += script_len;

        ensure_space!(4);
        let sequence_number = parse_le32(&data[p..]);
        p += 4;

        let mut elem = PsbtTxElem::TxIn(PsbtTxIn {
            txid,
            index,
            script,
            sequence_number,
        });
        if let Some(h) = handler {
            h(&mut elem, user_data);
        }
    }

    ensure_space!(1);
    size_len = compactsize_peek_length(data[p]) as usize;
    ensure_space!(size_len);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // parse outputs
    for _ in 0..count {
        ensure_space!(8);
        let amount = parse_le64(&data[p..]);
        p += 8;

        ensure_space!(1);
        let sl = compactsize_peek_length(data[p]) as usize;
        ensure_space!(sl);
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += sl;

        let script_len = script_len as usize;
        if p + script_len > data.len() {
            return PsbtResult::ReadError;
        }
        let script = data[p..p + script_len].to_vec();
        p += script_len;

        let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
        if let Some(h) = handler {
            h(&mut elem, user_data);
        }
    }

    let flag: u8 = 0;
    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs as i32 {
            ensure_space!(1);
            size_len = compactsize_peek_length(data[p]) as usize;
            ensure_space!(size_len);
            let (count, res) = compactsize_read(&data[p..]);
            if res != PsbtResult::Ok {
                return res;
            }
            p += size_len;

            for j in 0..count as i32 {
                ensure_space!(1);
                let il = compactsize_peek_length(data[p]) as usize;
                ensure_space!(il);
                let (item_len, res) = compactsize_read(&data[p..]);
                if res != PsbtResult::Ok {
                    return res;
                }
                p += il;

                let item_len = item_len as usize;
                if p + item_len > data.len() {
                    return PsbtResult::ReadError;
                }
                let item = data[p..p + item_len].to_vec();
                p += item_len;

                let mut elem = PsbtTxElem::WitnessItem(PsbtWitnessItem {
                    input_index: i,
                    item_index: j,
                    item,
                });
                if let Some(h) = handler {
                    h(&mut elem, user_data);
                }
            }
        }
    }

    ensure_space!(4);
    let lock_time = parse_le32(&data[p..]);
    p += 4;

    if p != data.len() {
        return PsbtResult::ReadError;
    }

    let mut elem = PsbtTxElem::Tx(PsbtTx {
        version,
        lock_time,
    });
    if let Some(h) = handler {
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}
