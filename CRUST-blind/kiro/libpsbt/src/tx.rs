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

fn parse_le32(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

fn parse_le64(data: &[u8]) -> u64 {
    u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ])
}

macro_rules! assert_space {
    ($p:expr, $s:expr, $data_size:expr) => {
        if $p + $s > $data_size {
            return PsbtResult::ReadError;
        }
    };
}

macro_rules! assert_space_err {
    ($p:expr, $s:expr, $data_size:expr) => {
        if $p + $s > $data_size {
            return Err(PsbtResult::ReadError);
        }
    };
}

fn parse_txin(data: &[u8], cursor: &mut usize, data_size: usize) -> Result<PsbtTxIn, PsbtResult> {
    let p = *cursor;
    assert_space_err!(p, 32, data_size);
    let txid = data[p..p + 32].to_vec();
    let p = p + 32;

    assert_space_err!(p, 4, data_size);
    let index = parse_le32(&data[p..]);
    let p = p + 4;

    assert_space_err!(p, 1, data_size);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space_err!(p, size_len, data_size);
    let (script_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    let p = p + size_len;
    let script_len = script_len as usize;

    let script = if script_len > 0 {
        data[p..p + script_len].to_vec()
    } else {
        Vec::new()
    };
    let p = p + script_len;

    assert_space_err!(p, 4, data_size);
    let sequence_number = parse_le32(&data[p..]);
    let p = p + 4;

    *cursor = p;
    Ok(PsbtTxIn { txid, index, script, sequence_number })
}

fn parse_txout(data: &[u8], cursor: &mut usize, data_size: usize) -> Result<PsbtTxOut, PsbtResult> {
    let p = *cursor;
    assert_space_err!(p, 8, data_size);
    let amount = parse_le64(&data[p..]);
    let p = p + 8;

    assert_space_err!(p, 1, data_size);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space_err!(p, size_len, data_size);
    let (script_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    let p = p + size_len;
    let script_len = script_len as usize;

    let script = data[p..p + script_len].to_vec();
    assert_space_err!(p, script_len, data_size);
    let p = p + script_len;

    *cursor = p;
    Ok(PsbtTxOut { amount, script })
}

fn parse_witness_item(
    data: &[u8],
    cursor: &mut usize,
    data_size: usize,
) -> Result<PsbtWitnessItem, PsbtResult> {
    let p = *cursor;
    assert_space_err!(p, 1, data_size);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space_err!(p, size_len, data_size);
    let (item_len, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    let p = p + size_len;
    let item_len = item_len as usize;

    let item = data[p..p + item_len].to_vec();
    let p = p + item_len;

    *cursor = p;
    Ok(PsbtWitnessItem {
        input_index: 0,
        item_index: 0,
        item,
    })
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    _tx: &[u8],
    _tx_size: usize,
    _user_data: &mut dyn std::any::Any,
    _handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = _tx;
    let data_size = _tx_size;
    let mut p: usize = 0;

    // version
    assert_space!(p, 4, data_size);
    let version = parse_le32(&data[p..]);
    p += 4;

    // input count
    assert_space!(p, 1, data_size);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space!(p, size_len, data_size);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    let inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        match parse_txin(data, &mut p, data_size) {
            Ok(txin) => {
                if let Some(handler) = _handler {
                    let mut elem = PsbtTxElem::TxIn(txin);
                    handler(&mut elem, _user_data);
                }
            }
            Err(e) => return e,
        }
    }

    // output count
    assert_space!(p, 1, data_size);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space!(p, size_len, data_size);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok {
        return res;
    }
    p += size_len;

    // parse outputs
    for _ in 0..count {
        match parse_txout(data, &mut p, data_size) {
            Ok(txout) => {
                if let Some(handler) = _handler {
                    let mut elem = PsbtTxElem::TxOut(txout);
                    handler(&mut elem, _user_data);
                }
            }
            Err(e) => return e,
        }
    }

    // Note: flag is always 0 in the C code (never set to SEGREGATED_WITNESS_FLAG),
    // so the witness parsing block is never entered. We mirror that behavior.
    let flag: u8 = 0;
    if flag & 0x1 != 0 {
        for i in 0..inputs {
            assert_space!(p, 1, data_size);
            let size_len = compactsize_peek_length(data[p]) as usize;
            assert_space!(p, size_len, data_size);
            let (wi_count, res) = compactsize_read(&data[p..]);
            if res != PsbtResult::Ok {
                return res;
            }
            p += size_len;

            for j in 0..wi_count as usize {
                match parse_witness_item(data, &mut p, data_size) {
                    Ok(mut wi) => {
                        wi.input_index = i as i32;
                        wi.item_index = j as i32;
                        if let Some(handler) = _handler {
                            let mut elem = PsbtTxElem::WitnessItem(wi);
                            handler(&mut elem, _user_data);
                        }
                    }
                    Err(e) => return e,
                }
            }
        }
    }

    // lock_time
    assert_space!(p, 4, data_size);
    let lock_time = parse_le32(&data[p..]);
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
