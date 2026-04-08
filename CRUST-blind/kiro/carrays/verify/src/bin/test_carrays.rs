use carrays::carrays::*;
use std::cmp::Ordering;

fn cmp_i32(a: &i32, b: &i32) -> Ordering { a.cmp(b) }

#[test]
fn test_roundup32() {
    let cases: &[(u32, u32)] = &[
        (0, 0), (1, 1), (2, 2), (3, 4), (4, 4), (5, 8), (6, 8), (7, 8),
        (8, 8), (9, 16), (15, 16), (16, 16), (17, 32), (100, 128),
        (255, 256), (256, 256), (257, 512),
    ];
    for &(input, expected) in cases {
        assert_eq!(gca_roundup32(input), expected, "roundup32({input})");
    }
    // Powers of two boundaries
    for i in 2..32u32 {
        let v = 1u32 << i;
        assert_eq!(gca_roundup32(v - 1), v);
        assert_eq!(gca_roundup32(v), v);
    }
}

#[test]
fn test_roundup64() {
    let cases: &[(u64, u64)] = &[
        (0, 0), (1, 1), (2, 2), (3, 4), (4, 4), (5, 8), (100, 128),
        (255, 256), (256, 256), (257, 512),
        (4294967295, 4294967296), (4294967296, 4294967296), (4294967297, 8589934592),
    ];
    for &(input, expected) in cases {
        assert_eq!(gca_roundup64(input), expected, "roundup64({input})");
    }
    for i in 2..64u64 {
        let v = 1u64 << i;
        assert_eq!(gca_roundup64(v - 1), v);
        assert_eq!(gca_roundup64(v), v);
    }
}

#[test]
fn test_calc_gcd() {
    let cases: &[(u32, u32, u32)] = &[
        (0, 0, 0), (1, 0, 1), (0, 1, 1), (1, 1, 1), (2, 4, 2), (4, 2, 2),
        (6, 9, 3), (9, 6, 3), (10, 0, 10), (0, 10, 10), (100, 120, 20),
        (100, 125, 25), (18, 6, 6), (3, 6, 3), (7, 5, 1), (12, 8, 4), (48, 36, 12),
    ];
    for &(a, b, expected) in cases {
        assert_eq!(gca_calc_gcd(a, b), expected, "gcd({a},{b})");
    }
    // gcd(i,i)==i, gcd(i,0)==i, gcd(0,i)==i, gcd(i,1)==1, gcd(1,i)==1
    for i in 0u32..100 {
        assert_eq!(gca_calc_gcd(i, i), i);
        assert_eq!(gca_calc_gcd(i, 0), i);
        assert_eq!(gca_calc_gcd(0, i), i);
        assert_eq!(gca_calc_gcd(i, 1), 1);
        assert_eq!(gca_calc_gcd(1, i), 1);
    }
}

fn to_bytes(arr: &[usize]) -> Vec<u8> {
    arr.iter().flat_map(|v| v.to_ne_bytes()).collect()
}

fn from_bytes(buf: &[u8]) -> Vec<usize> {
    buf.chunks_exact(std::mem::size_of::<usize>())
        .map(|c| usize::from_ne_bytes(c.try_into().unwrap()))
        .collect()
}

#[test]
fn test_cycle_left() {
    let es = std::mem::size_of::<usize>();

    // shift=2 on [0,1,2,3,4] => [2,3,4,0,1]
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_left(&mut buf, 5, es, 2);
    assert_eq!(from_bytes(&buf), vec![2, 3, 4, 0, 1]);

    // shift=3 on [0..5] => [3,4,5,0,1,2]
    let mut buf = to_bytes(&[0, 1, 2, 3, 4, 5]);
    gca_cycle_left(&mut buf, 6, es, 3);
    assert_eq!(from_bytes(&buf), vec![3, 4, 5, 0, 1, 2]);

    // shift=0 => no change
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_left(&mut buf, 5, es, 0);
    assert_eq!(from_bytes(&buf), vec![0, 1, 2, 3, 4]);

    // shift=n => no change
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_left(&mut buf, 5, es, 5);
    assert_eq!(from_bytes(&buf), vec![0, 1, 2, 3, 4]);

    // shift=2n => no change
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_left(&mut buf, 5, es, 10);
    assert_eq!(from_bytes(&buf), vec![0, 1, 2, 3, 4]);

    // All shifts for n=5
    for shift in 0..5usize {
        let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
        gca_cycle_left(&mut buf, 5, es, shift);
        let result = from_bytes(&buf);
        for i in 0..5 {
            assert_eq!(result[i], (i + shift) % 5);
        }
    }
}

