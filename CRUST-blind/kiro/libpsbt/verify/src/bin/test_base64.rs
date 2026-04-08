use libpsbt::base64::*;

#[test]
fn test_base64_encode_empty() {
    let mut out = [0u8; 64];
    let len = base64_encode(b"", &mut out);
    assert_eq!(len, Some(0));
}

#[test]
fn test_base64_encode_standard_vectors() {
    let mut out = [0u8; 64];

    let len = base64_encode(b"f", &mut out).unwrap();
    assert_eq!(&out[..len], b"Zg==");
    assert_eq!(len, 4);

    let len = base64_encode(b"fo", &mut out).unwrap();
    assert_eq!(&out[..len], b"Zm8=");
    assert_eq!(len, 4);

    let len = base64_encode(b"foo", &mut out).unwrap();
    assert_eq!(&out[..len], b"Zm9v");
    assert_eq!(len, 4);

    let len = base64_encode(b"foobar", &mut out).unwrap();
    assert_eq!(&out[..len], b"Zm9vYmFy");
    assert_eq!(len, 8);
}

#[test]
fn test_base64_decode_standard_vectors() {
    let mut out = [0u8; 64];

    let len = base64_decode(b"Zg==", &mut out).unwrap();
    assert_eq!(&out[..len], b"f");

    let len = base64_decode(b"Zm8=", &mut out).unwrap();
    assert_eq!(&out[..len], b"fo");

    let len = base64_decode(b"Zm9v", &mut out).unwrap();
    assert_eq!(&out[..len], b"foo");

    let len = base64_decode(b"Zm9vYmFy", &mut out).unwrap();
    assert_eq!(&out[..len], b"foobar");
}

#[test]
fn test_base64_roundtrip() {
    let input = b"Hello, World!";
    let mut encoded = [0u8; 256];
    let enc_len = base64_encode(input, &mut encoded).unwrap();
    let mut decoded = [0u8; 256];
    let dec_len = base64_decode(&encoded[..enc_len], &mut decoded).unwrap();
    assert_eq!(&decoded[..dec_len], input);
}

#[test]
fn test_base64_decode_invalid() {
    let mut out = [0u8; 64];
    // Not a multiple of 4
    assert_eq!(base64_decode(b"Zg=", &mut out), None);
    // Empty
    assert_eq!(base64_decode(b"", &mut out), None);
}

#[test]
fn test_base64_encode_buffer_too_small() {
    let mut out = [0u8; 2]; // too small
    assert_eq!(base64_encode(b"foobar", &mut out), None);
}

#[test]
fn test_base62_encode() {
    let mut out = [0u8; 64];
    let len = base62_encode(b"foo", &mut out).unwrap();
    assert_eq!(&out[..len], b"Pczl");
    assert_eq!(len, 4);
}

#[test]
fn test_base62_encode_empty() {
    let mut out = [0u8; 64];
    let len = base62_encode(b"", &mut out);
    assert_eq!(len, Some(0));
}

fn main() {}
