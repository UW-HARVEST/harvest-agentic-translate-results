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

fn parse_le32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn parse_le64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
        data[offset + 4],
        data[offset + 5],
        data[offset + 6],
        data[offset + 7],
    ])
}

/// Parse a Bitcoin transaction. The handler is called once per parsed element.
pub fn psbt_btc_tx_parse(
    _tx: &[u8],
    _tx_size: usize,
    _user_data: &mut dyn std::any::Any,
    _handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = _tx;
    let data_size = _tx_size;
    if data_size > data.len() {
        return PsbtResult::ReadError;
    }

    let mut p: usize = 0;

    // tx.version
    if p + 4 > data_size {
        return PsbtResult::ReadError;
    }
    let version = parse_le32(data, p);
    p += 4;

    // input count
    if p + 1 > data_size {
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

    let inputs = count as usize;

    // parse inputs
    for _i in 0..count {
        // txid (32 bytes)
        if p + 32 > data_size {
            return PsbtResult::ReadError;
        }
        let txid = data[p..p + 32].to_vec();
        p += 32;

        // index
        if p + 4 > data_size {
            return PsbtResult::ReadError;
        }
        let index = parse_le32(data, p);
        p += 4;

        // script_len
        if p + 1 > data_size {
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

        // sequence_number
        if p + 4 > data_size {
            return PsbtResult::ReadError;
        }
        let sequence_number = parse_le32(data, p);
        p += 4;

        if let Some(handler) = _handler {
            let mut elem = PsbtTxElem::TxIn(PsbtTxIn {
                txid,
                index,
                script,
                sequence_number,
            });
            handler(&mut elem, _user_data);
        }
    }

    // output count
    if p + 1 > data_size {
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

    // parse outputs
    for _i in 0..count {
        if p + 8 > data_size {
            return PsbtResult::ReadError;
        }
        let amount = parse_le64(data, p);
        p += 8;

        if p + 1 > data_size {
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

        if let Some(handler) = _handler {
            let mut elem = PsbtTxElem::TxOut(PsbtTxOut { amount, script });
            handler(&mut elem, _user_data);
        }
    }

    // witness items - controlled by `flag` which is initialized to 0 in the C code
    // and never assigned (a bug). We mirror that behavior: never enter this branch.
    let flag: u8 = 0;
    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs {
            if p + 1 > data_size {
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

            for j in 0..count {
                if p + 1 > data_size {
                    return PsbtResult::ReadError;
                }
                let size_len = compactsize_peek_length(data[p]) as usize;
                if p + size_len > data_size {
                    return PsbtResult::ReadError;
                }
                let (item_len, res) = compactsize_read(&data[p..]);
                if res != PsbtResult::Ok {
                    return res;
                }
                p += size_len;
                let item_len = item_len as usize;
                if p + item_len > data_size {
                    return PsbtResult::ReadError;
                }
                let item = data[p..p + item_len].to_vec();
                p += item_len;

                if let Some(handler) = _handler {
                    let mut elem = PsbtTxElem::WitnessItem(PsbtWitnessItem {
                        input_index: i as i32,
                        item_index: j as i32,
                        item,
                    });
                    handler(&mut elem, _user_data);
                }
            }
        }
    }

    // lock_time
    if p + 4 > data_size {
        return PsbtResult::ReadError;
    }
    let lock_time = parse_le32(data, p);
    p += 4;

    if p != data_size {
        return PsbtResult::ReadError;
    }

    if let Some(handler) = _handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        handler(&mut elem, _user_data);
    }

    PsbtResult::Ok
}
