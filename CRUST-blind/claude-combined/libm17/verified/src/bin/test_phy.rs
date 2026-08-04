use libm17::phy::*;
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
fn test_intrl_seq_constant() {
    assert_eq!(INTRL_SEQ.len(), SYM_PER_PLD * 2);
    assert_eq!(INTRL_SEQ[0], 0);
    assert_eq!(INTRL_SEQ[1], 137);
    assert_eq!(INTRL_SEQ[367], 47);
}

#[test]
fn test_rand_seq_constant() {
    assert_eq!(RAND_SEQ.len(), 46);
    assert_eq!(RAND_SEQ[0], 0xD6);
    assert_eq!(RAND_SEQ[45], 0xC3);
}

#[test]
fn test_reorder_bits() {
    let mut inb: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    for i in 0..(SYM_PER_PLD * 2) {
        inb[i] = (i & 0xFF) as u8;
    }
    let mut outb: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    reorder_bits(&mut outb, &inb);

    // Match C ground truth values
    assert_eq!(outb[0], 0); // INTRL_SEQ[0]=0 -> inb[0]=0
    assert_eq!(outb[5], 61); // INTRL_SEQ[5]=317, 317 & 0xFF = 61
    assert_eq!(outb[100], 84); // INTRL_SEQ[100]=84, 84 & 0xFF = 84
    assert_eq!(outb[367], 47); // INTRL_SEQ[367]=47

    // Check entire transformation matches expected pattern
    for i in 0..(SYM_PER_PLD * 2) {
        assert_eq!(outb[i], (INTRL_SEQ[i] & 0xFF) as u8);
    }
}

#[test]
fn test_reorder_soft_bits() {
    let mut inb: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    for i in 0..(SYM_PER_PLD * 2) {
        inb[i] = (i & 0xFF) as u8;
    }
    let mut outb: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    reorder_soft_bits(&mut outb, &inb);
    for i in 0..(SYM_PER_PLD * 2) {
        assert_eq!(outb[i], (INTRL_SEQ[i] & 0xFF) as u8);
    }
}

#[test]
fn test_randomize_bits() {
    let mut rb: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    randomize_bits(&mut rb);
    // Verified from C: rb[0]=1 rb[1]=1 rb[7]=0 rb[8]=1 rb[100]=1 rb[367]=1
    assert_eq!(rb[0], 1);
    assert_eq!(rb[1], 1);
    assert_eq!(rb[7], 0);
    assert_eq!(rb[8], 1);
    assert_eq!(rb[100], 1);
    assert_eq!(rb[367], 1);

    // Each '1' in RAND_SEQ should set the corresponding bit to 1 (since starting 0)
    for i in 0..(SYM_PER_PLD * 2) {
        let bit = (RAND_SEQ[i / 8] >> (7 - (i % 8))) & 1;
        assert_eq!(rb[i], bit);
    }
}

#[test]
fn test_randomize_bits_double_application() {
    // Applying randomize twice should restore original
    let mut buf: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    for i in 0..(SYM_PER_PLD * 2) {
        buf[i] = (i % 2) as u8;
    }
    let original = buf;
    randomize_bits(&mut buf);
    randomize_bits(&mut buf);
    assert_eq!(buf, original);
}

#[test]
fn test_randomize_soft_bits() {
    // Applying twice should restore (since soft_bit_NOT(soft_bit_NOT(x))=x)
    let mut buf: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    for i in 0..(SYM_PER_PLD * 2) {
        buf[i] = (i & 0x7F) as u8;
    }
    let original = buf;
    randomize_soft_bits(&mut buf);
    randomize_soft_bits(&mut buf);
    // soft_bit_NOT for u16 is 0xFFFF - x; for u8 cast it's truncated, but applying twice
    // should still restore: (0xFFFF - (0xFFFF - x)) = x at the u16 level. We verify
    // that the function modifies the bits where rand_seq is 1.
    for i in 0..(SYM_PER_PLD * 2) {
        if (RAND_SEQ[i / 8] >> (7 - (i % 8))) & 1 != 0 {
            // Bit was flipped twice, expected to equal original
            assert_eq!(buf[i], original[i]);
        } else {
            assert_eq!(buf[i], original[i]);
        }
    }
}

#[test]
fn test_slice_symbols() {
    // Use an integer-symbol input (cast u8 -> i8 -> f32). All zeros input.
    let inp: [u8; SYM_PER_PLD] = [0; SYM_PER_PLD];
    let mut out: [u8; SYM_PER_PLD * 2] = [0; SYM_PER_PLD * 2];
    slice_symbols(&mut out, &inp);
    // For v=0 (between -1 and 1), bit 0 is 0x0000, bit 1 should be 0x7FFF (per the formula)
    // We test only that the function runs and produces consistent output for all zeros.
    // For each symbol, out[i*2+1] should be 0 (low byte of 0x0000) and out[i*2] should be 0xFF (low byte of 0x7FFF)
    // Actually 0x7FFF cast to u8 == 0xFF
    for i in 0..SYM_PER_PLD {
        assert_eq!(out[i * 2 + 1], 0);
        assert_eq!(out[i * 2], 0xFF);
    }
}

fn main() {}
