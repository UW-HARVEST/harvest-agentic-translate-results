use FastHamming::fast_hamming;

#[test]
fn test_log2uint8_values() {
    assert_eq!(fast_hamming::log2uint8(1), 0);
    assert_eq!(fast_hamming::log2uint8(2), 1);
    assert_eq!(fast_hamming::log2uint8(3), 1);
    assert_eq!(fast_hamming::log2uint8(4), 2);
    assert_eq!(fast_hamming::log2uint8(7), 2);
    assert_eq!(fast_hamming::log2uint8(8), 3);
    assert_eq!(fast_hamming::log2uint8(16), 4);
    assert_eq!(fast_hamming::log2uint8(32), 5);
    assert_eq!(fast_hamming::log2uint8(64), 6);
    assert_eq!(fast_hamming::log2uint8(128), 7);
    assert_eq!(fast_hamming::log2uint8(255), 7);
}

#[test]
fn test_log2uint8_full_sweep() {
    for v in 1u16..=255u16 {
        let v8 = v as u8;
        // log2 floor: position of highest set bit
        let mut expected = 0u8;
        let mut x = v8;
        while x > 1 {
            x >>= 1;
            expected += 1;
        }
        assert_eq!(fast_hamming::log2uint8(v8), expected, "v={}", v8);
    }
}

#[test]
fn test_encode_known_data() {
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let expected: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
    ];
    let out = fast_hamming::hecc_encode(&data8);
    assert_eq!(out.len(), 16);
    assert_eq!(out.as_slice(), &expected);
}

#[test]
fn test_encode_empty() {
    let out = fast_hamming::hecc_encode(&[]);
    assert_eq!(out.len(), 0);
    assert!(out.is_empty());
}

#[test]
fn test_encode_one_byte() {
    // From C: insize=1 outsize=2: 42 8e
    let out = fast_hamming::hecc_encode(&[0x42]);
    assert_eq!(out.len(), 2);
    assert_eq!(out, vec![0x42, 0x8e]);
}

#[test]
fn test_encode_five_bytes() {
    // From C: insize=5 outsize=6: 01 02 03 04 05 23
    let out = fast_hamming::hecc_encode(&[0x01, 0x02, 0x03, 0x04, 0x05]);
    assert_eq!(out.len(), 6);
    assert_eq!(out, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x23]);
}

#[test]
fn test_encode_fourteen_bytes() {
    // From C: insize=14 outsize=15: 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 49
    let input: [u8; 14] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13];
    let out = fast_hamming::hecc_encode(&input);
    assert_eq!(out.len(), 15);
    assert_eq!(
        out,
        vec![0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d, 0x49]
    );
}

#[test]
fn test_encode_fifteen_bytes_sequential() {
    // From C: insize=15 outsize=16: 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e b1
    let input: [u8; 15] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14];
    let out = fast_hamming::hecc_encode(&input);
    assert_eq!(out.len(), 16);
    assert_eq!(
        out,
        vec![0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x0e, 0xb1]
    );
}

#[test]
fn test_encode_sixteen_bytes() {
    // From C: insize=16 outsize=18: 00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e b1 0f 87
    let input: [u8; 16] = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
    let out = fast_hamming::hecc_encode(&input);
    assert_eq!(out.len(), 18);
    assert_eq!(
        out,
        vec![0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x0e,0xb1,
             0x0f, 0x87]
    );
}

#[test]
fn test_encode_thirty_bytes() {
    // From C: insize=30 outsize=32:
    // 01 08 0f 16 1d 24 2b 32 39 40 47 4e 55 5c 63 1a 6a 71 78 7f 86 8d 94 9b a2 a9 b0 b7 be c5 cc 9c
    let mut input = [0u8; 30];
    for i in 0..30 {
        input[i] = (i as u8).wrapping_mul(7).wrapping_add(1);
    }
    let out = fast_hamming::hecc_encode(&input);
    let expected: [u8; 32] = [
        0x01,0x08,0x0f,0x16,0x1d,0x24,0x2b,0x32,0x39,0x40,0x47,0x4e,0x55,0x5c,0x63,0x1a,
        0x6a,0x71,0x78,0x7f,0x86,0x8d,0x94,0x9b,0xa2,0xa9,0xb0,0xb7,0xbe,0xc5,0xcc,0x9c,
    ];
    assert_eq!(out.len(), 32);
    assert_eq!(out.as_slice(), &expected);
}

