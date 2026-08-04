#![allow(unused_imports)]
use carrays::carrays::{
    gca_calc_gcd, gca_capacity, gca_cycle_left, gca_cycle_right, gca_is_rsorted, gca_is_sorted,
    gca_max, gca_min, gca_reverse, gca_roundup32, gca_roundup64, gca_swapm,
};
use std::cmp::Ordering;

// ---- roundup32 / roundup64 ----

#[test]
fn test_roundup32_zero() {
    // C: gca_roundup32(0) == 0
    assert_eq!(gca_roundup32(0), 0);
}

#[test]
fn test_roundup64_zero() {
    assert_eq!(gca_roundup64(0), 0);
}

#[test]
fn test_roundup32_small_values() {
    assert_eq!(gca_roundup32(1), 1);
    assert_eq!(gca_roundup32(2), 2);
    assert_eq!(gca_roundup32(3), 4);
    assert_eq!(gca_roundup32(4), 4);
    assert_eq!(gca_roundup32(5), 8);
    assert_eq!(gca_roundup32(6), 8);
    assert_eq!(gca_roundup32(7), 8);
    assert_eq!(gca_roundup32(8), 8);
    assert_eq!(gca_roundup32(100), 128);
}

#[test]
fn test_roundup64_small_values() {
    assert_eq!(gca_roundup64(1), 1);
    assert_eq!(gca_roundup64(2), 2);
    assert_eq!(gca_roundup64(3), 4);
    assert_eq!(gca_roundup64(4), 4);
    assert_eq!(gca_roundup64(5), 8);
    assert_eq!(gca_roundup64(6), 8);
    assert_eq!(gca_roundup64(7), 8);
    assert_eq!(gca_roundup64(8), 8);
    assert_eq!(gca_roundup64(100), 128);
}

#[test]
fn test_roundup32_powers_of_two() {
    for i in 2u32..32 {
        assert_eq!(gca_roundup32((1u32 << i) - 1), 1u32 << i);
        assert_eq!(gca_roundup32(1u32 << i), 1u32 << i);
    }
}

#[test]
fn test_roundup64_powers_of_two() {
    for i in 2u64..64 {
        assert_eq!(gca_roundup64((1u64 << i) - 1), 1u64 << i);
        assert_eq!(gca_roundup64(1u64 << i), 1u64 << i);
    }
}

// ---- gca_calc_gcd ----

#[test]
fn test_gcd_zero() {
    assert_eq!(gca_calc_gcd(0, 0), 0);
    assert_eq!(gca_calc_gcd(10, 0), 10);
    assert_eq!(gca_calc_gcd(0, 10), 10);
}

#[test]
fn test_gcd_self_and_one() {
    for i in 0u32..100 {
        assert_eq!(gca_calc_gcd(i, i), i);
        assert_eq!(gca_calc_gcd(i, 0), i);
        assert_eq!(gca_calc_gcd(0, i), i);
        assert_eq!(gca_calc_gcd(i, 1), 1);
        assert_eq!(gca_calc_gcd(1, i), 1);
    }
}

#[test]
fn test_gcd_assorted() {
    assert_eq!(gca_calc_gcd(2, 4), 2);
    assert_eq!(gca_calc_gcd(4, 2), 2);
    assert_eq!(gca_calc_gcd(6, 9), 3);
    assert_eq!(gca_calc_gcd(9, 6), 3);
    assert_eq!(gca_calc_gcd(2, 2), 2);
    assert_eq!(gca_calc_gcd(1, 1), 1);
    assert_eq!(gca_calc_gcd(1, 2), 1);
    assert_eq!(gca_calc_gcd(1, 100), 1);
    assert_eq!(gca_calc_gcd(7, 5), 1);
    assert_eq!(gca_calc_gcd(18, 6), 6);
    assert_eq!(gca_calc_gcd(3, 6), 3);
    assert_eq!(gca_calc_gcd(100, 120), 20);
    assert_eq!(gca_calc_gcd(100, 125), 25);
}

// ---- gca_capacity ----

