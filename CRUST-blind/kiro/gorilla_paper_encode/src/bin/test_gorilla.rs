use gorilla_paper_encode::gorilla::*;

// ===== Helper function tests =====

#[test]
fn test_bitslen() {
    assert_eq!(bitslen(0), 0);
    assert_eq!(bitslen(1), 1);
    assert_eq!(bitslen(2), 2);
    assert_eq!(bitslen(3), 2);
    assert_eq!(bitslen(0x7f), 7);
    assert_eq!(bitslen(0x80), 8);
    assert_eq!(bitslen(0xff), 8);
    assert_eq!(bitslen(0x100), 9);
    assert_eq!(bitslen(0xffff), 16);
    assert_eq!(bitslen(0xffffffff), 32);
    assert_eq!(bitslen(0xffffffffffffffff), 64);
    assert_eq!(bitslen(0x8000000000000000), 64);
}

#[test]
fn test_leading_zero64() {
    assert_eq!(leading_zero64(0), 64);
    assert_eq!(leading_zero64(1), 63);
    assert_eq!(leading_zero64(2), 62);
    assert_eq!(leading_zero64(0x80), 56);
    assert_eq!(leading_zero64(0x8000000000000000), 0);
    assert_eq!(leading_zero64(0xffffffffffffffff), 0);
    assert_eq!(leading_zero64(0x100), 55);
}

#[test]
fn test_trailing_zero64() {
    assert_eq!(trailing_zero64(0), 64);
    assert_eq!(trailing_zero64(1), 0);
    assert_eq!(trailing_zero64(2), 1);
    assert_eq!(trailing_zero64(4), 2);
    assert_eq!(trailing_zero64(0x80), 7);
    assert_eq!(trailing_zero64(0x8000000000000000), 63);
    assert_eq!(trailing_zero64(0xffffffffffffffff), 0);
    assert_eq!(trailing_zero64(0x100), 8);
}

// ===== Constants tests =====

#[test]
fn test_constants() {
    assert_eq!(Nan, 0x7FF8000000000001);
    assert_eq!(de_bruijn64, 0x03f79d71b4ca8b09);
    assert_eq!(de_bruijn64_tab.len(), 64);
    assert_eq!(len8_tab.len(), 256);
    assert_eq!(len8_tab[0], 0);
    assert_eq!(len8_tab[1], 1);
    assert_eq!(len8_tab[255], 8);
}

// ===== BitWriter tests =====

#[test]
fn test_bitwriter_init() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 99, byte: 99, bit_count: 99 };
    w.bitwriter_init();
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_bit_full_byte() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    // Write 10110010 = 0xB2
    let bits = [true, false, true, true, false, false, true, false];
    for &b in &bits {
        w.write_bit(b);
    }
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xB2);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_byte() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_byte(0xAB);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xAB);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_bits_16() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bits(0xDEAD, 16);
    assert_eq!(w.pos, 2);
    assert_eq!(w.cache[0], 0xDE);
    assert_eq!(w.cache[1], 0xAD);
    assert_eq!(w.byte, 0);
}

#[test]
fn test_write_bits_64() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bits(0xDEADBEEFCAFEBABE, 64);
    assert_eq!(w.pos, 8);
    assert_eq!(w.cache[0], 0xDE);
    assert_eq!(w.cache[1], 0xAD);
    assert_eq!(w.cache[2], 0xBE);
    assert_eq!(w.cache[3], 0xEF);
    assert_eq!(w.cache[4], 0xCA);
    assert_eq!(w.cache[5], 0xFE);
    assert_eq!(w.cache[6], 0xBA);
    assert_eq!(w.cache[7], 0xBE);
}

#[test]
fn test_write_bits_edge_cases() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    assert_eq!(w.write_bits(0, 65), -1);
    assert_eq!(w.write_bits(0, -1), -1);
    assert_eq!(w.write_bits(0, 0), 0);
}

#[test]
fn test_write_flush_zero() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    w.write_flush(false);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xA0);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_write_flush_one() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    w.write_flush(true);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xBF);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
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

// ===== FloatEncoder tests =====

