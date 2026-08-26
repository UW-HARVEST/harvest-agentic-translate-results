// Phase B -- CONFIGS.md rows 1..16: the lowest-level entry point,
// `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`,
// driven directly through both shared objects with every aliasing pattern,
// length class and value class the C code distinguishes.

mod common;

use common::{assert_fma_matches, FmaCase, Rng, EXTREMES};
use std::os::raw::c_int;

const ITERS: usize = 200;

fn rand_vals(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| rng.next_i32()).collect()
}

fn small_vals(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| rng.range_incl(-100, 100) as i32).collect()
}

fn extreme_vals(rng: &mut Rng, n: usize) -> Vec<i32> {
    (0..n).map(|_| *rng.pick(EXTREMES)).collect()
}

// --------------------------------------------------- rows 1-4: disjoint ----

fn disjoint_row(seed: u64, label: &str, len_of: impl Fn(&mut Rng) -> c_int) {
    let mut rng = Rng::new(seed);
    for i in 0..ITERS {
        let len = len_of(&mut rng);
        let n = len.max(0) as usize;
        let vals = rand_vals(&mut rng, 4 * n);
        assert_fma_matches(&format!("{label}#{i}"), &FmaCase::disjoint(vals, len));
    }
}

#[test]
fn row01_disjoint_len1_random() {
    disjoint_row(0x0101, "row01", |_| 1);
}

#[test]
fn row02_disjoint_len2_to_8_random() {
    disjoint_row(0x0202, "row02", |r| r.range_incl(2, 8) as c_int);
}

#[test]
fn row03_disjoint_len100_random() {
    disjoint_row(0x0303, "row03", |_| 100);
}

#[test]
fn row04_disjoint_len1000_random() {
    let mut rng = Rng::new(0x0404);
    for i in 0..20 {
        let len: c_int = 1000;
        let vals = rand_vals(&mut rng, 4 * len as usize);
        assert_fma_matches(&format!("row04#{i}"), &FmaCase::disjoint(vals, len));
    }
}

// ------------------------------------------------ rows 5-7: value shape ----

#[test]
fn row05_disjoint_small_values_no_overflow() {
    let mut rng = Rng::new(0x0505);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let vals = small_vals(&mut rng, 4 * len as usize);
        assert_fma_matches(&format!("row05#{i}"), &FmaCase::disjoint(vals, len));
    }
}

#[test]
fn row06_disjoint_extreme_values() {
    let mut rng = Rng::new(0x0606);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let vals = extreme_vals(&mut rng, 4 * len as usize);
        assert_fma_matches(&format!("row06#{i}"), &FmaCase::disjoint(vals, len));
    }
}

#[test]
fn row07_disjoint_uniform_buffers() {
    let mut rng = Rng::new(0x0707);
    let uniform = [0i32, 1, -1, i32::MIN, i32::MAX, 46_341, -46_341];
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        // Each of the four windows filled with its own constant.
        let mut vals = Vec::with_capacity(4 * n);
        for _ in 0..4 {
            let v = *rng.pick(&uniform);
            vals.extend(std::iter::repeat(v).take(n));
        }
        assert_fma_matches(&format!("row07#{i}"), &FmaCase::disjoint(vals, len));
    }
}

// -------------------------------------------------- rows 8-14: aliasing ----

/// Builds a case whose four pointers use the given element offsets into one
/// allocation of `total` elements.
fn aliased_case(
    rng: &mut Rng,
    len: c_int,
    total: usize,
    out: usize,
    mul1: usize,
    mul2: usize,
    add: usize,
    values: fn(&mut Rng, usize) -> Vec<i32>,
) -> FmaCase {
    FmaCase {
        buf: values(rng, total),
        out,
        mul1,
        mul2,
        add,
        len,
    }
}

#[test]
fn row08_out_aliases_mul1() {
    let mut rng = Rng::new(0x0808);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, 3 * n, 0, 0, n, 2 * n, rand_vals);
        assert_fma_matches(&format!("row08#{i}"), &case);
    }
}

#[test]
fn row09_out_aliases_mul2() {
    let mut rng = Rng::new(0x0909);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, 3 * n, 0, n, 0, 2 * n, rand_vals);
        assert_fma_matches(&format!("row09#{i}"), &case);
    }
}

#[test]
fn row10_out_aliases_add() {
    let mut rng = Rng::new(0x0a0a);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, 3 * n, 0, n, 2 * n, 0, rand_vals);
        assert_fma_matches(&format!("row10#{i}"), &case);
    }
}

