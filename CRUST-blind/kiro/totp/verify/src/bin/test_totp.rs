use totp::totp::{unpack32, unpack64, pack32, rotl, sha1, hmac_sha1, hotp, totp, from_base32};

fn to_hex(a: &[u8]) -> String {
    a.iter().map(|b| format!("{:02x}", b)).collect()
}

// --- pack/unpack ---

#[test]
fn test_unpack32_basic() {
    let mut a = [0u8; 4];
    unpack32(0x12345678, &mut a);
    assert_eq!(a, [0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn test_unpack32_zero() {
    let mut a = [0xFFu8; 4];
    unpack32(0x00000000, &mut a);
    assert_eq!(a, [0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_unpack32_max() {
    let mut a = [0u8; 4];
    unpack32(0xFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_unpack64_basic() {
    let mut a = [0u8; 8];
    unpack64(0x123456789ABCDEF0, &mut a);
    assert_eq!(a, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
}

#[test]
fn test_unpack64_zero() {
    let mut a = [0xFFu8; 8];
    unpack64(0, &mut a);
    assert_eq!(a, [0; 8]);
}

#[test]
fn test_unpack64_max() {
    let mut a = [0u8; 8];
    unpack64(0xFFFFFFFFFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF; 8]);
}

#[test]
fn test_pack32_basic() {
    assert_eq!(pack32(&[0x12, 0x34, 0x56, 0x78]), 0x12345678);
}

#[test]
fn test_pack32_zero() {
    assert_eq!(pack32(&[0, 0, 0, 0]), 0);
}

#[test]
fn test_pack32_max() {
    assert_eq!(pack32(&[0xFF, 0xFF, 0xFF, 0xFF]), 0xFFFFFFFF);
}

#[test]
fn test_pack32_unpack32_roundtrip() {
    let mut a = [0u8; 4];
    unpack32(0x12345678, &mut a);
    assert_eq!(pack32(&a), 0x12345678);
}

// --- rotl ---

#[test]
fn test_rotl() {
    assert_eq!(rotl(1, 1), 2);
    assert_eq!(rotl(0x80000000, 1), 1);
    assert_eq!(rotl(0xDEADBEEF, 4), 0xEADBEEFD);
    assert_eq!(rotl(0xDEADBEEF, 16), 0xBEEFDEAD);
}

// --- sha1 ---

#[test]
fn test_sha1_empty() {
    let mut buf = [0u8; 512];
    let mut hash = [0u8; 20];
    let ret = sha1(&mut buf, 0, 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
}

#[test]
fn test_sha1_abc() {
    let mut buf = [0u8; 512];
    buf[..3].copy_from_slice(b"abc");
    let mut hash = [0u8; 20];
    let ret = sha1(&mut buf, 3, 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "a9993e364706816aba3e25717850c26c9cd0d89d");
}

#[test]
fn test_sha1_fox() {
    let msg = b"The quick brown fox jumps over the lazy dog";
    let mut buf = [0u8; 512];
    buf[..msg.len()].copy_from_slice(msg);
    let mut hash = [0u8; 20];
    let ret = sha1(&mut buf, msg.len(), 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
}

#[test]
fn test_sha1_cap_too_small() {
    let mut buf = [0u8; 4];
    let mut hash = [0u8; 20];
    let ret = sha1(&mut buf, 3, 4, &mut hash);
    assert_eq!(ret, 1); // TOTP_EBOUNDS
}

// --- hmac_sha1 ---

#[test]
fn test_hmac_sha1_rfc2202() {
    let mut key = [0u8; 64];
    for i in 0..20 { key[i] = 0xAA; }
    let mut data = [0u8; 64];
    for i in 0..50 { data[i] = 0xDD; }
    let mut hash = [0u8; 20];
    let ret = hmac_sha1(&key, &data, 50, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "125d7342b9ac11cd91a39af48aa17b4f63f175d3");
}

#[test]
fn test_hmac_sha1_len_too_large() {
    let key = [0u8; 64];
    let data = [0u8; 64];
    let mut hash = [0u8; 20];
    let ret = hmac_sha1(&key, &data, 65, &mut hash);
    assert_eq!(ret, 1); // TOTP_EBOUNDS
}

// --- hotp ---

#[test]
fn test_hotp_rfc4226() {
    let mut secret = [0u8; 64];
    secret[..20].copy_from_slice(&[
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30,
    ]);
    let expected = [755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(hotp(&secret, i as u64), exp, "hotp counter={}", i);
    }
}

// --- totp ---

#[test]
fn test_totp() {
    let mut secret = [0u8; 64];
    secret[..20].copy_from_slice(&[
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30,
    ]);
    assert_eq!(totp(&secret, 0), 755224);
    assert_eq!(totp(&secret, 30), 287082);
    assert_eq!(totp(&secret, 59), 287082); // same 30s window as t=30
    assert_eq!(totp(&secret, 60), 359152);
}

// --- from_base32 ---

#[test]
fn test_from_base32_mixed_case_3bytes() {
    let mut buf = [0u8; 64];
    let n = from_base32("MZxw6===", &mut buf, 64);
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], &[0x66, 0x6f, 0x6f]); // "foo"
}

#[test]
fn test_from_base32_mixed_case_4bytes() {
    let mut buf = [0u8; 64];
    let n = from_base32("MZxw6YQ=", &mut buf, 64);
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], &[0x66, 0x6f, 0x6f, 0x62]); // "foob"
}

#[test]
fn test_from_base32_mixed_case_5bytes() {
    let mut buf = [0u8; 64];
    let n = from_base32("MZxw6YTB", &mut buf, 64);
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], &[0x66, 0x6f, 0x6f, 0x62, 0x61]); // "fooba"
}

#[test]
fn test_from_base32_two_chunks() {
    let mut buf = [0u8; 64];
    let n = from_base32("MZxw6YTBOI======", &mut buf, 64);
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], &[0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72]); // "foobar"
}

#[test]
fn test_from_base32_uppercase() {
    let mut buf = [0u8; 64];
    let n = from_base32("MZXW6===", &mut buf, 64);
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], &[0x66, 0x6f, 0x6f]);
}

#[test]
fn test_from_base32_longer() {
    let mut buf = [0u8; 64];
    let n = from_base32("JBSWY3DPEHPK3PXP", &mut buf, 64);
    assert_eq!(n, 10);
    assert_eq!(to_hex(&buf[..10]), "48656c6c6f21deadbeef");
}

#[test]
fn test_from_base32_empty() {
    let mut buf = [0u8; 64];
    assert_eq!(from_base32("", &mut buf, 64), 0);
}

#[test]
fn test_from_base32_not_multiple_of_8() {
    let mut buf = [0u8; 64];
    assert_eq!(from_base32("ABC", &mut buf, 64), 0);
}

#[test]
fn test_from_base32_cap_too_small() {
    let mut buf = [0u8; 64];
    assert_eq!(from_base32("MZXW6===", &mut buf, 2), 0);
}

fn main() {}