#[test]
fn test_float_encoder_init() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: true,
    };
    enc.float_encoder_init();
    assert!(enc.first);
    assert!(!enc.finished);
    assert_eq!(enc.leading, !0u64);
    assert_eq!(enc.w.pos, 1);
    assert_eq!(enc.w.cache[0], 0x10);
}

#[test]
fn test_encode_standard_sequence() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    let arr = [2300.0, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for v in &arr {
        enc.float_encode_write(*v);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);
    assert_eq!(len, 31);
    let expected: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e, 0xe2,
        0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb,
        0x80, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00,
    ];
    assert_eq!(&buf[..31], &expected);
}

#[test]
fn test_encode_decode_standard_roundtrip() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    let arr = [2300.0f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for v in &arr {
        enc.float_encode_write(*v);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);

    let mut dec = FloatDecoder {
        val: 0, leading: 0, trailing: 0,
        br: BitReader { data: &[], len: 0, v: 0, n: 0 },
        b: [0u8; 1024], first: false, finished: false, err: 0,
    };
    let mut res = [0.0f64; 64];
    let mut res_len: U32 = 0;
    dec.float_decode_block(&buf[..len as usize], &mut res, &mut res_len);
    assert_eq!(res_len, 8);
    for i in 0..8 {
        assert_eq!(res[i], arr[i]);
    }
}

#[test]
fn test_encode_single_value() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);
    assert_eq!(len, 20);
    let expected: [u8; 20] = [
        0x10, 0x40, 0x45, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xc5, 0xf7, 0xf7, 0xa0, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x20, 0x00,
    ];
    assert_eq!(&buf[..20], &expected);
}

#[test]
fn test_decode_single_value() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);

    let mut dec = FloatDecoder {
        val: 0, leading: 0, trailing: 0,
        br: BitReader { data: &[], len: 0, v: 0, n: 0 },
        b: [0u8; 1024], first: false, finished: false, err: 0,
    };
    let mut res = [0.0f64; 64];
    let mut res_len: U32 = 0;
    dec.float_decode_block(&buf[..len as usize], &mut res, &mut res_len);
    assert_eq!(res_len, 1);
    assert_eq!(res[0], 42.0);
}

#[test]
fn test_encode_identical_values() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    for _ in 0..5 {
        enc.float_encode_write(100.0);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);
    assert_eq!(len, 26);
    let expected: [u8; 26] = [
        0x10, 0x40, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x40, 0x02, 0x00, 0x10, 0x00, 0x80, 0x0c,
        0x5f, 0x7f, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x02, 0x00,
    ];
    assert_eq!(&buf[..26], &expected);
}

#[test]
fn test_encode_large_spread() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    let arr = [1e-300f64, 1e300, 0.0, -0.0];
    for v in &arr {
        enc.float_encode_write(*v);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);
    assert_eq!(len, 41);
    let expected: [u8; 41] = [
        0x10, 0x01, 0xa5, 0x6e, 0x1f, 0xc2, 0xf8, 0xf3,
        0x59, 0xc3, 0xff, 0xf9, 0x28, 0xa2, 0x34, 0xaf,
        0x88, 0x6c, 0x5c, 0x3e, 0xfe, 0x37, 0xe4, 0x3c,
        0x88, 0x00, 0x75, 0x9f, 0x00, 0x3c, 0x00, 0x7f,
        0xfc, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
        0x00,
    ];
    assert_eq!(&buf[..41], &expected);
}

#[test]
fn test_decode_large_spread_roundtrip() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    let arr = [1e-300f64, 1e300, 0.0, -0.0];
    for v in &arr {
        enc.float_encode_write(*v);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);

    let mut dec = FloatDecoder {
        val: 0, leading: 0, trailing: 0,
        br: BitReader { data: &[], len: 0, v: 0, n: 0 },
        b: [0u8; 1024], first: false, finished: false, err: 0,
    };
    let mut res = [0.0f64; 64];
    let mut res_len: U32 = 0;
    dec.float_decode_block(&buf[..len as usize], &mut res, &mut res_len);
    assert_eq!(res_len, 4);
    assert_eq!(res[0], 1e-300);
    assert_eq!(res[1], 1e300);
    assert_eq!(res[2], 0.0);
    // -0.0 check: bits should match
    assert_eq!(res[3].to_bits(), (-0.0f64).to_bits());
}

