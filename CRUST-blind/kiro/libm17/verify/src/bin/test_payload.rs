use libm17::payload;
use libm17::types::LSF;

#[test]
fn test_encode_callsign_value_sp5wwp() {
    let result = payload::encode_callsign_value(b"SP5WWP");
    assert_eq!(result, Some(1698803859));
}

#[test]
fn test_encode_callsign_value_all() {
    let result = payload::encode_callsign_value(b"@ALL");
    assert_eq!(result, Some(0xFFFFFFFFFFFF));
}

#[test]
fn test_encode_callsign_value_hash() {
    let result = payload::encode_callsign_value(b"#1234");
    assert_eq!(result, Some(262144002033188));
}

#[test]
fn test_encode_callsign_value_empty() {
    let result = payload::encode_callsign_value(b"");
    assert_eq!(result, Some(0));
}

#[test]
fn test_decode_callsign_value_sp5wwp() {
    let mut outp = [0u8; 10];
    payload::decode_callsign_value(&mut outp, 1698803859);
    // null-terminated "SP5WWP"
    assert_eq!(&outp[..7], b"SP5WWP\0");
}

#[test]
fn test_decode_callsign_value_broadcast() {
    let mut outp = [0u8; 10];
    payload::decode_callsign_value(&mut outp, 0xFFFFFFFFFFFF);
    assert_eq!(&outp[..5], b"@ALL\0");
}

#[test]
fn test_decode_callsign_value_hash() {
    let mut outp = [0u8; 10];
    payload::decode_callsign_value(&mut outp, 262144002033188);
    assert_eq!(&outp[..6], b"#1234\0");
}

#[test]
fn test_encode_callsign_bytes() {
    let result = payload::encode_callsign_bytes(b"SP5WWP");
    assert_eq!(result, Some([0x00, 0x00, 0x65, 0x41, 0xB0, 0x93]));
}

#[test]
fn test_decode_callsign_bytes() {
    let inp: [u8; 6] = [0x00, 0x00, 0x65, 0x41, 0xB0, 0x93];
    let mut outp = [0u8; 10];
    payload::decode_callsign_bytes(&mut outp, &inp);
    assert_eq!(&outp[..7], b"SP5WWP\0");
}

#[test]
fn test_callsign_roundtrip() {
    let bytes = payload::encode_callsign_bytes(b"SP5WWP").unwrap();
    let mut decoded = [0u8; 10];
    payload::decode_callsign_bytes(&mut decoded, &bytes);
    assert_eq!(&decoded[..7], b"SP5WWP\0");
}

#[test]
fn test_crc_m17_zeros() {
    let data = [0u8; 6];
    assert_eq!(payload::crc_m17(&data), 13253);
}

#[test]
fn test_crc_m17_ones() {
    let data = [0xFFu8; 6];
    assert_eq!(payload::crc_m17(&data), 51234);
}

#[test]
fn test_crc_m17_seq() {
    let data = [0x01u8, 0x02, 0x03, 0x04];
    assert_eq!(payload::crc_m17(&data), 37761);
}

#[test]
fn test_crc_m17_hello() {
    assert_eq!(payload::crc_m17(b"Hello"), 16161);
}

#[test]
fn test_lsf_crc_zeros() {
    let lsf = LSF::default();
    assert_eq!(payload::lsf_crc(&lsf), 38368);
}

#[test]
fn test_extract_lich() {
    let lsf = LSF {
        dst: [0x10, 0x11, 0x12, 0x13, 0x14, 0x15],
        src: [0x20, 0x21, 0x22, 0x23, 0x24, 0x25],
        type_field: [0x30, 0x31],
        meta: [0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D],
        crc: [0x50, 0x51],
    };

    let expected: [[u8; 6]; 6] = [
        [0x10, 0x11, 0x12, 0x13, 0x14, 0x00],
        [0x15, 0x20, 0x21, 0x22, 0x23, 0x20],
        [0x24, 0x25, 0x30, 0x31, 0x40, 0x40],
        [0x41, 0x42, 0x43, 0x44, 0x45, 0x60],
        [0x46, 0x47, 0x48, 0x49, 0x4A, 0x80],
        [0x4B, 0x4C, 0x4D, 0x50, 0x51, 0xA0],
    ];

    for cnt in 0..6u8 {
        let mut outp = [0u8; 6];
        payload::extract_lich(&mut outp, cnt, &lsf);
        assert_eq!(outp, expected[cnt as usize], "extract_lich cnt={}", cnt);
    }
}

#[test]
fn test_unpack_lich() {
    let input: [u8; 12] = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01];
    let mut out = [0u8; 96];
    payload::unpack_lich(&mut out, &input);
    let expected_str = "101010111100110111101111000000010010001101000101011001111000100110101011110011011110111100000001";
    let expected: Vec<u8> = expected_str.bytes().map(|b| b - b'0').collect();
    assert_eq!(&out[..], &expected[..]);
}

fn main() {}
