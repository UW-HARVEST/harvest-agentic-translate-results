use libm17::phy::{
    randomize_bits, randomize_soft_bits, reorder_bits, reorder_soft_bits, slice_symbols, EOT_MRKR,
    INTRL_SEQ, RAND_SEQ, SYNC_BER, SYNC_LSF, SYNC_PKT, SYNC_STR,
};
use libm17::types::SYM_PER_PLD;

#[test]
fn test_sync_constants() {
    assert_eq!(SYNC_LSF, 0x55F7);
    assert_eq!(SYNC_STR, 0xFF5D);
    assert_eq!(SYNC_PKT, 0x75FF);
    assert_eq!(SYNC_BER, 0xDF55);
    assert_eq!(EOT_MRKR, 0x555D);
}

#[test]
fn test_intrl_seq_first_few() {
    assert_eq!(INTRL_SEQ[0], 0);
    assert_eq!(INTRL_SEQ[1], 137);
    assert_eq!(INTRL_SEQ[2], 90);
    assert_eq!(INTRL_SEQ[3], 227);
    assert_eq!(INTRL_SEQ[5], 317);
    assert_eq!(INTRL_SEQ[INTRL_SEQ.len() - 1], 47);
}

#[test]
fn test_rand_seq_constants() {
    assert_eq!(RAND_SEQ[0], 0xD6);
    assert_eq!(RAND_SEQ[1], 0xB5);
    assert_eq!(RAND_SEQ[45], 0xC3);
    assert_eq!(RAND_SEQ.len(), 46);
}

#[test]
fn test_reorder_bits_alternating() {
    let mut input = [0u8; SYM_PER_PLD * 2];
    for i in 0..SYM_PER_PLD * 2 {
        input[i] = (i & 1) as u8;
    }
    let mut out = [0u8; SYM_PER_PLD * 2];
    reorder_bits(&mut out, &input);

    // From C run, first 32 bits = 01010101010101010101010101010101
    let expected: String = "01010101010101010101010101010101".into();
    let actual: String = out[..32].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected);
    // last 32 bits also alternating (because intrl_seq is just permutation)
    let actual_last: String = out[SYM_PER_PLD * 2 - 32..]
        .iter()
        .map(|&b| if b == 0 { '0' } else { '1' })
        .collect();
    assert_eq!(actual_last, "01010101010101010101010101010101");
}

#[test]
fn test_reorder_bits_indexing() {
    let mut input = [0u8; SYM_PER_PLD * 2];
    // Use values modulo 256 of (i*3+7)
    for i in 0..SYM_PER_PLD * 2 {
        input[i] = ((i as u32 * 3 + 7) & 0xFF) as u8;
    }
    let mut out = [0u8; SYM_PER_PLD * 2];
    reorder_bits(&mut out, &input);

    // sum should be 44648 (from C run)
    let sum: u64 = out.iter().map(|&v| v as u64).sum();
    assert_eq!(sum, 44648);

    // out[0] = in[INTRL_SEQ[0]] = in[0] = 7
    assert_eq!(out[0], 7);
    // out[1] = in[INTRL_SEQ[1]] = in[137] = (137*3+7)&0xff = (411+7)&0xff = 418 & 0xff = 162
    assert_eq!(out[1], 162);
}

#[test]
fn test_reorder_soft_bits_indexing() {
    let mut input = [0u8; SYM_PER_PLD * 2];
    // Use varying byte values
    for i in 0..SYM_PER_PLD * 2 {
        input[i] = (i & 0xFF) as u8;
    }
    let mut out = [0u8; SYM_PER_PLD * 2];
    reorder_soft_bits(&mut out, &input);
    assert_eq!(out[0], input[INTRL_SEQ[0]]);
    assert_eq!(out[1], input[INTRL_SEQ[1]]);
    assert_eq!(out[5], input[INTRL_SEQ[5]]);
}

#[test]
fn test_randomize_bits_zeros() {
    let mut buf = [0u8; SYM_PER_PLD * 2];
    randomize_bits(&mut buf);
    // C output first 32 = 11010110101101011110001000110000 (matches RAND_SEQ first 4 bytes 0xD6 0xB5 0xE2 0x30)
    // 0xD6=11010110, 0xB5=10110101, 0xE2=11100010, 0x30=00110000
    let expected: String = "11010110101101011110001000110000".into();
    let actual: String = buf[..32].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_randomize_bits_ones() {
    let mut buf = [1u8; SYM_PER_PLD * 2];
    randomize_bits(&mut buf);
    // ones flipped where rand seq is 1. Result first 32 = NOT of zeros result.
    let expected: String = "00101001010010100001110111001111".into();
    let actual: String = buf[..32].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_randomize_bits_involution() {
    let mut buf = [0u8; SYM_PER_PLD * 2];
    randomize_bits(&mut buf);
    randomize_bits(&mut buf);
    for &b in &buf[..] {
        assert_eq!(b, 0);
    }
}

#[test]
fn test_slice_symbols_specific_levels() {
    let mut input = [0f32; SYM_PER_PLD];
    input[0] = 3.0;
    input[1] = 1.0;
    input[2] = -1.0;
    input[3] = -3.0;
    input[4] = 2.0;
    input[5] = 0.0;
    input[6] = -2.0;
    let mut out = [0u8; SYM_PER_PLD * 2];
    // Note: the Rust signature uses u8 for both. The Rust implementation uses
    // input bytes as symbol indices. To exercise the Rust API that takes u8 input,
    // build an indexed input.
    // Map: index 0 = -3, 1 = -1, 2 = +1, 3 = +3
    // For testing the Rust slice_symbols, use input bytes directly as symbol indices.
    let idx_input: [u8; SYM_PER_PLD] = {
        let mut a = [0u8; SYM_PER_PLD];
        a[0] = 3; // +3
        a[1] = 2; // +1
        a[2] = 1; // -1
        a[3] = 0; // -3
        a
    };
    slice_symbols(&mut out, &idx_input);
    // For symbol 3 (+3): bits = (b1=0, b0=1)
    assert_eq!(out[0], 0);
    assert_eq!(out[1], 1);
    // For symbol 2 (+1): bits = (b1=0, b0=0)
    assert_eq!(out[2], 0);
    assert_eq!(out[3], 0);
    // For symbol 1 (-1): bits = (b1=1, b0=0)
    assert_eq!(out[4], 1);
    assert_eq!(out[5], 0);
    // For symbol 0 (-3): bits = (b1=1, b0=1)
    assert_eq!(out[6], 1);
    assert_eq!(out[7], 1);
    // remaining symbols are index 0 -> (-3): (1, 1)
    for i in 4..SYM_PER_PLD {
        assert_eq!(out[i * 2], 1);
        assert_eq!(out[i * 2 + 1], 1);
    }
    // Use input variable to silence warnings
    let _ = input;
}

#[test]
fn test_randomize_soft_bits_runs() {
    let mut buf = [0u8; SYM_PER_PLD * 2];
    // shouldn't panic
    randomize_soft_bits(&mut buf);
    // Applying twice should restore (involution)
    let mut buf2 = [0u8; SYM_PER_PLD * 2];
    randomize_soft_bits(&mut buf2);
    randomize_soft_bits(&mut buf2);
    assert_eq!(buf2, [0u8; SYM_PER_PLD * 2]);
}

fn main() {}
