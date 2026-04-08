use libpsbt::compactsize::*;
use libpsbt::psbt::PsbtResult;

#[test]
fn test_peek_length() {
    assert_eq!(compactsize_peek_length(0), 1);
    assert_eq!(compactsize_peek_length(1), 1);
    assert_eq!(compactsize_peek_length(252), 1);
    assert_eq!(compactsize_peek_length(253), 3);
    assert_eq!(compactsize_peek_length(254), 5);
    assert_eq!(compactsize_peek_length(255), 9);
}

#[test]
fn test_length() {
    assert_eq!(compactsize_length(0), 1);
    assert_eq!(compactsize_length(252), 1);
    assert_eq!(compactsize_length(253), 3);
    assert_eq!(compactsize_length(0xFFFF), 3);
    assert_eq!(compactsize_length(0x10000), 5);
    assert_eq!(compactsize_length(0xFFFFFFFF), 5);
    assert_eq!(compactsize_length(0x100000000), 9);
}

#[test]
fn test_write_read_small_values() {
    let mut buf = [0u8; 16];

    compactsize_write(&mut buf, 0);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 0);

    compactsize_write(&mut buf, 1);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 1);

    compactsize_write(&mut buf, 124);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 124);

    compactsize_write(&mut buf, 252);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 252);
}

#[test]
fn test_write_read_u16_range() {
    let mut buf = [0u8; 16];

    compactsize_write(&mut buf, 253);
    assert_eq!(buf[0], 253);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 253);

    compactsize_write(&mut buf, 0xFFFF);
    assert_eq!(buf[0], 253);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 0xFFFF);
}

#[test]
fn test_write_read_u32_range() {
    let mut buf = [0u8; 16];

    compactsize_write(&mut buf, 0x10000);
    assert_eq!(buf[0], 254);
    let (val, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::Ok);
    assert_eq!(val, 0x10000);
}

#[test]
fn test_read_non_canonical_errors() {
    // 253 marker but value < 253 → error
    let data = [253u8, 0, 0];
    let (_, res) = compactsize_read(&data);
    assert_eq!(res, PsbtResult::CompactReadError);

    // 254 marker but value < 0x10000 → error
    let data = [254u8, 0, 0, 0, 0];
    let (_, res) = compactsize_read(&data);
    assert_eq!(res, PsbtResult::CompactReadError);

    // 255 marker but value < 0x100000000 → error
    let data = [255u8, 0, 0, 0, 0, 0, 0, 0, 0];
    let (_, res) = compactsize_read(&data);
    assert_eq!(res, PsbtResult::CompactReadError);
}

#[test]
fn test_read_over_max_serialize_size() {
    // Value > MAX_SERIALIZE_SIZE (0x02000000) should error
    let mut buf = [0u8; 16];
    compactsize_write(&mut buf, 0x02000001);
    let (_, res) = compactsize_read(&buf);
    assert_eq!(res, PsbtResult::CompactReadError);
}

#[test]
fn test_write_byte_layout() {
    let mut buf = [0u8; 16];

    // Single byte for values < 253
    compactsize_write(&mut buf, 42);
    assert_eq!(buf[0], 42);

    // 253 marker + LE u16
    compactsize_write(&mut buf, 300);
    assert_eq!(buf[0], 253);
    assert_eq!(buf[1], 0x2C); // 300 & 0xFF
    assert_eq!(buf[2], 0x01); // 300 >> 8
}

fn main() {}
