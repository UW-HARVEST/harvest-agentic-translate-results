use libm17::payload::*;
use libm17::types::LSF;

#[test]
fn test_crc_m17() {
    let s = b"123456789";
    assert_eq!(crc_m17(s), 0x772B);
    assert_eq!(crc_m17(&[]), 0xFFFF);
    let hello = [0x48u8, 0x65, 0x6C, 0x6C, 0x6F];
    assert_eq!(crc_m17(&hello), 0x3F21);
}

#[test]
fn test_lsf_crc() {
    let lsf = LSF::default();
    assert_eq!(lsf_crc(&lsf), 0x95E0);

    let mut lsf = LSF::default();
    for i in 0..6u8 {
        lsf.dst[i as usize] = i + 1;
        lsf.src[i as usize] = 0x10 + i;
    }
    lsf.type_field[0] = 0xAB;
    lsf.type_field[1] = 0xCD;
    for i in 0..14u8 {
        lsf.meta[i as usize] = i * 3;
    }
    assert_eq!(lsf_crc(&lsf), 0x0174);
}

#[test]
fn test_encode_callsign_value() {
    assert_eq!(encode_callsign_value(b"AB1CD"), Some(10476881));
    assert_eq!(encode_callsign_value(b"@ALL"), Some(281474976710655));
    assert_eq!(encode_callsign_value(b"#TEST"), Some(262144001310620));
    assert_eq!(encode_callsign_value(b""), Some(0));
    assert_eq!(encode_callsign_value(b"SP5WWP"), Some(1698803859));
    // Long string returns None (-1)
    assert_eq!(encode_callsign_value(b"TOOLONGCALLSIGN"), None);
}

#[test]
fn test_encode_callsign_bytes() {
    let r = encode_callsign_bytes(b"SP5WWP");
    assert_eq!(r, Some([0x00, 0x00, 0x65, 0x41, 0xB0, 0x93]));

    let r = encode_callsign_bytes(b"@ALL");
    assert_eq!(r, Some([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]));

    let r = encode_callsign_bytes(b"TOOLONGCALLSIGN");
    assert_eq!(r, None);
}

#[test]
fn test_decode_callsign_value() {
    let mut buf: [u8; 20] = [0; 20];
    decode_callsign_value(&mut buf, 1698803859);
    // Find null-terminator
    let len = buf.iter().position(|&c| c == 0).unwrap();
    assert_eq!(&buf[..len], b"SP5WWP");

    let mut buf: [u8; 20] = [0; 20];
    decode_callsign_value(&mut buf, 0xFFFFFFFFFFFF);
    let len = buf.iter().position(|&c| c == 0).unwrap();
    assert_eq!(&buf[..len], b"@ALL");

    let mut buf: [u8; 20] = [0; 20];
    decode_callsign_value(&mut buf, 262144001310620);
    let len = buf.iter().position(|&c| c == 0).unwrap();
    assert_eq!(&buf[..len], b"#TEST");
}

#[test]
fn test_decode_callsign_bytes() {
    let bytes = [0x00, 0x00, 0x65, 0x41, 0xB0, 0x93];
    let mut buf: [u8; 20] = [0; 20];
    decode_callsign_bytes(&mut buf, &bytes);
    let len = buf.iter().position(|&c| c == 0).unwrap();
    assert_eq!(&buf[..len], b"SP5WWP");
}

#[test]
fn test_extract_lich() {
    let mut lsf = LSF::default();
    for i in 0..6u8 {
        lsf.dst[i as usize] = i + 1;
        lsf.src[i as usize] = 0x10 + i;
    }
    lsf.type_field[0] = 0xAB;
    lsf.type_field[1] = 0xCD;
    for i in 0..14u8 {
        lsf.meta[i as usize] = i * 3;
    }

    let mut out: [u8; 6] = [0xAA; 6];
    extract_lich(&mut out, 0, &lsf);
    assert_eq!(out, [0x01, 0x02, 0x03, 0x04, 0x05, 0x00]);

    out = [0xAA; 6];
    extract_lich(&mut out, 1, &lsf);
    assert_eq!(out, [0x06, 0x10, 0x11, 0x12, 0x13, 0x20]);

    out = [0xAA; 6];
    extract_lich(&mut out, 2, &lsf);
    assert_eq!(out, [0x14, 0x15, 0xAB, 0xCD, 0x00, 0x40]);

    out = [0xAA; 6];
    extract_lich(&mut out, 3, &lsf);
    assert_eq!(out, [0x03, 0x06, 0x09, 0x0C, 0x0F, 0x60]);

    out = [0xAA; 6];
    extract_lich(&mut out, 4, &lsf);
    assert_eq!(out, [0x12, 0x15, 0x18, 0x1B, 0x1E, 0x80]);

    out = [0xAA; 6];
    extract_lich(&mut out, 5, &lsf);
    assert_eq!(out, [0x21, 0x24, 0x27, 0x00, 0x00, 0xA0]);
}

#[test]
fn test_unpack_lich() {
    let inp: [u8; 12] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x00];
    let mut out: [u8; 96] = [0; 96];
    unpack_lich(&mut out, &inp);
    let expected_str = "101010111100110111101111000100100011010001010110011110001001101010111100110111101111000000000000";
    let expected: Vec<u8> = expected_str.chars().map(|c| (c as u8) - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

fn main() {}
