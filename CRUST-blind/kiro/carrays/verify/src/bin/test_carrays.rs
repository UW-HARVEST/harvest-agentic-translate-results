use carrays::carrays::*;
use std::cmp::Ordering;

fn cmp_u8(a: &u8, b: &u8) -> Ordering { a.cmp(b) }
fn cmp_i32(a: &i32, b: &i32) -> Ordering { a.cmp(b) }

// --- roundup32 / roundup64 ---

#[test]
fn test_roundup_basic() {
    assert_eq!(gca_roundup32(0), 0);
    assert_eq!(gca_roundup64(0), 0);
    assert_eq!(gca_roundup32(1), 1);
    assert_eq!(gca_roundup64(1), 1);
    assert_eq!(gca_roundup32(2), 2);
    assert_eq!(gca_roundup64(2), 2);
    assert_eq!(gca_roundup32(3), 4);
    assert_eq!(gca_roundup64(3), 4);
    assert_eq!(gca_roundup32(4), 4);
    assert_eq!(gca_roundup64(4), 4);
    assert_eq!(gca_roundup32(5), 8);
    assert_eq!(gca_roundup64(5), 8);
    assert_eq!(gca_roundup32(7), 8);
    assert_eq!(gca_roundup64(7), 8);
    assert_eq!(gca_roundup32(8), 8);
    assert_eq!(gca_roundup64(8), 8);
    assert_eq!(gca_roundup32(100), 128);
    assert_eq!(gca_roundup64(100), 128);
}

#[test]
fn test_roundup32_powers() {
    for i in 2..32u32 {
        let p = 1u32 << i;
        assert_eq!(gca_roundup32(p - 1), p);
        assert_eq!(gca_roundup32(p), p);
    }
}

#[test]
fn test_roundup64_powers() {
    for i in 2..64u32 {
        let p = 1u64 << i;
        assert_eq!(gca_roundup64(p - 1), p);
        assert_eq!(gca_roundup64(p), p);
    }
}

// --- gca_calc_gcd ---

#[test]
fn test_gcd_identity_and_zero() {
    for i in 0..100u32 {
        assert_eq!(gca_calc_gcd(i, i), i);
        assert_eq!(gca_calc_gcd(i, 0), i);
        assert_eq!(gca_calc_gcd(0, i), i);
    }
    assert_eq!(gca_calc_gcd(0, 0), 0);
}

#[test]
fn test_gcd_with_one() {
    for i in 1..100u32 {
        assert_eq!(gca_calc_gcd(i, 1), 1);
        assert_eq!(gca_calc_gcd(1, i), 1);
    }
}

#[test]
fn test_gcd_known_values() {
    assert_eq!(gca_calc_gcd(2, 4), 2);
    assert_eq!(gca_calc_gcd(4, 2), 2);
    assert_eq!(gca_calc_gcd(6, 9), 3);
    assert_eq!(gca_calc_gcd(9, 6), 3);
    assert_eq!(gca_calc_gcd(7, 5), 1);
    assert_eq!(gca_calc_gcd(18, 6), 6);
    assert_eq!(gca_calc_gcd(3, 6), 3);
    assert_eq!(gca_calc_gcd(100, 120), 20);
    assert_eq!(gca_calc_gcd(100, 125), 25);
}

// --- gca_capacity ---

#[test]
fn test_capacity_grow() {
    let mut v: Vec<u8> = Vec::new();
    let mut size = 0usize;
    let result = gca_capacity(&mut v, &mut size, 1, 10);
    assert!(result.is_some());
    assert!(size >= 10);
    // size should be power of two
    assert_eq!(size & (size - 1), 0);
    assert_eq!(size, 16);
    assert_eq!(v.len(), 16);
}

#[test]
fn test_capacity_no_shrink() {
    let mut v: Vec<u8> = vec![0; 32];
    let mut size = 32usize;
    let result = gca_capacity(&mut v, &mut size, 1, 10);
    assert!(result.is_some());
    assert_eq!(size, 32); // unchanged
}

// --- gca_swapm ---

#[test]
fn test_swapm() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [5u8, 6, 7, 8];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [5, 6, 7, 8]);
    assert_eq!(b, [1, 2, 3, 4]);
}

#[test]
fn test_swapm_single() {
    let mut a = [42u8];
    let mut b = [99u8];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [99]);
    assert_eq!(b, [42]);
}

// --- gca_cycle_left / gca_cycle_right ---

// Helper: convert a &[usize] to byte slice and back for cycle operations
fn cycle_left_usize(arr: &mut [usize], shift: usize) {
    let n = arr.len();
    let es = std::mem::size_of::<usize>();
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(arr.as_mut_ptr() as *mut u8, n * es)
    };
    gca_cycle_left(bytes, n, es, shift);
}

fn cycle_right_usize(arr: &mut [usize], shift: usize) {
    let n = arr.len();
    let es = std::mem::size_of::<usize>();
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(arr.as_mut_ptr() as *mut u8, n * es)
    };
    gca_cycle_right(bytes, n, es, shift);
}

