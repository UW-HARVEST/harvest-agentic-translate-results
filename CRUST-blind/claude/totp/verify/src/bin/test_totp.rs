use totp::totp;

fn to_hex(a: &[u8]) -> String {
    let hex = b"0123456789abcdef";
    let mut s = String::with_capacity(a.len() * 2);
    for &byte in a {
        s.push(hex[(byte >> 4) as usize] as char);
        s.push(hex[(byte & 0xF) as usize] as char);
    }
    s
}

#[test]
fn test_export_constant() {
    assert_eq!(
        totp::TOTP_EXPORT,
        "__attribute__((visibility(\"default\")))"
    );
}

#[test]
fn test_unpack32_basic() {
    let mut a = [0u8; 4];
    totp::unpack32(0x12345678, &mut a);
    assert_eq!(a[0], 0x12);
    assert_eq!(a[1], 0x34);
    assert_eq!(a[2], 0x56);
    assert_eq!(a[3], 0x78);
}

#[test]
fn test_unpack32_zero() {
    let mut a = [0xFFu8; 4];
    totp::unpack32(0, &mut a);
    assert_eq!(a, [0u8; 4]);
}

#[test]
fn test_unpack32_max() {
    let mut a = [0u8; 4];
    totp::unpack32(0xFFFFFFFF, &mut a);
    assert_eq!(a, [0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_unpack64_basic() {
    let mut a = [0u8; 8];
    totp::unpack64(0x123456789ABCDEF0, &mut a);
    assert_eq!(a[0], 0x12);
    assert_eq!(a[1], 0x34);
    assert_eq!(a[2], 0x56);
    assert_eq!(a[3], 0x78);
    assert_eq!(a[4], 0x9A);
    assert_eq!(a[5], 0xBC);
    assert_eq!(a[6], 0xDE);
    assert_eq!(a[7], 0xF0);
}

#[test]
fn test_unpack64_zero() {
    let mut a = [0xFFu8; 8];
    totp::unpack64(0, &mut a);
    assert_eq!(a, [0u8; 8]);
}

#[test]
fn test_pack32_basic() {
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
fn test_pack32_max() {
    let a: [u8; 4] = [0xFF; 4];
    assert_eq!(totp::pack32(&a), 0xFFFFFFFF);
}

#[test]
fn test_rotl_one_one() {
    assert_eq!(totp::rotl(1, 1), 2);
}

#[test]
fn test_rotl_one_31() {
    assert_eq!(totp::rotl(1, 31), 0x8000_0000);
}

#[test]
fn test_rotl_msb_one() {
    assert_eq!(totp::rotl(0x8000_0000, 1), 1);
}

#[test]
fn test_rotl_complex() {
    assert_eq!(totp::rotl(0xDEADBEEF, 4), 0xEADBEEFD);
    assert_eq!(totp::rotl(0xDEADBEEF, 16), 0xBEEFDEAD);
}

#[test]
fn test_sha1_empty() {
    let mut buf = [0u8; 512];
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 0, 512, &mut hash);
    assert_eq!(rc, 0);
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
    assert_eq!(rc, 0);
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
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12");
}

#[test]
fn test_sha1_55_a_boundary() {
    // 55 bytes still fits one block
    let mut buf = [0u8; 512];
    for i in 0..55 {
        buf[i] = b'a';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 55, 512, &mut hash);
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "c1c8bbdc22796e28c0e15163d20899b65621d65a");
}

#[test]
fn test_sha1_56_a_boundary() {
    // 56 bytes requires two blocks
    let mut buf = [0u8; 512];
    for i in 0..56 {
        buf[i] = b'a';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 56, 512, &mut hash);
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "c2db330f6083854c99d4b5bfb6e8f29f201be699");
}

#[test]
fn test_sha1_64_a() {
    let mut buf = [0u8; 512];
    for i in 0..64 {
        buf[i] = b'a';
    }
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 64, 512, &mut hash);
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "0098ba824b5c16427bd7a1122a5a442a25ec644d");
}

#[test]
fn test_sha1_cap_too_small() {
    // len=100, cap=50 → not enough for padding
    let mut buf = [0u8; 200];
    let mut hash = [0u8; 20];
    let rc = totp::sha1(&mut buf, 100, 50, &mut hash);
    assert_eq!(rc, 1); // TOTP_EBOUNDS
}

