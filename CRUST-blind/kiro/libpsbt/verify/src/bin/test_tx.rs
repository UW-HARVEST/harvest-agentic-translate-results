use libpsbt::tx::*;
use libpsbt::psbt::PsbtResult;
use std::any::Any;

// The transaction from the C test.c
const TRANSACTION: [u8; 124] = [
    0x02, 0x00, 0x00, 0x00, 0x02, 0x2e, 0x8c, 0x7d, 0x8d, 0x37, 0xc4, 0x27,
    0xe0, 0x60, 0xec, 0x00, 0x2e, 0xc1, 0xc2, 0xbc, 0x30, 0x19, 0x6f, 0xc2,
    0xf7, 0x5d, 0x6a, 0x88, 0x44, 0xcb, 0xc0, 0x36, 0x51, 0xc0, 0x81, 0x43,
    0x0a, 0x01, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x96, 0xa0,
    0x4e, 0x0c, 0xc6, 0x36, 0xf3, 0x77, 0x93, 0x3e, 0x3d, 0x93, 0xac, 0xcc,
    0x62, 0x7f, 0xaa, 0xcd, 0xbc, 0xdb, 0x5a, 0x96, 0x24, 0xdf, 0x1b, 0x49,
    0x0b, 0xd0, 0x45, 0xf2, 0x4d, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff,
    0xff, 0xff, 0xff, 0x01, 0xe0, 0x2b, 0xe5, 0x0e, 0x00, 0x00, 0x00, 0x00,
    0x17, 0xa9, 0x14, 0xb5, 0x3b, 0xb0, 0xdc, 0x1d, 0xb8, 0xc8, 0xd8, 0x03,
    0xe3, 0xe3, 0x9f, 0x78, 0x4d, 0x42, 0xe4, 0x73, 0x7f, 0xfa, 0x0d, 0x87,
    0x00, 0x00, 0x00, 0x00,
];

struct TxTestData {
    txins: Vec<(u32, u32, usize)>,  // (index, sequence, script_len)
    txouts: Vec<(u64, Vec<u8>)>,     // (amount, script)
    tx: Option<(u32, u32)>,          // (version, lock_time)
}

fn tx_test_handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
    let data = user_data.downcast_mut::<TxTestData>().unwrap();
    match elem {
        PsbtTxElem::TxIn(txin) => {
            data.txins.push((txin.index, txin.sequence_number, txin.script.len()));
        }
        PsbtTxElem::TxOut(txout) => {
            data.txouts.push((txout.amount, txout.script.clone()));
        }
        PsbtTxElem::Tx(tx) => {
            data.tx = Some((tx.version, tx.lock_time));
        }
        _ => {}
    }
}

#[test]
fn test_tx_parse_counts() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };
    let res = psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut data, Some(tx_test_handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(data.txins.len(), 2);
    assert_eq!(data.txouts.len(), 1);
    assert!(data.tx.is_some());
}

#[test]
fn test_tx_parse_txin_0() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };
    psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut data, Some(tx_test_handler));
    let (index, seq, script_len) = data.txins[0];
    assert_eq!(index, 1);
    assert_eq!(seq, 4294967295);
    assert_eq!(script_len, 0);
}

#[test]
fn test_tx_parse_txin_1() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };
    psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut data, Some(tx_test_handler));
    let (index, seq, script_len) = data.txins[1];
    assert_eq!(index, 0);
    assert_eq!(seq, 4294967295);
    assert_eq!(script_len, 0);
}

#[test]
fn test_tx_parse_txout() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };
    psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut data, Some(tx_test_handler));
    let (amount, ref script) = data.txouts[0];
    assert_eq!(amount, 249900000);
    assert_eq!(script.len(), 23);
    let expected_script: Vec<u8> = vec![
        0xa9, 0x14, 0xb5, 0x3b, 0xb0, 0xdc, 0x1d, 0xb8, 0xc8, 0xd8,
        0x03, 0xe3, 0xe3, 0x9f, 0x78, 0x4d, 0x42, 0xe4, 0x73, 0x7f,
        0xfa, 0x0d, 0x87,
    ];
    assert_eq!(script, &expected_script);
}

#[test]
fn test_tx_parse_tx_elem() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };
    psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut data, Some(tx_test_handler));
    let (version, lock_time) = data.tx.unwrap();
    assert_eq!(version, 2);
    assert_eq!(lock_time, 0);
}

#[test]
fn test_tx_parse_no_handler() {
    let mut dummy: () = ();
    let res = psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut dummy, None);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_tx_parse_txin_0_txid() {
    let mut data = TxTestData { txins: vec![], txouts: vec![], tx: None };

    struct TxidData {
        txids: Vec<Vec<u8>>,
        inner: TxTestData,
    }
    let mut tdata = TxidData {
        txids: vec![],
        inner: TxTestData { txins: vec![], txouts: vec![], tx: None },
    };

    fn handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
        let data = user_data.downcast_mut::<TxidData>().unwrap();
        if let PsbtTxElem::TxIn(txin) = elem {
            data.txids.push(txin.txid.clone());
        }
    }

    psbt_btc_tx_parse(&TRANSACTION, TRANSACTION.len(), &mut tdata, Some(handler));

    // First txin txid (raw bytes, not reversed)
    let expected_txid_0: Vec<u8> = vec![
        0x2e, 0x8c, 0x7d, 0x8d, 0x37, 0xc4, 0x27, 0xe0, 0x60, 0xec, 0x00, 0x2e,
        0xc1, 0xc2, 0xbc, 0x30, 0x19, 0x6f, 0xc2, 0xf7, 0x5d, 0x6a, 0x88, 0x44,
        0xcb, 0xc0, 0x36, 0x51, 0xc0, 0x81, 0x43, 0x0a,
    ];
    assert_eq!(tdata.txids[0], expected_txid_0);
    assert_eq!(tdata.txids[0].len(), 32);
}

fn main() {}
