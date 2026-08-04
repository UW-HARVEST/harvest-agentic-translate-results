use hamta::hamta::*;

#[test]
fn test_hamt_int_hash_zero() {
    let mut v: i32 = 0;
    assert_eq!(hamt_int_hash(&mut v), 2647528437);
}

#[test]
fn test_hamt_int_hash_one() {
    let mut v: i32 = 1;
    assert_eq!(hamt_int_hash(&mut v), 2565215562);
}

#[test]
fn test_hamt_int_hash_two() {
    let mut v: i32 = 2;
    assert_eq!(hamt_int_hash(&mut v), 2482902687);
}

#[test]
fn test_hamt_int_hash_three() {
    let mut v: i32 = 3;
    assert_eq!(hamt_int_hash(&mut v), 2400589812);
}

#[test]
fn test_hamt_int_hash_seven() {
    let mut v: i32 = 7;
    assert_eq!(hamt_int_hash(&mut v), 2071338312);
}

#[test]
fn test_hamt_int_hash_42() {
    let mut v: i32 = 42;
    assert_eq!(hamt_int_hash(&mut v), 163444391);
}

#[test]
fn test_hamt_int_hash_100() {
    let mut v: i32 = 100;
    assert_eq!(hamt_int_hash(&mut v), 3979232233);
}

#[test]
fn test_hamt_int_hash_negative_one() {
    let mut v: i32 = -1;
    assert_eq!(hamt_int_hash(&mut v), 2729652521);
}

#[test]
fn test_hamt_int_hash_1234567() {
    let mut v: i32 = 1234567;
    assert_eq!(hamt_int_hash(&mut v), 2410359928);
}

#[test]
fn test_hamt_int_hash_large() {
    let mut v: i32 = 0x12345678;
    assert_eq!(hamt_int_hash(&mut v), 2323536117);
}

#[test]
fn test_hamt_str_hash_empty() {
    // Need a NUL-terminated string in memory, addressed via the pointer.
    let bytes: [u8; 1] = [0];
    let mut buf = bytes;
    assert_eq!(hamt_str_hash(&mut buf), 2216829733);
}

#[test]
fn test_hamt_str_hash_a() {
    let mut buf: [u8; 2] = [b'a', 0];
    assert_eq!(hamt_str_hash(&mut buf), 2248259518);
}

#[test]
fn test_hamt_str_hash_hello() {
    let mut buf: [u8; 6] = [b'h', b'e', b'l', b'l', b'o', 0];
    assert_eq!(hamt_str_hash(&mut buf), 3183334599);
}

#[test]
fn test_hamt_str_hash_world() {
    let mut buf: [u8; 6] = [b'w', b'o', b'r', b'l', b'd', 0];
    assert_eq!(hamt_str_hash(&mut buf), 3299234831);
}

#[test]
fn test_hamt_str_hash_aut() {
    let mut buf: [u8; 4] = [b'a', b'u', b't', 0];
    assert_eq!(hamt_str_hash(&mut buf), 1806671401);
}

#[test]
fn test_hamt_str_hash_bus() {
    let mut buf: [u8; 4] = [b'b', b'u', b's', 0];
    assert_eq!(hamt_str_hash(&mut buf), 1806519589);
}

#[test]
fn test_hamt_str_hash_banan() {
    let mut buf: [u8; 6] = [b'b', b'a', b'n', b'a', b'n', 0];
    assert_eq!(hamt_str_hash(&mut buf), 1227426209);
}

#[test]
fn test_hamt_str_hash_kokos() {
    let mut buf: [u8; 6] = [b'k', b'o', b'k', b'o', b's', 0];
    assert_eq!(hamt_str_hash(&mut buf), 3779452536);
}

#[test]
fn test_hamt_str_hash_bubakov() {
    let mut buf: [u8; 8] = [b'b', b'u', b'b', b'a', b'k', b'o', b'v', 0];
    assert_eq!(hamt_str_hash(&mut buf), 502409863);
}

#[test]
fn test_hamt_str_hash_high_byte_ff() {
    let mut buf: [u8; 2] = [0xff, 0];
    assert_eq!(hamt_str_hash(&mut buf), 2046707744);
}

#[test]
fn test_hamt_str_hash_high_byte_80() {
    let mut buf: [u8; 2] = [0x80, 0];
    assert_eq!(hamt_str_hash(&mut buf), 2046707807);
}

#[test]
fn test_hamt_int_equals_same() {
    let mut a: i32 = 1;
    let mut b: i32 = 1;
    assert!(hamt_int_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_int_equals_different() {
    let mut a: i32 = 1;
    let mut b: i32 = 2;
    assert!(!hamt_int_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_int_equals_negative() {
    let mut a: i32 = -1;
    let mut b: i32 = -1;
    assert!(hamt_int_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_str_equals_same() {
    let mut a: [u8; 3] = [b'a', b'a', 0];
    let mut b: [u8; 3] = [b'a', b'a', 0];
    assert!(hamt_str_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_str_equals_different() {
    let mut a: [u8; 3] = [b'a', b'a', 0];
    let mut b: [u8; 3] = [b'b', b'b', 0];
    assert!(!hamt_str_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_str_equals_prefix() {
    // a is "aa", b is "aab" — same first two chars but different terminator.
    let mut a: [u8; 4] = [b'a', b'a', 0, 0];
    let mut b: [u8; 4] = [b'a', b'a', b'b', 0];
    assert!(!hamt_str_equals(&mut a, &mut b));
}

#[test]
fn test_hamt_fnv1_hash_returns_unit() {
    // hamt_fnv1_hash signature is `fn(&mut T, usize)` (no return value).
    let mut v: i32 = 42;
    let _: () = hamt_fnv1_hash(&mut v, std::mem::size_of::<i32>());
}

#[test]
fn test_hamt_get_symbol_returns_unit() {
    // hamt_get_symbol signature is `fn(u32, i32)` (no return value).
    let _: () = hamt_get_symbol(0xdeadbeef, 0);
    let _: () = hamt_get_symbol(0xdeadbeef, 1);
    let _: () = hamt_get_symbol(0xdeadbeef, 2);
    let _: () = hamt_get_symbol(0, 0);
    let _: () = hamt_get_symbol(u32::MAX, 5);
}

#[test]
fn test_constants() {
    assert_eq!(FNV_BASE, 14695981039346656037u64);
    assert_eq!(FNV_PRIME, 1099511628211u64);
    assert_eq!(HAMT_NODE_T_FLAG, 1);
    assert_eq!(KEY_VALUE_T_FLAG, 0);
    assert_eq!(CHUNK_SIZE, 6);
}

fn main() {}
