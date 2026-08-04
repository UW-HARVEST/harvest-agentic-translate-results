use bhshell::xalloc;

#[test]
fn test_xmalloc_zero() {
    let v = xalloc::xmalloc(0);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_xmalloc_size() {
    let v = xalloc::xmalloc(16);
    assert_eq!(v.len(), 16);
    // The C version returns uninitialized memory; in the Rust version
    // we initialize to zero. Verify all zero.
    for byte in &v {
        assert_eq!(*byte, 0);
    }
}

#[test]
fn test_xmalloc_large() {
    let v = xalloc::xmalloc(1024);
    assert_eq!(v.len(), 1024);
}

#[test]
fn test_xrealloc_grow() {
    let v = vec![1u8, 2, 3, 4];
    let v2 = xalloc::xrealloc(v, 8);
    assert_eq!(v2.len(), 8);
    assert_eq!(v2[0], 1);
    assert_eq!(v2[1], 2);
    assert_eq!(v2[2], 3);
    assert_eq!(v2[3], 4);
}

#[test]
fn test_xrealloc_shrink() {
    let v = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
    let v2 = xalloc::xrealloc(v, 4);
    assert_eq!(v2.len(), 4);
    assert_eq!(v2[0], 1);
    assert_eq!(v2[1], 2);
    assert_eq!(v2[2], 3);
    assert_eq!(v2[3], 4);
}

#[test]
fn test_xrealloc_from_empty() {
    let v: Vec<u8> = Vec::new();
    let v2 = xalloc::xrealloc(v, 16);
    assert_eq!(v2.len(), 16);
}

fn main() {}
