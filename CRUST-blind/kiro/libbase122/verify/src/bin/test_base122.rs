use libbase122::base122::{self, BitReader, BitWriter};

// ===== BitReader tests =====

#[test]
fn test_bitreader_one_byte_0xff() {
    let data = [0xFFu8];
    let mut r = BitReader::new(&data);
    assert_eq!(r.read(7), (7, 127));
    assert_eq!(r.read(7), (1, 1));
    assert_eq!(r.read(7), (0, 0));
}

#[test]
fn test_bitreader_two_bytes_0xaa_0xff() {
    let data = [0xAAu8, 0xFF];
    let mut r = BitReader::new(&data);
    assert_eq!(r.read(7), (7, 85));
    assert_eq!(r.read(7), (7, 63));
    assert_eq!(r.read(7), (2, 3));
}

#[test]
fn test_bitreader_empty() {
    let data: [u8; 0] = [];
    let mut r = BitReader::new(&data);
    assert_eq!(r.read(7), (0, 0));
}

#[test]
fn test_bitreader_various_bit_counts() {
    let data = [0b10110011u8, 0b11001010];
    let mut r = BitReader::new(&data);
    assert_eq!(r.read(3), (3, 5));
    assert_eq!(r.read(5), (5, 19));
    assert_eq!(r.read(8), (8, 202));
}

// ===== BitWriter tests =====

#[test]
fn test_bitwriter_one_byte() {
    let mut buf = [0u8; 1];
    let buf_ptr = buf.as_mut_ptr();
    let mut w = BitWriter::new(Some(&mut buf), 1);
    assert!(w.write(1, 0x0F).is_ok());
    assert_eq!(unsafe { *buf_ptr }, 0x80);
    assert!(w.write(1, 0x0F).is_ok());
    assert_eq!(unsafe { *buf_ptr }, 0xC0);
    assert!(w.write(5, 0x0F).is_ok());
    assert_eq!(unsafe { *buf_ptr }, 0xDE);
    assert!(w.write(5, 0x0F).is_err());
}

#[test]
fn test_bitwriter_two_bytes() {
    let mut buf = [0u8; 2];
    let buf_ptr = buf.as_mut_ptr();
    let mut w = BitWriter::new(Some(&mut buf), 2);
    assert!(w.write(1, 0xFF).is_ok());
    assert_eq!(unsafe { std::slice::from_raw_parts(buf_ptr, 2) }, &[0x80, 0x00]);
    assert!(w.write(8, 0x0F).is_ok());
    assert_eq!(unsafe { std::slice::from_raw_parts(buf_ptr, 2) }, &[0x87, 0x80]);
    assert!(w.write(1, 0x0).is_ok());
    assert_eq!(unsafe { std::slice::from_raw_parts(buf_ptr, 2) }, &[0x87, 0x80]);
    assert!(w.write(1, 0xFF).is_ok());
    assert_eq!(unsafe { std::slice::from_raw_parts(buf_ptr, 2) }, &[0x87, 0xA0]);
    assert!(w.write(5, 0xFF).is_ok());
    assert_eq!(unsafe { std::slice::from_raw_parts(buf_ptr, 2) }, &[0x87, 0xBF]);
    assert!(w.write(1, 0xFF).is_err());
}

#[test]
fn test_bitwriter_count_only() {
    let mut w = BitWriter::new(None, 0);
    let _ = w.write(7, 0x55);
    let _ = w.write(7, 0x33);
    assert_eq!(w.cur_bit, 14);
}

// ===== Encode tests =====

#[test]
fn test_encode_empty() {
    assert_eq!(base122::encode(&[]).unwrap(), vec![]);
}

#[test]
fn test_encode_one_byte_ff() {
    assert_eq!(base122::encode(&[0xFF]).unwrap(), vec![0x7F, 0x40]);
}

#[test]
fn test_encode_four_bytes_aa() {
    assert_eq!(
        base122::encode(&[0xAA, 0xAA, 0xAA, 0xAA]).unwrap(),
        vec![0x55, 0x2A, 0x55, 0x2A, 0x50]
    );
}

#[test]
fn test_encode_null_ff() {
    assert_eq!(
        base122::encode(&[0x00, 0xFF]).unwrap(),
        vec![0xC2, 0xBF, 0x60]
    );
}

#[test]
fn test_encode_fuzz_crash_1() {
    assert_eq!(base122::encode(&[0x15]).unwrap(), vec![0xC7, 0x80]);
}

#[test]
fn test_encode_single_null() {
    assert_eq!(base122::encode(&[0x00]).unwrap(), vec![0xC2, 0x80]);
}

#[test]
fn test_encode_three_nulls() {
    assert_eq!(
        base122::encode(&[0x00, 0x00, 0x00]).unwrap(),
        vec![0xC2, 0x80, 0xC2, 0x80]
    );
}

#[test]
fn test_encode_sequential() {
    assert_eq!(
        base122::encode(&[0x01, 0x02, 0x03, 0x04, 0x05]).unwrap(),
        vec![0xC3, 0x80, 0x40, 0x30, 0x20, 0x14]
    );
}

#[test]
fn test_encode_fffc() {
    assert_eq!(
        base122::encode(&[0xFF, 0xFC]).unwrap(),
        vec![0x7F, 0x7F, 0xDE, 0x80]
    );
}

// ===== Decode tests =====

#[test]
fn test_decode_one_byte_ff() {
    assert_eq!(base122::decode(&[0x7F, 0x40]).unwrap(), vec![0xFF]);
}

