use fleur::fnv::{fnv1, getDigest, FNV_OFFSET_BASIS, FNV_PRIME};

#[test]
fn test_fnv_constants() {
    assert_eq!(FNV_PRIME, 1099511628211u64);
    assert_eq!(FNV_OFFSET_BASIS, 14695981039346656037u64);
}

#[test]
fn test_fnv1_test() {
    assert_eq!(fnv1(b"test"), 10090666253179731817u64);
}

#[test]
fn test_fnv1_empty() {
    assert_eq!(fnv1(b""), 14695981039346656037u64);
}

#[test]
fn test_fnv1_hello() {
    assert_eq!(fnv1(b"hello"), 8883723591023973575u64);
}

#[test]
fn test_fnv1_bar() {
    assert_eq!(fnv1(b"bar"), 15625701906442958976u64);
}

#[test]
fn test_fnv1_a() {
    assert_eq!(fnv1(b"a"), 12638153115695167422u64);
}

#[test]
fn test_digest_test() {
    let h = fnv1(b"test");
    assert_eq!(getDigest(h), "8c093f7e9fccbf69");
}

#[test]
fn test_digest_empty() {
    let h = fnv1(b"");
    assert_eq!(getDigest(h), "cbf29ce484222325");
}

#[test]
fn test_digest_hello() {
    let h = fnv1(b"hello");
    assert_eq!(getDigest(h), "7b495389bdbdd4c7");
}

#[test]
fn test_digest_bar() {
    let h = fnv1(b"bar");
    assert_eq!(getDigest(h), "d8d9a5186bad3880");
}

#[test]
fn test_digest_a() {
    let h = fnv1(b"a");
    assert_eq!(getDigest(h), "af63bd4c8601b7be");
}

fn main() {}
