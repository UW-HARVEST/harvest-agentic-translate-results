// Phase B -- mid-level entry point: `call_fma`.
// CONFIGS.md rows C10..C15.

mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn c10_len_one_full_range() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..4000 {
        let v = rng.i32_any();
        let got = assert_call_fma_same(&[v, POISON, POISON], 1, "C10");
        assert_eq!(got, v, "call_fma(len=1) must return data[0]");
    }
    for &v in EXTREMES {
        assert_call_fma_same(&[v], 1, "C10/extremes");
    }
}

#[test]
fn c11_len_2_to_16_full_range() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..4000 {
        let len = rng.range(2, 16);
        let data = rng.vec_any(len);
        let got = assert_call_fma_same(&data, len as c_int, "C11");
        assert_eq!(got, data[len - 1], "call_fma must return data[len-1]");
    }
}

#[test]
fn c12_driver_cap_boundary_lengths() {
    let mut rng = Rng::new(SEED ^ 12);
    for len in [99usize, 100, 101] {
        for _ in 0..200 {
            let data = rng.vec_any(len);
            let got = assert_call_fma_same(&data, len as c_int, "C12");
            assert_eq!(got, data[len - 1]);
        }
        // extremes at the returned slot
        for &v in EXTREMES {
            let mut data = rng.vec_any(len);
            data[len - 1] = v;
            assert_call_fma_same(&data, len as c_int, "C12/extremes");
        }
    }
}

#[test]
fn c13_len_1000() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..60 {
        let data = rng.vec_any(1000);
        let got = assert_call_fma_same(&data, 1000, "C13");
        assert_eq!(got, data[999]);
    }
}

#[test]
fn c14_extremes_at_first_and_last_index() {
    let mut rng = Rng::new(SEED ^ 14);
    for &v in EXTREMES {
        for len in [1usize, 2, 3, 8, 100] {
            let mut data = rng.vec_any(len);
            data[0] = v;
            assert_call_fma_same(&data, len as c_int, "C14/first");

            let mut data = rng.vec_any(len);
            data[len - 1] = v;
            assert_call_fma_same(&data, len as c_int, "C14/last");

            let data = vec![v; len];
            assert_call_fma_same(&data, len as c_int, "C14/uniform");
        }
    }
}

#[test]
fn c15_buffer_longer_than_len() {
    // Trailing elements past `len` must be ignored by both.
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..2000 {
        let len = rng.range(1, 32);
        let extra = rng.range(1, 32);
        let mut data = rng.vec_any(len);
        let head = data.clone();
        data.extend((0..extra).map(|_| rng.i32_any()));
        let got = assert_call_fma_same(&data, len as c_int, "C15");
        assert_eq!(got, head[len - 1]);
    }
}
