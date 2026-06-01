use libpsbt::compactsize::{
    compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write,
};
use libpsbt::psbt::PsbtResult;

#[test]
fn test_compactsize_length() {
    assert_eq!(compactsize_length(0), 1);
    assert_eq!(compactsize_length(252), 1);
    assert_eq!(compactsize_length(253), 3);
    assert_eq!(compactsize_length(0xFFFF), 3);
    assert_eq!(compactsize_length(0x10000), 5);
    assert_eq!(compactsize_length(0xFFFFFFFF), 5);
    assert_eq!(compactsize_length(0x100000000u64), 9);
}

#[test]
fn test_compactsize_peek_length() {
    assert_eq!(compactsize_peek_length(0), 1);
    assert_eq!(compactsize_peek_length(252), 1);
    assert_eq!(compactsize_peek_length(253), 3);
    assert_eq!(compactsize_peek_length(254), 5);
    assert_eq!(compactsize_peek_length(255), 9);
}

#[test]
fn test_compactsize_write_small() {
    let mut buf = [0u8; 16];
    compactsize_write(&mut buf, 5);
    assert_eq!(buf[0], 5);
}

#[test]
fn test_compactsize_write_u16() {
    let mut buf = [0u8; 16];
    compactsize_write(&mut buf, 1000);
    assert_eq!(buf[0], 0xfd);
    assert_eq!(buf[1], 0xe8);
    assert_eq!(buf[2], 0x03);
}

#[test]
fn test_compactsize_write_u32() {
    let mut buf = [0u8; 16];
    compactsize_write(&mut buf, 0x12345);
    assert_eq!(buf[0], 0xfe);
    assert_eq!(buf[1], 0x45);
    assert_eq!(buf[2], 0x23);
    assert_eq!(buf[3], 0x01);
    assert_eq!(buf[4], 0x00);
}

#[test]
fn test_compactsize_write_u64() {
    let mut buf = [0u8; 16];
    compactsize_write(&mut buf, 0x123456789u64);
    assert_eq!(buf[0], 0xff);
    assert_eq!(&buf[1..9], &[0x89, 0x67, 0x45, 0x23, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn test_compactsize_read_small() {
    let data = [5u8];
    let (v, err) = compactsize_read(&data);
    assert_eq!(v, 5);
    assert_eq!(err, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_u16() {
    let data = [253u8, 0xe8, 0x03];
    let (v, err) = compactsize_read(&data);
    assert_eq!(v, 1000);
    assert_eq!(err, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_u16_noncanonical() {
    let data = [253u8, 0x00, 0x00];
    let (_v, err) = compactsize_read(&data);
    assert_eq!(err, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_u32() {
    let data = [254u8, 0x45, 0x23, 0x01, 0x00];
    let (v, err) = compactsize_read(&data);
    assert_eq!(v, 0x12345);
    assert_eq!(err, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_u32_noncanonical() {
    // u32 representation but value < 0x10000 is non-canonical
    let data = [254u8, 0x00, 0x00, 0x00, 0x00];
    let (_v, err) = compactsize_read(&data);
    assert_eq!(err, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_max_serialize_too_large() {
    // u32 representation larger than MAX_SERIALIZE_SIZE
    let data = [254u8, 0x00, 0x00, 0x00, 0x03]; // 0x03000000
    let (_v, err) = compactsize_read(&data);
    assert_eq!(err, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_roundtrip() {
    let values = [0u64, 1, 5, 252, 253, 1000, 0xFFFF, 0x10000, 0x12345, 0x1FFFFFF];
    for &v in &values {
        let mut buf = [0u8; 16];
        compactsize_write(&mut buf, v);
        let (got, err) = compactsize_read(&buf);
        assert_eq!(err, PsbtResult::Ok);
        assert_eq!(got, v);
    }
}

fn main() {}