#[test]
fn test_capacity_no_grow() {
    // new_size <= size: shouldn't change ptr or size
    let mut buf: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut size: usize = 4; // 4 elements of size 2
    let es: usize = 2;
    let result = gca_capacity(&mut buf, &mut size, es, 4);
    assert!(result.is_some());
    assert_eq!(size, 4);
    assert_eq!(buf, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_capacity_grow_to_power_of_two() {
    // Initial: 0 elements of size 4 bytes (empty Vec)
    let mut buf: Vec<u8> = Vec::new();
    let mut size: usize = 0;
    let es: usize = 4;
    // Request capacity of 5 -> rounds up to 8
    let result = gca_capacity(&mut buf, &mut size, es, 5);
    assert!(result.is_some());
    assert_eq!(size, 8);
    // 8 * 4 = 32 bytes
    assert_eq!(buf.len(), 32);
    // New bytes should be 0
    for b in buf.iter() {
        assert_eq!(*b, 0);
    }
}

#[test]
fn test_capacity_grow_preserves_existing() {
    // Pre-existing data: size=2 elements of 2 bytes = 4 bytes
    let mut buf: Vec<u8> = vec![0xAA, 0xBB, 0xCC, 0xDD];
    let mut size: usize = 2;
    let es: usize = 2;
    let result = gca_capacity(&mut buf, &mut size, es, 3);
    assert!(result.is_some());
    assert_eq!(size, 4); // rounded up
    assert_eq!(buf.len(), 8); // 4 elements * 2 bytes
    // Existing bytes preserved
    assert_eq!(buf[0], 0xAA);
    assert_eq!(buf[1], 0xBB);
    assert_eq!(buf[2], 0xCC);
    assert_eq!(buf[3], 0xDD);
    // New bytes zero
    assert_eq!(buf[4], 0);
    assert_eq!(buf[5], 0);
    assert_eq!(buf[6], 0);
    assert_eq!(buf[7], 0);
}

// ---- gca_swapm ----

#[test]
fn test_swapm_equal_length() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [10u8, 20, 30, 40];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [10, 20, 30, 40]);
    assert_eq!(b, [1, 2, 3, 4]);
}

#[test]
fn test_swapm_single_byte() {
    let mut a = [42u8];
    let mut b = [99u8];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [99]);
    assert_eq!(b, [42]);
}

#[test]
fn test_swapm_eight_bytes() {
    let mut a: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut b: [u8; 8] = [9, 10, 11, 12, 13, 14, 15, 16];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [9, 10, 11, 12, 13, 14, 15, 16]);
    assert_eq!(b, [1, 2, 3, 4, 5, 6, 7, 8]);
}

// ---- gca_cycle_left / gca_cycle_right ----

fn usize_arr_to_bytes(arr: &[usize]) -> Vec<u8> {
    let mut out = Vec::with_capacity(arr.len() * std::mem::size_of::<usize>());
    for v in arr {
        out.extend_from_slice(&v.to_ne_bytes());
    }
    out
}

fn bytes_to_usize_arr(bytes: &[u8]) -> Vec<usize> {
    bytes
        .chunks(std::mem::size_of::<usize>())
        .map(|chunk| {
            let mut buf = [0u8; std::mem::size_of::<usize>()];
            buf.copy_from_slice(chunk);
            usize::from_ne_bytes(buf)
        })
        .collect()
}

#[test]
fn test_cycle_left_zero_shift_no_change() {
    let n = 5usize;
    let arr: Vec<usize> = (0..n).collect();
    let mut bytes = usize_arr_to_bytes(&arr);
    let es = std::mem::size_of::<usize>();
    gca_cycle_left(&mut bytes, n, es, 0);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, arr);
}

#[test]
fn test_cycle_left_shift_n_no_change() {
    let n = 5usize;
    let arr: Vec<usize> = (0..n).collect();
    let mut bytes = usize_arr_to_bytes(&arr);
    let es = std::mem::size_of::<usize>();
    gca_cycle_left(&mut bytes, n, es, n);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, arr);
}

#[test]
fn test_cycle_left_shift_2n_no_change() {
    let n = 7usize;
    let arr: Vec<usize> = (0..n).collect();
    let mut bytes = usize_arr_to_bytes(&arr);
    let es = std::mem::size_of::<usize>();
    gca_cycle_left(&mut bytes, n, es, 2 * n);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, arr);
}

