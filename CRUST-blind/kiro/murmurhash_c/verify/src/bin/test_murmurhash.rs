use murmurhash_c::murmurhash;

// All expected values derived from running the C implementation.

#[test]
fn test_empty_string_seed_0() {
    assert_eq!(murmurhash::murmurhash(b"", 0), 0);
}

#[test]
fn test_empty_string_seed_1() {
    assert_eq!(murmurhash::murmurhash(b"", 1), 0x514e28b7);
}

#[test]
fn test_single_digit_strings() {
    assert_eq!(murmurhash::murmurhash(b"0", 0), 0xd271c07f);
    assert_eq!(murmurhash::murmurhash(b"2", 0), 0x0129e217);
}

#[test]
fn test_incrementing_digit_strings() {
    assert_eq!(murmurhash::murmurhash(b"01", 0), 0x61ec6600);
    assert_eq!(murmurhash::murmurhash(b"012", 0), 0xec6cff8c);
    assert_eq!(murmurhash::murmurhash(b"0123", 0), 0xd41994a0);
    assert_eq!(murmurhash::murmurhash(b"01234", 0), 0x19d02170);
}

#[test]
fn test_two_digit_string() {
    assert_eq!(murmurhash::murmurhash(b"88", 0), 0x7a0040a5);
}

#[test]
fn test_alpha_strings() {
    assert_eq!(murmurhash::murmurhash(b"asdfqwer", 0), 0xa46b5209);
    assert_eq!(murmurhash::murmurhash(b"asdfqwerty", 0), 0xa3cfe04b);
    assert_eq!(murmurhash::murmurhash(b"asd", 0), 0x14570c6f);
}

#[test]
fn test_hello_variants() {
    assert_eq!(murmurhash::murmurhash(b"Hello", 0), 0x12da77c8);
    assert_eq!(murmurhash::murmurhash(b"Hello1", 0), 0x6357e0a6);
    assert_eq!(murmurhash::murmurhash(b"Hello2", 0), 0xe5ce223e);
}

#[test]
fn test_short_words() {
    assert_eq!(murmurhash::murmurhash(b"hey", 0), 0x12f94418);
    assert_eq!(murmurhash::murmurhash(b"dude", 0), 0xef0487f3);
    assert_eq!(murmurhash::murmurhash(b"test", 0), 0xba6bd213);
    assert_eq!(murmurhash::murmurhash(b"kinkajou", 0), 0xb6d99cf8);
}

#[test]
fn test_remainder_lengths() {
    // len%4 == 1
    assert_eq!(murmurhash::murmurhash(b"a", 0), 1009084850);
    // len%4 == 2
    assert_eq!(murmurhash::murmurhash(b"ab", 0), 2613040991);
    // len%4 == 3
    assert_eq!(murmurhash::murmurhash(b"abc", 0), 3017643002);
    // len%4 == 0
    assert_eq!(murmurhash::murmurhash(b"abcd", 0), 1139631978);
    // len%4 == 1 (5 bytes)
    assert_eq!(murmurhash::murmurhash(b"abcde", 0), 3902511862);
}

#[test]
fn test_various_seeds() {
    assert_eq!(murmurhash::murmurhash(b"test", 1), 2579507938);
    assert_eq!(murmurhash::murmurhash(b"test", 42), 3959873882);
    assert_eq!(murmurhash::murmurhash(b"test", u32::MAX), 1708948417);
}

#[test]
fn test_binary_bytes() {
    assert_eq!(murmurhash::murmurhash(&[0xff], 0), 4251775245);
    assert_eq!(murmurhash::murmurhash(&[0x00], 0), 1364076727);
}

#[test]
fn test_version() {
    assert_eq!(murmurhash::MURMURHASH_VERSION, "0.2.0");
}

#[test]
fn test_has_htole32() {
    assert_eq!(murmurhash::MURMURHASH_HAS_HTOLE32, 1);
}

#[test]
fn test_htole32() {
    assert_eq!(murmurhash::htole32(0), 0);
    assert_eq!(murmurhash::htole32(1), 1);
    assert_eq!(murmurhash::htole32(0xDEADBEEF), 0xDEADBEEFu32.to_le());
}

fn main() {}
