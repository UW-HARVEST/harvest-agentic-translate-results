use libm17::phy;
use libm17::types::SYM_PER_PLD;

#[test]
fn test_sync_constants() {
    assert_eq!(phy::SYNC_LSF, 0x55F7);
    assert_eq!(phy::SYNC_STR, 0xFF5D);
    assert_eq!(phy::SYNC_PKT, 0x75FF);
    assert_eq!(phy::SYNC_BER, 0xDF55);
    assert_eq!(phy::EOT_MRKR, 0x555D);
}

#[test]
fn test_rand_seq() {
    let expected: [u8; 46] = [
        0xD6, 0xB5, 0xE2, 0x30, 0x82, 0xFF, 0x84, 0x62, 0xBA, 0x4E, 0x96, 0x90, 0xD8, 0x98, 0xDD, 0x5D,
        0x0C, 0xC8, 0x52, 0x43, 0x91, 0x1D, 0xF8, 0x6E, 0x68, 0x2F, 0x35, 0xDA, 0x14, 0xEA, 0xCD, 0x76,
        0x19, 0x8D, 0xD5, 0x80, 0xD1, 0x33, 0x87, 0x13, 0x57, 0x18, 0x2D, 0x29, 0x78, 0xC3,
    ];
    assert_eq!(phy::RAND_SEQ, expected);
}

#[test]
fn test_intrl_seq_first_20() {
    let expected = [0, 137, 90, 227, 180, 317, 270, 39, 360, 129, 82, 219, 172, 309, 262, 31, 352, 121, 74, 211];
    for i in 0..20 {
        assert_eq!(phy::INTRL_SEQ[i], expected[i], "intrl_seq[{}]", i);
    }
}

#[test]
fn test_reorder_bits() {
    let mut inp = [0u8; SYM_PER_PLD * 2];
    for i in 0..SYM_PER_PLD * 2 { inp[i] = (i % 2) as u8; }
    let mut out = [0u8; SYM_PER_PLD * 2];
    phy::reorder_bits(&mut out, &inp);
    // C ground truth first 20: 01010101010101010101
    // Since alternating 0,1 and intrl_seq[0]=0 (even->0), intrl_seq[1]=137 (odd->1), etc.
    let expected_first = "01010101010101010101";
    for (i, ch) in expected_first.chars().enumerate() {
        assert_eq!(out[i], ch.to_digit(10).unwrap() as u8, "reorder_bits[{}]", i);
    }
}

#[test]
fn test_reorder_bits_known_pattern() {
    // First 184 entries = 1, rest = 0
    let mut inp = [0u8; SYM_PER_PLD * 2];
    for i in 0..184 { inp[i] = 1; }
    let mut out = [0u8; SYM_PER_PLD * 2];
    phy::reorder_bits(&mut out, &inp);
    let expected_first = "1110100101101001011010010110100101101001";
    for (i, ch) in expected_first.chars().enumerate() {
        assert_eq!(out[i], ch.to_digit(10).unwrap() as u8, "reorder_bits_known[{}]", i);
    }
}

#[test]
fn test_randomize_bits_zeros() {
    let mut rz = [0u8; SYM_PER_PLD * 2];
    phy::randomize_bits(&mut rz);
    // C ground truth first 20 bits of randomize_bits(all-zeros): 11010110101101011110
    let expected = "11010110101101011110";
    for (i, ch) in expected.chars().enumerate() {
        assert_eq!(rz[i], ch.to_digit(10).unwrap() as u8, "randomize_bits_zeros[{}]", i);
    }
}

#[test]
fn test_randomize_bits_ones() {
    let mut rz = [1u8; SYM_PER_PLD * 2];
    phy::randomize_bits(&mut rz);
    let expected = "0010100101001010000111011100111101111101";
    for (i, ch) in expected.chars().enumerate() {
        assert_eq!(rz[i], ch.to_digit(10).unwrap() as u8, "randomize_bits_ones[{}]", i);
    }
}

fn main() {}
