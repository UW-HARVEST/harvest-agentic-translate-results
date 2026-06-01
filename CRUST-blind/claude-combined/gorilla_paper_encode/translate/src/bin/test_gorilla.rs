#![allow(dead_code)]
use gorilla_paper_encode::gorilla::{
    bitslen, leading_zero64, trailing_zero64, BitReader, BitWriter, FloatDecoder, FloatEncoder,
};

fn make_encoder() -> FloatEncoder {
    let mut e = FloatEncoder {
        w: BitWriter {
            cache: [0u8; 1024],
            pos: 0,
            byte: 0,
            bit_count: 0,
        },
        val: 0,
        leading: 0,
        trailing: 0,
        first: false,
        finished: false,
    };
    e.float_encoder_init();
    e
}

fn make_decoder() -> FloatDecoder<'static> {
    FloatDecoder {
        val: 0,
        leading: 0,
        trailing: 0,
        br: BitReader {
            data: &[],
            len: 0,
            v: 0,
            n: 0,
        },
        b: [0u8; 1024],
        first: false,
        finished: false,
        err: 0,
    }
}

fn make_bitwriter() -> BitWriter {
    let mut w = BitWriter {
        cache: [0u8; 1024],
        pos: 0,
        byte: 0,
        bit_count: 0,
    };
    w.bitwriter_init();
    w
}

#[test]
fn test_bitslen_zero() {
    assert_eq!(bitslen(0), 0);
}

#[test]
fn test_bitslen_one() {
    assert_eq!(bitslen(1), 1);
}

#[test]
fn test_bitslen_255() {
    assert_eq!(bitslen(255), 8);
}

#[test]
fn test_bitslen_256() {
    assert_eq!(bitslen(256), 9);
}

#[test]
fn test_bitslen_u16_max() {
    assert_eq!(bitslen(65535), 16);
}

#[test]
fn test_bitslen_u16_max_plus_one() {
    assert_eq!(bitslen(65536), 17);
}

#[test]
fn test_bitslen_u32_max() {
    assert_eq!(bitslen(0xFFFFFFFF), 32);
}

#[test]
fn test_bitslen_u32_max_plus_one() {
    assert_eq!(bitslen(0x100000000), 33);
}

#[test]
fn test_bitslen_u64_max() {
    assert_eq!(bitslen(0xFFFFFFFFFFFFFFFF), 64);
}

#[test]
fn test_leading_zero64_zero() {
    assert_eq!(leading_zero64(0), 64);
}

#[test]
fn test_leading_zero64_one() {
    assert_eq!(leading_zero64(1), 63);
}

#[test]
fn test_leading_zero64_u64_max() {
    assert_eq!(leading_zero64(0xFFFFFFFFFFFFFFFF), 0);
}

#[test]
fn test_leading_zero64_high_bit() {
    assert_eq!(leading_zero64(0x8000000000000000), 0);
}

#[test]
fn test_leading_zero64_256() {
    assert_eq!(leading_zero64(256), 55);
}

#[test]
fn test_trailing_zero64_zero() {
    assert_eq!(trailing_zero64(0), 64);
}

#[test]
fn test_trailing_zero64_one() {
    assert_eq!(trailing_zero64(1), 0);
}

#[test]
fn test_trailing_zero64_two() {
    assert_eq!(trailing_zero64(2), 1);
}

#[test]
fn test_trailing_zero64_eight() {
    assert_eq!(trailing_zero64(8), 3);
}

#[test]
fn test_trailing_zero64_high_bit() {
    assert_eq!(trailing_zero64(0x8000000000000000), 63);
}

#[test]
fn test_trailing_zero64_256() {
    assert_eq!(trailing_zero64(256), 8);
}

#[test]
fn test_trailing_zero64_u64_max() {
    assert_eq!(trailing_zero64(0xFFFFFFFFFFFFFFFF), 0);
}

