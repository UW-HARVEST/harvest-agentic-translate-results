use libm17::encode::{
    conv_encode_lsf, conv_encode_packet_frame, conv_encode_stream_frame, EOT_SYMBOLS,
    PUNCTURE_PATTERN_1, PUNCTURE_PATTERN_2, PUNCTURE_PATTERN_3, SYMBOL_LIST, SYMBOL_MAP,
};
use libm17::types::LSF;

#[test]
fn test_symbol_map_constants() {
    assert_eq!(SYMBOL_MAP, [1, 3, -1, -3]);
    assert_eq!(SYMBOL_LIST, [-3, -1, 1, 3]);
}

#[test]
fn test_eot_symbols() {
    assert_eq!(EOT_SYMBOLS, [3.0, 3.0, 3.0, 3.0, 3.0, 3.0, -3.0, 3.0]);
}

#[test]
fn test_puncture_patterns() {
    assert_eq!(PUNCTURE_PATTERN_2, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(PUNCTURE_PATTERN_3, [1, 1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(
        PUNCTURE_PATTERN_1,
        [
            1u8, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1,
            1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1,
            0, 1, 1
        ]
    );
}

#[test]
fn test_conv_encode_stream_frame_known_first_bits() {
    let mut input = [0u8; 16];
    for i in 0..16 {
        input[i] = i as u8;
    }
    let mut out = [0u8; 272];
    conv_encode_stream_frame(&mut out, &input, 0x1234);
    // First 64 bits from C run: 0000001101001100110001000000101010000000000000000000000001101011
    let expected_first_64 = "0000001101001100110001000000101010000000000000000000000001101011";
    let actual: String = out[..64].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected_first_64);

    // bits 100..164 from C: 1011011000001101101111011001110011011100001101110000111101011011
    let expected_bits_100_164 = "1011011000001101101111011001110011011100001101110000111101011011";
    let actual2: String = out[100..164].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual2, expected_bits_100_164);
}

#[test]
fn test_conv_encode_packet_frame_known_first_bits() {
    let mut input = [0u8; 26];
    for i in 0..26 {
        input[i] = i as u8;
    }
    let mut out = [0u8; 368];
    conv_encode_packet_frame(&mut out, &input);
    // First 64 bits from C: 0000000000000000000000000001010110100001100110110000011100110110
    let expected_first_64 = "0000000000000000000000000001010110100001100110110000011100110110";
    let actual: String = out[..64].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected_first_64);

    // bits 100..164: 0111000111011000011110101111000001101010100110111011011010110110
    let expected_bits_100_164 = "0111000111011000011110101111000001101010100110111011011010110110";
    let actual2: String = out[100..164].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual2, expected_bits_100_164);
}

#[test]
fn test_conv_encode_lsf_known_first_bits() {
    let mut lsf = LSF::default();
    for i in 0..6 {
        lsf.dst[i] = 0x10 + i as u8;
    }
    for i in 0..6 {
        lsf.src[i] = 0x20 + i as u8;
    }
    lsf.type_field[0] = 0xAA;
    lsf.type_field[1] = 0xBB;
    for i in 0..14 {
        lsf.meta[i] = 0x40 + i as u8;
    }
    lsf.crc[0] = 0x12;
    lsf.crc[1] = 0x34;

    let mut out = [0u8; 368];
    conv_encode_lsf(&mut out, &lsf);

    // First 64 bits from C: 0000010111010000010111000111000110100101110110100010100111111010
    let expected_first_64 = "0000010111010000010111000111000110100101110110100010100111111010";
    let actual: String = out[..64].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual, expected_first_64);

    // bits 200..264: 0111101110110111000000010001110001010001111011110010011110010010
    let expected_bits_200_264 = "0111101110110111000000010001110001010001111011110010011110010010";
    let actual2: String = out[200..264].iter().map(|&b| if b == 0 { '0' } else { '1' }).collect();
    assert_eq!(actual2, expected_bits_200_264);
}

#[test]
fn test_conv_encode_stream_frame_zeros() {
    // All-zero input + fn=0 -> all-zero output
    let input = [0u8; 16];
    let mut out = [0u8; 272];
    conv_encode_stream_frame(&mut out, &input, 0);
    for &b in &out[..272] {
        assert_eq!(b, 0);
    }
}

#[test]
fn test_conv_encode_packet_frame_zeros() {
    let input = [0u8; 26];
    let mut out = [0u8; 368];
    conv_encode_packet_frame(&mut out, &input);
    for &b in &out[..368] {
        assert_eq!(b, 0);
    }
}

fn main() {}