#[test]
fn test_cycle_right() {
    let es = std::mem::size_of::<usize>();

    // shift=2 on [0,1,2,3,4] => [3,4,0,1,2]
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_right(&mut buf, 5, es, 2);
    assert_eq!(from_bytes(&buf), vec![3, 4, 0, 1, 2]);

    // shift=0 => no change
    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_cycle_right(&mut buf, 5, es, 0);
    assert_eq!(from_bytes(&buf), vec![0, 1, 2, 3, 4]);

    // cycle_right then cycle_left cancels out
    for shift in 0..5usize {
        let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
        gca_cycle_left(&mut buf, 5, es, shift);
        gca_cycle_right(&mut buf, 5, es, shift);
        assert_eq!(from_bytes(&buf), vec![0, 1, 2, 3, 4]);
    }
}

#[test]
fn test_reverse() {
    let es = std::mem::size_of::<usize>();

    let mut buf = to_bytes(&[0, 1, 2, 3, 4]);
    gca_reverse(&mut buf, 5, es);
    assert_eq!(from_bytes(&buf), vec![4, 3, 2, 1, 0]);

    let mut buf = to_bytes(&[0, 1, 2, 3, 4, 5]);
    gca_reverse(&mut buf, 6, es);
    assert_eq!(from_bytes(&buf), vec![5, 4, 3, 2, 1, 0]);

    // n<=1 => no change
    let mut buf = to_bytes(&[42]);
    gca_reverse(&mut buf, 1, es);
    assert_eq!(from_bytes(&buf), vec![42]);

    // Double reverse = identity
    for n in 0..20usize {
        let orig: Vec<usize> = (0..n).collect();
        let mut buf = to_bytes(&orig);
        gca_reverse(&mut buf, n, es);
        gca_reverse(&mut buf, n, es);
        assert_eq!(from_bytes(&buf), orig);
    }
}

#[test]
fn test_swapm() {
    let mut a = [1u8, 2, 3];
    let mut b = [4u8, 5, 6];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [4, 5, 6]);
    assert_eq!(b, [1, 2, 3]);
}

#[test]
fn test_is_sorted() {
    assert!(gca_is_sorted(&[1i32, 2, 3, 4, 5], cmp_i32));
    assert!(!gca_is_sorted(&[1i32, 3, 2, 4, 5], cmp_i32));
    assert!(!gca_is_sorted(&[5i32, 4, 3, 2, 1], cmp_i32));
    // empty and single
    assert!(gca_is_sorted(&[] as &[i32], cmp_i32));
    assert!(gca_is_sorted(&[42i32], cmp_i32));
    // equal elements
    assert!(gca_is_sorted(&[3i32, 3, 3], cmp_i32));
}

#[test]
fn test_is_rsorted() {
    assert!(gca_is_rsorted(&[5i32, 4, 3, 2, 1], cmp_i32));
    assert!(!gca_is_rsorted(&[1i32, 2, 3, 4, 5], cmp_i32));
    assert!(!gca_is_rsorted(&[1i32, 3, 2, 4, 5], cmp_i32));
    assert!(gca_is_rsorted(&[] as &[i32], cmp_i32));
    assert!(gca_is_rsorted(&[42i32], cmp_i32));
    assert!(gca_is_rsorted(&[3i32, 3, 3], cmp_i32));
}

#[test]
fn test_max() {
    let arr = [3i32, 1, 4, 1, 5, 9, 2, 6];
    assert_eq!(*gca_max(&arr, cmp_i32).unwrap(), 9);
    assert_eq!(*gca_max(&[42i32], cmp_i32).unwrap(), 42);
}

#[test]
fn test_min() {
    let arr = [3i32, 1, 4, 1, 5, 9, 2, 6];
    assert_eq!(*gca_min(&arr, cmp_i32).unwrap(), 1);
    assert_eq!(*gca_min(&[42i32], cmp_i32).unwrap(), 42);
}

#[test]
fn test_capacity() {
    let mut buf: Vec<u8> = vec![1, 2, 3, 4];
    let mut size = 1usize; // 1 element of 4 bytes
    let result = gca_capacity(&mut buf, &mut size, 4, 10);
    assert!(result.is_some());
    assert_eq!(size, 16); // roundup64(10) = 16
    assert_eq!(buf[0], 1);
    assert_eq!(buf[1], 2);
    assert_eq!(buf[2], 3);
    assert_eq!(buf[3], 4);

    // No resize when new_size <= size
    let mut buf2: Vec<u8> = vec![0; 16];
    let mut size2 = 16usize;
    gca_capacity(&mut buf2, &mut size2, 1, 10);
    assert_eq!(size2, 16);
}

fn main() {}
