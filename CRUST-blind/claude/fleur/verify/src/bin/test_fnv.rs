use fleur::fnv::{fnv1, getDigest, FNV_OFFSET_BASIS, FNV_PRIME};

#[test]
fn test_fnv_constants() {
    assert_eq!(FNV_PRIME, 1099511628211u64);
    assert_eq!(FNV_OFFSET_BASIS, 14695981039346656037u64);
}

#[test]
fn test_fnv_empty_string() {
    // C: fnv1("", 0) returns FNV_OFFSET unchanged
    let h = fnv1(b"");
    assert_eq!(h, 14695981039346656037u64);
    assert_eq!(getDigest(h), "cbf29ce484222325");
}

#[test]
fn test_fnv_test_string() {
    // From C suite_1: fnv1("test") -> "8c093f7e9fccbf69"
    let h = fnv1(b"test");
    assert_eq!(h, 10090666253179731817u64);
    assert_eq!(getDigest(h), "8c093f7e9fccbf69");
}

#[test]
fn test_fnv_single_char_a() {
    let h = fnv1(b"a");
    assert_eq!(h, 12638153115695167422u64);
    assert_eq!(getDigest(h), "af63bd4c8601b7be");
}

#[test]
fn test_fnv_hello_world() {
    let h = fnv1(b"hello world");
    assert_eq!(h, 9065573210506989167u64);
    assert_eq!(getDigest(h), "7dcf62cdb1910e6f");
}

#[test]
fn test_fnv_foo() {
    let h = fnv1(b"foo");
    assert_eq!(h, 15621798640163566899u64);
    assert_eq!(getDigest(h), "d8cbc7186ba13533");
}

#[test]
fn test_fnv_bar() {
    let h = fnv1(b"bar");
    assert_eq!(h, 15625701906442958976u64);
    assert_eq!(getDigest(h), "d8d9a5186bad3880");
}

#[test]
fn test_fnv_high_bytes_sign_extended() {
    // C reads char as signed; bytes >=0x80 sign-extend before XOR.
    let h = fnv1(&[0x80, 0x81, 0xff]);
    assert_eq!(h, 2847762498850077947u64);
    assert_eq!(getDigest(h), "278548e794a33cfb");
}

#[test]
fn test_get_digest_known_values() {
    // The digest is the big-endian hex representation of the 64-bit hash.
    assert_eq!(getDigest(0u64), "0000000000000000");
    assert_eq!(getDigest(1u64), "0000000000000001");
    assert_eq!(
        getDigest(0xFFFFFFFFFFFFFFFFu64),
        "ffffffffffffffff"
    );
    assert_eq!(
        getDigest(0x0123456789ABCDEFu64),
        "0123456789abcdef"
    );
}

fn main() {}
