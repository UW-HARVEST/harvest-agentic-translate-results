use libm17::decode::*;
use libm17::encode::{conv_encode_stream_frame, PUNCTURE_PATTERN_2};
use std::sync::Mutex;

// Serialize viterbi state-mutating tests with a static mutex (the underlying C
// implementation uses global state and is not thread-safe).
static VITERBI_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_constants() {
    assert_eq!(LSF_SYNC_SYMBOLS, [3, 3, 3, 3, -3, -3, 3, -3]);
    assert_eq!(STR_SYNC_SYMBOLS, [-3, -3, -3, -3, 3, 3, -3, 3]);
    assert_eq!(PKT_SYNC_SYMBOLS, [3, -3, 3, 3, -3, -3, -3, -3]);
    assert_eq!(SYMBOL_LEVELS, [-3.0, -1.0, 1.0, 3.0]);
    assert_eq!(NUM_STATES, 16);
}

#[test]
fn test_viterbi_decode_zeros() {
    let _g = VITERBI_LOCK.lock().unwrap();
    let input: [u16; 10] = [0; 10];
    let mut out: [u8; 5] = [0; 5];
    let err = viterbi_decode(&mut out, &input, 10);
    assert_eq!(err, 0);
    assert_eq!(out[0], 0);
}

#[test]
fn test_viterbi_decode_punctured_stream_roundtrip() {
    let _g = VITERBI_LOCK.lock().unwrap();
    // Encode all zeros with fn=0, then decode (puncture pattern 2)
    let in_zero = [0u8; 16];
    let mut enc: [u8; 272] = [0; 272];
    conv_encode_stream_frame(&mut enc, &in_zero, 0);
    let mut soft: [u16; 272] = [0; 272];
    for i in 0..272 {
        soft[i] = if enc[i] != 0 { 0xFFFF } else { 0x0000 };
    }
    let mut dec: [u8; 20] = [0; 20];
    let err = viterbi_decode_punctured(
        &mut dec,
        &soft,
        &PUNCTURE_PATTERN_2,
        272,
        PUNCTURE_PATTERN_2.len() as u16,
    );
    assert_eq!(err, 0);
    for &b in &dec[..] {
        assert_eq!(b, 0);
    }
}

#[test]
fn test_viterbi_decode_bit() {
    let _g = VITERBI_LOCK.lock().unwrap();
    // Just call to ensure no crash; behavior is internal state mutation.
    viterbi_decode_bit(0, 0, 0);
    viterbi_decode_bit(0xFFFF, 0xFFFF, 1);
}

fn main() {}
