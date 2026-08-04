use bhshell::xalloc;

#[test]
fn test_xmalloc_zero() {
    let v = xalloc::xmalloc(0);
    assert_eq!(v.len(), 0);
    assert!(v.is_empty());
}

#[test]
fn test_xmalloc_size_1() {
    let v = xalloc::xmalloc(1);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0], 0u8);
}

#[test]
fn test_xmalloc_size_16() {
    let v = xalloc::xmalloc(16);
    assert_eq!(v.len(), 16);
    for b in &v {
        assert_eq!(*b, 0u8);
    }
}

#[test]
fn test_xmalloc_size_4096() {
    let v = xalloc::xmalloc(4096);
    assert_eq!(v.len(), 4096);
    for b in &v {
        assert_eq!(*b, 0u8);
    }
}

#[test]
fn test_xrealloc_grow_from_empty() {
    let v = xalloc::xrealloc(Vec::new(), 8);
    assert_eq!(v.len(), 8);
    for b in &v {
        assert_eq!(*b, 0u8);
    }
}

#[test]
fn test_xrealloc_grow_keeps_existing_bytes() {
    let mut data = vec![1u8, 2, 3, 4];
    data = xalloc::xrealloc(data, 8);
    assert_eq!(data.len(), 8);
    assert_eq!(data[0], 1);
    assert_eq!(data[1], 2);
    assert_eq!(data[2], 3);
    assert_eq!(data[3], 4);
    assert_eq!(data[4], 0);
    assert_eq!(data[5], 0);
    assert_eq!(data[6], 0);
    assert_eq!(data[7], 0);
}

#[test]
fn test_xrealloc_shrink() {
    let data: Vec<u8> = (0u8..32u8).collect();
    let shrunk = xalloc::xrealloc(data, 4);
    assert_eq!(shrunk.len(), 4);
    assert_eq!(shrunk[0], 0);
    assert_eq!(shrunk[1], 1);
    assert_eq!(shrunk[2], 2);
    assert_eq!(shrunk[3], 3);
}

#[test]
fn test_xrealloc_to_zero() {
    let data = vec![5u8, 6, 7];
    let shrunk = xalloc::xrealloc(data, 0);
    assert_eq!(shrunk.len(), 0);
    assert!(shrunk.is_empty());
}

#[test]
fn test_xmalloc_independent_allocations() {
    let mut a = xalloc::xmalloc(4);
    let b = xalloc::xmalloc(4);
    a[0] = 7u8;
    assert_eq!(a[0], 7);
    assert_eq!(b[0], 0);
}

fn main() {}