#[test]
fn test_cycle_left_identity() {
    for n in 0..20usize {
        let mut arr: Vec<usize> = (0..n).collect();
        cycle_left_usize(&mut arr, 0);
        for i in 0..n { assert_eq!(arr[i], i); }
        cycle_left_usize(&mut arr, n);
        for i in 0..n { assert_eq!(arr[i], i); }
        cycle_left_usize(&mut arr, 2 * n);
        for i in 0..n { assert_eq!(arr[i], i); }
    }
}

#[test]
fn test_cycle_right_identity() {
    for n in 0..20usize {
        let mut arr: Vec<usize> = (0..n).collect();
        cycle_right_usize(&mut arr, 0);
        for i in 0..n { assert_eq!(arr[i], i); }
        cycle_right_usize(&mut arr, n);
        for i in 0..n { assert_eq!(arr[i], i); }
        cycle_right_usize(&mut arr, 2 * n);
        for i in 0..n { assert_eq!(arr[i], i); }
    }
}

#[test]
fn test_cycle_left_all_shifts() {
    for n in 2..30usize {
        for shift in 0..n {
            let mut arr: Vec<usize> = (0..n).collect();
            cycle_left_usize(&mut arr, shift);
            for i in 0..n {
                assert_eq!(arr[i], (i + shift) % n);
            }
            // shift back
            cycle_right_usize(&mut arr, shift);
            for i in 0..n { assert_eq!(arr[i], i); }
        }
    }
}

// --- gca_reverse ---

fn reverse_usize(arr: &mut [usize]) {
    let n = arr.len();
    let es = std::mem::size_of::<usize>();
    let bytes: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(arr.as_mut_ptr() as *mut u8, n * es)
    };
    gca_reverse(bytes, n, es);
}

#[test]
fn test_reverse() {
    for n in 0..=30usize {
        let mut arr: Vec<usize> = (0..n).collect();
        reverse_usize(&mut arr);
        for i in 0..n {
            assert_eq!(arr[i], n - i - 1);
        }
    }
}

#[test]
fn test_reverse_double() {
    let mut arr: Vec<usize> = (0..10).collect();
    reverse_usize(&mut arr);
    reverse_usize(&mut arr);
    for i in 0..10 { assert_eq!(arr[i], i); }
}

// --- gca_is_sorted / gca_is_rsorted ---

#[test]
fn test_is_sorted() {
    let sorted = vec![1, 2, 3, 4, 5];
    assert!(gca_is_sorted(&sorted, cmp_i32));

    let unsorted = vec![1, 3, 2, 4, 5];
    assert!(!gca_is_sorted(&unsorted, cmp_i32));

    let empty: Vec<i32> = vec![];
    assert!(gca_is_sorted(&empty, cmp_i32));

    let single = vec![42];
    assert!(gca_is_sorted(&single, cmp_i32));

    // equal elements
    let equal = vec![5, 5, 5, 5];
    assert!(gca_is_sorted(&equal, cmp_i32));
}

#[test]
fn test_is_rsorted() {
    let rsorted = vec![5, 4, 3, 2, 1];
    assert!(gca_is_rsorted(&rsorted, cmp_i32));

    let not_rsorted = vec![5, 3, 4, 2, 1];
    assert!(!gca_is_rsorted(&not_rsorted, cmp_i32));

    let empty: Vec<i32> = vec![];
    assert!(gca_is_rsorted(&empty, cmp_i32));

    let single = vec![42];
    assert!(gca_is_rsorted(&single, cmp_i32));

    let equal = vec![5, 5, 5, 5];
    assert!(gca_is_rsorted(&equal, cmp_i32));
}

// --- gca_max / gca_min ---

#[test]
fn test_max() {
    let arr = vec![3, 1, 4, 1, 5, 9, 2, 6];
    assert_eq!(gca_max(&arr, cmp_i32), Some(&9));

    let single = vec![42];
    assert_eq!(gca_max(&single, cmp_i32), Some(&42));

    let empty: Vec<i32> = vec![];
    assert_eq!(gca_max(&empty, cmp_i32), None);
}

#[test]
fn test_min() {
    let arr = vec![3, 1, 4, 1, 5, 9, 2, 6];
    assert_eq!(gca_min(&arr, cmp_i32), Some(&1));

    let single = vec![42];
    assert_eq!(gca_min(&single, cmp_i32), Some(&42));

    let empty: Vec<i32> = vec![];
    assert_eq!(gca_min(&empty, cmp_i32), None);
}

#[test]
fn test_max_first_occurrence() {
    // C gca_max returns first max (uses < not <=)
    let arr = vec![5u8, 3, 5, 1];
    let result = gca_max(&arr, cmp_u8).unwrap();
    assert!(std::ptr::eq(result, &arr[0]));
}

#[test]
fn test_min_first_occurrence() {
    // C gca_min returns first min (uses > not >=)
    let arr = vec![3u8, 1, 5, 1];
    let result = gca_min(&arr, cmp_u8).unwrap();
    assert!(std::ptr::eq(result, &arr[1]));
}

fn main() {}
