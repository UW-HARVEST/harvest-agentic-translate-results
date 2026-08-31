//! Level 1: `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
//!
//! The lowest-level exported function. Compared for byte-identical output
//! buffers between the C and Rust shared objects.

mod common;

use common::{EDGE_VALUES, Impl, Rng, fma_array, show};

/// Runs `fma_array` from one implementation over freshly copied inputs and
/// returns the resulting `out` buffer.
fn run_distinct(
    which: Impl,
    out_init: &[i32],
    mul1: &[i32],
    mul2: &[i32],
    add: &[i32],
    len: i32,
) -> Vec<i32> {
    let f = fma_array(which);
    let mut out = out_init.to_vec();
    let m1 = mul1.to_vec();
    let m2 = mul2.to_vec();
    let a = add.to_vec();
    unsafe { f(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len) };
    out
}

fn check_distinct(out_init: &[i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: i32) {
    let c = run_distinct(Impl::C, out_init, mul1, mul2, add, len);
    let rust = run_distinct(Impl::Rust, out_init, mul1, mul2, add, len);
    assert_eq!(
        c,
        rust,
        "fma_array mismatch\n  out_init={}\n  mul1={}\n  mul2={}\n  add={}\n  len={len}\n  C   ={}\n  Rust={}",
        show(out_init),
        show(mul1),
        show(mul2),
        show(add),
        show(&c),
        show(&rust)
    );
}

#[test]
fn distinct_buffers_small_values() {
    let mut rng = Rng::new(0x5eed_0001);
    for len in 0..=17i32 {
        for _ in 0..40 {
            let n = len.max(0) as usize;
            let out_init: Vec<i32> = (0..n).map(|_| rng.next_small()).collect();
            let mul1: Vec<i32> = (0..n).map(|_| rng.next_small()).collect();
            let mul2: Vec<i32> = (0..n).map(|_| rng.next_small()).collect();
            let add: Vec<i32> = (0..n).map(|_| rng.next_small()).collect();
            check_distinct(&out_init, &mul1, &mul2, &add, len);
        }
    }
}

#[test]
fn distinct_buffers_full_range_values() {
    let mut rng = Rng::new(0x5eed_0002);
    for len in [1i32, 2, 3, 8, 33, 64, 129] {
        for _ in 0..30 {
            let n = len as usize;
            let out_init: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            let mul1: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            let mul2: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            let add: Vec<i32> = (0..n).map(|_| rng.next_i32()).collect();
            check_distinct(&out_init, &mul1, &mul2, &add, len);
        }
    }
}

/// Every triple of interesting operand values, exercising signed overflow of
/// both the multiply and the add.
#[test]
fn edge_value_matrix() {
    let n = EDGE_VALUES.len();
    let out_init = vec![0x5a5a_5a5a; n * n];

    // mul1[k] x mul2[k] for all pairs, with `add` sweeping the edge values.
    for &addend in EDGE_VALUES.iter() {
        let mut mul1 = Vec::with_capacity(n * n);
        let mut mul2 = Vec::with_capacity(n * n);
        for &x in EDGE_VALUES.iter() {
            for &y in EDGE_VALUES.iter() {
                mul1.push(x);
                mul2.push(y);
            }
        }
        let add = vec![addend; n * n];
        check_distinct(&out_init, &mul1, &mul2, &add, (n * n) as i32);
    }
}

#[test]
fn zero_and_negative_len_write_nothing() {
    let out_init = vec![0x1234_5678i32; 8];
    let inputs = vec![7i32; 8];
    // C's loop condition `i < len` is false immediately for len <= 0, so `out`
    // must be left untouched.
    for len in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        let c = run_distinct(Impl::C, &out_init, &inputs, &inputs, &inputs, len);
        let rust = run_distinct(Impl::Rust, &out_init, &inputs, &inputs, &inputs, len);
        assert_eq!(c, out_init, "C modified out for len={len}");
        assert_eq!(rust, c, "fma_array mismatch for len={len}");
    }
}

