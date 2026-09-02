//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so`s via `libloading` and compares captured stdout
//! byte-for-byte. Randomized rows use the fixed seed from `common::SEED`.

mod common;

use common::{assert_same, assert_same_all, driver_symbol, capture, Impl, Rng, SEED};

const I32_MIN: i32 = i32::MIN;
const I32_MAX: i32 = i32::MAX;
/// First `x` for which `2*x` fits but `y += 300` overflows.
const ADD_OVF_FIRST: i32 = 1_073_741_674;
/// Largest `x` for which `y += 300` does not overflow.
const ADD_OVF_LAST_OK: i32 = 1_073_741_673;

// ---------------------------------------------------------------- C1
#[test]
fn c1_zero() {
    let out = assert_same(0, "C1");
    assert_eq!(out, b"300\n", "C reference output for driver(0)");
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_small_positive() {
    let mut rng = Rng::new(SEED ^ 2);
    let xs = (1..=9)
        .chain((0..400).map(|_| rng.range_i32(1, 999)))
        .collect::<Vec<_>>();
    assert_same_all(xs, "C2");
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_small_negative() {
    let mut rng = Rng::new(SEED ^ 3);
    let xs = (-9..=-1)
        .chain((0..400).map(|_| rng.range_i32(-999, -1)))
        .collect::<Vec<_>>();
    assert_same_all(xs, "C3");
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_result_exactly_zero() {
    let out = assert_same(-150, "C4");
    assert_eq!(out, b"0\n", "C reference output for driver(-150)");
}

// ---------------------------------------------------------------- C5
#[test]
fn c5_either_side_of_zero_result() {
    assert_eq!(assert_same(-149, "C5"), b"2\n");
    assert_eq!(assert_same(-151, "C5"), b"-2\n");
    // Whole neighbourhood of the sign flip.
    assert_same_all(-200..=-100, "C5-neighbourhood");
}

// ---------------------------------------------------------------- C6
#[test]
fn c6_every_decimal_digit_count() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut xs = Vec::new();
    // Pick x so that |2x + 300| has exactly d digits, for d = 1..=10, both signs.
    for d in 1u32..=10 {
        let lo = if d == 1 { 0i64 } else { 10i64.pow(d - 1) };
        let hi = 10i64.pow(d) - 1;
        for sign in [1i64, -1i64] {
            for _ in 0..12 {
                let target = lo + (rng.next_u64() % (hi - lo + 1) as u64) as i64;
                let y = sign * target;
                // y = 2x + 300  =>  x = (y - 300)/2, keep it inside i32.
                let x = (y - 300) / 2;
                if x >= I32_MIN as i64 && x <= I32_MAX as i64 {
                    xs.push(x as i32);
                }
            }
        }
    }
    // Exact digit boundaries too.
    for d in 1u32..=9 {
        let p = 10i64.pow(d);
        for y in [p - 1, p, p + 1, -(p - 1), -p, -(p + 1)] {
            let x = (y - 300) / 2;
            if x >= I32_MIN as i64 && x <= I32_MAX as i64 {
                xs.push(x as i32);
            }
        }
    }
    assert_same_all(xs, "C6");
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_mid_positive_no_overflow() {
    let mut rng = Rng::new(SEED ^ 7);
    let xs = (0..500).map(|_| rng.range_i32(1000, I32_MAX / 2 - 1));
    assert_same_all(xs, "C7");
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_mid_negative_no_overflow() {
    let mut rng = Rng::new(SEED ^ 8);
    let xs = (0..500).map(|_| rng.range_i32(I32_MIN / 2 + 1, -1000));
    assert_same_all(xs, "C8");
}

// ---------------------------------------------------------------- C9
#[test]
fn c9_multiply_overflow_positive_band() {
    let mut rng = Rng::new(SEED ^ 9);
    let xs = (0..500).map(|_| rng.range_i32(I32_MAX / 2 + 1, I32_MAX));
    assert_same_all(xs, "C9");
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_multiply_overflow_negative_band() {
    let mut rng = Rng::new(SEED ^ 10);
    let xs = (0..500).map(|_| rng.range_i32(I32_MIN, I32_MIN / 2 - 1));
    assert_same_all(xs, "C10");
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_add_overflow_only_band() {
    let mut rng = Rng::new(SEED ^ 11);
    let mut xs: Vec<i32> = vec![ADD_OVF_LAST_OK, ADD_OVF_FIRST, I32_MAX / 2];
    xs.extend((0..150).map(|_| rng.range_i32(ADD_OVF_FIRST, I32_MAX / 2)));
    assert_same_all(xs, "C11");
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_exact_boundary_values() {
    let xs = [
        I32_MIN,
        I32_MIN + 1,
        I32_MIN / 2 - 1,
        I32_MIN / 2,
        I32_MIN / 2 + 1,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        ADD_OVF_LAST_OK,
        ADD_OVF_FIRST,
        I32_MAX / 2 - 1,
        I32_MAX / 2,
        I32_MAX / 2 + 1,
        I32_MAX - 1,
        I32_MAX,
    ];
    assert_same_all(xs, "C12");
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_uniform_full_i32_range() {
    let mut rng = Rng::new(SEED ^ 13);
    let xs = (0..4000).map(|_| rng.next_i32());
    assert_eq!(assert_same_all(xs, "C13"), 4000);
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_many_interleaved_calls_are_stateless() {
    let mut rng = Rng::new(SEED ^ 14);
    let xs: Vec<i32> = (0..200).map(|_| rng.next_i32()).collect();

    let c = driver_symbol(Impl::C);
    let rust = driver_symbol(Impl::Rust);

    // All calls in ONE capture each: verifies output ordering across a whole
    // sequence, not just per call, and that no state carries between calls.
    let c_all = capture(|| {
        for &x in &xs {
            unsafe { c(x) };
        }
    })
    .1;
    let rust_all = capture(|| {
        for &x in &xs {
            unsafe { rust(x) };
        }
    })
    .1;
    assert_eq!(
        String::from_utf8_lossy(&c_all),
        String::from_utf8_lossy(&rust_all),
        "C14: sequential-call output must match line for line"
    );
    assert_eq!(c_all.iter().filter(|&&b| b == b'\n').count(), xs.len());

    // Interleaved C -> Rust -> C: the same input must yield the same line no
    // matter which library ran before it.
    for &x in xs.iter().take(40) {
        let a = capture(|| unsafe { c(x) }).1;
        let b = capture(|| unsafe { rust(x) }).1;
        let c2 = capture(|| unsafe { c(x) }).1;
        assert_eq!(a, b, "C14 interleaved mismatch at {x}");
        assert_eq!(a, c2, "C14 C-side not idempotent at {x}");
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_loading_alone_emits_nothing() {
    let out = capture(|| {
        let _c = driver_symbol(Impl::C);
        let _r = driver_symbol(Impl::Rust);
    })
    .1;
    assert!(
        out.is_empty(),
        "C15: loading/resolving must not write to stdout, got {:?}",
        String::from_utf8_lossy(&out)
    );
}

// ---------------------------------------------------------------- C16
#[test]
fn c16_powers_of_two_sweep() {
    let mut xs: Vec<i32> = Vec::new();
    for k in 0..32u32 {
        let p = 1i64 << k;
        for cand in [p - 1, p, p + 1, -(p - 1), -p, -(p + 1)] {
            if cand >= I32_MIN as i64 && cand <= I32_MAX as i64 {
                xs.push(cand as i32);
            }
        }
    }
    assert_same_all(xs, "C16");
}
