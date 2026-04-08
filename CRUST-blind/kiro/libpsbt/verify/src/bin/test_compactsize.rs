use libpsbt::compactsize::{compactsize_peek_length, compactsize_length, compactsize_write, compactsize_read};
use libpsbt::psbt::PsbtResult;

#[test]
fn test_peek_length() {
    assert_eq!(compactsize_peek_length(0), 1);
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
    assert_eq!(compactsize_length(65535), 3);
    assert_eq!(compactsize_length(65536), 5);
    assert_eq!(compactsize_length(4294967295), 5);
    assert_eq!(compactsize_length(4294967296), 9);
}

#[test]
fn test_write_read_zero() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0);
    assert_eq!(buf[0], 0x00);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 0);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_write_read_252() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 252);
    assert_eq!(buf[0], 0xfc);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 252);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_write_read_253() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 253);
    // Rust fixed the C bug: prefix=0xfd, then LE u16 253 = fd 00
    assert_eq!(buf[0], 0xfd);
    assert_eq!(buf[1], 0xfd);
    assert_eq!(buf[2], 0x00);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 253);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_write_read_65535() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 65535);
    assert_eq!(buf[0], 0xfd);
    assert_eq!(buf[1], 0xff);
    assert_eq!(buf[2], 0xff);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 65535);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_write_read_65536() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 65536);
    assert_eq!(buf[0], 0xfe);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x00);
    assert_eq!(buf[3], 0x01);
    assert_eq!(buf[4], 0x00);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 65536);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_read_noncanonical_253_prefix() {
    // prefix=0xfd but value=0 (< 253) -> non-canonical
    let buf = [0xfdu8, 0x00, 0x00];
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, u64::MAX);
    assert_eq!(res, PsbtResult::CompactReadError);
}

#[test]
fn test_read_noncanonical_254_prefix() {
    // prefix=0xfe but value=1 (< 0x10000) -> non-canonical
    let buf = [0xfeu8, 0x01, 0x00, 0x00, 0x00];
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, u64::MAX);
    assert_eq!(res, PsbtResult::CompactReadError);
}

#[test]
fn test_write_read_1() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 1);
    assert_eq!(buf[0], 0x01);
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 1);
    assert_eq!(res, PsbtResult::Ok);
}

#[test]
fn test_read_1000() {
    // prefix=0xfd, then u16 LE 1000 = e8 03
    let buf = [0xfdu8, 0xe8, 0x03];
    let (v, res) = compactsize_read(&buf);
    assert_eq!(v, 1000);
    assert_eq!(res, PsbtResult::Ok);
}

fn main() {}
