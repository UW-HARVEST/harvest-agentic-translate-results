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

const SEGREGATED_WITNESS_FLAG: u8 = 0x1;

fn parse_le32(slice: &[u8]) -> u32 {
    u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]])
}

fn parse_le64(slice: &[u8]) -> u64 {
    u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ])
}

fn assert_space(p: usize, need: usize, data_size: usize) -> bool {
    p.checked_add(need).map_or(false, |end| end <= data_size)
}

fn parse_txin(data: &[u8], cursor: &mut usize) -> Result<PsbtTxIn, PsbtResult> {
    let data_size = data.len();
    if !assert_space(*cursor, 32, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let txid = data[*cursor..*cursor + 32].to_vec();
    *cursor += 32;

    if !assert_space(*cursor, 4, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let index = parse_le32(&data[*cursor..]);
    *cursor += 4;

    if !assert_space(*cursor, 1, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    if !assert_space(*cursor, size_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let script_len = script_len as usize;
    if !assert_space(*cursor, script_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let script = if script_len > 0 {
        data[*cursor..*cursor + script_len].to_vec()
    } else {
        Vec::new()
    };
    *cursor += script_len;

    if !assert_space(*cursor, 4, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let sequence_number = parse_le32(&data[*cursor..]);
    *cursor += 4;

    Ok(PsbtTxIn {
        txid,
        index,
        script,
        sequence_number,
    })
}

fn parse_txout(data: &[u8], cursor: &mut usize) -> Result<PsbtTxOut, PsbtResult> {
    let data_size = data.len();
    if !assert_space(*cursor, 8, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let amount = parse_le64(&data[*cursor..]);
    *cursor += 8;

    if !assert_space(*cursor, 1, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    if !assert_space(*cursor, size_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let (script_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let script_len = script_len as usize;
    if !assert_space(*cursor, script_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let script = data[*cursor..*cursor + script_len].to_vec();
    *cursor += script_len;

    Ok(PsbtTxOut { amount, script })
}

fn parse_witness_item(
    data: &[u8],
    cursor: &mut usize,
) -> Result<PsbtWitnessItem, PsbtResult> {
    let data_size = data.len();
    if !assert_space(*cursor, 1, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    if !assert_space(*cursor, size_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let (item_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let item_len = item_len as usize;
    if !assert_space(*cursor, item_len, data_size) {
        return Err(PsbtResult::ReadError);
    }
    let item = data[*cursor..*cursor + item_len].to_vec();
    *cursor += item_len;

    Ok(PsbtWitnessItem {
        input_index: 0,
        item_index: 0,
        item,
    })
}

/// Parse a Bitcoin transaction. Mirrors the C `psbt_btc_tx_parse` routine and
/// invokes the optional handler for each parsed element.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    _tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data_size = tx.len();
    let mut cursor = 0usize;

    if !assert_space(cursor, 4, data_size) {
        return PsbtResult::ReadError;
    }
    let version = parse_le32(&tx[cursor..]);
    cursor += 4;

    if !assert_space(cursor, 1, data_size) {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(tx[cursor]) as usize;
    if !assert_space(cursor, size_len, data_size) {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&tx[cursor..]);
    if res != PsbtResult::Ok {
        return res;
    }
    cursor += size_len;

    let inputs_count = count as usize;

    for _ in 0..inputs_count {
        match parse_txin(tx, &mut cursor) {
            Err(e) => return e,
            Ok(txin) => {
                if let Some(h) = handler {
                    let mut elem = PsbtTxElem::TxIn(txin);
                    h(&mut elem, user_data);
                }
            }
        }
    }

    if !assert_space(cursor, 1, data_size) {
        return PsbtResult::ReadError;
    }
    let size_len = compactsize_peek_length(tx[cursor]) as usize;
    if !assert_space(cursor, size_len, data_size) {
        return PsbtResult::ReadError;
    }
    let (count, res) = compactsize_read(&tx[cursor..]);
    if res != PsbtResult::Ok {
        return res;
    }
    cursor += size_len;

    let outputs_count = count as usize;
    for _ in 0..outputs_count {
        match parse_txout(tx, &mut cursor) {
            Err(e) => return e,
            Ok(txout) => {
                if let Some(h) = handler {
                    let mut elem = PsbtTxElem::TxOut(txout);
                    h(&mut elem, user_data);
                }
            }
        }
    }

    // Witness flag is always 0 in the original C code (it is never set);
    // preserve that behaviour.
    let flag: u8 = 0;
    if flag & SEGREGATED_WITNESS_FLAG != 0 {
        for i in 0..inputs_count {
            if !assert_space(cursor, 1, data_size) {
                return PsbtResult::ReadError;
            }
            let size_len = compactsize_peek_length(tx[cursor]) as usize;
            if !assert_space(cursor, size_len, data_size) {
                return PsbtResult::ReadError;
            }
            let (count, res) = compactsize_read(&tx[cursor..]);
            if res != PsbtResult::Ok {
                return res;
            }
            cursor += size_len;

            for j in 0..(count as usize) {
                match parse_witness_item(tx, &mut cursor) {
                    Err(e) => return e,
                    Ok(mut wi) => {
                        wi.input_index = i as i32;
                        wi.item_index = j as i32;
                        if let Some(h) = handler {
                            let mut elem = PsbtTxElem::WitnessItem(wi);
                            h(&mut elem, user_data);
                        }
                    }
                }
            }
        }
    }

    if !assert_space(cursor, 4, data_size) {
        return PsbtResult::ReadError;
    }
    let lock_time = parse_le32(&tx[cursor..]);
    cursor += 4;

    if cursor != data_size {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        h(&mut elem, user_data);
    }

    PsbtResult::Ok
}