#[test]
fn test_hmac_sha1_rfc2202_case1() {
    // key = 0x0b * 20, data = "Hi There"
    let mut key = [0u8; 64];
    for i in 0..20 {
        key[i] = 0x0b;
    }
    let data = b"Hi There";
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, data, data.len(), &mut hash);
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "b617318655057264e28bc0b6fb378c8ef146be00");
}

#[test]
fn test_hmac_sha1_rfc2202_case2() {
    // key = "Jefe", data = "what do ya want for nothing?"
    let mut key = [0u8; 64];
    let key_bytes = b"Jefe";
    for i in 0..key_bytes.len() {
        key[i] = key_bytes[i];
    }
    let data = b"what do ya want for nothing?";
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, data, data.len(), &mut hash);
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
}

#[test]
fn test_hmac_sha1_rfc2202_case3() {
    // key = 0xaa * 20, text = 0xdd * 50
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
    assert_eq!(rc, 0);
    assert_eq!(to_hex(&hash), "125d7342b9ac11cd91a39af48aa17b4f63f175d3");
}

#[test]
fn test_hmac_sha1_too_long() {
    let key = [0u8; 64];
    let data = [0u8; 128];
    let mut hash = [0u8; 20];
    let rc = totp::hmac_sha1(&key, &data, 65, &mut hash);
    assert_eq!(rc, 1); // TOTP_EBOUNDS
}

#[test]
fn test_hotp_rfc4226_appendix_d() {
    // key = "12345678901234567890" (ASCII)
    let mut secret = [0u8; 64];
    let seed = b"12345678901234567890";
    for i in 0..seed.len() {
        secret[i] = seed[i];
    }
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
    // totp(key, time) == hotp(key, time/30)
    let mut secret = [0u8; 64];
    let seed = b"12345678901234567890";
    for i in 0..seed.len() {
        secret[i] = seed[i];
    }
    assert_eq!(totp::totp(&secret, 0), 755224);
    assert_eq!(totp::totp(&secret, 29), 755224);
    assert_eq!(totp::totp(&secret, 30), 287082);
    assert_eq!(totp::totp(&secret, 59), 287082);
    assert_eq!(totp::totp(&secret, 60), 359152);
    assert_eq!(totp::totp(&secret, 1234567890), 5924);
}

#[test]
fn test_from_base32_foo() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("MZxw6===", &mut buf, 10);
    assert_eq!(r, 3);
    assert_eq!(buf[0], 0x66);
    assert_eq!(buf[1], 0x6f);
    assert_eq!(buf[2], 0x6f);
}

#[test]
fn test_from_base32_foob() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("MZxw6YQ=", &mut buf, 10);
    assert_eq!(r, 4);
    assert_eq!(&buf[..4], b"foob");
}

#[test]
fn test_from_base32_fooba() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("MZxw6YTB", &mut buf, 10);
    assert_eq!(r, 5);
    assert_eq!(&buf[..5], b"fooba");
}

#[test]
fn test_from_base32_foobar() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("MZxw6YTBOI======", &mut buf, 10);
    assert_eq!(r, 6);
    assert_eq!(&buf[..6], b"foobar");
}

#[test]
fn test_from_base32_empty() {
    let mut buf = [0u8; 10];
    // strlen == 0, 0 % 8 == 0, cap >= (0+1)/8*5 == 0. Loop doesn't execute. Returns 0.
    let r = totp::from_base32("", &mut buf, 10);
    assert_eq!(r, 0);
}

#[test]
fn test_from_base32_bad_length() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("ABC", &mut buf, 10);
    assert_eq!(r, 0);
}

#[test]
fn test_from_base32_invalid_char() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("!!!!!!!!", &mut buf, 10);
    assert_eq!(r, 0);
}

#[test]
fn test_from_base32_cap_too_small() {
    let mut buf = [0u8; 4];
    // strlen=8, (8+1)/8*5 = 5 > cap=4
    let r = totp::from_base32("MZXW6YTB", &mut buf, 4);
    assert_eq!(r, 0);
}

#[test]
fn test_from_base32_lowercase() {
    let mut buf = [0u8; 10];
    let r = totp::from_base32("mzxw6ytb", &mut buf, 10);
    assert_eq!(r, 5);
    assert_eq!(&buf[..5], b"fooba");
}

fn main() {}
