// Phase B -- lowest-level entry point: `fma_array`.
// CONFIGS.md rows C1..C9.
//
// `call_fma` only ever calls `fma_array` with mul1 = all-ones and add = all-zeros,
// so the general `mul1[i]*mul2[i] + add[i]` path is unreachable from the wrappers.
// These tests drive it directly through both `.so` exports.

mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn c1_len_one_random() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..2000 {
        let m1 = vec![rng.i32_small(10_000)];
        let m2 = vec![rng.i32_small(10_000)];
        let a = vec![rng.i32_small(1_000_000)];
        assert_fma_array_same(&m1, &m2, &a, 1, 8, "C1");
    }
}

#[test]
fn c2_len_two_and_three() {
    let mut rng = Rng::new(SEED ^ 2);
    for len in [2usize, 3] {
        for _ in 0..1500 {
            let m1 = rng.vec_small(len, 10_000);
            let m2 = rng.vec_small(len, 10_000);
            let a = rng.vec_small(len, 1_000_000);
            assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 5, "C2");
        }
    }
}

#[test]
fn c3_len_4_to_64_random() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..1500 {
        let len = rng.range(4, 64);
        let m1 = rng.vec_small(len, 30_000);
        let m2 = rng.vec_small(len, 30_000);
        let a = rng.vec_small(len, i32::MAX / 2);
        assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 7, "C3");
    }
}

#[test]
fn c4_large_shapes() {
    let mut rng = Rng::new(SEED ^ 4);
    for len in [100usize, 1000] {
        for _ in 0..40 {
            let m1 = rng.vec_any(len);
            let m2 = rng.vec_any(len);
            let a = rng.vec_any(len);
            assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 3, "C4");
        }
    }
}

#[test]
fn c5_identity_and_uniform_shapes() {
    for len in [1usize, 2, 5, 17, 64, 100] {
        // all zeros
        let z = vec![0; len];
        assert_fma_array_same(&z, &z, &z, len as c_int, len + 4, "C5/zeros");

        // all ones
        let o = vec![1; len];
        assert_fma_array_same(&o, &o, &o, len as c_int, len + 4, "C5/ones");

        // the exact shape `call_fma` builds: mul1 = ones, add = zeros
        let mut rng = Rng::new(SEED ^ 5 ^ len as u64);
        let data = rng.vec_any(len);
        let got = assert_fma_array_same(&o, &data, &z, len as c_int, len + 4, "C5/identity");
        // sanity: identity really is the identity
        assert_eq!(&got[..len], &data[..]);
    }
}

#[test]
fn c6_negative_and_mixed_sign() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..1500 {
        let len = rng.range(1, 32);
        let m1: Vec<c_int> = (0..len).map(|_| -(rng.range(0, 1000) as i32)).collect();
        let m2 = rng.vec_small(len, 1000);
        let a: Vec<c_int> = (0..len).map(|_| -(rng.range(0, 1000) as i32)).collect();
        assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 2, "C6/neg");

        let m1 = rng.vec_small(len, 1000);
        let m2 = rng.vec_small(len, 1000);
        let a = rng.vec_small(len, 1000);
        assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 2, "C6/mixed");
    }
}

#[test]
fn c7_overflow_shapes() {
    // Exhaustive over EXTREMES^3 at len == 1 (15^3 = 3375 combinations), so every
    // wrapping multiply/add boundary is covered.
    for &m1v in EXTREMES {
        for &m2v in EXTREMES {
            for &av in EXTREMES {
                assert_fma_array_same(&[m1v], &[m2v], &[av], 1, 4, "C7/exhaustive");
            }
        }
    }
    // And in bulk, so a vectorized loop sees them too.
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..500 {
        let len = rng.range(1, 40);
        let m1: Vec<c_int> = (0..len).map(|_| rng.pick(EXTREMES)).collect();
        let m2: Vec<c_int> = (0..len).map(|_| rng.pick(EXTREMES)).collect();
        let a: Vec<c_int> = (0..len).map(|_| rng.pick(EXTREMES)).collect();
        assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 6, "C7/bulk");
    }
}

#[test]
fn c8_fully_random_i32() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..4000 {
        let len = rng.range(1, 24);
        let m1 = rng.vec_any(len);
        let m2 = rng.vec_any(len);
        let a = rng.vec_any(len);
        assert_fma_array_same(&m1, &m2, &a, len as c_int, len + 9, "C8");
    }
}

#[test]
fn c9_partial_write_len_less_than_buffers() {
    // Buffers of 64 elements, but `len` is smaller: only the first `len` outputs
    // may be written; the poisoned tail must survive in both implementations.
    let mut rng = Rng::new(SEED ^ 9);
    let cap = 64usize;
    for _ in 0..1200 {
        let len = rng.range(0, cap);
        let m1 = rng.vec_any(cap);
        let m2 = rng.vec_any(cap);
        let a = rng.vec_any(cap);
        let out = assert_fma_array_same(&m1, &m2, &a, len as c_int, cap, "C9");
        for i in len..cap {
            assert_eq!(out[i], POISON, "wrote past len at {i}");
        }
    }
}
