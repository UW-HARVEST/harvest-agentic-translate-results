use gorilla_paper_encode::gorilla::*;

// ── Free functions ──────────────────────────────────────────────────

#[test]
fn test_bitslen() {
    assert_eq!(bitslen(0), 0);
    assert_eq!(bitslen(1), 1);
    assert_eq!(bitslen(2), 2);
    assert_eq!(bitslen(255), 8);
    assert_eq!(bitslen(256), 9);
    assert_eq!(bitslen(65535), 16);
    assert_eq!(bitslen(65536), 17);
    assert_eq!(bitslen(0xFFFFFFFF), 32);
    assert_eq!(bitslen(0x100000000), 33);
    assert_eq!(bitslen(0xFFFFFFFFFFFFFFFF), 64);
}

#[test]
fn test_leading_zero64() {
    assert_eq!(leading_zero64(0), 64);
    assert_eq!(leading_zero64(1), 63);
    assert_eq!(leading_zero64(2), 62);
    assert_eq!(leading_zero64(0x8000000000000000), 0);
    assert_eq!(leading_zero64(0xFFFFFFFFFFFFFFFF), 0);
    assert_eq!(leading_zero64(0x00000000FFFFFFFF), 32);
    assert_eq!(leading_zero64(100), 57);
}

#[test]
fn test_trailing_zero64() {
    assert_eq!(trailing_zero64(0), 64);
    assert_eq!(trailing_zero64(1), 0);
    assert_eq!(trailing_zero64(2), 1);
    assert_eq!(trailing_zero64(4), 2);
    assert_eq!(trailing_zero64(0x8000000000000000), 63);
    assert_eq!(trailing_zero64(0xFFFFFFFFFFFFFFFF), 0);
    assert_eq!(trailing_zero64(0x100), 8);
    assert_eq!(trailing_zero64(0x10000), 16);
}

// ── BitWriter ───────────────────────────────────────────────────────

#[test]
fn test_bitwriter_init() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 99, byte: 0xFF, bit_count: 0 };
    w.bitwriter_init();
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0);
}

#[test]
fn test_write_bit() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    // C: after 3 bits (1,0,1): byte=160 (0xa0), bit_count=5, pos=0
    assert_eq!(w.byte, 160);
    assert_eq!(w.bit_count, 5);
    assert_eq!(w.pos, 0);
}

#[test]
fn test_write_byte() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_byte(0xAB);
    // C: pos=1, cache[0]=0xab, byte=0x00, bit_count=8
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xAB);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_bits() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bits(0x1F, 5);
    // C: byte=0xf8, bit_count=3, pos=0
    assert_eq!(w.byte, 0xf8);
    assert_eq!(w.bit_count, 3);
    assert_eq!(w.pos, 0);

    w.write_flush(false);
    // C: pos=1, cache[0]=0xf8
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xf8);
}

#[test]
fn test_write_bits_64() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    let ret = w.write_bits(0xDEADBEEFCAFEBABE, 64);
    assert_eq!(ret, 0);
    assert_eq!(w.pos, 8);
    assert_eq!(w.cache[0], 0xDE);
    assert_eq!(w.cache[1], 0xAD);
    assert_eq!(w.cache[7], 0xBE);
    assert_eq!(w.byte, 0x00);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_bits_invalid() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    assert_eq!(w.write_bits(0xFF, 65), -1);
    assert_eq!(w.write_bits(0xFF, -1), -1);
    // 0 bits is valid, no-op
    assert_eq!(w.write_bits(0, 0), 0);
    assert_eq!(w.pos, 0);
}

#[test]
fn test_write_flush_true() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bit(true);
    w.write_flush(true);
    // C: pos=1, cache[0]=0xff
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xFF);
}

#[test]
fn test_write_flush_already_aligned() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_flush(false);
    // C: pos=0 (no-op)
    assert_eq!(w.pos, 0);
}

#[test]
fn test_append_to_cache() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.byte = 0x42;
    assert_eq!(w.append_to_cache(), 0);
    assert_eq!(w.cache[0], 0x42);
    assert_eq!(w.pos, 1);
}

// ── FloatEncoder ────────────────────────────────────────────────────

#[test]
fn test_float_encoder_init() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 0 },
        val: 0, leading: 0, trailing: 0, first: false, finished: true,
    };
    enc.float_encoder_init();
    assert!(enc.first);
    assert!(!enc.finished);
    assert_eq!(enc.val, 0);
    assert_eq!(enc.leading, !0u64);
    assert_eq!(enc.trailing, 0);
    // write_byte(0x10) was called: pos=1, cache[0]=0x10
    assert_eq!(enc.w.pos, 1);
    assert_eq!(enc.w.cache[0], 0x10);
    assert_eq!(enc.w.bit_count, 8);
}

