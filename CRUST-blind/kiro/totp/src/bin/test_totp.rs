use totp::totp as totp_mod;

fn to_hex(a: &[u8]) -> String {
    a.iter().map(|b| format!("{:02x}", b)).collect()
}

// --- unpack32 ---

#[test]
fn test_unpack32_zero() {
    let mut a = [0u8; 4];
    totp_mod::unpack32(0, &mut a);
    assert_eq!(a, [0, 0, 0, 0]);
}

#[test]
fn test_unpack32_known() {
    let mut a = [0u8; 4];
    totp_mod::unpack32(0x12345678, &mut a);
    assert_eq!(a, [0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn test_unpack32_max() {
    let mut a = [0u8; 4];
    totp_mod::unpack32(0xFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF, 0xFF, 0xFF, 0xFF]);
}

// --- pack32 ---

#[test]
fn test_pack32_zero() {
    assert_eq!(totp_mod::pack32(&[0, 0, 0, 0]), 0);
}

#[test]
fn test_pack32_known() {
    assert_eq!(totp_mod::pack32(&[0x12, 0x34, 0x56, 0x78]), 0x12345678);
}

#[test]
fn test_pack32_max() {
    assert_eq!(totp_mod::pack32(&[0xFF, 0xFF, 0xFF, 0xFF]), 0xFFFFFFFF);
}

#[test]
fn test_pack32_unpack32_roundtrip() {
    let mut a = [0u8; 4];
    totp_mod::unpack32(0x12345678, &mut a);
    assert_eq!(totp_mod::pack32(&a), 0x12345678);
}

// --- unpack64 ---

#[test]
fn test_unpack64_zero() {
    let mut a = [0u8; 8];
    totp_mod::unpack64(0, &mut a);
    assert_eq!(a, [0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_unpack64_known() {
    let mut a = [0u8; 8];
    totp_mod::unpack64(0x123456789ABCDEF0, &mut a);
    assert_eq!(a, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
}

#[test]
fn test_unpack64_max() {
    let mut a = [0u8; 8];
    totp_mod::unpack64(0xFFFFFFFFFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF; 8]);
}

// --- rotl ---

#[test]
fn test_rotl() {
    assert_eq!(totp_mod::rotl(1, 1), 2);
    assert_eq!(totp_mod::rotl(0x80000000, 1), 1);
}

// --- sha1 ---

#[test]
fn test_sha1_empty() {
    let mut buf = [0u8; 512];
    let mut hash = [0u8; 20];
    let ret = totp_mod::sha1(&mut buf, 0, 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
}

#[test]
fn test_sha1_abc() {
    let mut buf = [0u8; 512];
    buf[..3].copy_from_slice(b"abc");
    let mut hash = [0u8; 20];
    let ret = totp_mod::sha1(&mut buf, 3, 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "a9993e364706816aba3e25717850c26c9cd0d89d");
}

#[test]
fn test_sha1_fox() {
    let msg = b"The quick brown fox jumps over the lazy dog";
    let mut buf = [0u8; 512];
    buf[..msg.len()].copy_from_slice(msg);
    let mut hash = [0u8; 20];
    let ret = totp_mod::sha1(&mut buf, msg.len(), 512, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
}

#[test]
fn test_sha1_cap_too_small() {
    let mut buf = [0u8; 1];
    let mut hash = [0u8; 20];
    let ret = totp_mod::sha1(&mut buf, 0, 0, &mut hash);
    assert_eq!(ret, 1);
}

// --- hmac_sha1 ---

#[test]
fn test_hmac_sha1_rfc2202() {
    let mut key = [0u8; 64];
    for i in 0..20 { key[i] = 0xAA; }
    let mut text = [0u8; 64];
    for i in 0..50 { text[i] = 0xDD; }
    let mut hash = [0u8; 20];
    let ret = totp_mod::hmac_sha1(&key, &text, 50, &mut hash);
    assert_eq!(ret, 0);
    assert_eq!(to_hex(&hash), "125d7342b9ac11cd91a39af48aa17b4f63f175d3");
}

#[test]
fn test_hmac_sha1_len_too_large() {
    let key = [0u8; 64];
    let data = [0u8; 65];
    let mut hash = [0u8; 20];
    let ret = totp_mod::hmac_sha1(&key, &data, 65, &mut hash);
    assert_eq!(ret, 1);
}

// --- hotp (RFC 4226 Appendix D) ---

#[test]
fn test_hotp_rfc4226() {
    let mut secret = [0u8; 64];
    secret[..20].copy_from_slice(b"12345678901234567890");
    let expected = [755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489];
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(totp_mod::hotp(&secret, i as u64), exp, "hotp counter={}", i);
    }
}

// --- totp ---

#[test]
fn test_totp_time_boundaries() {
    let mut secret = [0u8; 64];
    secret[..20].copy_from_slice(b"12345678901234567890");
    // time 0 and 29 -> same 30s window (counter 0) -> 755224
    assert_eq!(totp_mod::totp(&secret, 0), 755224);
    assert_eq!(totp_mod::totp(&secret, 29), 755224);
    // time 30 and 59 -> counter 1 -> 287082
    assert_eq!(totp_mod::totp(&secret, 30), 287082);
    assert_eq!(totp_mod::totp(&secret, 59), 287082);
    // time 60 -> counter 2 -> 359152
    assert_eq!(totp_mod::totp(&secret, 60), 359152);
}

// --- from_base32 ---

#[test]
fn test_from_base32_empty() {
    let mut buf = [0u8; 64];
    assert_eq!(totp_mod::from_base32("", &mut buf, 64), 0);
}

#[test]
fn test_from_base32_foo() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("MZXW6===", &mut buf, 64);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], b"foo");
}

#[test]
fn test_from_base32_foob() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("MZXW6YQ=", &mut buf, 64);
    assert_eq!(n, 4);
    assert_eq!(&buf[..n], b"foob");
}

#[test]
fn test_from_base32_fooba() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("MZXW6YTB", &mut buf, 64);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"fooba");
}

#[test]
fn test_from_base32_foobar() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("MZXW6YTBOI======", &mut buf, 64);
    assert_eq!(n, 6);
    assert_eq!(&buf[..n], b"foobar");
}

#[test]
fn test_from_base32_lowercase() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("mzxw6===", &mut buf, 64);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], b"foo");
}

#[test]
fn test_from_base32_single_char() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("ME======", &mut buf, 64);
    assert_eq!(n, 1);
    assert_eq!(buf[0], b'a');
}

#[test]
fn test_from_base32_hello_binary() {
    let mut buf = [0u8; 64];
    let n = totp_mod::from_base32("JBSWY3DPEHPK3PXP", &mut buf, 64);
    assert_eq!(n, 10);
    assert_eq!(&buf[..6], b"Hello!");
    assert_eq!(&buf[6..10], &[0xde, 0xad, 0xbe, 0xef]);
}

#[test]
fn test_from_base32_invalid_length() {
    let mut buf = [0u8; 64];
    assert_eq!(totp_mod::from_base32("ABC", &mut buf, 64), 0);
}

#[test]
fn test_from_base32_invalid_char() {
    let mut buf = [0u8; 64];
    assert_eq!(totp_mod::from_base32("MZXW6!==", &mut buf, 64), 0);
}

#[test]
fn test_from_base32_cap_too_small() {
    let mut buf = [0u8; 64];
    assert_eq!(totp_mod::from_base32("MZXW6===", &mut buf, 2), 0);
}

fn main() {}
