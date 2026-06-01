use carrays::carrays::*;

#[test]
fn test_roundup32() {
    assert_eq!(gca_roundup32(0), 0);
    assert_eq!(gca_roundup32(1), 1);
    assert_eq!(gca_roundup32(2), 2);
    assert_eq!(gca_roundup32(3), 4);
    assert_eq!(gca_roundup32(4), 4);
    assert_eq!(gca_roundup32(5), 8);
    assert_eq!(gca_roundup32(6), 8);
    assert_eq!(gca_roundup32(7), 8);
    assert_eq!(gca_roundup32(8), 8);
    assert_eq!(gca_roundup32(100), 128);

    for i in 2u32..32 {
        assert_eq!(gca_roundup32((1u32 << i) - 1), 1u32 << i);
        assert_eq!(gca_roundup32(1u32 << i), 1u32 << i);
    }
}

#[test]
fn test_roundup64() {
    assert_eq!(gca_roundup64(0), 0);
    assert_eq!(gca_roundup64(1), 1);
    assert_eq!(gca_roundup64(2), 2);
    assert_eq!(gca_roundup64(3), 4);
    assert_eq!(gca_roundup64(4), 4);
    assert_eq!(gca_roundup64(5), 8);
    assert_eq!(gca_roundup64(8), 8);
    assert_eq!(gca_roundup64(100), 128);

    for i in 2u64..64 {
        assert_eq!(gca_roundup64((1u64 << i) - 1), 1u64 << i);
        assert_eq!(gca_roundup64(1u64 << i), 1u64 << i);
    }
}

#[test]
fn test_gcd() {
    for i in 0u32..100 {
        assert_eq!(gca_calc_gcd(i, i), i);
        assert_eq!(gca_calc_gcd(i, 0), i);
        assert_eq!(gca_calc_gcd(0, i), i);
        if i > 0 {
            assert_eq!(gca_calc_gcd(i, 1), 1);
            assert_eq!(gca_calc_gcd(1, i), 1);
        }
    }
    assert_eq!(gca_calc_gcd(2, 4), 2);
    assert_eq!(gca_calc_gcd(4, 2), 2);
    assert_eq!(gca_calc_gcd(6, 9), 3);
    assert_eq!(gca_calc_gcd(9, 6), 3);
    assert_eq!(gca_calc_gcd(0, 0), 0);
    assert_eq!(gca_calc_gcd(10, 0), 10);
    assert_eq!(gca_calc_gcd(0, 10), 10);
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

#[test]
fn test_swapm() {
    let mut a = [1u8, 2, 3, 4];
    let mut b = [9u8, 8, 7, 6];
    gca_swapm(&mut a, &mut b);
    assert_eq!(a, [9, 8, 7, 6]);
    assert_eq!(b, [1, 2, 3, 4]);
}

#[test]
fn test_capacity() {
    let mut v: Vec<u8> = vec![1, 2, 3, 4];
    let mut size: usize = 4;
    let _ = gca_capacity(&mut v, &mut size, 1, 5);
    // 5 -> roundup64(5) = 8, so size becomes 8
    assert_eq!(size, 8);
    assert_eq!(v.len(), 8);
    assert_eq!(&v[0..4], &[1, 2, 3, 4]);
    // new bytes initialized to zero
    assert_eq!(&v[4..8], &[0, 0, 0, 0]);

    // No-op if already big enough
    let mut v2: Vec<u8> = vec![5, 6, 7, 8];
    let mut size2: usize = 4;
    let _ = gca_capacity(&mut v2, &mut size2, 1, 4);
    assert_eq!(size2, 4);
    assert_eq!(v2, vec![5, 6, 7, 8]);
}

#[test]
fn test_reverse() {
    // n=0 - no change (empty)
    let mut empty: [u8; 0] = [];
    gca_reverse(&mut empty, 0, 8);

    // n=1 - no change
    let mut one = [42u8; 8];
    gca_reverse(&mut one, 1, 8);
    assert_eq!(one, [42u8; 8]);

    // Full test using usize-sized elements (8 bytes on 64-bit)
    let n = 5usize;
    let es = std::mem::size_of::<usize>();
    let mut data: Vec<u8> = Vec::new();
    for i in 0..n {
        let bytes = (i as u64).to_ne_bytes();
        data.extend_from_slice(&bytes[..es]);
    }
    gca_reverse(&mut data, n, es);
    for i in 0..n {
        let mut chunk = [0u8; 8];
        chunk[..es].copy_from_slice(&data[i * es..(i + 1) * es]);
        let val = u64::from_ne_bytes(chunk);
        assert_eq!(val as usize, n - i - 1);
    }

    // Test all sizes 0..=20
    for nn in 0..=20usize {
        let es = std::mem::size_of::<usize>();
        let mut buf: Vec<u8> = Vec::new();
        for i in 0..nn {
            buf.extend_from_slice(&(i as u64).to_ne_bytes()[..es]);
        }
        gca_reverse(&mut buf, nn, es);
        for i in 0..nn {
            let mut chunk = [0u8; 8];
            chunk[..es].copy_from_slice(&buf[i * es..(i + 1) * es]);
            let val = u64::from_ne_bytes(chunk) as usize;
            assert_eq!(val, nn - i - 1);
        }
    }
}

fn arr_to_bytes(a: &[u64], es: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for x in a {
        v.extend_from_slice(&x.to_ne_bytes()[..es]);
    }
    v
}

fn bytes_to_arr(v: &[u8], n: usize, es: usize) -> Vec<u64> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut chunk = [0u8; 8];
        chunk[..es].copy_from_slice(&v[i * es..(i + 1) * es]);
        out.push(u64::from_ne_bytes(chunk));
    }
    out
}