#[test]
fn test_cycle_left_each_shift() {
    // From C test: arr[i] == ((i+shift) % n) after gca_cycle_left
    let n = 10usize;
    let es = std::mem::size_of::<usize>();
    for shift in 0..n {
        let initial: Vec<usize> = (0..n).collect();
        let mut bytes = usize_arr_to_bytes(&initial);
        gca_cycle_left(&mut bytes, n, es, shift);
        let result = bytes_to_usize_arr(&bytes);
        for i in 0..n {
            assert_eq!(
                result[i],
                (i + shift) % n,
                "shift={} i={} got {} expected {}",
                shift,
                i,
                result[i],
                (i + shift) % n
            );
        }
    }
}

#[test]
fn test_cycle_left_each_shift_various_n() {
    // C test runs n from 0 to 99
    let es = std::mem::size_of::<usize>();
    for n in 0usize..30 {
        for shift in 0..=n.max(1) {
            let initial: Vec<usize> = (0..n).collect();
            let mut bytes = usize_arr_to_bytes(&initial);
            gca_cycle_left(&mut bytes, n, es, shift);
            let result = bytes_to_usize_arr(&bytes);
            for i in 0..n {
                let expected = if n > 0 { (i + shift) % n } else { 0 };
                assert_eq!(result[i], expected, "n={} shift={} i={}", n, shift, i);
            }
        }
    }
}

#[test]
fn test_cycle_right_zero_shift_no_change() {
    let n = 5usize;
    let arr: Vec<usize> = (0..n).collect();
    let mut bytes = usize_arr_to_bytes(&arr);
    let es = std::mem::size_of::<usize>();
    gca_cycle_right(&mut bytes, n, es, 0);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, arr);
}

#[test]
fn test_cycle_right_shift_n_no_change() {
    let n = 5usize;
    let arr: Vec<usize> = (0..n).collect();
    let mut bytes = usize_arr_to_bytes(&arr);
    let es = std::mem::size_of::<usize>();
    gca_cycle_right(&mut bytes, n, es, n);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, arr);
}

#[test]
fn test_cycle_left_then_right_inverse() {
    // From C test: cycle_left(shift) then cycle_right(shift) restores
    let es = std::mem::size_of::<usize>();
    for n in 1usize..20 {
        for shift in 0..n {
            let initial: Vec<usize> = (0..n).collect();
            let mut bytes = usize_arr_to_bytes(&initial);
            gca_cycle_left(&mut bytes, n, es, shift);
            gca_cycle_right(&mut bytes, n, es, shift);
            let result = bytes_to_usize_arr(&bytes);
            assert_eq!(result, initial, "n={} shift={}", n, shift);
        }
    }
}

#[test]
fn test_cycle_left_n_zero_no_op() {
    let mut bytes: Vec<u8> = vec![];
    gca_cycle_left(&mut bytes, 0, 8, 5);
    assert_eq!(bytes, Vec::<u8>::new());
}

#[test]
fn test_cycle_left_n_one_no_op() {
    let initial = vec![42usize];
    let mut bytes = usize_arr_to_bytes(&initial);
    gca_cycle_left(&mut bytes, 1, std::mem::size_of::<usize>(), 5);
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, initial);
}

#[test]
fn test_cycle_right_n_zero_no_op() {
    let mut bytes: Vec<u8> = vec![];
    gca_cycle_right(&mut bytes, 0, 8, 5);
    assert_eq!(bytes, Vec::<u8>::new());
}

// ---- gca_reverse ----

#[test]
fn test_reverse_empty() {
    let mut bytes: Vec<u8> = vec![];
    gca_reverse(&mut bytes, 0, 8);
    assert_eq!(bytes, Vec::<u8>::new());
}

#[test]
fn test_reverse_single() {
    let initial = vec![42usize];
    let mut bytes = usize_arr_to_bytes(&initial);
    gca_reverse(&mut bytes, 1, std::mem::size_of::<usize>());
    let result = bytes_to_usize_arr(&bytes);
    assert_eq!(result, vec![42usize]);
}

