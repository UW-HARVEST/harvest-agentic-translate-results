use gorilla_paper_encode::gorilla::{BitReader, BitWriter, FloatDecoder, FloatEncoder};

fn fresh_decoder<'a>() -> FloatDecoder<'a> {
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

fn fresh_encoder() -> FloatEncoder {
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

#[test]
fn test_decode_block_eight_values() {
    // C ground truth: encoding 2300.0..3000.0 then decoding produces those same eight values.
    let buffer: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e,
        0xe2, 0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 8);
    let expected = [2300.0f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for i in 0..8 {
        assert_eq!(res[i], expected[i]);
    }
    assert_eq!(d.err, 0);
    assert!(d.finished);
}

#[test]
fn test_decode_block_single_value() {
    let buffer: [u8; 20] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc3, 0xfc, 0x00, 0x80, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x10, 0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 1);
    assert_eq!(res[0], 1.0);
}

#[test]
fn test_decode_block_two_different() {
    // 1.0, 2.0
    let buffer: [u8; 23] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x5f, 0xff, 0xc5, 0xf7, 0xff,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 2);
    assert_eq!(res[0], 1.0);
    assert_eq!(res[1], 2.0);
}

#[test]
fn test_decode_block_negatives() {
    // -1.0, -2.0, -3.0
    let buffer: [u8; 25] = [
        0x10, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x5f, 0xff, 0xd8, 0x0f, 0x00,
        0x17, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 3);
    assert_eq!(res[0], -1.0);
    assert_eq!(res[1], -2.0);
    assert_eq!(res[2], -3.0);
}

#[test]
fn test_decode_block_pi_e_phi() {
    let buffer: [u8; 37] = [
        0x10, 0x40, 0x09, 0x21, 0xf9, 0xf0, 0x1b, 0x86, 0x6e, 0xd9, 0x9e, 0x4f, 0x78, 0x32, 0xd8,
        0xb8, 0xff, 0xc3, 0xef, 0xff, 0xc5, 0xc7, 0xaf, 0x97, 0x5d, 0x1f, 0xf0, 0xff, 0x00, 0x07,
        0x8d, 0xcd, 0xb3, 0x7c, 0x99, 0xb4, 0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 3);
    assert_eq!(res[0], 3.14159);
    assert_eq!(res[1], 2.71828);
    assert_eq!(res[2], 1.61803);
}

#[test]
fn test_round_trip_eight_values() {
    let arr = [2300.0f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    let mut e = fresh_encoder();
    for v in arr.iter() {
        e.float_encode_write(*v);
    }
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer[..length as usize], &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 8);
    for i in 0..8 {
        assert_eq!(res[i], arr[i]);
    }
}

#[test]
fn test_round_trip_negatives() {
    let arr = [-1.0f64, -2.0, -3.0];
    let mut e = fresh_encoder();
    for v in arr.iter() {
        e.float_encode_write(*v);
    }
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer[..length as usize], &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 3);
    for i in 0..3 {
        assert_eq!(res[i], arr[i]);
    }
}

#[test]
fn test_round_trip_single_large_value() {
    let arr = [12345.6789f64];
    let mut e = fresh_encoder();
    for v in arr.iter() {
        e.float_encode_write(*v);
    }
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);

    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    let rc = d.float_decode_block(&buffer[..length as usize], &mut res, &mut res_len);
    assert_eq!(rc, 0);
    assert_eq!(res_len, 1);
    assert_eq!(res[0], 12345.6789);
}

#[test]
fn test_decode_block_state_after_run() {
    // After a successful decode, err remains 0 and finished is true.
    let buffer: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e,
        0xe2, 0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00,
    ];
    let mut d = fresh_decoder();
    let mut res = [0f64; 64];
    let mut res_len = 0u32;
    d.float_decode_block(&buffer, &mut res, &mut res_len);
    assert_eq!(d.err, 0);
    assert!(d.finished);
    // After encountering the sentinel NaN, the C `read_next` returns false
    // *before* storing into `s->val`. The last successful decode was 3000.0,
    // so `val` retains 3000.0's bit pattern.
    assert_eq!(d.val, 0x40a7700000000000u64);
}

fn main() {}
