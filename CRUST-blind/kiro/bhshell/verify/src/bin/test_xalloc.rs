use bhshell::xalloc;

#[test]
fn test_xmalloc_returns_zeroed_vec() {
    let v = xalloc::xmalloc(100);
    assert_eq!(v.len(), 100);
    assert!(v.iter().all(|&b| b == 0));
}

#[test]
fn test_xmalloc_zero_size() {
    let v = xalloc::xmalloc(0);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_xrealloc_grow() {
    let v = xalloc::xmalloc(10);
    assert_eq!(v.len(), 10);
    let v2 = xalloc::xrealloc(v, 200);
    assert_eq!(v2.len(), 200);
    assert!(v2[..10].iter().all(|&b| b == 0));
}

#[test]
fn test_xrealloc_shrink() {
    let v = xalloc::xmalloc(100);
    let v2 = xalloc::xrealloc(v, 10);
    assert_eq!(v2.len(), 10);
}

fn main() {}