#[test]
fn test_decode_four_bytes_aa() {
    assert_eq!(
        base122::decode(&[0x55, 0x2A, 0x55, 0x2A, 0x50]).unwrap(),
        vec![0xAA, 0xAA, 0xAA, 0xAA]
    );
}

#[test]
fn test_decode_null_ff() {
    assert_eq!(
        base122::decode(&[0xC2, 0xBF, 0x60]).unwrap(),
        vec![0x00, 0xFF]
    );
}

#[test]
fn test_decode_last7_ok() {
    // 0x7F 0xDE 0x80 -> 0xFE
    assert_eq!(base122::decode(&[0x7F, 0xDE, 0x80]).unwrap(), vec![0xFE]);
}

#[test]
fn test_decode_null_prefix() {
    // 0x00 0xDE 0x80 -> 0x00
    assert_eq!(base122::decode(&[0x00, 0xDE, 0x80]).unwrap(), vec![0x00]);
}

#[test]
fn test_decode_double_null() {
    assert_eq!(
        base122::decode(&[0x00, 0xC2, 0x80]).unwrap(),
        vec![0x00, 0x00]
    );
}

#[test]
fn test_decode_cf_81_60() {
    assert_eq!(
        base122::decode(&[0xCF, 0x81, 0x60]).unwrap(),
        vec![0x45, 0x07]
    );
}

#[test]
fn test_decode_empty() {
    assert_eq!(base122::decode(&[]).unwrap(), vec![]);
}

// ===== Decode error tests =====

#[test]
fn test_decode_err_last_extra_data() {
    let err = base122::decode(&[0x7F, 0x7F, 0x7F]).unwrap_err();
    assert!(err.message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_err_second_malformed() {
    let err = base122::decode(&[0xDE, 0xFF]).unwrap_err();
    assert!(err.message.contains("Second byte of two byte sequence malformed"));
}

#[test]
fn test_decode_err_first_malformed() {
    let err = base122::decode(&[0xFF]).unwrap_err();
    assert!(err.message.contains("First byte of two byte sequence malformed"));
}

#[test]
fn test_decode_err_missing_second() {
    let err = base122::decode(&[0xDE]).unwrap_err();
    assert!(err.message.contains("Two byte sequence is missing second byte"));
}

#[test]
fn test_decode_err_unexpected_extra() {
    let err = base122::decode(&[0xDE, 0xBF, 0x7F]).unwrap_err();
    assert!(err.message.contains("Got unexpected extra data after shortened two byte sequence"));
}

#[test]
fn test_decode_err_unrecognized_illegal() {
    let err = base122::decode(&[0xDA, 0xBF]).unwrap_err();
    assert!(err.message.contains("Got unrecognized illegal index"));
}

#[test]
fn test_decode_err_not_byte_multiple() {
    let err = base122::decode(&[0x7F]).unwrap_err();
    assert!(err.message.contains("Decoded data is not a byte multiple"));
}

#[test]
fn test_decode_err_last_byte_extra() {
    let err = base122::decode(&[0x7F, 0x7F]).unwrap_err();
    assert!(err.message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_err_shortened_extra() {
    let err = base122::decode(&[0x7F, 0xDF, 0xA0]).unwrap_err();
    assert!(err.message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_err_shortened_not_byte() {
    let err = base122::decode(&[0xDE, 0x80]).unwrap_err();
    assert!(err.message.contains("Decoded data is not a byte multiple"));
}

// ===== Roundtrip tests =====

#[test]
fn test_roundtrip_all_ones_0_to_20() {
    for i in 0..=20 {
        let input: Vec<u8> = vec![0xFF; i];
        let encoded = base122::encode(&input).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, input, "roundtrip failed for length {}", i);
    }
}

#[test]
fn test_roundtrip_single_byte_patterns() {
    for &b in &[0x00u8, 0x01, 0x7F, 0x80, 0xFE, 0x0A, 0x0D, 0x22, 0x26, 0x5C] {
        let input = vec![b];
        let encoded = base122::encode(&input).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, input, "roundtrip failed for byte 0x{:02x}", b);
    }
}

#[test]
fn test_encode_single_byte_patterns() {
    // Ground truth from C: specific encode outputs for single bytes
    assert_eq!(base122::encode(&[0x00]).unwrap(), vec![0xC2, 0x80]);
    assert_eq!(base122::encode(&[0x01]).unwrap(), vec![0xC3, 0x80]);
    assert_eq!(base122::encode(&[0x7F]).unwrap(), vec![0x3F, 0x40]);
    assert_eq!(base122::encode(&[0x80]).unwrap(), vec![0x40, 0xDE, 0x80]);
    assert_eq!(base122::encode(&[0xFE]).unwrap(), vec![0x7F, 0xDE, 0x80]);
    assert_eq!(base122::encode(&[0x0A]).unwrap(), vec![0x05, 0xDE, 0x80]);
    assert_eq!(base122::encode(&[0x0D]).unwrap(), vec![0x06, 0x40]);
    assert_eq!(base122::encode(&[0x22]).unwrap(), vec![0x11, 0xDE, 0x80]);
    assert_eq!(base122::encode(&[0x26]).unwrap(), vec![0x13, 0xDE, 0x80]);
    assert_eq!(base122::encode(&[0x5C]).unwrap(), vec![0x2E, 0xDE, 0x80]);
}

fn main() {}
