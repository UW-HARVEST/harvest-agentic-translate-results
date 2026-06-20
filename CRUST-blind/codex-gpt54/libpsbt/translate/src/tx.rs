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

fn read_u32(data: &[u8], p: &mut usize) -> Option<u32> {
    if data.len().checked_sub(*p)? < 4 {
        return None;
    }
    let out = u32::from_le_bytes(data[*p..*p + 4].try_into().ok()?);
    *p += 4;
    Some(out)
}

fn read_u64(data: &[u8], p: &mut usize) -> Option<u64> {
    if data.len().checked_sub(*p)? < 8 {
        return None;
    }
    let out = u64::from_le_bytes(data[*p..*p + 8].try_into().ok()?);
    *p += 8;
    Some(out)
}

fn parse_txin(data: &[u8], p: &mut usize) -> Option<PsbtTxIn> {
    if data.len().checked_sub(*p)? < 32 {
        return None;
    }
    let txid = data[*p..*p + 32].to_vec();
    *p += 32;
    let index = read_u32(data, p)?;
    let ch = *data.get(*p)?;
    let size_len = compactsize_peek_length(ch) as usize;
    if data.len().checked_sub(*p)? < size_len {
        return None;
    }
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return None;
    }
    *p += size_len;
    let script_len = usize::try_from(script_len).ok()?;
    if data.len().checked_sub(*p)? < script_len {
        return None;
    }
    let script = data[*p..*p + script_len].to_vec();
    *p += script_len;
    let sequence_number = read_u32(data, p)?;
    Some(PsbtTxIn {
        txid,
        index,
        script,
        sequence_number,
    })
}

fn parse_txout(data: &[u8], p: &mut usize) -> Option<PsbtTxOut> {
    let amount = read_u64(data, p)?;
    let ch = *data.get(*p)?;
    let size_len = compactsize_peek_length(ch) as usize;
    if data.len().checked_sub(*p)? < size_len {
        return None;
    }
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return None;
    }
    *p += size_len;
    let script_len = usize::try_from(script_len).ok()?;
    if data.len().checked_sub(*p)? < script_len {
        return None;
    }
    let script = data[*p..*p + script_len].to_vec();
    *p += script_len;
    Some(PsbtTxOut { amount, script })
}

fn parse_witness_item(
    data: &[u8],
    p: &mut usize,
    input_index: i32,
    item_index: i32,
) -> Option<PsbtWitnessItem> {
    let ch = *data.get(*p)?;
    let size_len = compactsize_peek_length(ch) as usize;
    if data.len().checked_sub(*p)? < size_len {
        return None;
    }
    let (item_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok {
        return None;
    }
    *p += size_len;
    let item_len = usize::try_from(item_len).ok()?;
    if data.len().checked_sub(*p)? < item_len {
        return None;
    }
    let item = data[*p..*p + item_len].to_vec();
    *p += item_len;
    Some(PsbtWitnessItem {
        input_index,
        item_index,
        item,
    })
}

pub(crate) fn parse_tx_with<F>(tx: &[u8], tx_size: usize, mut handler: F) -> PsbtResult
where
    F: FnMut(PsbtTxElem),
{
    let tx_size = tx_size.min(tx.len());
    let data = &tx[..tx_size];
    let mut p = 0usize;

    let version = match read_u32(data, &mut p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };

    let ch = match data.get(p) {
        Some(v) => *v,
        None => return PsbtResult::ReadError,
    };
    let size_len = compactsize_peek_length(ch) as usize;
    if data.len().saturating_sub(p) < size_len {
        return PsbtResult::ReadError;
    }
    let (input_count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = input_count as usize;
    for _ in 0..input_count {
        let txin = match parse_txin(data, &mut p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        handler(PsbtTxElem::TxIn(txin));
    }

    let ch = match data.get(p) {
        Some(v) => *v,
        None => return PsbtResult::ReadError,
    };
    let size_len = compactsize_peek_length(ch) as usize;
    if data.len().saturating_sub(p) < size_len {
        return PsbtResult::ReadError;
    }
    let (output_count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    for _ in 0..output_count {
        let txout = match parse_txout(data, &mut p) {
            Some(v) => v,
            None => return PsbtResult::ReadError,
        };
        handler(PsbtTxElem::TxOut(txout));
    }

    let segregated_witness_flag = 0u8;
    if (segregated_witness_flag & 0x1) != 0 {
        for i in 0..inputs {
            let ch = match data.get(p) {
                Some(v) => *v,
                None => return PsbtResult::ReadError,
            };
            let size_len = compactsize_peek_length(ch) as usize;
            if data.len().saturating_sub(p) < size_len {
                return PsbtResult::ReadError;
            }
            let (count, res) = compactsize_read(&data[p..]);
            if res != PsbtResult::Ok {
                return res;
            }
            p += size_len;

            for j in 0..count {
                let wi = match parse_witness_item(data, &mut p, i as i32, j as i32) {
                    Some(v) => v,
                    None => return PsbtResult::ReadError,
                };
                handler(PsbtTxElem::WitnessItem(wi));
            }
        }
    }

    let lock_time = match read_u32(data, &mut p) {
        Some(v) => v,
        None => return PsbtResult::ReadError,
    };

    if p != data.len() {
        return PsbtResult::ReadError;
    }

    handler(PsbtTxElem::Tx(PsbtTx { version, lock_time }));
    PsbtResult::Ok
}
/// Parse a Bitcoin transaction. (Not implemented.)
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    parse_tx_with(tx, tx_size, |elem| {
        if let Some(cb) = handler {
            let mut elem = elem;
            cb(&mut elem, user_data);
        }
    })
}
