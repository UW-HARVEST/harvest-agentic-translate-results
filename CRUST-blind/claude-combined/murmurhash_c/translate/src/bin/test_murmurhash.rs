// The crate's public module is named `murmurhash`. Reference it via the
// fully-qualified `murmurhash_c::murmurhash::...` path in each test below.
#[allow(unused_imports)]
use murmurhash_c::murmurhash;

#[test]
fn test_version_constant() {
    assert_eq!(murmurhash::MURMURHASH_VERSION, "0.2.0");
}

#[test]
fn test_has_htole32_constant() {
    assert_eq!(murmurhash::MURMURHASH_HAS_HTOLE32, 1);
}

#[test]
fn test_htole32_identity_on_le() {
    // The C `htole32` is a no-op on little-endian hosts (which is the
    // ubiquitous case for our test environment). On big-endian hosts the
    // C implementation byteswaps; Rust's u32::to_le() matches that
    // behavior.
    if cfg!(target_endian = "little") {
        assert_eq!(murmurhash::htole32(0), 0);
        assert_eq!(murmurhash::htole32(1), 1);
        assert_eq!(murmurhash::htole32(0xdeadbeef), 0xdeadbeef);
        assert_eq!(murmurhash::htole32(0x12345678), 0x12345678);
        assert_eq!(murmurhash::htole32(u32::MAX), u32::MAX);
    } else {
        assert_eq!(murmurhash::htole32(0x12345678), 0x78563412);
    }
}

// Expected hashes obtained by running the C `murmurhash` implementation
// (see c_src/test.c and c_src/extra_test.c).
#[test]
fn test_known_seed0_empty() {
    assert_eq!(murmurhash::murmurhash(b"", 0), 0x00000000);
}

#[test]
fn test_known_seed0_single_chars() {
    assert_eq!(murmurhash::murmurhash(b"0", 0), 0xd271c07f);
    assert_eq!(murmurhash::murmurhash(b"2", 0), 0x0129e217);
}

#[test]
fn test_known_seed0_short_strings() {
    assert_eq!(murmurhash::murmurhash(b"01", 0), 0x61ec6600);
    assert_eq!(murmurhash::murmurhash(b"012", 0), 0xec6cff8c);
    assert_eq!(murmurhash::murmurhash(b"0123", 0), 0xd41994a0);
    assert_eq!(murmurhash::murmurhash(b"01234", 0), 0x19d02170);
    assert_eq!(murmurhash::murmurhash(b"88", 0), 0x7a0040a5);
}

#[test]
fn test_known_seed0_words() {
    assert_eq!(murmurhash::murmurhash(b"asdfqwer", 0), 0xa46b5209);
    assert_eq!(murmurhash::murmurhash(b"asdfqwerty", 0), 0xa3cfe04b);
    assert_eq!(murmurhash::murmurhash(b"asd", 0), 0x14570c6f);
    assert_eq!(murmurhash::murmurhash(b"Hello", 0), 0x12da77c8);
    assert_eq!(murmurhash::murmurhash(b"Hello1", 0), 0x6357e0a6);
    assert_eq!(murmurhash::murmurhash(b"Hello2", 0), 0xe5ce223e);
    assert_eq!(murmurhash::murmurhash(b"hey", 0), 0x12f94418);
    assert_eq!(murmurhash::murmurhash(b"dude", 0), 0xef0487f3);
    assert_eq!(murmurhash::murmurhash(b"test", 0), 0xba6bd213);
    assert_eq!(murmurhash::murmurhash(b"kinkajou", 0), 0xb6d99cf8);
}

#[test]
fn test_empty_with_nonzero_seed() {
    assert_eq!(murmurhash::murmurhash(b"", 1), 0x514e28b7);
}

#[test]
fn test_alphabet_progressive_lengths() {
    // Tests every tail length (0..=3) plus more bytes.
    assert_eq!(murmurhash::murmurhash(b"a", 0), 0x3c2569b2);
    assert_eq!(murmurhash::murmurhash(b"ab", 0), 0x9bbfd75f);
    assert_eq!(murmurhash::murmurhash(b"abc", 0), 0xb3dd93fa);
    assert_eq!(murmurhash::murmurhash(b"abcd", 0), 0x43ed676a);
    assert_eq!(murmurhash::murmurhash(b"abcde", 0), 0xe89b9af6);
    assert_eq!(murmurhash::murmurhash(b"abcdef", 0), 0x6181c085);
    assert_eq!(murmurhash::murmurhash(b"abcdefg", 0), 0x883c9b06);
    assert_eq!(murmurhash::murmurhash(b"abcdefgh", 0), 0x49ddccc4);
}

#[test]
fn test_long_string_multiple_seeds() {
    let s = b"The quick brown fox jumps over the lazy dog";
    assert_eq!(murmurhash::murmurhash(s, 0), 0x2e4ff723);
    assert_eq!(murmurhash::murmurhash(s, 1), 0x78e69e27);
    assert_eq!(murmurhash::murmurhash(s, 42), 0x347ca102);
}

#[test]
fn test_with_large_seed() {
    assert_eq!(murmurhash::murmurhash(b"x", 12345), 0x86c7a251);
    assert_eq!(murmurhash::murmurhash(b"xx", 12345), 0x301cff36);
    assert_eq!(murmurhash::murmurhash(b"xxx", 12345), 0x0e4c4514);
    assert_eq!(murmurhash::murmurhash(b"xxxx", 12345), 0x26836ee3);
}

#[test]
fn test_with_max_seed_like() {
    assert_eq!(murmurhash::murmurhash(b"Lorem ipsum", 0xdeadbeef), 0x1a129808);
}

#[test]
fn test_zero_bytes() {
    // Four zero bytes is not the same as empty input.
    assert_eq!(murmurhash::murmurhash(&[0u8, 0, 0, 0], 0), 0x2362f9de);
}

#[test]
fn test_all_ones_bytes() {
    assert_eq!(murmurhash::murmurhash(&[0xffu8, 0xff, 0xff, 0xff], 0), 0x76293b50);
}

#[test]
fn test_binary_data_with_seed() {
    let buf: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
    assert_eq!(murmurhash::murmurhash(&buf, 0xcafebabe), 0x0e86ddfc);
}

#[test]
fn test_kinkajou_example() {
    // Matches the example documented in c_src/example.c
    let key = b"kinkajou";
    assert_eq!(murmurhash::murmurhash(key, 0), 0xb6d99cf8);
}

fn main() {}
