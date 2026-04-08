use bhshell::xalloc::{xmalloc, xrealloc};

#[test]
fn test_xmalloc_zero() {
    let v = xmalloc(0);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_xmalloc_nonzero() {
    let v = xmalloc(64);
    assert_eq!(v.len(), 64);
    assert!(v.iter().all(|&b| b == 0));
}

#[test]
fn test_xrealloc_grow() {
    let v = xmalloc(16);
    let v = xrealloc(v, 64);
    assert_eq!(v.len(), 64);
}

#[test]
fn test_xrealloc_shrink() {
    let v = xmalloc(64);
    let v = xrealloc(v, 16);
    assert_eq!(v.len(), 16);
}

#[test]
fn test_xrealloc_preserves_data() {
    let mut v = xmalloc(4);
    v[0] = 1;
    v[1] = 2;
    v[2] = 3;
    v[3] = 4;
    let v = xrealloc(v, 8);
    assert_eq!(v[0], 1);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 3);
    assert_eq!(v[3], 4);
    assert_eq!(v[4], 0);
}

fn main() {}
