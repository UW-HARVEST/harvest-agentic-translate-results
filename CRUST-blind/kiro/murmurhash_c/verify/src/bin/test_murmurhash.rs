use murmurhash_c::murmurhash;

// --- murmurhash function: test vectors from C test.c (seed=0) ---

#[test]
fn test_empty_string_seed0() {
    assert_eq!(murmurhash::murmurhash(b"", 0), 0);
}

#[test]
fn test_digit_strings_seed0() {
    assert_eq!(murmurhash::murmurhash(b"0", 0), 3530670207);
    assert_eq!(murmurhash::murmurhash(b"01", 0), 1642882560);
    assert_eq!(murmurhash::murmurhash(b"012", 0), 3966566284);
    assert_eq!(murmurhash::murmurhash(b"0123", 0), 3558446240);
    assert_eq!(murmurhash::murmurhash(b"01234", 0), 433070448);
    assert_eq!(murmurhash::murmurhash(b"2", 0), 19522071);
    assert_eq!(murmurhash::murmurhash(b"88", 0), 2046836901);
}

#[test]
fn test_alpha_strings_seed0() {
    assert_eq!(murmurhash::murmurhash(b"asdfqwer", 0), 2758496777);
    assert_eq!(murmurhash::murmurhash(b"asdfqwerty", 0), 2748309579);
    assert_eq!(murmurhash::murmurhash(b"asd", 0), 341249135);
    assert_eq!(murmurhash::murmurhash(b"Hello", 0), 316307400);
    assert_eq!(murmurhash::murmurhash(b"Hello1", 0), 1666703526);
    assert_eq!(murmurhash::murmurhash(b"Hello2", 0), 3855491646);
    assert_eq!(murmurhash::murmurhash(b"hey", 0), 318325784);
    assert_eq!(murmurhash::murmurhash(b"dude", 0), 4010051571);
    assert_eq!(murmurhash::murmurhash(b"test", 0), 3127628307);
    assert_eq!(murmurhash::murmurhash(b"kinkajou", 0), 3067714808);
}

#[test]
fn test_empty_string_seed1() {
    assert_eq!(murmurhash::murmurhash(b"", 1), 1364076727);
}

// --- Additional seed values ---

#[test]
fn test_empty_string_seed42() {
    assert_eq!(murmurhash::murmurhash(b"", 42), 142593372);
}

#[test]
fn test_empty_string_seed_max() {
    assert_eq!(murmurhash::murmurhash(b"", 0xFFFFFFFF), 2180083513);
}

#[test]
fn test_varying_seeds() {
    assert_eq!(murmurhash::murmurhash(b"test", 1), 2579507938);
    assert_eq!(murmurhash::murmurhash(b"test", 100), 1819757633);
    assert_eq!(murmurhash::murmurhash(b"test", 0xFFFFFFFF), 1708948417);
}

// --- Remainder path coverage (1, 2, 3 byte tails) ---

#[test]
fn test_remainder_paths() {
    assert_eq!(murmurhash::murmurhash(b"a", 0), 1009084850);       // 1 byte tail
    assert_eq!(murmurhash::murmurhash(b"ab", 0), 2613040991);      // 2 byte tail
    assert_eq!(murmurhash::murmurhash(b"abc", 0), 3017643002);     // 3 byte tail
    assert_eq!(murmurhash::murmurhash(b"abcd", 0), 1139631978);    // 0 byte tail (exact 4)
    assert_eq!(murmurhash::murmurhash(b"abcde", 0), 3902511862);   // 1 byte tail after chunk
}

// --- Binary data ---

#[test]
fn test_binary_data() {
    assert_eq!(murmurhash::murmurhash(&[0, 1, 2, 3, 4, 5, 6, 7], 0), 3512850035);
    assert_eq!(murmurhash::murmurhash(&[0, 1, 2], 0), 1372901591);
}

// --- Constants ---

#[test]
fn test_version() {
    assert_eq!(murmurhash::MURMURHASH_VERSION, "0.2.0");
}

#[test]
fn test_has_htole32() {
    assert_eq!(murmurhash::MURMURHASH_HAS_HTOLE32, 1);
}

// --- htole32 ---

#[test]
fn test_htole32() {
    // On little-endian (which this system is), htole32 is identity
    assert_eq!(murmurhash::htole32(0), 0);
    assert_eq!(murmurhash::htole32(1), 1);
    assert_eq!(murmurhash::htole32(0xFFFFFFFF), 0xFFFFFFFF);
    assert_eq!(murmurhash::htole32(0x12345678), 0x12345678);
}

fn main() {}
