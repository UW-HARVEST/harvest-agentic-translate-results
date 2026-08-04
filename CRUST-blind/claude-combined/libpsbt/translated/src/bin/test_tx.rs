use libpsbt::psbt::PsbtResult;
use libpsbt::tx::{psbt_btc_tx_parse, PsbtTxElem};

const TRANSACTION: &[u8] = &[
    0x02, 0x00, 0x00, 0x00, 0x02, 0x2e, 0x8c, 0x7d, 0x8d, 0x37, 0xc4, 0x27, 0xe0, 0x60, 0xec, 0x00,
    0x2e, 0xc1, 0xc2, 0xbc, 0x30, 0x19, 0x6f, 0xc2, 0xf7, 0x5d, 0x6a, 0x88, 0x44, 0xcb, 0xc0, 0x36,
    0x51, 0xc0, 0x81, 0x43, 0x0a, 0x01, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x96, 0xa0,
    0x4e, 0x0c, 0xc6, 0x36, 0xf3, 0x77, 0x93, 0x3e, 0x3d, 0x93, 0xac, 0xcc, 0x62, 0x7f, 0xaa, 0xcd,
    0xbc, 0xdb, 0x5a, 0x96, 0x24, 0xdf, 0x1b, 0x49, 0x0b, 0xd0, 0x45, 0xf2, 0x4d, 0x2c, 0x00, 0x00,
    0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0x01, 0xe0, 0x2b, 0xe5, 0x0e, 0x00, 0x00, 0x00, 0x00,
    0x17, 0xa9, 0x14, 0xb5, 0x3b, 0xb0, 0xdc, 0x1d, 0xb8, 0xc8, 0xd8, 0x03, 0xe3, 0xe3, 0x9f, 0x78,
    0x4d, 0x42, 0xe4, 0x73, 0x7f, 0xfa, 0x0d, 0x87, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Default)]
struct Counter {
    inputs: i32,
    outputs: i32,
    witness: i32,
    tx: i32,
    version: u32,
    lock_time: u32,
    first_input_index: u32,
    first_input_seq: u32,
    first_input_script_len: usize,
    first_output_amount: u64,
    first_output_script_len: usize,
}

fn handler(elem: &mut PsbtTxElem, ud: &mut dyn std::any::Any) {
    let c = ud.downcast_mut::<Counter>().unwrap();
    match elem {
        PsbtTxElem::TxIn(ti) => {
            if c.inputs == 0 {
                c.first_input_index = ti.index;
                c.first_input_seq = ti.sequence_number;
                c.first_input_script_len = ti.script.len();
            }
            c.inputs += 1;
        }
        PsbtTxElem::TxOut(to) => {
            if c.outputs == 0 {
                c.first_output_amount = to.amount;
                c.first_output_script_len = to.script.len();
            }
            c.outputs += 1;
        }
        PsbtTxElem::WitnessItem(_) => {
            c.witness += 1;
        }
        PsbtTxElem::Tx(t) => {
            c.tx += 1;
            c.version = t.version;
            c.lock_time = t.lock_time;
        }
    }
}

#[test]
fn test_parse_transaction() {
    let mut counter = Counter::default();
    let r = psbt_btc_tx_parse(
        TRANSACTION,
        TRANSACTION.len(),
        &mut counter,
        Some(handler),
    );
    assert_eq!(r, PsbtResult::Ok);
    assert_eq!(counter.inputs, 2);
    assert_eq!(counter.outputs, 1);
    assert_eq!(counter.witness, 0);
    assert_eq!(counter.tx, 1);
    assert_eq!(counter.version, 2);
    assert_eq!(counter.lock_time, 0);
    assert_eq!(counter.first_input_index, 1);
    assert_eq!(counter.first_input_seq, 0xffffffff);
    assert_eq!(counter.first_input_script_len, 0);
    assert_eq!(counter.first_output_amount, 249900000);
    assert_eq!(counter.first_output_script_len, 23);
}

#[test]
fn test_parse_truncated_returns_error() {
    let mut counter = Counter::default();
    let r = psbt_btc_tx_parse(
        &TRANSACTION[..10],
        10,
        &mut counter,
        Some(handler),
    );
    assert_eq!(r, PsbtResult::ReadError);
}

#[test]
fn test_parse_no_handler() {
    let mut nothing: i32 = 0;
    let r = psbt_btc_tx_parse(
        TRANSACTION,
        TRANSACTION.len(),
        &mut nothing,
        None,
    );
    assert_eq!(r, PsbtResult::Ok);
}

fn main() {}
