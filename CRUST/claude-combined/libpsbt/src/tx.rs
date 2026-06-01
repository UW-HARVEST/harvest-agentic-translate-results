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

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = &tx[..tx_size.min(tx.len())];
    let total = data.len();
    let mut p: usize = 0;

    // version
    if p + 4 > total {
        return PsbtResult::ReadError;
    }
    let version = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
    p += 4;

    // input count
    if p + 1 > total {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > total {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;
    let inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        // txid: 32 bytes
        if p + 32 > total {
            return PsbtResult::ReadError;
        }
        let txid = data[p..p + 32].to_vec();
        p += 32;

        // index: 4 bytes
        if p + 4 > total {
            return PsbtResult::ReadError;
        }
        let index = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
        p += 4;

        // script length
        if p + 1 > total {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > total {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;

        if p + script_len > total {
            return PsbtResult::ReadError;
        }
        let script = if script_len > 0 {
            data[p..p + script_len].to_vec()
        } else {
            Vec::new()
        };
        p += script_len;

        // sequence_number: 4 bytes
        if p + 4 > total {
            return PsbtResult::ReadError;
        }
        let sequence_number = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
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
    if p + 1 > total {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(data[p]) as usize;
    if p + size_len > total {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // parse outputs
    for _ in 0..count {
        if p + 8 > total {
            return PsbtResult::ReadError;
        }
        let amount = u64::from_le_bytes([
            data[p],
            data[p + 1],
            data[p + 2],
            data[p + 3],
            data[p + 4],
            data[p + 5],
            data[p + 6],
            data[p + 7],
        ]);
        p += 8;

        if p + 1 > total {
            return PsbtResult::ReadError;
        }
        let size_len = compactsize_peek_length(data[p]) as usize;
        if p + size_len > total {
            return PsbtResult::ReadError;
        }
        let (script_len, res) = compactsize_read(&data[p..]);
        if res != PsbtResult::Ok {
            return res;
        }
        p += size_len;
        let script_len = script_len as usize;

        if p + script_len > total {
            return PsbtResult::ReadError;
        }
        let script = data[p..p + script_len].to_vec();
        p += script_len;

        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
            h(&mut elem, user_data);
        }
    }

    // The C code references a `flag` variable but never sets it (always 0) and
    // never reads any segwit marker, so the segregated-witness branch is dead.
    // We mirror that behavior: do nothing here unless the (always-zero) flag is set.
    let flag: u8 = 0;
    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs {
            if p + 1 > total {
                return PsbtResult::ReadError;
            }
            let size_len = compactsize_peek_length(data[p]) as usize;
            if p + size_len > total {
                return PsbtResult::ReadError;
            }
            let (count, res) = compactsize_read(&data[p..]);
            if res != PsbtResult::Ok {
                return res;
            }
            p += size_len;

            for j in 0..count {
                if p + 1 > total {
                    return PsbtResult::ReadError;
                }
                let size_len = compactsize_peek_length(data[p]) as usize;
                if p + size_len > total {
                    return PsbtResult::ReadError;
                }
                let (item_len, res) = compactsize_read(&data[p..]);
                if res != PsbtResult::Ok {
                    return res;
                }
                p += size_len;
                let item_len = item_len as usize;
                if p + item_len > total {
                    return PsbtResult::ReadError;
                }
                let item = data[p..p + item_len].to_vec();
                p += item_len;

                if let Some(h) = handler {
                    let mut elem = PsbtTxElem::WitnessItem(PsbtWitnessItem {
                        input_index: i as i32,
                        item_index: j as i32,
                        item,
                    });
                    h(&mut elem, user_data);
                }
            }
        }
    }

    // lock_time: 4 bytes
    if p + 4 > total {
        return PsbtResult::ReadError;
    }
    let lock_time = u32::from_le_bytes([data[p], data[p + 1], data[p + 2], data[p + 3]]);
    p += 4;

    if p != total {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}