#[test]
fn test_cycle_left_right() {
    let es = std::mem::size_of::<u64>();
    // Test for n in 0..30
    for n in 0..30usize {
        // Trivial: shift 0, n, 2n, 3n -> identity
        let initial: Vec<u64> = (0..n as u64).collect();
        for &m in &[0usize, n, 2 * n, 3 * n] {
            let mut buf = arr_to_bytes(&initial, es);
            gca_cycle_left(&mut buf, n, es, m);
            let result = bytes_to_arr(&buf, n, es);
            assert_eq!(result, initial, "cycle_left n={} m={}", n, m);
            let mut buf = arr_to_bytes(&initial, es);
            gca_cycle_right(&mut buf, n, es, m);
            let result = bytes_to_arr(&buf, n, es);
            assert_eq!(result, initial, "cycle_right n={} m={}", n, m);
        }
        // Various shifts
        for shift in 0..n {
            let mut buf = arr_to_bytes(&initial, es);
            gca_cycle_left(&mut buf, n, es, shift);
            let result = bytes_to_arr(&buf, n, es);
            for i in 0..n {
                assert_eq!(result[i], ((i + shift) % n) as u64);
            }
            // shift back
            gca_cycle_right(&mut buf, n, es, shift);
            let result = bytes_to_arr(&buf, n, es);
            assert_eq!(result, initial);
        }
    }
}

#[test]
fn test_is_sorted() {
    let v = [1, 2, 3, 4, 5];
    assert!(gca_is_sorted(&v, |a: &i32, b: &i32| a.cmp(b)));
    let v2 = [1, 3, 2, 4, 5];
    assert!(!gca_is_sorted(&v2, |a: &i32, b: &i32| a.cmp(b)));
    let v3: [i32; 0] = [];
    assert!(gca_is_sorted(&v3, |a: &i32, b: &i32| a.cmp(b)));
    let v4 = [1];
    assert!(gca_is_sorted(&v4, |a: &i32, b: &i32| a.cmp(b)));
    let v5 = [3, 3, 3];
    assert!(gca_is_sorted(&v5, |a: &i32, b: &i32| a.cmp(b)));
}

#[test]
fn test_is_rsorted() {
    let v = [5, 4, 3, 2, 1];
    assert!(gca_is_rsorted(&v, |a: &i32, b: &i32| a.cmp(b)));
    let v2 = [5, 3, 4, 2, 1];
    assert!(!gca_is_rsorted(&v2, |a: &i32, b: &i32| a.cmp(b)));
    let v3: [i32; 0] = [];
    assert!(gca_is_rsorted(&v3, |a: &i32, b: &i32| a.cmp(b)));
    let v4 = [42];
    assert!(gca_is_rsorted(&v4, |a: &i32, b: &i32| a.cmp(b)));
    let v5 = [3, 3, 3];
    assert!(gca_is_rsorted(&v5, |a: &i32, b: &i32| a.cmp(b)));
}

#[test]
fn test_max() {
    let v = [3, 1, 4, 1, 5, 9, 2, 6];
    let m = gca_max(&v, |a: &i32, b: &i32| a.cmp(b));
    assert_eq!(m, Some(&9));
    let v2: [i32; 0] = [];
    assert_eq!(gca_max(&v2, |a: &i32, b: &i32| a.cmp(b)), None);
    let v3 = [42];
    assert_eq!(gca_max(&v3, |a: &i32, b: &i32| a.cmp(b)), Some(&42));
    // First in case of tie - C returns last larger one
    // Actually C uses < so equal elements keep first. Let's check.
    let v4 = [5, 5, 5];
    assert_eq!(gca_max(&v4, |a: &i32, b: &i32| a.cmp(b)), Some(&5));
}

#[test]
fn test_min() {
    let v = [3, 1, 4, 1, 5, 9, 2, 6];
    let m = gca_min(&v, |a: &i32, b: &i32| a.cmp(b));
    assert_eq!(m, Some(&1));
    let v2: [i32; 0] = [];
    assert_eq!(gca_min(&v2, |a: &i32, b: &i32| a.cmp(b)), None);
    let v3 = [42];
    assert_eq!(gca_min(&v3, |a: &i32, b: &i32| a.cmp(b)), Some(&42));
}

fn main() {}