#[test]
fn test_encode_negative_values() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    let arr = [-1.0f64, -2.0, 0.0, 1.0];
    for v in &arr {
        enc.float_encode_write(*v);
    }
    let mut buf = [0u8; 1024];
    let mut len: U32 = 0;
    enc.float_encode_flush(&mut buf, &mut len);
    assert_eq!(len, 28);
    let expected: [u8; 28] = [
        0x10, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0xc2, 0x5f, 0xff, 0xc0, 0x17, 0x88, 0xaf,
        0xff, 0x0f, 0xf0, 0x02, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x40, 0x00,
    ];
    assert_eq!(&buf[..28], &expected);
}

#[test]
fn test_encode_flush_idempotent() {
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    let mut buf1 = [0u8; 1024];
    let mut len1: U32 = 0;
    enc.float_encode_flush(&mut buf1, &mut len1);
    let mut buf2 = [0u8; 1024];
    let mut len2: U32 = 0;
    enc.float_encode_flush(&mut buf2, &mut len2);
    // Second flush should produce same result (finished flag prevents re-encoding)
    assert_eq!(len1, len2);
    assert_eq!(&buf1[..len1 as usize], &buf2[..len2 as usize]);
}

// ===== BitReader tests =====

#[test]
fn test_bitreader_read_bits() {
    let data: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34];
    let mut br = BitReader { data: &data, len: data.len() as u32, v: 0, n: 0 };
    br.bit_readbuf();
    let v = br.read_bits(8);
    assert_eq!(v, 0xDE);
    let v = br.read_bits(8);
    assert_eq!(v, 0xAD);
}

#[test]
fn test_bitreader_read_bit() {
    let data: Vec<u8> = vec![0x80]; // 10000000
    let mut br = BitReader { data: &data, len: data.len() as u32, v: 0, n: 0 };
    br.bit_readbuf();
    assert_eq!(br.read_bit(), 1);
    assert_eq!(br.read_bit(), 0);
}

#[test]
fn test_bitreader_can_read_bitfast() {
    let data: Vec<u8> = vec![0xFF, 0xFF];
    let mut br = BitReader { data: &data, len: data.len() as u32, v: 0, n: 0 };
    br.bit_readbuf();
    assert!(br.can_read_bitfast());
}

#[test]
fn test_bitreader_read_bitfast() {
    let data: Vec<u8> = vec![0xA0]; // 10100000
    let mut br = BitReader { data: &data, len: data.len() as u32, v: 0, n: 0 };
    br.bit_readbuf();
    assert!(br.read_bitfast()); // 1
    assert!(!br.read_bitfast()); // 0
    assert!(br.read_bitfast()); // 1
}

#[test]
fn test_write_read_roundtrip_bits() {
    // Write some bits, then read them back
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_bits(0xCAFE, 16);
    w.write_bits(0xBABE, 16);
    w.write_flush(false);

    let data = &w.cache[..w.pos as usize];
    let mut br = BitReader { data, len: data.len() as u32, v: 0, n: 0 };
    br.bit_readbuf();
    assert_eq!(br.read_bits(16), 0xCAFE);
    assert_eq!(br.read_bits(16), 0xBABE);
}

#[test]
fn test_float_cache_print() {
    // Just ensure it doesn't panic
    let mut enc = FloatEncoder {
        w: BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 },
        val: 0, leading: 0, trailing: 0, first: false, finished: false,
    };
    enc.float_encoder_init();
    enc.float_encode_write(42.0);
    enc.float_cache_print();
}

#[test]
fn test_write_flush_noop_when_aligned() {
    let mut w = BitWriter { cache: [0u8; 1024], pos: 0, byte: 0, bit_count: 8 };
    w.bitwriter_init();
    w.write_byte(0xFF);
    let pos_before = w.pos;
    w.write_flush(false);
    // Already aligned, flush should be a no-op
    assert_eq!(w.pos, pos_before);
}

fn main() {}
