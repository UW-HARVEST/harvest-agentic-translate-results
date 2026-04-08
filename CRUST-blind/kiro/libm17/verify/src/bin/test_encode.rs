use libm17::encode;
use libm17::types::LSF;

#[test]
fn test_puncture_patterns() {
    let pp1_str = "1101110111011101110111011101110111011101110111011101110111011";
    for (i, ch) in pp1_str.chars().enumerate() {
        assert_eq!(encode::PUNCTURE_PATTERN_1[i], ch.to_digit(10).unwrap() as u8, "pp1[{}]", i);
    }
    assert_eq!(encode::PUNCTURE_PATTERN_2, [1,1,1,1,1,1,1,1,1,1,1,0]);
    assert_eq!(encode::PUNCTURE_PATTERN_3, [1,1,1,1,1,1,1,0]);
}

#[test]
fn test_symbol_map() {
    assert_eq!(encode::SYMBOL_MAP, [1, 3, -1, -3]);
}

#[test]
fn test_symbol_list() {
    assert_eq!(encode::SYMBOL_LIST, [-3, -1, 1, 3]);
}

#[test]
fn test_eot_symbols() {
    assert_eq!(encode::EOT_SYMBOLS, [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0]);
}

#[test]
fn test_conv_encode_stream_frame_zeros() {
    let input = [0u8; 16];
    let mut out = [0u8; 272];
    encode::conv_encode_stream_frame(&mut out, &input, 0);
    // All zeros in -> all zeros out
    for i in 0..272 {
        assert_eq!(out[i], 0, "stream_frame_zeros[{}]", i);
    }
}

#[test]
fn test_conv_encode_stream_frame_nonzero() {
    let input = [0xFFu8; 16];
    let mut out = [0u8; 272];
    encode::conv_encode_stream_frame(&mut out, &input, 0x1234);
    let expected = "00000011010011001100010000001001011011010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010101000111";
    for (i, ch) in expected.chars().enumerate() {
        assert_eq!(out[i], ch.to_digit(10).unwrap() as u8, "stream_frame_nonzero[{}]", i);
    }
}

#[test]
fn test_conv_encode_packet_frame_zeros() {
    let input = [0u8; 26];
    let mut out = [0u8; 368];
    encode::conv_encode_packet_frame(&mut out, &input);
    for i in 0..368 {
        assert_eq!(out[i], 0, "packet_frame_zeros[{}]", i);
    }
}

#[test]
fn test_conv_encode_lsf_zeros() {
    let lsf = LSF::default();
    let mut out = [0u8; 368];
    encode::conv_encode_lsf(&mut out, &lsf);
    for i in 0..368 {
        assert_eq!(out[i], 0, "lsf_zeros[{}]", i);
    }
}

#[test]
fn test_conv_encode_lsf_nonzero() {
    let mut lsf = LSF::default();
    lsf.dst[0] = 0xFF;
    lsf.src[0] = 0xFF;
    lsf.type_field[0] = 0x05;
    lsf.type_field[1] = 0x05;
    lsf.crc[0] = 0xAA;
    lsf.crc[1] = 0xBB;
    let mut out = [0u8; 368];
    encode::conv_encode_lsf(&mut out, &lsf);
    let expected = "11011110010001001100000000000000000000000000000000000000000000000000000011111010110101001100000000000000000000000000000000000000000000000000000000000000110101101110110101101110000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000101111011011011000010111111011";
    for (i, ch) in expected.chars().enumerate() {
        assert_eq!(out[i], ch.to_digit(10).unwrap() as u8, "lsf_nonzero[{}]", i);
    }
}

fn main() {}