#[test]
fn test_encode_all_zeros_15() {
    let input = [0u8; 15];
    let out = fast_hamming::hecc_encode(&input);
    assert_eq!(out.len(), 16);
    // From C: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    assert_eq!(out, vec![0u8; 16]);
}

#[test]
fn test_encode_all_ones_15() {
    let input = [0xffu8; 15];
    let out = fast_hamming::hecc_encode(&input);
    assert_eq!(out.len(), 16);
    // From C: ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
    assert_eq!(out, vec![0xffu8; 16]);
}

#[test]
fn test_decode_known_data() {
    let encoded: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
    ];
    let expected: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let out = fast_hamming::hecc_decode(&encoded);
    assert!(out.is_some());
    let v = out.unwrap();
    assert_eq!(v.len(), 15);
    assert_eq!(v.as_slice(), &expected);
}

#[test]
fn test_decode_empty() {
    // empty input -> empty output
    let out = fast_hamming::hecc_decode(&[]);
    assert!(out.is_some());
    let v = out.unwrap();
    assert_eq!(v.len(), 0);
}

#[test]
fn test_decode_truncated_one_byte_fails() {
    // From C: insize=1 -> false (insize % 16 == 1)
    let out = fast_hamming::hecc_decode(&[0x42]);
    assert!(out.is_none());
}

#[test]
fn test_decode_seventeen_bytes_fails() {
    // From C: insize=17 -> false (insize % 16 == 1)
    let mut buf = [0u8; 17];
    buf[..16].copy_from_slice(&[
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
    ]);
    buf[16] = 0xab;
    let out = fast_hamming::hecc_decode(&buf);
    assert!(out.is_none());
}

#[test]
fn test_roundtrip_known() {
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let encoded = fast_hamming::hecc_encode(&data8);
    let decoded = fast_hamming::hecc_decode(&encoded).expect("decode failed");
    assert_eq!(decoded.len(), 15);
    assert_eq!(decoded.as_slice(), &data8);
}

#[test]
fn test_roundtrip_various_sizes() {
    for sz in [0usize, 1, 5, 14, 15, 16, 30, 50] {
        let mut input = vec![0u8; sz];
        for i in 0..sz {
            input[i] = (i as u8).wrapping_mul(13).wrapping_add(7);
        }
        let encoded = fast_hamming::hecc_encode(&input);
        let decoded = fast_hamming::hecc_decode(&encoded).expect("decode failed");
        assert_eq!(decoded.len(), sz, "size {}", sz);
        assert_eq!(decoded.as_slice(), input.as_slice(), "size {}", sz);
    }
}

#[test]
fn test_decode_single_bit_errors() {
    // From C output: every single-bit error in the 16-byte encoded block (128 bits)
    // should be corrected and decode back to data8.
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let data8_enc: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
    ];

    for idx in 0u32..(16 * 8) {
        let mut buf = data8_enc;
        buf[(idx >> 3) as usize] ^= 1 << (idx % 8);
        let decoded = fast_hamming::hecc_decode(&buf);
        assert!(decoded.is_some(), "decode should succeed at bit {}", idx);
        let v = decoded.unwrap();
        assert_eq!(v.len(), 15, "bit {}", idx);
        assert_eq!(v.as_slice(), &data8, "bit {} did not correct", idx);
    }
}

#[test]
fn test_decode_double_bit_error_returns_none() {
    // From C: outbuf[0] ^= 3 -> double error -> return false
    let mut buf: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
    ];
    buf[0] ^= 3;
    let out = fast_hamming::hecc_decode(&buf);
    assert!(out.is_none());
}

#[test]
fn test_decode_zero_block() {
    // 16 zero bytes decodes to 15 zero bytes
    let buf = [0u8; 16];
    let out = fast_hamming::hecc_decode(&buf).expect("decode failed");
    assert_eq!(out.len(), 15);
    assert_eq!(out, vec![0u8; 15]);
}

#[test]
fn test_decode_all_ones_block() {
    // From C: encode of 15 ones -> 16 ones, so decode of 16 ones -> 15 ones
    let buf = [0xffu8; 16];
    let out = fast_hamming::hecc_decode(&buf).expect("decode failed");
    assert_eq!(out.len(), 15);
    assert_eq!(out, vec![0xffu8; 15]);
}

fn main() {}
