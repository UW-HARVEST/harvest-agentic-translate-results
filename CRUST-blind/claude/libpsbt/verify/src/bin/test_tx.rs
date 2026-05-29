use libpsbt::psbt::PsbtResult;
use libpsbt::tx::{psbt_btc_tx_parse, PsbtTxElem};

#[derive(Default, Debug)]
struct Captured {
    txins: Vec<TxInRec>,
    txouts: Vec<TxOutRec>,
    txs: Vec<TxRec>,
}

#[derive(Debug)]
struct TxInRec {
    txid: Vec<u8>,
    index: u32,
    script: Vec<u8>,
    sequence_number: u32,
}

#[derive(Debug)]
struct TxOutRec {
    amount: u64,
    script: Vec<u8>,
}

#[derive(Debug)]
struct TxRec {
    version: u32,
    lock_time: u32,
}

fn capture_handler(elem: &mut PsbtTxElem, ud: &mut dyn std::any::Any) {
    let cap = ud.downcast_mut::<Captured>().unwrap();
    match elem {
        PsbtTxElem::TxIn(t) => cap.txins.push(TxInRec {
            txid: t.txid.clone(),
            index: t.index,
            script: t.script.clone(),
            sequence_number: t.sequence_number,
        }),
        PsbtTxElem::TxOut(t) => cap.txouts.push(TxOutRec {
            amount: t.amount,
            script: t.script.clone(),
        }),
        PsbtTxElem::Tx(t) => cap.txs.push(TxRec {
            version: t.version,
            lock_time: t.lock_time,
        }),
        PsbtTxElem::WitnessItem(_) => {}
    }
}

fn hex_to_bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn test_parse_minimal_tx_no_inputs_outputs() {
    // 010000000000ffffffff -> version=1, 0 inputs, 0 outputs, locktime=0xFFFFFFFF
    let bytes = hex_to_bytes("010000000000ffffffff");
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(cap.txins.len(), 0);
    assert_eq!(cap.txouts.len(), 0);
    assert_eq!(cap.txs.len(), 1);
    assert_eq!(cap.txs[0].version, 1);
    assert_eq!(cap.txs[0].lock_time, 0xFFFFFFFF);
}

#[test]
fn test_parse_minimal_tx_zero_locktime() {
    // 01000000000000000000 -> version=1, 0 inputs, 0 outputs, locktime=0
    let bytes = hex_to_bytes("01000000000000000000");
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(cap.txins.len(), 0);
    assert_eq!(cap.txouts.len(), 0);
    assert_eq!(cap.txs.len(), 1);
    assert_eq!(cap.txs[0].version, 1);
    assert_eq!(cap.txs[0].lock_time, 0);
}

#[test]
fn test_parse_bip174_unsigned_tx() {
    // From test.c "transaction" array.
    let hex = "02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000";
    let bytes = hex_to_bytes(hex);
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::Ok);

    // Expected from C tx_harness:
    assert_eq!(cap.txins.len(), 2);
    assert_eq!(cap.txouts.len(), 1);
    assert_eq!(cap.txs.len(), 1);

    // First TXIN
    assert_eq!(
        cap.txins[0].txid,
        hex_to_bytes("2e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a")
    );
    assert_eq!(cap.txins[0].index, 1);
    assert_eq!(cap.txins[0].script.len(), 0);
    assert_eq!(cap.txins[0].sequence_number, 4_294_967_295);

    // Second TXIN
    assert_eq!(
        cap.txins[1].txid,
        hex_to_bytes("96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c")
    );
    assert_eq!(cap.txins[1].index, 0);
    assert_eq!(cap.txins[1].script.len(), 0);
    assert_eq!(cap.txins[1].sequence_number, 4_294_967_295);

    // Single TXOUT
    assert_eq!(cap.txouts[0].amount, 249_900_000);
    assert_eq!(cap.txouts[0].script.len(), 23);
    assert_eq!(
        cap.txouts[0].script,
        hex_to_bytes("a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d87")
    );

    // TX
    assert_eq!(cap.txs[0].version, 2);
    assert_eq!(cap.txs[0].lock_time, 0);
}

#[test]
fn test_parse_truncated_returns_read_error() {
    // Truncated: only version
    let bytes = hex_to_bytes("01000000");
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_parse_short_for_input_count() {
    // Input count claims 1 but missing input bytes.
    let bytes = hex_to_bytes("010000000100000000");
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_parse_overruns_data_size_returns_error() {
    // Has trailing bytes after lock_time, so p != data + data_size.
    // tx with 0 inputs/outputs, locktime=0, then 1 extra byte.
    let bytes = hex_to_bytes("0100000000000000000000");
    let mut cap = Captured::default();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut cap, Some(capture_handler));
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_parse_no_handler_ok() {
    let bytes = hex_to_bytes("010000000000ffffffff");
    let mut ud: () = ();
    let res = psbt_btc_tx_parse(&bytes, bytes.len(), &mut ud, None);
    assert_eq!(res, PsbtResult::Ok);
}

fn main() {}
