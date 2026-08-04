use libm17::decode::{
    viterbi_decode, viterbi_decode_punctured, LSF_SYNC_SYMBOLS, NUM_STATES, PKT_SYNC_SYMBOLS,
    STR_SYNC_SYMBOLS, SYMBOL_LEVELS,
};
use libm17::encode::{
    conv_encode_lsf, conv_encode_packet_frame, conv_encode_stream_frame, PUNCTURE_PATTERN_1,
    PUNCTURE_PATTERN_2, PUNCTURE_PATTERN_3,
};
use libm17::types::LSF;

// Helper: convert hard bits to soft (0 -> 0, 1 -> 0xFFFF)
fn hard_to_soft(out: &mut [u16], bits: &[u8]) {
    for i in 0..bits.len() {
        out[i] = if bits[i] != 0 { 0xFFFF } else { 0 };
    }
}

#[test]
fn test_sync_symbol_constants() {
    assert_eq!(LSF_SYNC_SYMBOLS, [3, 3, 3, 3, -3, -3, 3, -3]);
    assert_eq!(STR_SYNC_SYMBOLS, [-3, -3, -3, -3, 3, 3, -3, 3]);
    assert_eq!(PKT_SYNC_SYMBOLS, [3, -3, 3, 3, -3, -3, -3, -3]);
}

#[test]
fn test_symbol_levels() {
    assert_eq!(SYMBOL_LEVELS, [-3.0, -1.0, 1.0, 3.0]);
}

#[test]
fn test_num_states() {
    assert_eq!(NUM_STATES, 16);
}