#[test]
fn test_bitwriter_init_state() {
    let w = make_bitwriter();
    assert_eq!(w.pos, 0);
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_bitwriter_write_bit_single() {
    let mut w = make_bitwriter();
    w.write_bit(true);
    assert_eq!(w.pos, 0);
    assert_eq!(w.bit_count, 7);
    assert_eq!(w.byte, 0b1000_0000);
}

#[test]
fn test_bitwriter_write_bit_full_byte() {
    let mut w = make_bitwriter();
    // Write 1010_1010
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    w.write_bit(false);
    w.write_bit(true);
    w.write_bit(false);
    assert_eq!(w.pos, 1);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.byte, 0);
    assert_eq!(w.cache[0], 0xAA);
}

#[test]
fn test_bitwriter_write_byte_aligned() {
    let mut w = make_bitwriter();
    // bit_count starts at 8 -> byte aligned
    w.write_byte(0xAB);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xAB);
    // After write_byte: byte = b << bit_count = 0xAB << 8 -> 0 (in our impl)
    assert_eq!(w.byte, 0);
    assert_eq!(w.bit_count, 8);
}

#[test]
fn test_bitwriter_write_bits_8_aligned() {
    let mut w = make_bitwriter();
    w.write_bits(0xAB, 8);
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xAB);
}

#[test]
fn test_bitwriter_write_bits_64() {
    let mut w = make_bitwriter();
    let val: u64 = 0x40A1F80000000000;
    w.write_bits(val, 64);
    assert_eq!(w.pos, 8);
    assert_eq!(w.cache[0], 0x40);
    assert_eq!(w.cache[1], 0xA1);
    assert_eq!(w.cache[2], 0xF8);
    assert_eq!(w.cache[3], 0x00);
    assert_eq!(w.cache[4], 0x00);
    assert_eq!(w.cache[5], 0x00);
    assert_eq!(w.cache[6], 0x00);
    assert_eq!(w.cache[7], 0x00);
}

#[test]
fn test_bitwriter_write_bits_invalid() {
    let mut w = make_bitwriter();
    assert_eq!(w.write_bits(0, 65), -1);
    assert_eq!(w.write_bits(0, -1), -1);
}

#[test]
fn test_bitwriter_write_flush() {
    let mut w = make_bitwriter();
    w.write_bit(true);
    w.write_bit(false);
    // bit_count is 6
    w.write_flush(false);
    // bit_count should now be 8 again, pos should be 1
    assert_eq!(w.pos, 1);
    assert_eq!(w.bit_count, 8);
    assert_eq!(w.cache[0], 0b1000_0000);
}

#[test]
fn test_bitwriter_append_to_cache() {
    let mut w = make_bitwriter();
    w.byte = 0xCD;
    w.append_to_cache();
    assert_eq!(w.pos, 1);
    assert_eq!(w.cache[0], 0xCD);
}

#[test]
fn test_float_encoder_init_state() {
    let e = make_encoder();
    assert_eq!(e.first, true);
    assert_eq!(e.finished, false);
    assert_eq!(e.leading, !0u64);
    assert_eq!(e.trailing, 0);
    assert_eq!(e.val, 0);
    // After init, the first byte 0x10 was written via write_byte
    assert_eq!(e.w.pos, 1);
    assert_eq!(e.w.cache[0], 0x10);
}

// Reference encoded bytes from C `helpers encode` runs:
//   2300,2400,2500,2600,2700,2800,2900,3000:
//   length = 31, bytes:
//   10 40 a1 f8 00 00 00 00 00 dc 3e 79 4e d2 3e e2 98 7e 69 8e f1 7d fa fb 80 00 00 00 00 08 00
#[test]
fn test_float_encode_block_2300_to_3000() {
    let mut e = make_encoder();
    let arr = [2300.0_f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for &v in arr.iter() {
        e.float_encode_write(v);
    }
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let expected: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e,
        0xe2, 0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00,
    ];
    assert_eq!(length, 31);
    assert_eq!(&buffer[..31], &expected[..]);
}

