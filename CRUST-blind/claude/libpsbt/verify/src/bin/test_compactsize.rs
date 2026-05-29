use libpsbt::compactsize::{
    compactsize_length, compactsize_peek_length, compactsize_read, compactsize_write,
};
use libpsbt::psbt::PsbtResult;

#[test]
fn test_compactsize_length_zero() {
    assert_eq!(compactsize_length(0), 1);
}

#[test]
fn test_compactsize_length_252() {
    assert_eq!(compactsize_length(252), 1);
}

#[test]
fn test_compactsize_length_253() {
    assert_eq!(compactsize_length(253), 3);
}

#[test]
fn test_compactsize_length_u16_max() {
    assert_eq!(compactsize_length(0xFFFF), 3);
}

#[test]
fn test_compactsize_length_u16_max_plus_1() {
    assert_eq!(compactsize_length(0x10000), 5);
}

#[test]
fn test_compactsize_length_u32_max() {
    assert_eq!(compactsize_length(0xFFFF_FFFF), 5);
}

#[test]
fn test_compactsize_length_u32_max_plus_1() {
    assert_eq!(compactsize_length(0x1_0000_0000), 9);
}

#[test]
fn test_compactsize_length_u64_max() {
    assert_eq!(compactsize_length(u64::MAX), 9);
}

#[test]
fn test_compactsize_peek_length_lt_253() {
    assert_eq!(compactsize_peek_length(0), 1);
    assert_eq!(compactsize_peek_length(1), 1);
    assert_eq!(compactsize_peek_length(252), 1);
}

#[test]
fn test_compactsize_peek_length_253() {
    assert_eq!(compactsize_peek_length(253), 3);
}

#[test]
fn test_compactsize_peek_length_254() {
    assert_eq!(compactsize_peek_length(254), 5);
}

#[test]
fn test_compactsize_peek_length_255() {
    assert_eq!(compactsize_peek_length(255), 9);
}

#[test]
fn test_compactsize_write_zero() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0);
    assert_eq!(buf[0], 0);
}

#[test]
fn test_compactsize_write_252() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 252);
    assert_eq!(buf[0], 0xfc);
}

#[test]
fn test_compactsize_write_253() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 253);
    assert_eq!(buf[0], 0xfd);
    assert_eq!(buf[1], 0xfd);
    assert_eq!(buf[2], 0x00);
}

#[test]
fn test_compactsize_write_u16_max() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0xFFFF);
    assert_eq!(buf[0], 0xfd);
    assert_eq!(buf[1], 0xff);
    assert_eq!(buf[2], 0xff);
}

#[test]
fn test_compactsize_write_u16_max_plus_1() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0x10000);
    assert_eq!(buf[0], 0xfe);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x00);
    assert_eq!(buf[3], 0x01);
    assert_eq!(buf[4], 0x00);
}

#[test]
fn test_compactsize_write_u32_max() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0xFFFF_FFFF);
    assert_eq!(buf[0], 0xfe);
    assert_eq!(buf[1], 0xff);
    assert_eq!(buf[2], 0xff);
    assert_eq!(buf[3], 0xff);
    assert_eq!(buf[4], 0xff);
}

#[test]
fn test_compactsize_write_u32_max_plus_1() {
    let mut buf = [0u8; 9];
    compactsize_write(&mut buf, 0x1_0000_0000);
    assert_eq!(buf[0], 0xff);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x00);
    assert_eq!(buf[3], 0x00);
    assert_eq!(buf[4], 0x00);
    assert_eq!(buf[5], 0x01);
    assert_eq!(buf[6], 0x00);
    assert_eq!(buf[7], 0x00);
    assert_eq!(buf[8], 0x00);
}

#[test]
fn test_compactsize_read_one_byte() {
    let buf = [0x00];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 0);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_252() {
    let buf = [0xfc];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 252);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_253_canonical() {
    let buf = [0xfd, 0xfd, 0x00];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 253);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_256() {
    let buf = [0xfd, 0x00, 0x01];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 256);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_u16_max() {
    let buf = [0xfd, 0xff, 0xff];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 0xFFFF);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_non_canonical_fd_zero() {
    // 0xfd 0x00 0x00 -> value 0, < 253 -> non-canonical -> error
    let buf = [0xfd, 0x00, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_non_canonical_fd_lt_253() {
    let buf = [0xfd, 0xfc, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_fe_canonical() {
    let buf = [0xfe, 0x00, 0x00, 0x01, 0x00];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 0x10000);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_fe_non_canonical_zero() {
    let buf = [0xfe, 0x00, 0x00, 0x00, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_fe_non_canonical_lt_0x10000() {
    let buf = [0xfe, 0xff, 0xff, 0x00, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_fe_max_serialize_size() {
    // value 0x02000000 == MAX_SERIALIZE_SIZE -> ok
    let buf = [0xfe, 0x00, 0x00, 0x00, 0x02];
    let (v, e) = compactsize_read(&buf);
    assert_eq!(v, 0x02000000);
    assert_eq!(e, PsbtResult::Ok);
}

#[test]
fn test_compactsize_read_fe_above_max_serialize_size() {
    // value 0x02000001 > MAX_SERIALIZE_SIZE -> error
    let buf = [0xfe, 0x01, 0x00, 0x00, 0x02];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_ff_non_canonical() {
    // ff with value < 0x100000000 -> non-canonical
    let buf = [0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_ff_canonical_above_max_serialize() {
    // ff with value >= 0x100000000 but > MAX_SERIALIZE_SIZE -> error
    let buf = [0xff, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_read_empty() {
    let buf: [u8; 0] = [];
    let (_v, e) = compactsize_read(&buf);
    assert_eq!(e, PsbtResult::CompactReadError);
}

#[test]
fn test_compactsize_roundtrip() {
    for &v in &[0u64, 1, 100, 252, 253, 254, 1000, 65535, 65536, 1_000_000, 0x01FF_FFFF, 0x0200_0000] {
        let mut buf = [0u8; 9];
        compactsize_write(&mut buf, v);
        let n = compactsize_length(v) as usize;
        // peek_length should match
        let peeked = compactsize_peek_length(buf[0]) as usize;
        assert_eq!(peeked, n, "peek length mismatch for {}", v);
        let (read, e) = compactsize_read(&buf);
        assert_eq!(e, PsbtResult::Ok, "read error for {}", v);
        assert_eq!(read, v, "roundtrip mismatch for {}", v);
    }
}

fn main() {}
