//! Phase C — error-path differential tests, one test per `ERRORS.md` row
//! (E1..E12) plus the generic FFI boundary rows (G1..G10).
//!
//! Each test asserts C and Rust return the *same specific* sentinel / clamped
//! value, not merely that both "failed somehow".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// E1 — safe_double_to_int(NaN) == 0
// ---------------------------------------------------------------------------
#[test]
fn e01_sdti_nan_returns_zero() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("E1");
    let mut rng = Rng::new(0xE001);
    let mut nans = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ];
    for _ in 0..4000 {
        let payload = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        nans.push(f64::from_bits(0x7FF0_0000_0000_0000 | payload));
        nans.push(f64::from_bits(0xFFF0_0000_0000_0000 | payload));
    }
    for v in nans {
        assert!(v.is_nan());
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("bits={:#018x}", v.to_bits()), cv, rv);
        assert_eq!(cv, 0, "C must return 0 for NaN {:#018x}", v.to_bits());
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E2 / E3 — safe_double_to_int(±inf)
// ---------------------------------------------------------------------------
#[test]
fn e02_sdti_pos_inf_returns_int_max() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("E2");
    for v in [f64::INFINITY, f64::from_bits(0x7FF0_0000_0000_0000)] {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check("+inf", cv, rv);
        assert_eq!(cv, i32::MAX);
    }
    d.finish();
}