// length=20, bytes: 10 40 59 00 00 00 00 00 00 c5 f7 f4 20 00 00 00 00 00 20 00
#[test]
fn test_float_encode_single_value() {
    let mut e = make_encoder();
    e.float_encode_write(100.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let expected: [u8; 20] = [
        0x10, 0x40, 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc5, 0xf7, 0xf4, 0x20, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    assert_eq!(length, 20);
    assert_eq!(&buffer[..20], &expected[..]);
}

// 1.0 1.0 1.0 -> length=23
// 10 3f f0 00 00 00 00 00 00 40 02 00 30 ff 00 20 00 00 00 00 00 04 00
#[test]
fn test_float_encode_three_ones() {
    let mut e = make_encoder();
    e.float_encode_write(1.0_f64);
    e.float_encode_write(1.0_f64);
    e.float_encode_write(1.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let expected: [u8; 23] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x02, 0x00, 0x30, 0xff, 0x00,
        0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
    ];
    assert_eq!(length, 23);
    assert_eq!(&buffer[..23], &expected[..]);
}

// 1.0 2.0 3.0 -> length=25
// 10 3f f0 00 00 00 00 00 00 c2 5f ff d8 0f 17 df f8 00 00 00 00 00 00 80 00
#[test]
fn test_float_encode_one_two_three() {
    let mut e = make_encoder();
    e.float_encode_write(1.0_f64);
    e.float_encode_write(2.0_f64);
    e.float_encode_write(3.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let expected: [u8; 25] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x5f, 0xff, 0xd8, 0x0f, 0x17,
        0xdf, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00,
    ];
    assert_eq!(length, 25);
    assert_eq!(&buffer[..25], &expected[..]);
}

#[test]
fn test_float_encode_decode_roundtrip_8_values() {
    let mut e = make_encoder();
    let arr = [2300.0_f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for &v in arr.iter() {
        e.float_encode_write(v);
    }
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 8);
    for i in 0..8 {
        assert_eq!(de_arr[i], arr[i]);
    }
}

#[test]
fn test_float_encode_decode_roundtrip_single() {
    let mut e = make_encoder();
    e.float_encode_write(100.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 1);
    assert_eq!(de_arr[0], 100.0);
}

// NOTE: C decoder has buggy behavior for sequences of repeated values.
// When delta=0, C does not trip the NaN sentinel and instead produces
// 21 corrupted outputs. We mirror this behavior in Rust and assert it.
// Ground truth captured from running the original C `float_decode_block`.
#[test]
fn test_float_encode_decode_three_ones() {
    let mut e = make_encoder();
    e.float_encode_write(1.0);
    e.float_encode_write(1.0);
    e.float_encode_write(1.0);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 21);
    // First two are real 1.0
    assert_eq!(de_arr[0].to_bits(), 0x3ff0000000000000u64);
    assert_eq!(de_arr[1].to_bits(), 0x3ff0000000000000u64);
    // Remaining 19 are the corrupted bit pattern from C
    for i in 2..21 {
        assert_eq!(de_arr[i].to_bits(), 0x3fe00187f8010000u64);
    }
}

#[test]
fn test_float_encode_decode_one_two_three() {
    let mut e = make_encoder();
    e.float_encode_write(1.0);
    e.float_encode_write(2.0);
    e.float_encode_write(3.0);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 3);
    assert_eq!(de_arr[0], 1.0);
    assert_eq!(de_arr[1], 2.0);
    assert_eq!(de_arr[2], 3.0);
}

#[test]
fn test_float_encode_decode_negative_and_fractional() {
    let mut e = make_encoder();
    let arr = [-1.5_f64, -0.25, 3.14159, 2.71828, 0.0];
    for &v in arr.iter() {
        e.float_encode_write(v);
    }
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, arr.len() as u32);
    for i in 0..arr.len() {
        assert_eq!(de_arr[i], arr[i]);
    }
}

