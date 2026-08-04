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

macro_rules! assert_space {
    ($p:expr, $s:expr, $data:expr) => {
        if $p + $s > $data.len() {
            return PsbtResult::ReadError;
        }
    };
}

fn parse_le32(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

fn parse_le64(data: &[u8]) -> u64 {
    u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]])
}

fn parse_txin(data: &[u8], p: &mut usize) -> Result<PsbtTxIn, PsbtResult> {
    assert_space_r(*p, 32, data)?;
    let txid = data[*p..*p + 32].to_vec();
    *p += 32;

    assert_space_r(*p, 4, data)?;
    let index = parse_le32(&data[*p..]);
    *p += 4;

    assert_space_r(*p, 1, data)?;
    let size_len = compactsize_peek_length(data[*p]) as usize;
    assert_space_r(*p, size_len, data)?;
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok { return Err(res); }
    *p += size_len;

    let sl = script_len as usize;
    let script = if sl > 0 { data[*p..*p + sl].to_vec() } else { vec![] };
    *p += sl;

    assert_space_r(*p, 4, data)?;
    let sequence_number = parse_le32(&data[*p..]);
    *p += 4;

    Ok(PsbtTxIn { txid, index, script, sequence_number })
}

fn parse_txout(data: &[u8], p: &mut usize) -> Result<PsbtTxOut, PsbtResult> {
    assert_space_r(*p, 8, data)?;
    let amount = parse_le64(&data[*p..]);
    *p += 8;

    assert_space_r(*p, 1, data)?;
    let size_len = compactsize_peek_length(data[*p]) as usize;
    assert_space_r(*p, size_len, data)?;
    let (script_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok { return Err(res); }
    *p += size_len;

    let sl = script_len as usize;
    let script = data[*p..*p + sl].to_vec();
    assert_space_r(*p, sl, data)?;
    *p += sl;

    Ok(PsbtTxOut { amount, script })
}

fn parse_witness_item(data: &[u8], p: &mut usize) -> Result<PsbtWitnessItem, PsbtResult> {
    assert_space_r(*p, 1, data)?;
    let size_len = compactsize_peek_length(data[*p]) as usize;
    assert_space_r(*p, size_len, data)?;
    let (item_len, res) = compactsize_read(&data[*p..]);
    if res != PsbtResult::Ok { return Err(res); }
    *p += size_len;

    let il = item_len as usize;
    let item = data[*p..*p + il].to_vec();
    *p += il;

    Ok(PsbtWitnessItem { input_index: 0, item_index: 0, item })
}

fn assert_space_r(p: usize, s: usize, data: &[u8]) -> Result<(), PsbtResult> {
    if p + s > data.len() { Err(PsbtResult::ReadError) } else { Ok(()) }
}

/// Parse a Bitcoin transaction.
pub fn psbt_btc_tx_parse(
    tx: &[u8],
    tx_size: usize,
    user_data: &mut dyn std::any::Any,
    handler: Option<PsbtTxElemHandler>,
) -> PsbtResult {
    let data = &tx[..tx_size];
    let mut p = 0usize;

    // version
    assert_space!(p, 4, data);
    let version = parse_le32(&data[p..]);
    p += 4;

    // input count
    assert_space!(p, 1, data);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space!(p, size_len, data);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok { return res; }
    p += size_len;

    let _inputs = count as usize;

    // parse inputs
    for _ in 0..count {
        match parse_txin(data, &mut p) {
            Ok(txin) => {
                if let Some(h) = handler {
                    h(&mut PsbtTxElem::TxIn(txin), user_data);
                }
            }
            Err(e) => return e,
        }
    }

    // output count
    assert_space!(p, 1, data);
    let size_len = compactsize_peek_length(data[p]) as usize;
    assert_space!(p, size_len, data);
    let (count, res) = compactsize_read(&data[p..]);
    if res != PsbtResult::Ok { return res; }
    p += size_len;

    // parse outputs
    for _ in 0..count {
        match parse_txout(data, &mut p) {
            Ok(txout) => {
                if let Some(h) = handler {
                    h(&mut PsbtTxElem::TxOut(txout), user_data);
                }
            }
            Err(e) => return e,
        }
    }

    // Note: flag is always 0 in the C code (never set to SEGREGATED_WITNESS_FLAG),
    // so witness parsing is never reached. We skip it here too.

    // lock_time
    assert_space!(p, 4, data);
    let lock_time = parse_le32(&data[p..]);
    p += 4;

    if p != data.len() {
        return PsbtResult::ReadError;
    }

    if let Some(h) = handler {
        h(&mut PsbtTxElem::Tx(PsbtTx { version, lock_time }), user_data);
    }

    PsbtResult::Ok
}
