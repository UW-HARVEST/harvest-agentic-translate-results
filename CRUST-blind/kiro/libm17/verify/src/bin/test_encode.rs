use libm17::encode;
use libm17::types::LSF;

#[test]
fn test_constants() {
    assert_eq!(encode::SYMBOL_MAP, [1, 3, -1, -3]);
    assert_eq!(encode::SYMBOL_LIST, [-3, -1, 1, 3]);
    assert_eq!(encode::EOT_SYMBOLS, [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0]);
    assert_eq!(encode::PUNCTURE_PATTERN_1.len(), 61);
    assert_eq!(encode::PUNCTURE_PATTERN_2, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(encode::PUNCTURE_PATTERN_3, [1, 1, 1, 1, 1, 1, 1, 0]);
}

#[test]
fn test_conv_encode_stream_frame_zeros() {
    let data = [0u8; 16];
    let mut out = [0u8; 272];
    encode::conv_encode_stream_frame(&mut out, &data, 0);
    // All zeros input with fn=0 should produce all zeros output
    for i in 0..272 {
        assert_eq!(out[i], 0, "stream zeros: bit {} should be 0", i);
    }
}

#[test]
fn test_conv_encode_stream_frame_ff() {
    let data = [0xFFu8; 16];
    let mut out = [0u8; 272];
    encode::conv_encode_stream_frame(&mut out, &data, 0x1234);
    let expected_str = "00000011010011001100010000001001011011010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010110101010101101010101011010101010101000111";
    let expected: Vec<u8> = expected_str.bytes().map(|b| b - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn test_conv_encode_stream_frame_seq() {
    let data: [u8; 16] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                           0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10];
    let mut out = [0u8; 272];
    encode::conv_encode_stream_frame(&mut out, &data, 1);
    let expected_str = "00000000000000000000000000001101010110000001010110110001101011011000001110001011100110101011000000101101110101001110001101100001110110000111110011011000001101010110010111101101101011001101000111101111100011011000011100000010101111111100001110111011010100100010110100000000";
    let expected: Vec<u8> = expected_str.bytes().map(|b| b - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn test_conv_encode_packet_frame_zeros() {
    let data = [0u8; 26];
    let mut out = [0u8; 368];
    encode::conv_encode_packet_frame(&mut out, &data);
    for i in 0..368 {
        assert_eq!(out[i], 0, "packet zeros: bit {} should be 0", i);
    }
}

#[test]
fn test_conv_encode_packet_frame_aa() {
    let data = [0xAAu8; 26];
    let mut out = [0u8; 368];
    encode::conv_encode_packet_frame(&mut out, &data);
    let expected_str = "11011010111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101110110111011011101101111011100";
    let expected: Vec<u8> = expected_str.bytes().map(|b| b - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn test_conv_encode_lsf_zeros() {
    let lsf = LSF::default();
    let mut out = [0u8; 368];
    encode::conv_encode_lsf(&mut out, &lsf);
    for i in 0..368 {
        assert_eq!(out[i], 0, "lsf zeros: bit {} should be 0", i);
    }
}

#[test]
fn test_conv_encode_lsf_sp5wwp() {
    let lsf = LSF {
        dst: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        src: [0x00, 0x00, 0x65, 0x41, 0xB0, 0x93],
        type_field: [0x00, 0x05],
        meta: [0; 14],
        crc: [0; 2],
    };
    let mut out = [0u8; 368];
    encode::conv_encode_lsf(&mut out, &lsf);
    let expected_str = "11011110010010010010010010010010010010010010010110110110110110110110110101001100000000000000000000110001100101011100110110000111111111011011010010110110000000000000110101101110000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    let expected: Vec<u8> = expected_str.bytes().map(|b| b - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

fn main() {}
