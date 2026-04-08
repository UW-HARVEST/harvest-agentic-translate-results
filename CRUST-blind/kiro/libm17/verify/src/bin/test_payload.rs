use libm17::payload;
use libm17::types::LSF;

#[test]
fn test_encode_callsign_value_sp5wwp() {
    let val = payload::encode_callsign_value(b"SP5WWP").unwrap();
    assert_eq!(val, 0x6541B093);
}

#[test]
fn test_encode_callsign_value_all() {
    let val = payload::encode_callsign_value(b"@ALL").unwrap();
    assert_eq!(val, 0xFFFFFFFFFFFF);
}

#[test]
fn test_encode_callsign_value_empty() {
    let val = payload::encode_callsign_value(b"").unwrap();
    assert_eq!(val, 0);
}

#[test]
fn test_encode_callsign_value_single() {
    let val = payload::encode_callsign_value(b"A").unwrap();
    assert_eq!(val, 1);
}

#[test]
fn test_encode_callsign_value_too_long() {
    assert!(payload::encode_callsign_value(b"ABCDEFGHIJ").is_none());
}

#[test]
fn test_encode_callsign_value_hash() {
    let val = payload::encode_callsign_value(b"#TEST").unwrap();
    assert_eq!(val, 0xEE6B2813FF9C);
}

#[test]
fn test_decode_callsign_value_sp5wwp() {
    let mut buf = [0u8; 10];
    payload::decode_callsign_value(&mut buf, 0x6541B093);
    // Find null terminator
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    assert_eq!(&buf[..len], b"SP5WWP");
}

#[test]
fn test_decode_callsign_value_broadcast() {
    let mut buf = [0u8; 10];
    payload::decode_callsign_value(&mut buf, 0xFFFFFFFFFFFF);
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    assert_eq!(&buf[..len], b"@ALL");
}

#[test]
fn test_encode_decode_callsign_bytes() {
    let bytes = payload::encode_callsign_bytes(b"SP5WWP").unwrap();
    assert_eq!(bytes, [0x00, 0x00, 0x65, 0x41, 0xB0, 0x93]);

    let mut dec = [0u8; 10];
    payload::decode_callsign_bytes(&mut dec, &bytes);
    let len = dec.iter().position(|&b| b == 0).unwrap_or(dec.len());
    assert_eq!(&dec[..len], b"SP5WWP");
}

#[test]
fn test_crc_m17() {
    assert_eq!(payload::crc_m17(&[0, 0, 0, 0]), 0xAF4E);
    assert_eq!(payload::crc_m17(&[0xFF, 0xFF]), 0x0000);
    assert_eq!(payload::crc_m17(&[0x01]), 0x1521);
    assert_eq!(payload::crc_m17(&[0x41, 0x42, 0x43, 0x44]), 0x51FA);
    assert_eq!(payload::crc_m17(&[0x01, 0x02, 0x03, 0x04, 0x05]), 0x9391);
}

#[test]
fn test_lsf_crc_all_zero() {
    let lsf = LSF::default();
    assert_eq!(payload::lsf_crc(&lsf), 0x95E0);
}

#[test]
fn test_lsf_crc_nonzero() {
    let mut lsf = LSF::default();
    lsf.dst[0] = 0xFF;
    lsf.src[0] = 0xFF;
    assert_eq!(payload::lsf_crc(&lsf), 0xEF5A);
}

#[test]
fn test_extract_lich() {
    let mut lsf = LSF::default();
    for i in 0..6 { lsf.dst[i] = 0x10 + i as u8; }
    for i in 0..6 { lsf.src[i] = 0x20 + i as u8; }
    lsf.type_field[0] = 0x30; lsf.type_field[1] = 0x31;
    for i in 0..14 { lsf.meta[i] = 0x40 + i as u8; }
    lsf.crc[0] = 0x50; lsf.crc[1] = 0x51;

    let expected: [[u8; 6]; 6] = [
        [0x10, 0x11, 0x12, 0x13, 0x14, 0x00],
        [0x15, 0x20, 0x21, 0x22, 0x23, 0x20],
        [0x24, 0x25, 0x30, 0x31, 0x40, 0x40],
        [0x41, 0x42, 0x43, 0x44, 0x45, 0x60],
        [0x46, 0x47, 0x48, 0x49, 0x4A, 0x80],
        [0x4B, 0x4C, 0x4D, 0x50, 0x51, 0xA0],
    ];

    for cnt in 0..6u8 {
        let mut lich = [0u8; 6];
        payload::extract_lich(&mut lich, cnt, &lsf);
        assert_eq!(lich, expected[cnt as usize], "extract_lich cnt={}", cnt);
    }
}

#[test]
fn test_unpack_lich() {
    let packed: [u8; 12] = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01];
    let mut unpacked = [0u8; 96];
    payload::unpack_lich(&mut unpacked, &packed);
    let expected_str = "101010111100110111101111000000010010001101000101011001111000100110101011110011011110111100000001";
    for (i, ch) in expected_str.chars().enumerate() {
        let expected_bit = ch.to_digit(10).unwrap() as u8;
        assert_eq!(unpacked[i], expected_bit, "unpack_lich bit {}", i);
    }
}

fn main() {}
