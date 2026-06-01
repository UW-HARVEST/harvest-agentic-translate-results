use libm17::encode::*;
use libm17::types::LSF;

#[test]
fn test_constants() {
    assert_eq!(SYMBOL_MAP, [1, 3, -1, -3]);
    assert_eq!(SYMBOL_LIST, [-3, -1, 1, 3]);
    assert_eq!(EOT_SYMBOLS, [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0]);
    assert_eq!(PUNCTURE_PATTERN_1.len(), 61);
    assert_eq!(PUNCTURE_PATTERN_2, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(PUNCTURE_PATTERN_3, [1, 1, 1, 1, 1, 1, 1, 0]);
}

#[test]
fn test_conv_encode_lsf() {
    let mut lsf = LSF::default();
    for i in 0..6u8 {
        lsf.dst[i as usize] = i + 1;
        lsf.src[i as usize] = 0x10 + i;
    }
    lsf.type_field[0] = 0xAB;
    lsf.type_field[1] = 0xCD;
    for i in 0..14u8 {
        lsf.meta[i as usize] = i * 3;
    }
    lsf.crc[0] = 0x01;
    lsf.crc[1] = 0x74;

    let mut out: [u8; 368] = [0; 368];
    conv_encode_lsf(&mut out, &lsf);

    // Match the C output bit-for-bit
    let expected_str = "00000000000101110100011101011000011000101100101010100000101111110100110011011101010100000101010001010001011101101111011010111001101100110110101010110000110000010001010000101110000000000000001100010110011001111101110111011011100010110001111100100100100110111110111101111001001101100011000011111100011101001001110111101110101101111101001011100011000011010000111010111000";
    for (i, c) in expected_str.chars().enumerate() {
        let expected = (c as u8) - b'0';
        assert_eq!(out[i], expected, "mismatch at index {}", i);
    }
}

#[test]
fn test_conv_encode_stream_frame() {
    let mut stream_in = [0u8; 16];
    for i in 0..16 {
        stream_in[i] = (i * 7) as u8;
    }
    let mut out: [u8; 272] = [0; 272];
    conv_encode_stream_frame(&mut out, &stream_in, 0x1234);

    let expected_str = "00000011010011001100010000001010100000000000000000000011011100001111101110000111101101101110100010110000111110011000100011010101101110100010001010001010101110000111011101101101011100101001000111010001010000000110111011111000011000100111100111110100010100100000010010011011";
    for (i, c) in expected_str.chars().enumerate() {
        let expected = (c as u8) - b'0';
        assert_eq!(out[i], expected, "mismatch at index {}", i);
    }
}

#[test]
fn test_conv_encode_stream_frame_zero() {
    // All-zeros input with fn=0 should yield all zero outputs (since memory init to 0)
    let stream_in = [0u8; 16];
    let mut out: [u8; 272] = [0xAA; 272];
    conv_encode_stream_frame(&mut out, &stream_in, 0);
    let mut sum = 0u32;
    for &b in &out[..] {
        sum += b as u32;
    }
    assert_eq!(sum, 0);
}

#[test]
fn test_conv_encode_packet_frame() {
    let mut pkt_in = [0u8; 26];
    for i in 0..26 {
        pkt_in[i] = ((i * 13) & 0xFF) as u8;
    }
    let mut out: [u8; 368] = [0; 368];
    conv_encode_packet_frame(&mut out, &pkt_in);

    let expected_str = "00000000000000000000011100000010100100000010100000101011100010000000011000010101100101101110101111001010100110011100111000010111111011011000001111011001101011101001011010010010111100101010101111000100001110011100111110111111110101100101011110000110010111000101100011010000001111101000011001101010110010101100010110010110100001000101101011111001110000111101010000101011";
    for (i, c) in expected_str.chars().enumerate() {
        let expected = (c as u8) - b'0';
        assert_eq!(out[i], expected, "mismatch at index {}", i);
    }
}

#[test]
fn test_conv_encode_packet_frame_zero() {
    let pkt_in = [0u8; 26];
    let mut out: [u8; 368] = [0xAA; 368];
    conv_encode_packet_frame(&mut out, &pkt_in);
    let sum: u32 = out.iter().map(|&b| b as u32).sum();
    assert_eq!(sum, 0);
}

fn main() {}
