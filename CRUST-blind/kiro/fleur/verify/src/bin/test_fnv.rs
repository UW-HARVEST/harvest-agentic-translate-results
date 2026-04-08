use fleur::fnv;

#[test]
fn test_fnv1_test() {
    assert_eq!(fnv::fnv1(b"test"), 10090666253179731817);
}

#[test]
fn test_fnv1_empty() {
    assert_eq!(fnv::fnv1(b""), 14695981039346656037);
}

#[test]
fn test_fnv1_hello() {
    assert_eq!(fnv::fnv1(b"hello"), 8883723591023973575);
}

#[test]
fn test_fnv1_foobar() {
    assert_eq!(fnv::fnv1(b"foobar"), 3750802935296928194);
}

#[test]
fn test_fnv1_single_byte() {
    assert_eq!(fnv::fnv1(b"A"), 12638153115695167390);
}

#[test]
fn test_digest_test() {
    let h = fnv::fnv1(b"test");
    assert_eq!(fnv::getDigest(h), "8c093f7e9fccbf69");
}

#[test]
fn test_digest_empty() {
    let h = fnv::fnv1(b"");
    assert_eq!(fnv::getDigest(h), "cbf29ce484222325");
}

#[test]
fn test_digest_hello() {
    let h = fnv::fnv1(b"hello");
    assert_eq!(fnv::getDigest(h), "7b495389bdbdd4c7");
}

#[test]
fn test_digest_foobar() {
    let h = fnv::fnv1(b"foobar");
    assert_eq!(fnv::getDigest(h), "340d8765a4dda9c2");
}

#[test]
fn test_digest_single_byte() {
    let h = fnv::fnv1(b"A");
    assert_eq!(fnv::getDigest(h), "af63bd4c8601b79e");
}

#[test]
fn test_constants() {
    assert_eq!(fnv::FNV_PRIME, 1099511628211);
    assert_eq!(fnv::FNV_OFFSET_BASIS, 14695981039346656037);
}

fn main() {}
