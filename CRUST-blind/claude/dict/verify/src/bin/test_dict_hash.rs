use dict::dict::{dict_get_hash, dict_new, DictType, HASH_BASE, HASH_MOD};

/// These tests check `dict_get_hash` against the C implementation.

#[test]
fn test_hash_i32_simple() {
    let d = dict_new(DictType::I32, 4, 4);
    // i32 hash returns the value sign-extended to u64. C: code = *(int32_t*)key
    // (effectively sign-extended through int64_t->u64 conversion).
    let key = 5i32.to_le_bytes();
    let h = dict_get_hash(&d, &key);
    assert_eq!(h, 5u64);
}

#[test]
fn test_hash_i32_zero() {
    let d = dict_new(DictType::I32, 4, 4);
    let h = dict_get_hash(&d, &0i32.to_le_bytes());
    assert_eq!(h, 0u64);
}

#[test]
fn test_hash_i32_negative_sign_extends() {
    // -7 sign-extended to u64 = 0xFFFF_FFFF_FFFF_FFF9
    let d = dict_new(DictType::I32, 4, 4);
    let h = dict_get_hash(&d, &(-7i32).to_le_bytes());
    assert_eq!(h, (-7i64) as u64);
}

#[test]
fn test_hash_u32_no_sign_extend() {
    let d = dict_new(DictType::U32, 4, 4);
    let h = dict_get_hash(&d, &u32::MAX.to_le_bytes());
    assert_eq!(h, u32::MAX as u64);
}

#[test]
fn test_hash_i64() {
    let d = dict_new(DictType::I64, 8, 4);
    let h = dict_get_hash(&d, &(-12345i64).to_le_bytes());
    assert_eq!(h, (-12345i64) as u64);
}

#[test]
fn test_hash_u64() {
    let d = dict_new(DictType::U64, 8, 4);
    let v: u64 = 0xDEAD_BEEF_CAFE_BABE;
    let h = dict_get_hash(&d, &v.to_le_bytes());
    assert_eq!(h, v);
}

#[test]
fn test_hash_char_signed() {
    // C: *(char*)key — char is signed on x86_64, sign-extended into u64.
    let d = dict_new(DictType::Char, 1, 4);
    let h = dict_get_hash(&d, &[0x80u8]); // -128 as signed char
    assert_eq!(h, (-128i64) as u64);
    let h = dict_get_hash(&d, &[0x41u8]); // 'A'
    assert_eq!(h, 0x41);
}

#[test]
fn test_hash_wchar() {
    let d = dict_new(DictType::WChar, 4, 4);
    let h = dict_get_hash(&d, &(0xFFFFu32 as i32).to_le_bytes());
    assert_eq!(h, 0xFFFFu64);
}

#[test]
fn test_hash_f32() {
    let d = dict_new(DictType::F32, 4, 4);
    // C does code = *(float*)key — implicit float->u64 truncation.
    let h = dict_get_hash(&d, &(3.14f32).to_le_bytes());
    assert_eq!(h, 3u64);
    let h = dict_get_hash(&d, &(0.0f32).to_le_bytes());
    assert_eq!(h, 0u64);
}

#[test]
fn test_hash_f64() {
    let d = dict_new(DictType::F64, 8, 4);
    let h = dict_get_hash(&d, &(2.718f64).to_le_bytes());
    assert_eq!(h, 2u64);
}

#[test]
fn test_hash_str_empty() {
    let d = dict_new(DictType::Str, 0, 4);
    let h = dict_get_hash(&d, b"");
    assert_eq!(h, 0u64);
}

#[test]
fn test_hash_str_value() {
    let d = dict_new(DictType::Str, 0, 4);

    fn expected(s: &[u8]) -> u64 {
        let mut code: u64 = 0;
        for &b in s {
            code = (code.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
        }
        code
    }

    // For "Hello" the C implementation produces 939247605 (verified separately).
    assert_eq!(dict_get_hash(&d, b"Hello"), 939_247_605);
    assert_eq!(dict_get_hash(&d, b"World"), 531_921_955);
    assert_eq!(dict_get_hash(&d, b"0"), 48);
    assert_eq!(dict_get_hash(&d, b"1"), 49);
    // sanity-check the polynomial
    assert_eq!(dict_get_hash(&d, b"abc"), expected(b"abc"));
}

#[test]
fn test_hash_struct_uses_byte_polynomial() {
    let d = dict_new(DictType::Struct, 5, 4);
    // For struct, the key buffer is rounded up to ptr-size (8 bytes) and the
    // polynomial runs over key.size bytes (which is the rounded-up size).
    let bytes = [1u8, 2, 3, 4, 5, 0, 0, 0]; // exactly key.size = 8 bytes
    let mut expected: u64 = 0;
    for &b in &bytes[..d.key.size] {
        expected = (expected.wrapping_mul(HASH_BASE).wrapping_add(b as u64)) % HASH_MOD;
    }
    let h = dict_get_hash(&d, &bytes);
    assert_eq!(h, expected);
}

#[test]
fn test_hash_constants_match_c() {
    assert_eq!(HASH_BASE, 256);
    assert_eq!(HASH_MOD, 1_000_000_007);
}

fn main() {}
