use super::compactsize::{compactsize_peek_length, compactsize_read};
use super::psbt::{set_psbt_errmsg, PsbtResult};

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

fn parse_le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[..4].try_into().expect("slice already bounds-checked"))
}

fn parse_le64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("slice already bounds-checked"))
}

fn assert_space(cursor: usize, needed: usize, data_size: usize) -> Result<(), PsbtResult> {
    if cursor.checked_add(needed).map_or(true, |end| end > data_size) {
        set_psbt_errmsg("out of bounds");
        Err(PsbtResult::ReadError)
    } else {
        Ok(())
    }
}

fn parse_txin(data: &[u8], cursor: &mut usize) -> Result<PsbtTxIn, PsbtResult> {
    assert_space(*cursor, 32, data.len())?;
    let txid = data[*cursor..*cursor + 32].to_vec();
    *cursor += 32;

    assert_space(*cursor, 4, data.len())?;
    let index = parse_le32(&data[*cursor..]);
    *cursor += 4;

    assert_space(*cursor, 1, data.len())?;
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    assert_space(*cursor, size_len, data.len())?;
    let (script_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let script_len = script_len as usize;
    assert_space(*cursor, script_len, data.len())?;
    let script = if script_len == 0 {
        Vec::new()
    } else {
        data[*cursor..*cursor + script_len].to_vec()
    };
    *cursor += script_len;

    assert_space(*cursor, 4, data.len())?;
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
    assert_space(*cursor, 8, data.len())?;
    let amount = parse_le64(&data[*cursor..]);
    *cursor += 8;

    assert_space(*cursor, 1, data.len())?;
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    assert_space(*cursor, size_len, data.len())?;
    let (script_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let script_len = script_len as usize;
    assert_space(*cursor, script_len, data.len())?;
    let script = data[*cursor..*cursor + script_len].to_vec();
    *cursor += script_len;

    Ok(PsbtTxOut { amount, script })
}

fn parse_witness_item(data: &[u8], cursor: &mut usize) -> Result<PsbtWitnessItem, PsbtResult> {
    assert_space(*cursor, 1, data.len())?;
    let size_len = compactsize_peek_length(data[*cursor]) as usize;
    assert_space(*cursor, size_len, data.len())?;
    let (item_len, res) = compactsize_read(&data[*cursor..]);
    if res != PsbtResult::Ok {
        return Err(res);
    }
    *cursor += size_len;

    let item_len = item_len as usize;
    assert_space(*cursor, item_len, data.len())?;
    let item = data[*cursor..*cursor + item_len].to_vec();
    *cursor += item_len;

    Ok(PsbtWitnessItem {
        input_index: 0,
        item_index: 0,
        item,
    })
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = if tx.len() >= tx_size { &tx[..tx_size] } else { tx };
    let mut cursor = 0usize;

    if let Err(err) = assert_space(cursor, 4, data.len()) {
        return err;
    }
    let version = parse_le32(&data[cursor..]);
    cursor += 4;

    if let Err(err) = assert_space(cursor, 1, data.len()) {
        return err;
    }
    let size_len = compactsize_peek_length(data[cursor]) as usize;
    if let Err(err) = assert_space(cursor, size_len, data.len()) {
        return err;
    }
    let (count, res) = compactsize_read(&data[cursor..]);
    if res != PsbtResult::Ok {
        return res;
    }
    cursor += size_len;

    let inputs = count as usize;
    for _ in 0..count {
        let txin = match parse_txin(data, &mut cursor) {
            Ok(txin) => txin,
            Err(err) => return err,
        };
        if let Some(handler) = handler {
            let mut elem = PsbtTxElem::TxIn(txin);
            handler(&mut elem, user_data);
        }
    }

    if let Err(err) = assert_space(cursor, 1, data.len()) {
        return err;
    }
    let size_len = compactsize_peek_length(data[cursor]) as usize;
    if let Err(err) = assert_space(cursor, size_len, data.len()) {
        return err;
    }
    let (count, res) = compactsize_read(&data[cursor..]);
    if res != PsbtResult::Ok {
        return res;
    }
    cursor += size_len;

    for _ in 0..count {
        let txout = match parse_txout(data, &mut cursor) {
            Ok(txout) => txout,
            Err(err) => return err,
        };
        if let Some(handler) = handler {
            let mut elem = PsbtTxElem::TxOut(txout);
            handler(&mut elem, user_data);
        }
    }

    if false && inputs > 0 {
        for input_index in 0..inputs {
            if let Err(err) = assert_space(cursor, 1, data.len()) {
                return err;
            }
            let size_len = compactsize_peek_length(data[cursor]) as usize;
            if let Err(err) = assert_space(cursor, size_len, data.len()) {
                return err;
            }
            let (count, res) = compactsize_read(&data[cursor..]);
            if res != PsbtResult::Ok {
                return res;
            }
            cursor += size_len;

            for item_index in 0..count {
                let mut item = match parse_witness_item(data, &mut cursor) {
                    Ok(item) => item,
                    Err(err) => return err,
                };
                item.input_index = input_index as i32;
                item.item_index = item_index as i32;
                if let Some(handler) = handler {
                    let mut elem = PsbtTxElem::WitnessItem(item);
                    handler(&mut elem, user_data);
                }
            }
        }
    }

    if let Err(err) = assert_space(cursor, 4, data.len()) {
        return err;
    }
    let lock_time = parse_le32(&data[cursor..]);
    cursor += 4;

    if cursor != data.len() {
        set_psbt_errmsg("psbt_btc_tx_parse: parsing fell short");
        return PsbtResult::ReadError;
    }

    if let Some(handler) = handler {
        let mut elem = PsbtTxElem::Tx(PsbtTx { version, lock_time });
        handler(&mut elem, user_data);
    }

    PsbtResult::Ok
}
