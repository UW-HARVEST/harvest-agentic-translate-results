use libpsbt::base64::{base64_encode, base64_decode, base62_encode};

#[test]
fn test_base64_encode_hello() {
    let mut out = [0u8; 256];
    let n = base64_encode(b"Hello", &mut out).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"SGVsbG8=");
}

#[test]
fn test_base64_decode_hello() {
    let mut out = [0u8; 256];
    let n = base64_decode(b"SGVsbG8=", &mut out).unwrap();
    assert_eq!(n, 5);
    assert_eq!(&out[..n], b"Hello");
}

#[test]
fn test_base62_encode_hello() {
    let mut out = [0u8; 256];
    let n = base62_encode(b"Hello", &mut out).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"I6LiR6y=");
}

#[test]
fn test_base64_encode_empty() {
    let mut out = [0u8; 256];
    let n = base64_encode(b"", &mut out).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn test_base64_encode_one_byte() {
    let mut out = [0u8; 256];
    let n = base64_encode(b"A", &mut out).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"QQ==");
}

#[test]
fn test_base64_encode_two_bytes() {
    let mut out = [0u8; 256];
    let n = base64_encode(b"AB", &mut out).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"QUI=");
}

#[test]
fn test_base64_encode_three_bytes() {
    let mut out = [0u8; 256];
    let n = base64_encode(b"ABC", &mut out).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"QUJD");
}

#[test]
fn test_base64_decode_single_pad() {
    let mut out = [0u8; 256];
    let n = base64_decode(b"QQ==", &mut out).unwrap();
    assert_eq!(n, 1);
    assert_eq!(out[0], b'A');
}

#[test]
fn test_base64_decode_invalid_returns_none() {
    let mut out = [0u8; 256];
    assert!(base64_decode(b"Q", &mut out).is_none());
}

#[test]
fn test_base64_encode_binary() {
    let bin = [0x00u8, 0xff, 0x80, 0x7f, 0x01];
    let mut out = [0u8; 256];
    let n = base64_encode(&bin, &mut out).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"AP+AfwE=");
}

#[test]
fn test_base64_roundtrip_binary() {
    let bin = [0x00u8, 0xff, 0x80, 0x7f, 0x01];
    let mut enc = [0u8; 256];
    let n = base64_encode(&bin, &mut enc).unwrap();
    let mut dec = [0u8; 256];
    let m = base64_decode(&enc[..n], &mut dec).unwrap();
    assert_eq!(m, 5);
    assert_eq!(&dec[..m], &bin);
}

fn main() {}