#[test]
fn test_float_encode_single_value() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 0 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    // C: pos=9, first=false
    assert_eq!(enc.w.pos, 9);
    assert!(!enc.first);
    assert_eq!(enc.w.byte, 0x00);
    assert_eq!(enc.w.bit_count, 8);
    // C cache: 10 40 45 00 00 00 00 00 00 00
    let expected: [u8; 10] = [0x10, 0x40, 0x45, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    for i in 0..10 {
        assert_eq!(enc.w.cache[i], expected[i], "cache[{}] mismatch", i);
    }
}

// ── Full encode/decode roundtrips ───────────────────────────────────

fn encode_values(values: &[f64]) -> (Vec<u8>, u32) {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 0 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    for &v in values {
        enc.float_encode_write(v);
    }
    let mut buf = [0u8; 1024];
    let mut length: u32 = 0;
    enc.float_encode_flush(&mut buf, &mut length);
    (buf[..length as usize].to_vec(), length)
}

fn decode_values(encoded: &[u8]) -> Vec<f64> {
    let mut dec = FloatDecoder {
        val: 0, leading: 0, trailing: 0,
        br: BitReader { data: &[], len: 0, v: 0, n: 0 },
        b: [0u8; 1024], first: false, finished: false, err: 0,
    };
    let mut res = [0f64; 64];
    let mut res_len: u32 = 0;
    let ret = dec.float_decode_block(encoded, &mut res, &mut res_len);
    assert_eq!(ret, 0);
    res[..res_len as usize].to_vec()
}

#[test]
fn test_roundtrip_standard_array() {
    let input = [2300.0, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    let (encoded, length) = encode_values(&input);
    assert_eq!(length, 31);
    let expected_bytes: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e, 0xe2,
        0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00,
    ];
    for i in 0..31 {
        assert_eq!(encoded[i], expected_bytes[i], "byte[{}] mismatch", i);
    }
    let decoded = decode_values(&encoded);
    assert_eq!(decoded.len(), 8);
    for i in 0..8 {
        assert_eq!(decoded[i], input[i], "decoded[{}] mismatch", i);
    }
}

#[test]
fn test_roundtrip_single_value() {
    let input = [3.14];
    let (encoded, length) = encode_values(&input);
    assert_eq!(length, 20);
    let expected_bytes: [u8; 20] = [
        0x10, 0x40, 0x09, 0x1e, 0xb8, 0x51, 0xeb, 0x85,
        0x1f, 0xc5, 0xef, 0xfe, 0x23, 0xd7, 0x0a, 0x3d,
        0x70, 0xa3, 0xc0, 0x00,
    ];
    for i in 0..20 {
        assert_eq!(encoded[i], expected_bytes[i], "byte[{}] mismatch", i);
    }
    let decoded = decode_values(&encoded);
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], 3.14);
}

#[test]
fn test_roundtrip_two_values() {
    let input = [1.0, 2.0];
    let (encoded, length) = encode_values(&input);
    assert_eq!(length, 23);
    let expected_bytes: [u8; 23] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xc2, 0x5f, 0xff, 0xc5, 0xf7, 0xff, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    for i in 0..23 {
        assert_eq!(encoded[i], expected_bytes[i], "byte[{}] mismatch", i);
    }
    let decoded = decode_values(&encoded);
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0], 1.0);
    assert_eq!(decoded[1], 2.0);
}

#[test]
fn test_roundtrip_negative_values() {
    let input = [-1.0, 0.0, 1.0, -100.5];
    let (encoded, length) = encode_values(&input);
    assert_eq!(length, 29);
    let expected_bytes: [u8; 29] = [
        0x10, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xc0, 0x65, 0xff, 0xc7, 0xff, 0x81, 0x3f,
        0xfa, 0x93, 0x80, 0x0b, 0xfa, 0x12, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x10, 0x00,
    ];
    for i in 0..29 {
        assert_eq!(encoded[i], expected_bytes[i], "byte[{}] mismatch", i);
    }
    let decoded = decode_values(&encoded);
    assert_eq!(decoded.len(), 4);
    assert_eq!(decoded[0], -1.0);
    assert_eq!(decoded[1], 0.0);
    assert_eq!(decoded[2], 1.0);
    assert_eq!(decoded[3], -100.5);
}

#[test]
fn test_roundtrip_zero() {
    let input = [0.0];
    let (encoded, length) = encode_values(&input);
    assert_eq!(length, 20);
    let expected_bytes: [u8; 20] = [
        0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xc3, 0xff, 0xff, 0x80, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x10, 0x00,
    ];
    for i in 0..20 {
        assert_eq!(encoded[i], expected_bytes[i], "byte[{}] mismatch", i);
    }
    let decoded = decode_values(&encoded);
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0], 0.0);
}

// ── float_cache_print (just ensure it doesn't panic) ────────────────

#[test]
fn test_float_cache_print() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 0 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    let ret = enc.float_cache_print();
    assert_eq!(ret, 0);
}

fn main() {}