#[test]
fn test_reverse_various_sizes() {
    let es = std::mem::size_of::<usize>();
    for n in 0..=20 {
        let initial: Vec<usize> = (0..n).collect();
        let mut bytes = usize_arr_to_bytes(&initial);
        gca_reverse(&mut bytes, n, es);
        let result = bytes_to_usize_arr(&bytes);
        for i in 0..n {
            assert_eq!(result[i], n - i - 1, "n={} i={}", n, i);
        }
    }
}

#[test]
fn test_reverse_es_zero_no_op() {
    let mut bytes: Vec<u8> = vec![1, 2, 3, 4];
    gca_reverse(&mut bytes, 4, 0);
    assert_eq!(bytes, vec![1, 2, 3, 4]);
}

#[test]
fn test_reverse_one_byte() {
    let mut bytes: Vec<u8> = vec![1, 2, 3, 4, 5];
    gca_reverse(&mut bytes, 5, 1);
    assert_eq!(bytes, vec![5, 4, 3, 2, 1]);
}

// ---- gca_is_sorted / gca_is_rsorted ----

fn cmp_int(a: &i32, b: &i32) -> Ordering {
    a.cmp(b)
}

#[test]
fn test_is_sorted_empty() {
    let arr: [i32; 0] = [];
    assert!(gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_sorted_single() {
    let arr = [42i32];
    assert!(gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_sorted_ascending() {
    let arr = [1i32, 2, 3, 4, 5];
    assert!(gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_sorted_with_dupes() {
    let arr = [1i32, 2, 2, 3, 4];
    assert!(gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_sorted_unsorted() {
    let arr = [1i32, 3, 2, 4];
    assert!(!gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_sorted_descending() {
    let arr = [5i32, 4, 3, 2, 1];
    assert!(!gca_is_sorted(&arr, cmp_int));
}

#[test]
fn test_is_rsorted_empty() {
    let arr: [i32; 0] = [];
    assert!(gca_is_rsorted(&arr, cmp_int));
}

#[test]
fn test_is_rsorted_descending() {
    let arr = [5i32, 4, 3, 2, 1];
    assert!(gca_is_rsorted(&arr, cmp_int));
}

#[test]
fn test_is_rsorted_with_dupes() {
    let arr = [5i32, 4, 4, 3, 2];
    assert!(gca_is_rsorted(&arr, cmp_int));
}

#[test]
fn test_is_rsorted_ascending() {
    let arr = [1i32, 2, 3, 4, 5];
    assert!(!gca_is_rsorted(&arr, cmp_int));
}

// ---- gca_max / gca_min ----

#[test]
fn test_max_empty() {
    let arr: [i32; 0] = [];
    assert!(gca_max(&arr, cmp_int).is_none());
}

#[test]
fn test_max_single() {
    let arr = [42i32];
    let m = gca_max(&arr, cmp_int).unwrap();
    assert_eq!(*m, 42);
}

#[test]
fn test_max_assorted() {
    let arr = [3i32, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    let m = gca_max(&arr, cmp_int).unwrap();
    assert_eq!(*m, 9);
}

#[test]
fn test_max_returns_first_of_ties() {
    // C's gca_max iterates with strict < (compar(max, ptr) < 0). So first occurrence
    // of the max value wins.
    let arr = [1i32, 5, 5, 3];
    let m = gca_max(&arr, cmp_int).unwrap();
    assert_eq!(*m, 5);
    // Pointer-equality wise, returns first 5
    assert!(std::ptr::eq(m, &arr[1]));
}

#[test]
fn test_min_empty() {
    let arr: [i32; 0] = [];
    assert!(gca_min(&arr, cmp_int).is_none());
}

#[test]
fn test_min_single() {
    let arr = [42i32];
    let m = gca_min(&arr, cmp_int).unwrap();
    assert_eq!(*m, 42);
}

#[test]
fn test_min_assorted() {
    let arr = [3i32, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5];
    let m = gca_min(&arr, cmp_int).unwrap();
    assert_eq!(*m, 1);
}

#[test]
fn test_min_returns_first_of_ties() {
    let arr = [3i32, 1, 1, 5];
    let m = gca_min(&arr, cmp_int).unwrap();
    assert_eq!(*m, 1);
    assert!(std::ptr::eq(m, &arr[1]));
}

fn main() {}