#[test]
fn row11_mul1_aliases_mul2_squaring() {
    let mut rng = Rng::new(0x0b0b);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        // out and add disjoint, mul1 == mul2 -> out[i] = x*x + z
        let case = aliased_case(&mut rng, len, 3 * n, 0, n, n, 2 * n, rand_vals);
        assert_fma_matches(&format!("row11#{i}"), &case);
    }
}

#[test]
fn row12_all_four_aliased_like_driver() {
    let mut rng = Rng::new(0x0c0c);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let vals = if rng.bool() {
            rand_vals(&mut rng, n)
        } else {
            extreme_vals(&mut rng, n)
        };
        let case = FmaCase {
            buf: vals,
            out: 0,
            mul1: 0,
            mul2: 0,
            add: 0,
            len,
        };
        assert_fma_matches(&format!("row12#{i}"), &case);
    }
}

#[test]
fn row13_forward_partial_overlap() {
    // out = base, operands = base + 1 : each store lands on an element that was
    // already loaded, so the loads stay pristine.
    let mut rng = Rng::new(0x0d0d);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, n + 1, 0, 1, 1, 1, rand_vals);
        assert_fma_matches(&format!("row13#{i}"), &case);
    }
}

#[test]
fn row14_backward_partial_overlap() {
    // out = base + 1, operands = base : every store clobbers the element the
    // next iteration is about to load.
    let mut rng = Rng::new(0x0e0e);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, n + 1, 1, 0, 0, 0, rand_vals);
        assert_fma_matches(&format!("row14#{i}"), &case);
    }
    // Same shape with values that overflow on every element.
    let mut rng = Rng::new(0x0e0f);
    for i in 0..ITERS {
        let len = rng.range_incl(1, 64) as c_int;
        let n = len as usize;
        let case = aliased_case(&mut rng, len, n + 1, 1, 0, 0, 0, extreme_vals);
        assert_fma_matches(&format!("row14x#{i}"), &case);
    }
}

// ----------------------------------------------- rows 15-16: len guards ----

#[test]
fn row15_len_zero_leaves_buffers_untouched() {
    let mut rng = Rng::new(0x0f0f);
    for i in 0..ITERS {
        let n = rng.range_incl(1, 32) as usize;
        let case = FmaCase {
            buf: rand_vals(&mut rng, 4 * n),
            out: 0,
            mul1: n,
            mul2: 2 * n,
            add: 3 * n,
            len: 0,
        };
        let before = case.buf.clone();
        assert_fma_matches(&format!("row15#{i}"), &case);

        // Prove *both* implementations really left the memory alone.
        let l = common::libs();
        let mut buf = before.clone();
        unsafe {
            let p = buf.as_mut_ptr();
            (l.c_fma())(p, p.add(n), p.add(2 * n), p.add(3 * n), 0);
        }
        assert_eq!(buf, before, "C fma_array wrote something with len==0");
        let mut buf = before.clone();
        unsafe {
            let p = buf.as_mut_ptr();
            (l.rust_fma())(p, p.add(n), p.add(2 * n), p.add(3 * n), 0);
        }
        assert_eq!(buf, before, "Rust fma_array wrote something with len==0");
    }
}

#[test]
fn row16_negative_len_is_a_no_op() {
    let mut rng = Rng::new(0x1010);
    let lens: [c_int; 6] = [-1, -2, -100, -1000, c_int::MIN + 1, c_int::MIN];
    for (i, &len) in lens.iter().enumerate() {
        for j in 0..20 {
            let n = rng.range_incl(1, 32) as usize;
            let case = FmaCase {
                buf: rand_vals(&mut rng, 4 * n),
                out: 0,
                mul1: n,
                mul2: 2 * n,
                add: 3 * n,
                len,
            };
            let before = case.buf.clone();
            assert_fma_matches(&format!("row16#{i}.{j}(len={len})"), &case);

            let l = common::libs();
            let mut buf = before.clone();
            unsafe {
                let p = buf.as_mut_ptr();
                (l.c_fma())(p, p.add(n), p.add(2 * n), p.add(3 * n), len);
            }
            assert_eq!(buf, before, "C fma_array wrote something with len={len}");
            let mut buf = before.clone();
            unsafe {
                let p = buf.as_mut_ptr();
                (l.rust_fma())(p, p.add(n), p.add(2 * n), p.add(3 * n), len);
            }
            assert_eq!(buf, before, "Rust fma_array wrote something with len={len}");
        }
    }
}