/// `inner` calls `fma_array(out, out, out, out, len)`, so full self-aliasing is
/// the production call shape and must be reproduced exactly.
#[test]
fn full_self_aliasing() {
    let mut rng = Rng::new(0x5eed_0003);
    for len in 0..=17i32 {
        for _ in 0..40 {
            let n = len.max(0) as usize;
            let data: Vec<i32> = (0..n).map(|_| rng.next_small()).collect();
            let mut results = Vec::new();
            for which in common::IMPLS {
                let f = fma_array(which);
                let mut buf = data.clone();
                let p = buf.as_mut_ptr();
                unsafe { f(p, p, p, p, len) };
                results.push(buf);
            }
            assert_eq!(
                results[0],
                results[1],
                "self-aliased fma_array mismatch: data={} len={len}\n  C   ={}\n  Rust={}",
                show(&data),
                show(&results[0]),
                show(&results[1])
            );
        }
    }
}

/// Partial aliasing at arbitrary offsets within one buffer: the C code stores
/// element by element, so a later read can observe an earlier store.
#[test]
fn partial_aliasing_offsets() {
    let mut rng = Rng::new(0x5eed_0004);
    let buf_len = 24usize;
    for _ in 0..500 {
        let len = 1 + rng.range(8);
        let max_off = buf_len - len;
        let offs = [
            rng.range(max_off + 1),
            rng.range(max_off + 1),
            rng.range(max_off + 1),
            rng.range(max_off + 1),
        ];
        let data: Vec<i32> = (0..buf_len).map(|_| rng.next_small()).collect();

        let mut results = Vec::new();
        for which in common::IMPLS {
            let f = fma_array(which);
            let mut buf = data.clone();
            let base = buf.as_mut_ptr();
            unsafe {
                f(
                    base.add(offs[0]),
                    base.add(offs[1]),
                    base.add(offs[2]),
                    base.add(offs[3]),
                    len as i32,
                )
            };
            results.push(buf);
        }
        assert_eq!(
            results[0],
            results[1],
            "partially aliased fma_array mismatch: offs={offs:?} len={len} data={}\n  C   ={}\n  Rust={}",
            show(&data),
            show(&results[0]),
            show(&results[1])
        );
    }
}

/// `out` overlapping the inputs shifted by one, in both directions.
#[test]
fn sliding_window_aliasing() {
    let mut rng = Rng::new(0x5eed_0005);
    for len in 1..=12usize {
        for _ in 0..50 {
            let buf_len = len + 2;
            let data: Vec<i32> = (0..buf_len).map(|_| rng.next_small()).collect();
            for (out_off, in_off) in [(0usize, 1usize), (1, 0), (0, 2), (2, 0), (1, 1)] {
                let mut results = Vec::new();
                for which in common::IMPLS {
                    let f = fma_array(which);
                    let mut buf = data.clone();
                    let base = buf.as_mut_ptr();
                    unsafe {
                        f(
                            base.add(out_off),
                            base.add(in_off),
                            base.add(in_off),
                            base.add(in_off),
                            len as i32,
                        )
                    };
                    results.push(buf);
                }
                assert_eq!(
                    results[0], results[1],
                    "sliding-window mismatch: out_off={out_off} in_off={in_off} len={len} data={}",
                    show(&data)
                );
            }
        }
    }
}

/// The function returns `void`; confirm neither implementation touches memory
/// past `len` elements.
#[test]
fn no_out_of_range_writes() {
    let mut rng = Rng::new(0x5eed_0006);
    const GUARD: i32 = 0x7f7f_7f7f;
    for len in 0..=16usize {
        let mut results = Vec::new();
        let mul1: Vec<i32> = (0..len).map(|_| rng.next_small()).collect();
        let mul2: Vec<i32> = (0..len).map(|_| rng.next_small()).collect();
        let add: Vec<i32> = (0..len).map(|_| rng.next_small()).collect();
        for which in common::IMPLS {
            let f = fma_array(which);
            let mut out = vec![GUARD; len + 8];
            unsafe {
                f(
                    out.as_mut_ptr(),
                    mul1.as_ptr(),
                    mul2.as_ptr(),
                    add.as_ptr(),
                    len as i32,
                )
            };
            assert!(
                out[len..].iter().all(|&v| v == GUARD),
                "{which:?} wrote past len={len}: {}",
                show(&out)
            );
            results.push(out);
        }
        assert_eq!(results[0], results[1], "guarded fma_array mismatch len={len}");
    }
}