// BitReader tests
#[test]
fn test_bitreader_reset_state() {
    let mut data = [0xABu8, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    // After reset+readbuf, since len=10, byte_n=8 (because n=0 -> 8-0=8, len>=8)
    // So v should be loaded from first 8 bytes (big-endian), n=64, len=2
    assert_eq!(br.n, 64);
    assert_eq!(br.len, 2);
    let expected_v: u64 = 0xABCDEF123456789Au64;
    assert_eq!(br.v, expected_v);
}

#[test]
fn test_bitreader_read_bits_partial() {
    let mut data = [0xABu8, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    let v = br.read_bits(8);
    assert_eq!(v, 0xAB);
    let v = br.read_bits(8);
    assert_eq!(v, 0xCD);
}

#[test]
fn test_bitreader_read_bit() {
    let mut data = [0x80u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    let b1 = br.read_bit();
    assert_eq!(b1, 1);
    let b2 = br.read_bit();
    assert_eq!(b2, 0);
}

#[test]
fn test_bitreader_can_read_bitfast() {
    let mut data = [0xFFu8; 8];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    assert_eq!(br.can_read_bitfast(), true);
    // Read down to one bit left
    br.read_bits(63);
    assert_eq!(br.n, 1);
    assert_eq!(br.can_read_bitfast(), false);
}

#[test]
fn test_bitreader_read_bitfast() {
    let mut data = [0xC0u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    let b1 = br.read_bitfast();
    assert_eq!(b1, true);
    assert_eq!(br.n, 63);
    let b2 = br.read_bitfast();
    assert_eq!(b2, true);
    assert_eq!(br.n, 62);
    let b3 = br.read_bitfast();
    assert_eq!(b3, false);
}

#[test]
fn test_bitreader_read_bits_64() {
    let mut data = [0xABu8, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    let v = br.read_bits(64);
    assert_eq!(v, 0xABCDEF123456789Au64);
    // Now we should have reloaded 2 bytes, n=16, len=0
    assert_eq!(br.len, 0);
    assert_eq!(br.n, 16);
}

#[test]
fn test_bitreader_bit_readbuf_partial() {
    // Use a 4-byte buffer
    let mut data = [0x12u8, 0x34, 0x56, 0x78];
    let mut br = BitReader {
        data: &[],
        len: 0,
        v: 0,
        n: 0,
    };
    br.bitread_reset(&mut data);
    // Should have loaded all 4 bytes, n=32
    assert_eq!(br.n, 32);
    assert_eq!(br.len, 0);
    // v has bytes shifted into the high portion
    let expected_v: u64 = 0x1234567800000000u64;
    assert_eq!(br.v, expected_v);
}

#[test]
fn test_float_decoder_setbytes() {
    // Encode a known value first
    let mut e = make_encoder();
    e.float_encode_write(42.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut buf_copy = buffer[..length as usize].to_vec();
    let ret = d.float_decode_setbytes(&mut buf_copy);
    assert_eq!(ret, 0);
    // First value is the raw bits of 42.0
    assert_eq!(d.val, (42.0_f64).to_bits());
    assert_eq!(d.first, true);
    assert_eq!(d.finished, false);
    assert_eq!(d.err, 0);
    assert_eq!(d.leading, 0);
    assert_eq!(d.trailing, 0);
}

// NOTE: Same C decoder bug for repeated values. See ground truth from C run.
#[test]
fn test_float_encode_decode_repeated_values() {
    let mut e = make_encoder();
    let arr = [5.0_f64, 5.0, 5.0, 5.0, 5.0];
    for &v in arr.iter() {
        e.float_encode_write(v);
    }
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 21);
    // First two are real 5.0
    assert_eq!(de_arr[0].to_bits(), 0x4014000000000000u64);
    assert_eq!(de_arr[1].to_bits(), 0x4014000000000000u64);
    // Remaining 19 are the corrupted bit pattern from C
    for i in 2..21 {
        assert_eq!(de_arr[i].to_bits(), 0x40040080040062fbu64);
    }
}

#[test]
fn test_float_encode_decode_two_values() {
    let mut e = make_encoder();
    e.float_encode_write(1.0_f64);
    e.float_encode_write(2.0_f64);
    let mut buffer = [0u8; 1024];
    let mut length: u32 = 0;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = make_decoder();
    let mut de_arr = [0.0_f64; 64];
    let mut de_len: u32 = 0;
    let ret = d.float_decode_block(&buffer[..length as usize], &mut de_arr, &mut de_len);
    assert_eq!(ret, 0);
    assert_eq!(de_len, 2);
    assert_eq!(de_arr[0], 1.0);
    assert_eq!(de_arr[1], 2.0);
}

fn main() {}