// All viterbi tests share static mutable state (PREV_METRICS, CURR_METRICS,
// VITERBI_HISTORY) so they can't run in parallel. Combine them into a single
// test function so they run sequentially.
#[test]
fn test_viterbi_decoders_round_trip() {
    // ---- Test 1: stream frame round-trip (known input) ----
    let known: [u8; 16] = [
        0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67,
        0x89,
    ];
    let mut enc = [0u8; 272];
    conv_encode_stream_frame(&mut enc, &known, 0xABCD);
    let mut soft = [0u16; 272];
    hard_to_soft(&mut soft, &enc);
    let mut out = [0u8; 40];
    let cost = viterbi_decode_punctured(&mut out, &soft, &PUNCTURE_PATTERN_2, 272, 12);
    // C reports cost=16
    assert_eq!(cost, 16);
    let expected = [
        0x00u8, 0xAB, 0xCD, 0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x01,
        0x23, 0x45, 0x67, 0x89,
    ];
    for i in 0..19 {
        assert_eq!(out[i], expected[i], "stream test mismatch at byte {}", i);
    }

    // ---- Test 2: packet frame round-trip ----
    let mut input26 = [0u8; 26];
    for i in 0..26 {
        input26[i] = (i * 7 + 1) as u8;
    }
    let mut enc26 = [0u8; 368];
    conv_encode_packet_frame(&mut enc26, &input26);
    let mut soft26 = [0u16; 368];
    hard_to_soft(&mut soft26, &enc26);
    let mut out26 = [0u8; 40];
    let cost = viterbi_decode_punctured(&mut out26, &soft26, &PUNCTURE_PATTERN_3, 368, 8);
    assert_eq!(cost, 26);
    let expected_pkt = [
        0x00u8, 0x01, 0x08, 0x0F, 0x16, 0x1D, 0x24, 0x2B, 0x32, 0x39, 0x40, 0x47, 0x4E, 0x55, 0x5C,
        0x63, 0x6A, 0x71, 0x78, 0x7F, 0x86, 0x8D, 0x94, 0x9B, 0xA2, 0xA9, 0xB0,
    ];
    for i in 0..27 {
        assert_eq!(out26[i], expected_pkt[i], "pkt test mismatch at byte {}", i);
    }

    // ---- Test 3: LSF round-trip ----
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
    let mut enc_lsf = [0u8; 368];
    conv_encode_lsf(&mut enc_lsf, &lsf);
    let mut soft_lsf = [0u16; 368];
    hard_to_soft(&mut soft_lsf, &enc_lsf);
    let mut out_lsf = [0u8; 40];
    let cost = viterbi_decode_punctured(&mut out_lsf, &soft_lsf, &PUNCTURE_PATTERN_1, 368, 61);
    assert_eq!(cost, 51);
    let expected_lsf = [
        0x00u8, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0xAA, 0xBB,
        0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x12,
        0x34,
    ];
    for i in 0..31 {
        assert_eq!(out_lsf[i], expected_lsf[i], "lsf test mismatch at byte {}", i);
    }

    // ---- Test 4: stream fn=0 input=zeros ----
    let zeros = [0u8; 16];
    let mut enc0 = [0u8; 272];
    conv_encode_stream_frame(&mut enc0, &zeros, 0);
    let mut soft0 = [0u16; 272];
    hard_to_soft(&mut soft0, &enc0);
    let mut out0 = [0u8; 40];
    let cost = viterbi_decode_punctured(&mut out0, &soft0, &PUNCTURE_PATTERN_2, 272, 12);
    assert_eq!(cost, 0);
    for i in 0..19 {
        assert_eq!(out0[i], 0);
    }

    // ---- Test 5: stream fn=0x8000, in=0 ----
    let mut enc8 = [0u8; 272];
    conv_encode_stream_frame(&mut enc8, &zeros, 0x8000);
    let mut soft8 = [0u16; 272];
    hard_to_soft(&mut soft8, &enc8);
    let mut out8 = [0u8; 40];
    let cost = viterbi_decode_punctured(&mut out8, &soft8, &PUNCTURE_PATTERN_2, 272, 12);
    assert_eq!(cost, 0);
    assert_eq!(out8[0], 0x00);
    assert_eq!(out8[1], 0x80);
    assert_eq!(out8[2], 0x00);
    for i in 3..19 {
        assert_eq!(out8[i], 0);
    }

    // ---- Test 6: viterbi_decode (unpunctured) round-trip ----
    // C's chainback uses bit_pos = len+4 (where len is len/2 = 12 here), so
    // the data bits are written into out[0] bits 4..7 and out[1] bits 0..7.
    // For 8 data bits + 4 trail bits the meaningful data ends up packed in out[1].
    let input_bits: [u8; 12] = [1, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0];
    let mut ud = [0u8; 16];
    for j in 0..8 {
        ud[4 + j] = input_bits[j];
    }
    let mut enc_bits = [0u8; 24];
    let mut idx = 0;
    for i in 0..12 {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;
        enc_bits[idx] = g1;
        idx += 1;
        enc_bits[idx] = g2;
        idx += 1;
    }
    let mut soft_v = [0u16; 24];
    hard_to_soft(&mut soft_v, &enc_bits);
    let mut out_v = [0u8; 4];
    let cost = viterbi_decode(&mut out_v, &soft_v, 24);
    assert_eq!(cost, 0);
    // For input_bits = {1,0,1,1,0,0,1,0, ...trail}, C outputs out = [0x00, 0xB2]
    // (the data bits packed MSB-first into the high byte).
    assert_eq!(out_v[0], 0x00);
    assert_eq!(out_v[1], 0xB2);

    // ---- Test 7: a single bit input ----
    let mut ud = [0u8; 16];
    ud[4] = 1; // first data bit = 1
    let mut enc_bits = [0u8; 24];
    let mut idx = 0;
    for i in 0..12 {
        let g1 = (ud[i + 4] + ud[i + 1] + ud[i + 0]) % 2;
        let g2 = (ud[i + 4] + ud[i + 3] + ud[i + 2] + ud[i + 0]) % 2;
        enc_bits[idx] = g1;
        idx += 1;
        enc_bits[idx] = g2;
        idx += 1;
    }
    let mut soft_v = [0u16; 24];
    hard_to_soft(&mut soft_v, &enc_bits);
    let mut out_v = [0u8; 4];
    let cost = viterbi_decode(&mut out_v, &soft_v, 24);
    assert_eq!(cost, 0);
    // From C: out: 00 80
    assert_eq!(out_v[0], 0x00);
    assert_eq!(out_v[1], 0x80);
}

fn main() {}
