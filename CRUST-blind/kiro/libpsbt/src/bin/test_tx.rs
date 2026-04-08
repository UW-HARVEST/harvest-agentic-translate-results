use libpsbt::tx::*;
use libpsbt::psbt::PsbtResult;
use std::any::Any;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

// Transaction from the BIP174 test vector
const TX_HEX: &str = "02000000022e8c7d8d37c427e060ec002ec1c2bc30196fc2f75d6a8844cbc03651c081430a0100000000ffffffff96a04e0cc636f377933e3d93accc627faacdbcdb5a9624df1b490bd045f24d2c0000000000ffffffff01e02be50e0000000017a914b53bb0dc1db8c8d803e3e39f784d42e4737ffa0d8700000000";

#[test]
fn test_tx_parse_basic() {
    let tx_data = hex_to_bytes(TX_HEX);
    let mut dummy = ();
    let res = psbt_btc_tx_parse(&tx_data, tx_data.len(), &mut dummy, None);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_tx_parse_counts_inputs_outputs() {
    let tx_data = hex_to_bytes(TX_HEX);

    struct Counter {
        inputs: i32,
        outputs: i32,
        got_tx: bool,
    }

    fn handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
        let counter = user_data.downcast_mut::<Counter>().unwrap();
        match elem {
            PsbtTxElem::TxIn(_) => counter.inputs += 1,
            PsbtTxElem::TxOut(_) => counter.outputs += 1,
            PsbtTxElem::Tx(tx) => {
                counter.got_tx = true;
                assert_eq!(tx.version, 2);
                assert_eq!(tx.lock_time, 0);
            }
            _ => {}
        }
    }

    let mut counter = Counter { inputs: 0, outputs: 0, got_tx: false };
    let res = psbt_btc_tx_parse(&tx_data, tx_data.len(), &mut counter, Some(handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(counter.inputs, 2);
    assert_eq!(counter.outputs, 1);
    assert!(counter.got_tx);
}

#[test]
fn test_tx_parse_txin_details() {
    let tx_data = hex_to_bytes(TX_HEX);

    fn handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
        let txins = user_data.downcast_mut::<Vec<(Vec<u8>, u32, u32)>>().unwrap();
        if let PsbtTxElem::TxIn(txin) = elem {
            txins.push((txin.txid.clone(), txin.index, txin.sequence_number));
        }
    }

    let mut txins: Vec<(Vec<u8>, u32, u32)> = Vec::new();
    let res = psbt_btc_tx_parse(&tx_data, tx_data.len(), &mut txins, Some(handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(txins.len(), 2);

    // First input: index=1, seq=0xffffffff
    assert_eq!(txins[0].1, 1);
    assert_eq!(txins[0].2, 0xffffffff);

    // Second input: index=0, seq=0xffffffff
    assert_eq!(txins[1].1, 0);
    assert_eq!(txins[1].2, 0xffffffff);
}

#[test]
fn test_tx_parse_txout_details() {
    let tx_data = hex_to_bytes(TX_HEX);

    fn handler(elem: &mut PsbtTxElem, user_data: &mut dyn Any) {
        let txouts = user_data.downcast_mut::<Vec<(u64, Vec<u8>)>>().unwrap();
        if let PsbtTxElem::TxOut(txout) = elem {
            txouts.push((txout.amount, txout.script.clone()));
        }
    }

    let mut txouts: Vec<(u64, Vec<u8>)> = Vec::new();
    let res = psbt_btc_tx_parse(&tx_data, tx_data.len(), &mut txouts, Some(handler));
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(txouts.len(), 1);
    assert_eq!(txouts[0].0, 0x0EE52BE0); // 249900000 satoshis
}

#[test]
fn test_tx_parse_too_short() {
    let tx_data = [0u8; 3]; // too short for version
    let mut dummy = ();
    let res = psbt_btc_tx_parse(&tx_data, tx_data.len(), &mut dummy, None);
    assert_eq!(res, PsbtResult::ReadError);
}

#[test]
fn test_tx_parse_empty() {
    let mut dummy = ();
    let res = psbt_btc_tx_parse(&[], 0, &mut dummy, None);
    assert_eq!(res, PsbtResult::ReadError);
}

fn main() {}
