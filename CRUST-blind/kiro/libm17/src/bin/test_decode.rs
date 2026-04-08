use libm17::decode;
use libm17::encode;

#[test]
fn test_sync_symbols() {
    assert_eq!(decode::LSF_SYNC_SYMBOLS, [3, 3, 3, 3, -3, -3, 3, -3]);
    assert_eq!(decode::STR_SYNC_SYMBOLS, [-3, -3, -3, -3, 3, 3, -3, 3]);
    assert_eq!(decode::PKT_SYNC_SYMBOLS, [3, -3, 3, 3, -3, -3, -3, -3]);
}

#[test]
fn test_symbol_levels() {
    assert_eq!(decode::SYMBOL_LEVELS, [-3.0, -1.0, 1.0, 3.0]);
}

// Viterbi uses static mut state, so all viterbi tests must be in one test to avoid races
#[test]
fn test_viterbi_decode_all() {
    // Test 1: all-zeros
    {
        let input = [0u8; 16];
        let mut enc = [0u8; 272];
        encode::conv_encode_stream_frame(&mut enc, &input, 0);
        let mut soft = [0u16; 272];
        for i in 0..272 {
            soft[i] = if enc[i] != 0 { 0xFFFF } else { 0 };
        }
        let mut out = [0u8; 20];
        let cost = decode::viterbi_decode_punctured(&mut out, &soft, &encode::PUNCTURE_PATTERN_2, 272, 12);
        assert_eq!(cost, 0);
        for i in 0..18 {
            assert_eq!(out[i], 0, "viterbi_zeros[{}]", i);
        }
    }

    // Test 2: nonzero data
    {
        let input: [u8; 16] = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
        let mut enc = [0u8; 272];
        encode::conv_encode_stream_frame(&mut enc, &input, 0x0001);
        let mut soft = [0u16; 272];
        for i in 0..272 {
            soft[i] = if enc[i] != 0 { 0xFFFF } else { 0 };
        }
        let mut out = [0u8; 20];
        let cost = decode::viterbi_decode_punctured(&mut out, &soft, &encode::PUNCTURE_PATTERN_2, 272, 12);
        assert_eq!(cost, 13);
        assert_eq!(out[0], 0x00);
        assert_eq!(out[1], 0x00);
        assert_eq!(out[2], 0x01);
        assert_eq!(&out[3..18], &[0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67]);
    }
}

fn main() {}
