use fleur::fnv;

#[test]
fn test_fnv_offset_basis_empty() {
    // Empty buffer => result equals offset basis
    let h = fnv::fnv1(b"");
    assert_eq!(h, 14695981039346656037u64);
}

#[test]
fn test_fnv_test_string() {
    // From C ground truth: fnv1("test") = 10090666253179731817
    let h = fnv::fnv1(b"test");
    assert_eq!(h, 10090666253179731817u64);
}

#[test]
fn test_fnv_hello() {
    // From C: fnv1("hello") = 8883723591023973575
    let h = fnv::fnv1(b"hello");
    assert_eq!(h, 8883723591023973575u64);
}

#[test]
fn test_fnv_foo_bar_baz() {
    // From C: fnv1("Foo Bar Baz") = 370735856047204459
    let h = fnv::fnv1(b"Foo Bar Baz");
    assert_eq!(h, 370735856047204459u64);
}

#[test]
fn test_fnv_high_bytes() {
    // From C: bytes {0xff, 0x80, 0x01} => 15659218319399092129
    let h = fnv::fnv1(&[0xffu8, 0x80, 0x01]);
    assert_eq!(h, 15659218319399092129u64);
}

#[test]
fn test_fnv_constants() {
    assert_eq!(fnv::FNV_PRIME, 1099511628211u64);
    assert_eq!(fnv::FNV_OFFSET_BASIS, 14695981039346656037u64);
}

#[test]
fn test_get_digest_test() {
    // From C test suite: getDigest of fnv1("test") => "8c093f7e9fccbf69"
    let h = fnv::fnv1(b"test");
    let s = fnv::getDigest(h);
    assert_eq!(s, "8c093f7e9fccbf69");
}

#[test]
fn test_get_digest_hello() {
    let h = fnv::fnv1(b"hello");
    let s = fnv::getDigest(h);
    assert_eq!(s, "7b495389bdbdd4c7");
}

#[test]
fn test_get_digest_foo_bar_baz() {
    let h = fnv::fnv1(b"Foo Bar Baz");
    let s = fnv::getDigest(h);
    assert_eq!(s, "05251e4bfd31a86b");
}

#[test]
fn test_get_digest_high_bytes() {
    let h = fnv::fnv1(&[0xffu8, 0x80, 0x01]);
    let s = fnv::getDigest(h);
    assert_eq!(s, "d950b8186c127fa1");
}

fn main() {}
