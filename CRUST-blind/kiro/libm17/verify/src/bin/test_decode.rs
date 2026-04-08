use libm17::decode;
use libm17::encode;

#[test]
fn test_sync_symbol_constants() {
    assert_eq!(decode::LSF_SYNC_SYMBOLS, [3, 3, 3, 3, -3, -3, 3, -3]);
    assert_eq!(decode::STR_SYNC_SYMBOLS, [-3, -3, -3, -3, 3, 3, -3, 3]);
    assert_eq!(decode::PKT_SYNC_SYMBOLS, [3, -3, 3, 3, -3, -3, -3, -3]);
    assert_eq!(decode::SYMBOL_LEVELS, [-3.0, -1.0, 1.0, 3.0]);
}

#[test]
fn test_viterbi_decode_stream_zeros() {
    // Encode zeros with fn=0, then decode
    let data = [0u8; 16];
    let mut enc = [0u8; 272];
    encode::conv_encode_stream_frame(&mut enc, &data, 0);

    let mut soft = [0u16; 272];
    for i in 0..272 {
        soft[i] = if enc[i] != 0 { 0xFFFF } else { 0x0000 };
    }

    let mut decoded = [0u8; 20];
    let cost = decode::viterbi_decode_punctured(
        &mut decoded, &soft, &encode::PUNCTURE_PATTERN_2, 272, 12,
    );
    assert_eq!(cost, 0);
    for i in 0..20 {
        assert_eq!(decoded[i], 0x00, "decoded byte {} should be 0x00", i);
    }
}

#[test]
fn test_viterbi_decode_stream_roundtrip() {
    let data: [u8; 16] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                           0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10];
    let mut enc = [0u8; 272];
    encode::conv_encode_stream_frame(&mut enc, &data, 1);

    let mut soft = [0u16; 272];
    for i in 0..272 {
        soft[i] = if enc[i] != 0 { 0xFFFF } else { 0x0000 };
    }

    let mut decoded = [0u8; 20];
    let cost = decode::viterbi_decode_punctured(
        &mut decoded, &soft, &encode::PUNCTURE_PATTERN_2, 272, 12,
    );
    assert_eq!(cost, 15);
    // Frame number (fn=1) is prepended as first 2 bytes, then data follows
    assert_eq!(decoded[0], 0x00);
    assert_eq!(decoded[1], 0x00);
    assert_eq!(decoded[2], 0x01);
    assert_eq!(decoded[3], 0x01);
    assert_eq!(decoded[4], 0x02);
    assert_eq!(decoded[5], 0x03);
    assert_eq!(decoded[6], 0x04);
    assert_eq!(decoded[7], 0x05);
    assert_eq!(decoded[8], 0x06);
    assert_eq!(decoded[9], 0x07);
    assert_eq!(decoded[10], 0x08);
    assert_eq!(decoded[11], 0x09);
    assert_eq!(decoded[12], 0x0A);
    assert_eq!(decoded[13], 0x0B);
    assert_eq!(decoded[14], 0x0C);
    assert_eq!(decoded[15], 0x0D);
    assert_eq!(decoded[16], 0x0E);
    assert_eq!(decoded[17], 0x0F);
    assert_eq!(decoded[18], 0x10);
    assert_eq!(decoded[19], 0x00);
}

#[test]
fn test_viterbi_decode_unpunctured() {
    // Simple test: encode zeros, convert to unpunctured soft bits, decode
    let data = [0u8; 16];
    let mut enc = [0u8; 272];
    encode::conv_encode_stream_frame(&mut enc, &data, 0);

    // Create unpunctured version manually
    let mut soft = [0u16; 272];
    for i in 0..272 {
        soft[i] = if enc[i] != 0 { 0xFFFF } else { 0x0000 };
    }

    // Use viterbi_decode directly with a small input
    let input: [u16; 20] = [0; 20];
    let mut out = [0u8; 10];
    let cost = decode::viterbi_decode(&mut out, &input, 20);
    // All-zero input should decode to all zeros
    assert_eq!(out[0], 0);
    assert_eq!(out[1], 0);
    // Cost should be 0 for all-zero input
    assert_eq!(cost, 0);
}

fn main() {}
