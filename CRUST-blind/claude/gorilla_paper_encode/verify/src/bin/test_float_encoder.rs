use gorilla_paper_encode::gorilla::{BitWriter, FloatEncoder};

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
fn test_init_state() {
    let e = fresh_encoder();
    assert_eq!(e.val, 0);
    // C's float_encoder_init sets s->leading = ~0.
    assert_eq!(e.leading, !0u64);
    assert_eq!(e.trailing, 0);
    assert!(e.first);
    assert!(!e.finished);
    // The init writes 0x10 byte to the bitwriter cache.
    assert_eq!(e.w.cache[0], 0x10);
    assert_eq!(e.w.pos, 1);
    assert_eq!(e.w.byte, 0x00);
    assert_eq!(e.w.bit_count, 8);
}

#[test]
fn test_encode_single_value_round_trip_via_known_bytes() {
    // The C ground-truth output for the 8-element 2300..3000 sequence.
    // This exercises encode + flush together.
    let mut e = fresh_encoder();
    let arr = [2300.0f64, 2400.0, 2500.0, 2600.0, 2700.0, 2800.0, 2900.0, 3000.0];
    for v in arr.iter() {
        let rc = e.float_encode_write(*v);
        assert_eq!(rc, 0);
    }
    // C ground truth: pre-flush pos=20 byte=c0 bit_count=6 finished=0.
    assert_eq!(e.w.pos, 20);
    assert_eq!(e.w.byte, 0xC0);
    assert_eq!(e.w.bit_count, 6);
    assert!(!e.finished);

    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    let rc = e.float_encode_flush(&mut buffer, &mut length);
    assert_eq!(rc, 31);
    assert_eq!(length, 31);
    assert!(e.finished);
    let expected: [u8; 31] = [
        0x10, 0x40, 0xa1, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x3e, 0x79, 0x4e, 0xd2, 0x3e,
        0xe2, 0x98, 0x7e, 0x69, 0x8e, 0xf1, 0x7d, 0xfa, 0xfb, 0x80, 0x00, 0x00, 0x00, 0x00, 0x08,
        0x00,
    ];
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_encode_first_value_64bit_payload() {
    // After writing 1.0 once, the encoder stores its raw bits as `val` and
    // emits 64 bits to the cache. C ground truth from probe4:
    //   first=0 val=3ff0000000000000 leading=ffffffffffffffff trailing=0
    //   pos=9 byte=00 bit_count=8.
    let mut e = fresh_encoder();
    let rc = e.float_encode_write(1.0);
    assert_eq!(rc, 0);
    assert_eq!(e.val, 0x3FF0000000000000u64);
    assert_eq!(e.leading, !0u64);
    assert_eq!(e.trailing, 0);
    assert!(!e.first);
    assert_eq!(e.w.pos, 9);
    assert_eq!(e.w.byte, 0x00);
    assert_eq!(e.w.bit_count, 8);
    // The 8-byte payload follows the initial 0x10 byte.
    assert_eq!(e.w.cache[0], 0x10);
    assert_eq!(e.w.cache[1], 0x3F);
    assert_eq!(e.w.cache[2], 0xF0);
    assert_eq!(e.w.cache[3], 0x00);
    assert_eq!(e.w.cache[4], 0x00);
    assert_eq!(e.w.cache[5], 0x00);
    assert_eq!(e.w.cache[6], 0x00);
    assert_eq!(e.w.cache[7], 0x00);
    assert_eq!(e.w.cache[8], 0x00);
}

#[test]
fn test_encode_two_equal_values_state() {
    // Writing the same value twice: vdelta=0, so just one zero bit is added.
    // C ground truth from probe4:
    //   first=0 val=3ff0000000000000 leading=0000000000000000 trailing=64
    //   pos=10 byte=00 bit_count=3.
    // (leading=0/trailing=64 result from leading_zero64(0)=64 masked with 0x1F→0,
    //  trailing_zero64(0)=64.)
    let mut e = fresh_encoder();
    e.float_encode_write(1.0);
    e.float_encode_write(1.0);
    assert_eq!(e.val, 0x3FF0000000000000u64);
    assert_eq!(e.leading, 0);
    assert_eq!(e.trailing, 64);
    assert!(!e.first);
    assert_eq!(e.w.pos, 10);
    assert_eq!(e.w.byte, 0x00);
    assert_eq!(e.w.bit_count, 3);
}

#[test]
fn test_encode_flush_produces_known_single_value_bytes() {
    // C ground truth: encoding the single value 1.0 then flushing produces
    // 20 bytes: 10 3f f0 00 00 00 00 00 00 c3 fc 00 80 00 00 00 00 00 10 00.
    let mut e = fresh_encoder();
    e.float_encode_write(1.0);
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    let rc = e.float_encode_flush(&mut buffer, &mut length);
    assert_eq!(rc, 20);
    assert_eq!(length, 20);
    let expected: [u8; 20] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc3, 0xfc, 0x00, 0x80, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x10, 0x00,
    ];
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_encode_two_different_values_bytes() {
    // C ground truth: two_neq len=23 bytes:
    //   10 3f f0 00 00 00 00 00 00 c2 5f ff c5 f7 ff 00 00 00 00 00 00 20 00.
    let mut e = fresh_encoder();
    e.float_encode_write(1.0);
    e.float_encode_write(2.0);
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);
    let expected: [u8; 23] = [
        0x10, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x5f, 0xff, 0xc5, 0xf7, 0xff,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    assert_eq!(length, 23);
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_encode_negative_values_bytes() {
    // C ground truth: neg len=25 bytes:
    //   10 bf f0 00 00 00 00 00 00 c2 5f ff d8 0f 00 17 fe 00 00 00 00 00 00 20 00.
    let mut e = fresh_encoder();
    e.float_encode_write(-1.0);
    e.float_encode_write(-2.0);
    e.float_encode_write(-3.0);
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);
    let expected: [u8; 25] = [
        0x10, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc2, 0x5f, 0xff, 0xd8, 0x0f, 0x00,
        0x17, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    assert_eq!(length, 25);
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_encode_zeros_bytes() {
    // C ground truth: zeros (4x 0.0) len=25 bytes:
    //   10 00 00 00 00 00 00 00 00 40 02 00 10 01 87 ff ff 00 00 00 00 00 00 20 00.
    let mut e = fresh_encoder();
    for _ in 0..4 {
        e.float_encode_write(0.0);
    }
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);
    let expected: [u8; 25] = [
        0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x02, 0x00, 0x10, 0x01, 0x87,
        0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00,
    ];
    assert_eq!(length, 25);
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_encode_three_consecutive_pi_e_phi_bytes() {
    // C ground truth: a5 len=37.
    let mut e = fresh_encoder();
    e.float_encode_write(3.14159);
    e.float_encode_write(2.71828);
    e.float_encode_write(1.61803);
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);
    let expected: [u8; 37] = [
        0x10, 0x40, 0x09, 0x21, 0xf9, 0xf0, 0x1b, 0x86, 0x6e, 0xd9, 0x9e, 0x4f, 0x78, 0x32, 0xd8,
        0xb8, 0xff, 0xc3, 0xef, 0xff, 0xc5, 0xc7, 0xaf, 0x97, 0x5d, 0x1f, 0xf0, 0xff, 0x00, 0x07,
        0x8d, 0xcd, 0xb3, 0x7c, 0x99, 0xb4, 0x00,
    ];
    assert_eq!(length, 37);
    assert_eq!(&buffer[..length as usize], &expected[..]);
}

#[test]
fn test_flush_marks_finished_and_appends_nan() {
    // The flush call appends the sentinel NaN and pads. After flush, finished=true.
    let mut e = fresh_encoder();
    e.float_encode_write(1.0);
    let mut buffer = [0u8; 1024];
    let mut length = 0u32;
    e.float_encode_flush(&mut buffer, &mut length);
    assert!(e.finished);
    // Calling flush a second time is idempotent (the !finished guard skips appending).
    let prev_pos = e.w.pos;
    let mut buffer2 = [0u8; 1024];
    let mut length2 = 0u32;
    e.float_encode_flush(&mut buffer2, &mut length2);
    assert_eq!(length2, prev_pos + 1);
    assert_eq!(&buffer2[..length2 as usize], &buffer[..length as usize]);
}

fn main() {}
