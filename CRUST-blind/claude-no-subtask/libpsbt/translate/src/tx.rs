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
        cursor[0], cursor[1], cursor[2], cursor[3], cursor[4], cursor[5], cursor[6], cursor[7],
    ])
}

fn parse_txin(p: &mut usize, data: &[u8]) -> Result<PsbtTxIn, PsbtResult> {
    let data_size = data.len();

    if *p + 32 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let txid = data[*p..*p + 32].to_vec();
    *p += 32;

    if *p + 4 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let index = parse_le32(&data[*p..]);
    *p += 4;

    if *p + 1 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*p]) as usize;

    if *p + size_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *p += size_len;

    let script_len = script_len as usize;
    if *p + script_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let script = if script_len > 0 {
        data[*p..*p + script_len].to_vec()
    } else {
        Vec::new()
    };
    *p += script_len;

    if *p + 4 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let sequence_number = parse_le32(&data[*p..]);
    *p += 4;

    Ok(PsbtTxIn {
        txid,
        index,
        script,
        sequence_number,
    })
}

fn parse_txout(p: &mut usize, data: &[u8]) -> Result<PsbtTxOut, PsbtResult> {
    let data_size = data.len();

    if *p + 8 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let amount = parse_le64(&data[*p..]);
    *p += 8;

    if *p + 1 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*p]) as usize;

    if *p + size_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *p += size_len;

    let script_len = script_len as usize;
    if *p + script_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let script = data[*p..*p + script_len].to_vec();
    *p += script_len;

    Ok(PsbtTxOut { amount, script })
}

fn parse_witness_item(p: &mut usize, data: &[u8]) -> Result<PsbtWitnessItem, PsbtResult> {
    let data_size = data.len();

    if *p + 1 > data_size {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*p]) as usize;

    if *p + size_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let (item_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *p += size_len;

    let item_len = item_len as usize;
    if *p + item_len > data_size {
        return Err(PsbtResult::ReadError);
    }
    let item = data[*p..*p + item_len].to_vec();
    *p += item_len;

    Ok(PsbtWitnessItem {
        input_index: 0,
        item_index: 0,
        item,
    })
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    _tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = tx;
    let data_size = data.len();
    let mut p: usize = 0;

    if p + 4 > data_size {
        return PsbtResult::ReadError;
    }
    let version = parse_le32(&data[p..]);
    p += 4;

    if p + 1 > data_size {
        return PsbtResult::ReadError;
    }
    let mut size_len = compactsize_peek_length(data[p]) as usize;

    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (mut count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        let txin = match parse_txin(&mut p, data) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxIn(txin);
            h(&mut elem, user_data);
        }
    }

    if p + 1 > data_size {
        return PsbtResult::ReadError;
    }
    size_len = compactsize_peek_length(data[p]) as usize;

    if p + size_len > data_size {
        return PsbtResult::ReadError;
    }
    let (cnt, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    count = cnt;
    p += size_len;

    // parse outputs
    for _ in 0..count {
        let txout = match parse_txout(&mut p, data) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if let Some(h) = handler {
            let mut elem = PsbtTxElem::TxOut(txout);
            h(&mut elem, user_data);
        }
    }

    let flag: u8 = 0;
    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs {
            if p + 1 > data_size {
                return PsbtResult::ReadError;
            }
            size_len = compactsize_peek_length(data[p]) as usize;

            if p + size_len > data_size {
                return PsbtResult::ReadError;
            }
            let (cnt2, res) = compactsize_read(&data[p..]);
            if res != PsbtResult::Ok {
                return res;
            }
            p += size_len;

            for j in 0..cnt2 {
                let mut wi = match parse_witness_item(&mut p, data) {
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

    if p + 4 > data_size {
        return PsbtResult::ReadError;
    }
    let lock_time = parse_le32(&data[p..]);
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
