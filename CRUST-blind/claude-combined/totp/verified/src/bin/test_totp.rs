use totp::totp;

fn to_hex(a: &[u8]) -> String {
    let mut s = String::with_capacity(a.len() * 2);
    for &b in a {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn make_secret() -> [u8; 64] {
    let mut secret = [0u8; 64];
    let init: [u8; 20] = [
        0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
        0x39, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38, 0x39, 0x30,
    ];
    for i in 0..20 {
        secret[i] = init[i];
    }
    secret
}

#[test]
fn test_unpack32() {
    let mut a = [0u8; 4];
    totp::unpack32(0x12345678, &mut a);
    assert_eq!(a, [0x12, 0x34, 0x56, 0x78]);
}

#[test]
fn test_unpack32_zero() {
    let mut a = [0xFFu8; 4];
    totp::unpack32(0, &mut a);
    assert_eq!(a, [0, 0, 0, 0]);
}

#[test]
fn test_unpack32_max() {
    let mut a = [0u8; 4];
    totp::unpack32(0xFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_unpack32_deadbeef() {
    let mut a = [0u8; 4];
    totp::unpack32(0xDEADBEEF, &mut a);
    assert_eq!(a, [0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_pack32() {
    let a: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
    assert_eq!(totp::pack32(&a), 0x12345678);
}

#[test]
fn test_pack32_deadbeef() {
    let a: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    assert_eq!(totp::pack32(&a), 0xDEADBEEF);
}

#[test]
fn test_pack32_zero() {
    let a: [u8; 4] = [0; 4];
    assert_eq!(totp::pack32(&a), 0);
}

#[test]
fn test_unpack64() {
    let mut a = [0u8; 8];
    totp::unpack64(0x123456789ABCDEF0, &mut a);
    assert_eq!(
        a,
        [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]
    );
}

#[test]
fn test_unpack64_zero() {
    let mut a = [0xFFu8; 8];
    totp::unpack64(0, &mut a);
    assert_eq!(a, [0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_unpack64_simple() {
    let mut a = [0u8; 8];
    totp::unpack64(0x0102030405060708, &mut a);
    assert_eq!(a, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
}

#[test]
fn test_rotl() {
    assert_eq!(totp::rotl(0x12345678, 4), 0x23456781);
    assert_eq!(totp::rotl(0x80000000, 1), 0x00000001);
    assert_eq!(totp::rotl(0x00000001, 31), 0x80000000);
}

#[test]
fn test_sha1_empty() {
    let mut buf = [0u8; 512];
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 0, 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
}

#[test]
fn test_sha1_abc() {
    let mut buf = [0u8; 512];
    buf[0] = b'a';
    buf[1] = b'b';
    buf[2] = b'c';
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 3, 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "a9993e364706816aba3e25717850c26c9cd0d89d");
}

#[test]
fn test_sha1_fox() {
    let msg = b"The quick brown fox jumps over the lazy dog";
    let mut buf = [0u8; 512];
    for (i, &b) in msg.iter().enumerate() {
        buf[i] = b;
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, msg.len(), 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
}

#[test]
fn test_sha1_64_a() {
    let mut buf = [0u8; 512];
    for i in 0..64 {
        buf[i] = b'a';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 64, 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "0098ba824b5c16427bd7a1122a5a442a25ec644d");
}

#[test]
fn test_sha1_55_b() {
    // boundary - len + 9 == 64, fits in single block
    let mut buf = [0u8; 512];
    for i in 0..55 {
        buf[i] = b'b';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 55, 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "4d055f5334ac4bca50260deff4707cd8d4fc1454");
}

#[test]
fn test_sha1_56_c() {
    // boundary - must extend to 2 blocks
    let mut buf = [0u8; 512];
    for i in 0..56 {
        buf[i] = b'c';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 56, 512, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "ee59278c72f2b1f1c6be889b06f4c47d7a220b3d");
}

#[test]
fn test_sha1_cap_too_small() {
    let mut buf = [0u8; 32];
    let mut hash = [0u8; 20];
    // cap=32 too small for any non-trivial message (64 needed minimum for empty)
    let rc = totp::sha1(&mut buf, 0, 32, &mut hash);
    assert_eq!(rc, totp::TOTP_EBOUNDS);
}

#[test]
fn test_hmac_sha1_rfc2202_v1() {
    // 0x0b * 20, "Hi There"
    let mut key = [0u8; 64];
    for i in 0..20 {
        key[i] = 0x0b;
    }
    let text = b"Hi There";
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, text, text.len(), &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "b617318655057264e28bc0b6fb378c8ef146be00");
}

#[test]
fn test_hmac_sha1_rfc2202_v2() {
    let mut key = [0u8; 64];
    let kbytes = b"Jefe";
    for i in 0..kbytes.len() {
        key[i] = kbytes[i];
    }
    let text = b"what do ya want for nothing?";
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, text, text.len(), &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
}

#[test]
fn test_hmac_sha1_rfc2202_v3() {
    let mut key = [0u8; 64];
    for i in 0..20 {
        key[i] = 0xAA;
    }
    let mut text = [0u8; 64];
    for i in 0..50 {
        text[i] = 0xDD;
    }
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, &text, 50, &mut hash);
    assert_eq!(rc, totp::TOTP_OK);
    assert_eq!(to_hex(&hash), "125d7342b9ac11cd91a39af48aa17b4f63f175d3");
}

#[test]
fn test_hmac_sha1_too_long() {
    let key = [0u8; 64];
    let text = [0u8; 100];
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, &text, 65, &mut hash);
    assert_eq!(rc, totp::TOTP_EBOUNDS);
}

#[test]
fn test_hotp_rfc4226() {
    let secret = make_secret();
    assert_eq!(totp::hotp(&secret, 0), 755224);
    assert_eq!(totp::hotp(&secret, 1), 287082);
    assert_eq!(totp::hotp(&secret, 2), 359152);
    assert_eq!(totp::hotp(&secret, 3), 969429);
    assert_eq!(totp::hotp(&secret, 4), 338314);
    assert_eq!(totp::hotp(&secret, 5), 254676);
    assert_eq!(totp::hotp(&secret, 6), 287922);
    assert_eq!(totp::hotp(&secret, 7), 162583);
    assert_eq!(totp::hotp(&secret, 8), 399871);
    assert_eq!(totp::hotp(&secret, 9), 520489);
}

#[test]
fn test_totp_basic() {
    let secret = make_secret();
    assert_eq!(totp::totp(&secret, 0), 755224);
    assert_eq!(totp::totp(&secret, 30), 287082);
    assert_eq!(totp::totp(&secret, 59), 287082);
    assert_eq!(totp::totp(&secret, 60), 359152);
    assert_eq!(totp::totp(&secret, 90), 969429);
    assert_eq!(totp::totp(&secret, 1234567890), 5924);
}

#[test]
fn test_from_base32_3_byte() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("MZxw6===", &mut buf, 10);
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], b"foo");
}

#[test]
fn test_from_base32_4_byte() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("MZxw6YQ=", &mut buf, 10);
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"foob");
}

#[test]
fn test_from_base32_5_byte() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("MZxw6YTB", &mut buf, 10);
    assert_eq!(n, 5);
    assert_eq!(&buf[..5], b"fooba");
}

#[test]
fn test_from_base32_6_byte() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("MZxw6YTBOI======", &mut buf, 10);
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"foobar");
}

#[test]
fn test_from_base32_invalid_length() {
    let mut buf = [0u8; 10];
    // length 3 is not multiple of 8
    let n = totp::from_base32("ABC", &mut buf, 10);
    assert_eq!(n, 0);
}

#[test]
fn test_from_base32_bad_char() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("AB!DEFGH", &mut buf, 10);
    assert_eq!(n, 0);
}

#[test]
fn test_from_base32_invalid_digit_1() {
    let mut buf = [0u8; 10];
    // '1' is not in base32 alphabet (only 2-7)
    let n = totp::from_base32("11111111", &mut buf, 10);
    assert_eq!(n, 0);
}

#[test]
fn test_from_base32_empty_string() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("", &mut buf, 10);
    assert_eq!(n, 0);
}

#[test]
fn test_from_base32_cap_too_small() {
    let mut buf = [0u8; 10];
    let n = totp::from_base32("MZxw6YTB", &mut buf, 4);
    assert_eq!(n, 0);
}

#[test]
fn test_totp_export_constant() {
    assert_eq!(totp::TOTP_EXPORT, "__attribute__((visibility(\"default\")))");
}

#[test]
fn test_totp_ok_constant() {
    assert_eq!(totp::TOTP_OK, 0);
}

fn main() {}
