use libpsbt::base64::{base62_encode, base64_decode, base64_encode};

#[test]
fn test_base64_encode_hello() {
    let mut out = [0u8; 64];
    let n = base64_encode(b"hello", &mut out).expect("encode failed");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"aGVsbG8=");
}

#[test]
fn test_base64_encode_six_bytes() {
    let mut out = [0u8; 64];
    let n = base64_encode(&[0u8, 1, 2, 3, 4, 5], &mut out).expect("encode failed");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"AAECAwQF");
}

#[test]
fn test_base64_encode_one_byte() {
    let mut out = [0u8; 64];
    let n = base64_encode(b"f", &mut out).expect("encode failed");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"Zg==");
}

#[test]
fn test_base64_encode_two_bytes() {
    let mut out = [0u8; 64];
    let n = base64_encode(b"fo", &mut out).expect("encode failed");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"Zm8=");
}

#[test]
fn test_base64_encode_empty() {
    let mut out = [0u8; 64];
    let n = base64_encode(b"", &mut out).expect("encode failed");
    assert_eq!(n, 0);
}

#[test]
fn test_base64_encode_too_small() {
    let mut out = [0u8; 4];
    let r = base64_encode(b"hello", &mut out);
    assert!(r.is_none());
}

#[test]
fn test_base62_encode_hello() {
    let mut out = [0u8; 64];
    let n = base62_encode(b"hello", &mut out).expect("encode failed");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"Q6LiR6y=");
}

#[test]
fn test_base64_decode_hello() {
    let mut out = [0u8; 64];
    let n = base64_decode(b"aGVsbG8=", &mut out).expect("decode failed");
    assert_eq!(n, 5);
    assert_eq!(&out[..n], b"hello");
}

#[test]
fn test_base64_decode_six_bytes() {
    let mut out = [0u8; 64];
    let n = base64_decode(b"AAECAwQF", &mut out).expect("decode failed");
    assert_eq!(n, 6);
    assert_eq!(&out[..n], &[0u8, 1, 2, 3, 4, 5]);
}

#[test]
fn test_base64_decode_invalid_count() {
    // 3 valid base64 chars (not multiple of 4)
    let mut out = [0u8; 64];
    let r = base64_decode(b"abc", &mut out);
    assert!(r.is_none());
}

#[test]
fn test_base64_roundtrip() {
    let data = b"The quick brown fox jumps over the lazy dog!!";
    let mut enc = [0u8; 128];
    let n = base64_encode(data, &mut enc).expect("encode failed");
    let mut dec = [0u8; 128];
    let m = base64_decode(&enc[..n], &mut dec).expect("decode failed");
    assert_eq!(m, data.len());
    assert_eq!(&dec[..m], data);
}

fn main() {}