#[test]
fn e03_sdti_neg_inf_returns_int_min() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("E3");
    for v in [f64::NEG_INFINITY, f64::from_bits(0xFFF0_0000_0000_0000)] {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check("-inf", cv, rv);
        assert_eq!(cv, i32::MIN);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E4 — finite d >= (double)INT_MAX  =>  INT_MAX
// ---------------------------------------------------------------------------
#[test]
fn e04_sdti_upper_clamp() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("E4");
    let bound = 2147483647.0f64;
    let mut vals = vec![
        bound,                                            // exactly the bound
        f64::from_bits(bound.to_bits() + 1),              // one ULP above
        f64::from_bits(bound.to_bits() - 1),              // one ULP below (NOT clamped)
        bound + 1.0,
        2147483648.0,
        1e10,
        1e300,
        f64::MAX,
    ];
    let mut rng = Rng::new(0xE004);
    for _ in 0..4000 {
        let e = rng.range_i32(31, 1023);
        let m = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 + 1.0;
        vals.push(m * 2f64.powi(e));
    }
    for v in vals {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("{v:?}"), cv, rv);
        if v >= bound {
            assert_eq!(cv, i32::MAX, "C should clamp {v:?} to INT_MAX");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E5 — finite d <= (double)INT_MIN  =>  INT_MIN
// ---------------------------------------------------------------------------
#[test]
fn e05_sdti_lower_clamp() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("E5");
    let bound = -2147483648.0f64;
    let mut vals = vec![
        bound,
        f64::from_bits(bound.to_bits() + 1), // one ULP more negative
        f64::from_bits(bound.to_bits() - 1), // one ULP less negative (NOT clamped)
        bound - 1.0,
        -2147483649.0,
        -1e10,
        -1e300,
        f64::MIN,
    ];
    let mut rng = Rng::new(0xE005);
    for _ in 0..4000 {
        let e = rng.range_i32(31, 1023);
        let m = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64 + 1.0;
        vals.push(-(m * 2f64.powi(e)));
    }
    for v in vals {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("{v:?}"), cv, rv);
        if v <= bound {
            assert_eq!(cv, i32::MIN, "C should clamp {v:?} to INT_MIN");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E6 — allocate_and_compute malloc failure  =>  -1
// ---------------------------------------------------------------------------
#[test]
fn e06_allocate_malloc_failure_returns_minus_one() {
    let (c, r) = allocate_and_compute();
    let mut d = Diff::new("E6");
    let mut rng = Rng::new(0xE006);
    let mut sizes = vec![-1, -2, -3, -15, -16, -17, -1024, i32::MIN, i32::MIN + 1, -1_000_000];
    for _ in 0..2000 {
        sizes.push(-(rng.range_i32(1, i32::MAX - 1)));
    }
    for size in sizes {
        for m in [1.5, -1.5, 0.0, f64::NAN, f64::INFINITY] {
            let cv = unsafe { c(size, m) };
            let rv = unsafe { r(size, m) };
            d.check(format!("size={size} mult={m:?}"), cv, rv);
            assert_eq!(cv, -1, "C must return -1 for negative size {size}");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E7 — allocate_and_compute(0, *)  =>  0  (malloc(0) succeeds on glibc)
// ---------------------------------------------------------------------------
#[test]
fn e07_allocate_size_zero_returns_zero() {
    let (c, r) = allocate_and_compute();
    let mut d = Diff::new("E7");
    let mut rng = Rng::new(0xE007);
    let mut mults = vec![
        0.0,
        -0.0,
        1.5,
        -1.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
    ];
    for _ in 0..2000 {
        mults.push(rng.next_f64_bits());
    }
    for m in mults {
        let cv = unsafe { c(0, m) };
        let rv = unsafe { r(0, m) };
        d.check(format!("size=0 mult={m:?}"), cv, rv);
        assert_eq!(cv, 0, "C must return 0 for size=0 (mult={m:?})");
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E8 — allocate_and_compute saturation through safe_double_to_int
// ---------------------------------------------------------------------------
#[test]
fn e08_allocate_saturation_paths() {
    let (c, r) = allocate_and_compute();
    let mut d = Diff::new("E8");
    // (size, multiplier, expected C result)
    let expectations: Vec<(i32, f64, i32)> = vec![
        (10_000, 1.5, i32::MAX),        // finite sum >= INT_MAX
        (10_000, -1.5, i32::MIN),       // finite sum <= INT_MIN
        (4, f64::MAX, i32::MAX),        // sum overflows to +inf
        (4, f64::MIN, i32::MIN),        // sum overflows to -inf
        (4, f64::INFINITY, 0),          // element 0: 0 * (0.0*inf) = 0*NaN = NaN
        (4, f64::NEG_INFINITY, 0),      // ditto
        (4, f64::NAN, 0),               // NaN propagates
        (1, f64::INFINITY, 0),          // single element is index 0 -> NaN
        (1, f64::NAN, 0),
    ];
    for (size, m, expect_c) in expectations {
        let cv = unsafe { c(size, m) };
        let rv = unsafe { r(size, m) };
        d.check(format!("size={size} mult={m:?}"), cv, rv);
        assert_eq!(cv, expect_c, "C behaviour changed for size={size} mult={m:?}");
    }
    // Randomised saturation sweep.
    let mut rng = Rng::new(0xE008);
    for _ in 0..2000 {
        let size = rng.range_i32(1, 4000);
        let m = if rng.next_u64() & 1 == 0 { 1e300 } else { -1e300 };
        let cv = unsafe { c(size, m) };
        let rv = unsafe { r(size, m) };
        d.check(format!("size={size} mult={m:?}"), cv, rv);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E9 / G6 / G7 — switch default branch, incl. out-of-range "enum" ints
// ---------------------------------------------------------------------------
#[test]
fn e09_switch_default_returns_zero() {
    let (c, r) = switch_fallthrough_calculator();
    let mut d = Diff::new("E9");
    let mut rng = Rng::new(0xE009);
    // G6: one step past each end of the valid range. G7: exotic out-of-range ints.
    let mut ops: Vec<i32> = vec![
        -1,
        5,
        6,
        7,
        255,
        256,
        -2,
        -5,
        1000,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        0x8000_0000u32 as i32,
        0x7FFF_FFFFu32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x0000_0100,
    ];
    for _ in 0..4000 {
        let op = rng.next_i32();
        if !(0..=4).contains(&op) {
            ops.push(op);
        }
    }
    let values = [0, 1, -1, 511, 512, i32::MIN, i32::MAX, 12345, -12345];
    for &op in &ops {
        assert!(!(0..=4).contains(&op));
        for &v in &values {
            let cv = unsafe { c(v, op) };
            let rv = unsafe { r(v, op) };
            d.check(format!("value={v} op={op}"), cv, rv);
            assert_eq!(cv, 0, "C default branch must yield 0 (value={v} op={op})");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E10 — fallcalc's malloc-failure branch is unreachable (20-byte request):
//       both libraries must therefore NEVER return -1; every result is masked
//       with 0777 so it is always in [0, 511].
// ---------------------------------------------------------------------------
#[test]
fn e10_fallcalc_never_takes_alloc_failure_branch() {
    let (c, r) = fallcalc();
    let mut d = Diff::new("E10");
    let mut rng = Rng::new(0xE010);
    let corners = [i32::MIN, i32::MIN + 1, -1, 0, 1, 128, 129, i32::MAX - 1, i32::MAX];
    let mut cases: Vec<(i32, i32, i32, i32)> = Vec::new();
    for &a in &corners {
        for &b in &corners {
            cases.push((a, b, 0, 0));
            cases.push((a, 0, b, 0));
            cases.push((a, 0, 0, b));
        }
    }
    for _ in 0..10_000 {
        cases.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    for (a, b, cc, dd) in cases {
        let cv = unsafe { c(a, b, cc, dd) };
        let rv = unsafe { r(a, b, cc, dd) };
        d.check(format!("({a}, {b}, {cc}, {dd})"), cv, rv);
        assert_ne!(cv, -1, "C fallcalc unexpectedly hit the -1 branch");
        assert!(
            (0..=511).contains(&cv),
            "C fallcalc result {cv} outside the 0777 mask range"
        );
        assert!(
            (0..=511).contains(&rv),
            "Rust fallcalc result {rv} outside the 0777 mask range"
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E11 / G1 / G3 — process_array_reverse with non-positive count (no deref)
// ---------------------------------------------------------------------------
#[test]
fn e11_process_array_reverse_non_positive_count() {
    let (c, r) = process_array_reverse();
    let mut d = Diff::new("E11");
    let mut rng = Rng::new(0xE011);
    let mut buf = [11i32, 22, 33, 44];
    let ptrs: [*mut i32; 4] = [
        std::ptr::null_mut(),
        buf.as_mut_ptr(),
        unsafe { buf.as_mut_ptr().add(3) },
        usize::MAX as *mut i32,
    ];
    let mut counts = vec![0, -1, -2, -7, i32::MIN, i32::MIN + 1];
    for _ in 0..1000 {
        counts.push(-(rng.range_i32(1, i32::MAX - 1)));
    }
    for &p in &ptrs {
        for &n in &counts {
            let cv = unsafe { c(p, n) };
            let rv = unsafe { r(p, n) };
            d.check(format!("end={p:?} count={n}"), cv, rv);
            assert_eq!(cv, 0, "C must return 0 for count={n}");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// E12 / G2 / G4 — foreach_sum with non-positive count (no deref)
// ---------------------------------------------------------------------------
#[test]
fn e12_foreach_sum_non_positive_count() {
    let (c, r) = foreach_sum();
    let mut d = Diff::new("E12");
    let mut rng = Rng::new(0xE012);
    let mut buf = [11i32, 22, 33, 44];
    let ptrs: [*mut i32; 4] = [
        std::ptr::null_mut(),
        buf.as_mut_ptr(),
        unsafe { buf.as_mut_ptr().add(3) },
        usize::MAX as *mut i32,
    ];
    let mut counts = vec![0, -1, -2, -7, i32::MIN, i32::MIN + 1];
    for _ in 0..1000 {
        counts.push(-(rng.range_i32(1, i32::MAX - 1)));
    }
    for &p in &ptrs {
        for &n in &counts {
            let cv = unsafe { c(p, n) };
            let rv = unsafe { r(p, n) };
            d.check(format!("array={p:?} count={n}"), cv, rv);
            assert_eq!(cv, 0, "C must return 0 for count={n}");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// G5 — allocate_and_compute at the size boundary {INT_MIN, -1, 0, 1}
// ---------------------------------------------------------------------------
#[test]
fn g05_allocate_size_boundary() {
    let (c, r) = allocate_and_compute();
    let mut d = Diff::new("G5");
    let expect: [(i32, i32); 4] = [(i32::MIN, -1), (-1, -1), (0, 0), (1, 0)];
    for (size, want) in expect {
        for m in [0.0, 1.5, -1.5, f64::NAN, f64::INFINITY, f64::MAX] {
            let cv = unsafe { c(size, m) };
            let rv = unsafe { r(size, m) };
            d.check(format!("size={size} mult={m:?}"), cv, rv);
            assert_eq!(cv, want, "C behaviour for size={size} mult={m:?}");
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// G8 — fallcalc corner grid across all four params
// ---------------------------------------------------------------------------
#[test]
fn g08_fallcalc_param_corners() {
    let (c, r) = fallcalc();
    let mut d = Diff::new("G8");
    let corners = [i32::MIN, i32::MAX, 0, -1];
    for &a in &corners {
        for &b in &corners {
            for &cc in &corners {
                for &dd in &corners {
                    let cv = unsafe { c(a, b, cc, dd) };
                    let rv = unsafe { r(a, b, cc, dd) };
                    d.check(format!("({a}, {b}, {cc}, {dd})"), cv, rv);
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// G9 — fallcalc driving the inner allocation into each of its three regimes
// ---------------------------------------------------------------------------
#[test]
fn g09_fallcalc_inner_alloc_regimes() {
    let (c, r) = fallcalc();
    let mut d = Diff::new("G9");
    let mut rng = Rng::new(0xE099);
    // param4 covering every residue of `% 10` for both signs.
    let mut p4s: Vec<i32> = Vec::new();
    for m in 0..10 {
        p4s.push(m);
        p4s.push(-m);
        p4s.push(1_000_000 + m);
        p4s.push(-1_000_000 - m);
        p4s.push(i32::MAX - m);
        p4s.push(i32::MIN + m);
    }
    for p4 in p4s {
        for _ in 0..40 {
            let a = rng.next_i32();
            let b = rng.next_i32();
            let cc = rng.next_i32();
            let cv = unsafe { c(a, b, cc, p4) };
            let rv = unsafe { r(a, b, cc, p4) };
            d.check(format!("({a}, {b}, {cc}, {p4}) p4%10={}", p4 % 10), cv, rv);
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// G10 — safe_double_to_int on -0.0, subnormals, and clamp-bound neighbours
// ---------------------------------------------------------------------------
#[test]
fn g10_sdti_signed_zero_subnormal_and_neighbours() {
    let (c, r) = safe_double_to_int();
    let mut d = Diff::new("G10");
    let mut vals = vec![0.0, -0.0, f64::MIN_POSITIVE, -f64::MIN_POSITIVE, 5e-324, -5e-324];
    for bound in [2147483647.0f64, -2147483648.0f64] {
        for k in 0..8u64 {
            vals.push(f64::from_bits(bound.to_bits().wrapping_add(k)));
            vals.push(f64::from_bits(bound.to_bits().wrapping_sub(k)));
        }
    }
    for v in vals {
        let cv = unsafe { c(v) };
        let rv = unsafe { r(v) };
        d.check(format!("{v:?} bits={:#018x}", v.to_bits()), cv, rv);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Symbol-presence smoke test: both libraries must resolve all six symbols.
// ---------------------------------------------------------------------------
#[test]
fn symbols_resolve_in_both_libraries() {
    let l = libs();
    for name in [
        "safe_double_to_int\0",
        "process_array_reverse\0",
        "switch_fallthrough_calculator\0",
        "allocate_and_compute\0",
        "foreach_sum\0",
        "fallcalc\0",
    ] {
        unsafe {
            l.c.get::<*const ()>(name.as_bytes())
                .unwrap_or_else(|e| panic!("C .so missing {name:?}: {e}"));
            l.r.get::<*const ()>(name.as_bytes())
                .unwrap_or_else(|e| panic!("Rust .so missing {name:?}: {e}"));
        }
    }
    eprintln!("C   .so: {}", l.c_path.display());
    eprintln!("Rust.so: {}", l.r_path.display());
}
