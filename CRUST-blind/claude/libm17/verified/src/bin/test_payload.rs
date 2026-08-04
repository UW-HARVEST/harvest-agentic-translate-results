use libm17::payload::{
    crc_m17, decode_callsign_bytes, decode_callsign_value, encode_callsign_bytes,
    encode_callsign_value, extract_lich, lsf_crc, unpack_lich,
};
use libm17::types::LSF;

#[test]
fn test_crc_m17_basic() {
    let buf: [u8; 3] = [1, 2, 3];
    assert_eq!(crc_m17(&buf), 0xD15F);

    let buf2 = [0u8; 10];
    assert_eq!(crc_m17(&buf2), 0x6C95);

    let buf3 = b"Hello";
    assert_eq!(crc_m17(buf3), 0x3F21);

    // empty input - init value is 0xFFFF
    let empty: [u8; 0] = [];
    assert_eq!(crc_m17(&empty), 0xFFFF);
}

#[test]
fn test_lsf_crc() {
    let mut lsf = LSF::default();
    lsf.dst.copy_from_slice(b"ABCDEF");
    lsf.src.copy_from_slice(b"GHIJKL");
    lsf.type_field[0] = 0x12;
    lsf.type_field[1] = 0x34;
    for i in 0..14 {
        lsf.meta[i] = i as u8;
    }
    assert_eq!(lsf_crc(&lsf), 0x4ADD);
}

#[test]
fn test_encode_callsign_value_normal() {
    let v = encode_callsign_value(b"AB1CDE\0").unwrap();
    assert_eq!(v, 0x00001F245D51u64);
}

#[test]
fn test_encode_callsign_value_at_all() {
    let v = encode_callsign_value(b"@ALL\0").unwrap();
    assert_eq!(v, 0xFFFFFFFFFFFFu64);
}

#[test]
fn test_encode_callsign_value_hash() {
    let v = encode_callsign_value(b"#FOO\0").unwrap();
    assert_eq!(v, 0xEE6B2800601Eu64);
}

#[test]
fn test_encode_callsign_value_too_long() {
    // 10 chars > 9 char limit
    let r = encode_callsign_value(b"TOOLONGCALL\0");
    assert!(r.is_none());
}

#[test]
fn test_encode_callsign_value_empty() {
    let v = encode_callsign_value(b"\0").unwrap();
    assert_eq!(v, 0u64);
}

#[test]
fn test_encode_callsign_bytes() {
    let b = encode_callsign_bytes(b"AB1CDE\0").unwrap();
    assert_eq!(b, [0x00u8, 0x00, 0x1F, 0x24, 0x5D, 0x51]);
}

#[test]
fn test_decode_callsign_value_at_all() {
    let mut out = [0u8; 16];
    decode_callsign_value(&mut out, 0xFFFFFFFFFFFFu64);
    // "@ALL" then NUL terminator
    assert_eq!(&out[..4], b"@ALL");
    assert_eq!(out[4], 0);
}

#[test]
fn test_decode_callsign_value_zero() {
    let mut out = [0u8; 16];
    decode_callsign_value(&mut out, 0);
    // Encoded value 0 -> empty string with NUL at index 0.
    assert_eq!(out[0], 0);
}

#[test]
fn test_decode_callsign_value_round_trip() {
    let mut out = [0u8; 16];
    let v = encode_callsign_value(b"AB1CDE\0").unwrap();
    decode_callsign_value(&mut out, v);
    assert_eq!(&out[..6], b"AB1CDE");
    assert_eq!(out[6], 0);
}

#[test]
fn test_decode_callsign_bytes() {
    let inp = [0u8, 0, 0, 0xFF, 0xFF, 0xFF];
    let mut out = [0u8; 16];
    decode_callsign_bytes(&mut out, &inp);
    // From C: "O3EVF"
    assert_eq!(&out[..5], b"O3EVF");
    assert_eq!(out[5], 0);
}

fn build_lsf() -> LSF {
    let mut lsf = LSF::default();
    for i in 0..6 {
        lsf.dst[i] = 0x10 + i as u8;
    }
    for i in 0..6 {
        lsf.src[i] = 0x20 + i as u8;
    }
    lsf.type_field[0] = 0x30;
    lsf.type_field[1] = 0x31;
    for i in 0..14 {
        lsf.meta[i] = 0x40 + i as u8;
    }
    lsf.crc[0] = 0xCC;
    lsf.crc[1] = 0xDD;
    lsf
}

#[test]
fn test_extract_lich_cnt_0() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 0, &lsf);
    assert_eq!(out, [0x10, 0x11, 0x12, 0x13, 0x14, 0x00]);
}

#[test]
fn test_extract_lich_cnt_1() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 1, &lsf);
    assert_eq!(out, [0x15, 0x20, 0x21, 0x22, 0x23, 0x20]);
}

#[test]
fn test_extract_lich_cnt_2() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 2, &lsf);
    assert_eq!(out, [0x24, 0x25, 0x30, 0x31, 0x40, 0x40]);
}

#[test]
fn test_extract_lich_cnt_3() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 3, &lsf);
    assert_eq!(out, [0x41, 0x42, 0x43, 0x44, 0x45, 0x60]);
}

#[test]
fn test_extract_lich_cnt_4() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 4, &lsf);
    assert_eq!(out, [0x46, 0x47, 0x48, 0x49, 0x4A, 0x80]);
}

#[test]
fn test_extract_lich_cnt_5() {
    let lsf = build_lsf();
    let mut out = [0u8; 6];
    extract_lich(&mut out, 5, &lsf);
    assert_eq!(out, [0x4B, 0x4C, 0x4D, 0xCC, 0xDD, 0xA0]);
}

#[test]
fn test_unpack_lich() {
    let inp = [
        0xAAu8, 0x55, 0x00, 0xFF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
    ];
    let mut out = [0u8; 96];
    unpack_lich(&mut out, &inp);
    // 0xAA = 10101010, 0x55 = 01010101
    let expected_first_16 = [1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1];
    assert_eq!(&out[..16], &expected_first_16);
    // bits 32..48 from 0x12 (0001 0010) and 0x34 (0011 0100)
    let expected_32_48 = [0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0];
    assert_eq!(&out[32..48], &expected_32_48);
}

fn main() {}
