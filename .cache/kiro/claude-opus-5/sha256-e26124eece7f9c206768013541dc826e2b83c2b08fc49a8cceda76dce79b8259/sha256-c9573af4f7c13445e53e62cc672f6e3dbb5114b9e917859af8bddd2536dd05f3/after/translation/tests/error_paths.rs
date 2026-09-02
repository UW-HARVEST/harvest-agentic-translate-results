//! Phase C — error/rejection-path differential tests, one test per
//! `ERRORS.md` row. Every test constructs the exact invalid input/condition,
//! calls BOTH `.so`s, and asserts the SAME sentinel or value — not merely
//! "both failed somehow".
//!
//! `ERRORS.md` rows 21-24 concern `doubleneg`'s error branches and need
//! process-wide fd-1 capture; they live in `tests/doubleneg_error.rs`.

mod harness;

use std::ffi::{c_char, c_int};
use std::ptr;

use harness::{Rng, apis};

// ---------------------------------------------------------------------------
// Row 1 — find_value_in_buffer: memchr returns NULL -> sentinel -1
// ---------------------------------------------------------------------------

#[test]
fn err_find_not_found() {
    let p = apis();
    let mut rng = Rng::new(0x0E01);
    for _ in 0..2000 {
        let n = 1 + rng.below(256) as usize;
        // Buffer of a single repeated byte; search for anything else.
        let fill = (rng.next_u32() & 0xff) as u8;
        let buf = vec![fill as i8; n];
        let mut missing = ((fill as u32 + 1 + rng.below(255) as u32) & 0xff) as i32;
        if missing == fill as i32 {
            missing = (missing + 1) & 0xff;
        }
        let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, n, missing) };
        let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, n, missing) };
        assert_eq!(c, -1, "C must return the -1 sentinel (fill={fill}, n={n})");
        assert_eq!(r, c, "row1 sentinel mismatch (fill={fill}, missing={missing})");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — size == 0 must reject even when the byte exists just past the end
// ---------------------------------------------------------------------------

#[test]
fn err_find_zero_size() {
    let p = apis();
    let buf: Vec<i8> = (0..64).map(|i| i as u8 as i8).collect();
    for sv in [
        0,
        1,
        42,
        63,
        100,
        255,
        256,
        -1,
        -128,
        i32::MIN,
        i32::MAX,
        0x1FF,
    ] {
        let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 0, sv) };
        let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, 0, sv) };
        assert_eq!(c, -1, "row2: C must reject size==0 (sv={sv})");
        assert_eq!(r, c, "row2 mismatch (sv={sv})");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — null pointer with size == 0
// ---------------------------------------------------------------------------

#[test]
fn err_find_null_zero_size() {
    let p = apis();
    for sv in [0, 1, 42, 255, -1, i32::MIN, i32::MAX] {
        let c = unsafe { (p.c.find_value_in_buffer)(ptr::null(), 0, sv) };
        let r = unsafe { (p.rust.find_value_in_buffer)(ptr::null(), 0, sv) };
        assert_eq!(c, -1, "row3: C must reject NULL/size==0 (sv={sv})");
        assert_eq!(r, c, "row3 mismatch (sv={sv})");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — `(char)search_val` narrowing: out-of-char-range values alias
// ---------------------------------------------------------------------------

#[test]
fn err_find_search_val_narrowing() {
    let p = apis();
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    let mut rng = Rng::new(0x0E04);
    for low in 0..256i32 {
        let mut variants = vec![low, low - 256, low + 256, low + 0x1_0000, low - 0x1_0000];
        for _ in 0..3 {
            let k = rng.next_i32() >> 9;
            variants.push(k.wrapping_mul(256).wrapping_add(low));
        }
        let mut results = Vec::new();
        for v in variants {
            let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, v) };
            let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, v) };
            assert_eq!(r, c, "row4 mismatch (low={low}, search_val={v})");
            results.push((v, c));
        }
        // All aliases of the same low byte must agree with each other in C.
        let first = results[0].1;
        for (v, got) in &results {
            assert_eq!(*got, first, "row4: C alias {v} disagreed for low byte {low}");
        }
        assert_eq!(first, low, "row4: expected index == low byte value");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — searching for the NUL byte is a valid search, not a sentinel
// ---------------------------------------------------------------------------

#[test]
fn err_find_nul_byte() {
    let p = apis();
    // NUL present at various positions, and absent.
    for pos in [Some(0usize), Some(1), Some(31), Some(63), None] {
        let mut buf = vec![0x41i8; 64];
        if let Some(i) = pos {
            buf[i] = 0;
        }
        for sv in [0, 256, -256, 0x1_0000] {
            let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 64, sv) };
            let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, 64, sv) };
            assert_eq!(r, c, "row5 mismatch (pos={pos:?}, sv={sv})");
            match pos {
                Some(i) => assert_eq!(c, i as c_int, "row5: expected NUL at {i}"),
                None => assert_eq!(c, -1, "row5: expected the -1 sentinel"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — search_val == INT_MIN / INT_MAX (one step past every range)
// ---------------------------------------------------------------------------

#[test]
fn err_find_search_val_extremes() {
    let p = apis();
    let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
    let empty = vec![0x41i8; 256];
    for buf in [&buf, &empty] {
        for sv in [
            i32::MIN,
            i32::MIN + 1,
            i32::MAX,
            i32::MAX - 1,
            -1,
            255,
            256,
            511,
            -255,
            -256,
            -511,
        ] {
            let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, sv) };
            let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, sv) };
            assert_eq!(r, c, "row6 mismatch (sv={sv})");
        }
    }
    // INT_MIN's low byte is 0x00, INT_MAX's is 0xFF.
    let idx_min = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, i32::MIN) };
    let idx_max = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, i32::MAX) };
    assert_eq!((idx_min, idx_max), (0, 255), "row6: unexpected C narrowing");
}

// ---------------------------------------------------------------------------
// Row 7 — size exactly equal to the allocation, target absent
// ---------------------------------------------------------------------------

#[test]
fn err_find_size_equals_len() {
    let p = apis();
    let mut rng = Rng::new(0x0E07);
    for _ in 0..1000 {
        let n = 1 + rng.below(1024) as usize;
        let buf = vec![0x11i8; n];
        for sv in [0x22, 0, -1, 255, i32::MIN, i32::MAX] {
            let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, n, sv) };
            let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, n, sv) };
            assert_eq!(c, -1, "row7: C must return -1 (n={n}, sv={sv})");
            assert_eq!(r, c, "row7 mismatch (n={n}, sv={sv})");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8, 9 — create_numeric_buffer with size == 0 / size < 0: no writes
// ---------------------------------------------------------------------------

fn assert_create_leaves_buffer_alone(size: c_int, seed: c_int) {
    let p = apis();
    let sentinel = 0xC3u8 as i8;
    let mut bc = vec![sentinel; 64];
    let mut br = vec![sentinel; 64];
    unsafe {
        (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, size, seed);
        (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, size, seed);
    }
    assert!(
        bc.iter().all(|&b| b == sentinel),
        "C wrote to the buffer for size={size}"
    );
    assert_eq!(bc, br, "create_numeric_buffer(size={size}, seed={seed})");
}

#[test]
fn err_create_zero_size() {
    for seed in [0, 1, -1, 42, 255, 256, i32::MIN, i32::MAX] {
        assert_create_leaves_buffer_alone(0, seed);
    }
}

#[test]
fn err_create_negative_size() {
    let mut rng = Rng::new(0x0E09);
    let mut sizes: Vec<c_int> = vec![-1, -2, -7, -64, -255, -256, i32::MIN, i32::MIN + 1];
    for _ in 0..300 {
        sizes.push(-(1 + rng.below(1 << 24) as c_int));
    }
    for size in sizes {
        assert_create_leaves_buffer_alone(size, rng.spicy_i32());
    }
}

// ---------------------------------------------------------------------------
// Rows 10, 11 — create_numeric_buffer must not dereference a NULL pointer
// when size <= 0
// ---------------------------------------------------------------------------

#[test]
fn err_create_null_zero_size() {
    let p = apis();
    for seed in [0, 1, -1, i32::MIN, i32::MAX] {
        unsafe {
            (p.c.create_numeric_buffer)(ptr::null_mut(), 0, seed);
            (p.rust.create_numeric_buffer)(ptr::null_mut(), 0, seed);
        }
    }
    // Reaching here means neither implementation dereferenced NULL.
}

#[test]
fn err_create_null_negative_size() {
    let p = apis();
    for size in [-1, -2, -256, i32::MIN, i32::MIN + 1] {
        for seed in [0, 1, -1, i32::MIN, i32::MAX] {
            unsafe {
                (p.c.create_numeric_buffer)(ptr::null_mut(), size, seed);
                (p.rust.create_numeric_buffer)(ptr::null_mut(), size, seed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — `seed + i * 7` overflows int
// ---------------------------------------------------------------------------

#[test]
fn err_create_seed_overflow() {
    let p = apis();
    let mut seeds: Vec<c_int> = vec![i32::MAX, i32::MIN];
    for k in 0..64 {
        seeds.push(i32::MAX - k);
        seeds.push(i32::MIN + k);
    }
    let mut rng = Rng::new(0x0E0C);
    for _ in 0..300 {
        seeds.push(i32::MAX - rng.below(8192) as i32);
        seeds.push(i32::MIN.wrapping_add(rng.below(8192) as i32));
    }
    for seed in seeds {
        for size in [1, 2, 3, 256, 1024, 4096] {
            let mut bc = vec![0i8; size as usize];
            let mut br = vec![0i8; size as usize];
            unsafe {
                (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, size, seed);
                (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, size, seed);
            }
            assert_eq!(bc, br, "row12: overflow seed={seed}, size={size}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — calculate_with_doubles with b == 0: division skipped, pow still runs
// ---------------------------------------------------------------------------

#[test]
fn err_calc_b_zero() {
    let p = apis();
    let mut rng = Rng::new(0x0E0D);
    let mut cs: Vec<c_int> = (-25..=25).collect();
    cs.extend([i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1]);
    for _ in 0..500 {
        cs.push(rng.spicy_i32());
    }
    for c in cs {
        for a in [0, 1, -1, i32::MIN, i32::MAX, 987654321, -987654321] {
            let rc = unsafe { (p.c.calculate_with_doubles)(a, 0, c) };
            let rr = unsafe { (p.rust.calculate_with_doubles)(a, 0, c) };
            assert_eq!(
                rc.to_bits(),
                rr.to_bits(),
                "row13: calculate_with_doubles({a}, 0, {c}) C={rc:?} Rust={rr:?}"
            );
            // The C skips the division but still multiplies, so the result is
            // exactly +0.0 regardless of `a` and `c`.
            assert_eq!(
                rc.to_bits(),
                0.0f64.to_bits(),
                "row13: expected +0.0 from C for ({a}, 0, {c}), got {rc:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — INT_MIN / -1 would trap as integer division; it must not happen
// ---------------------------------------------------------------------------

#[test]
fn err_calc_intmin_div_minus1() {
    let p = apis();
    for c in -12..=12 {
        let rc = unsafe { (p.c.calculate_with_doubles)(i32::MIN, -1, c) };
        let rr = unsafe { (p.rust.calculate_with_doubles)(i32::MIN, -1, c) };
        assert_eq!(
            rc.to_bits(),
            rr.to_bits(),
            "row14: calculate_with_doubles(INT_MIN, -1, {c}) C={rc:?} Rust={rr:?}"
        );
        assert!(rc.is_finite(), "row14: C produced {rc:?}");
    }
    // Same for the mirrored operand order and the neighbouring extremes.
    for (a, b) in [
        (i32::MIN, 1),
        (i32::MIN, -1),
        (i32::MAX, -1),
        (i32::MIN + 1, -1),
        (-1, i32::MIN),
        (1, i32::MIN),
    ] {
        for c in [-9, -1, 0, 1, 9, i32::MIN, i32::MAX] {
            let rc = unsafe { (p.c.calculate_with_doubles)(a, b, c) };
            let rr = unsafe { (p.rust.calculate_with_doubles)(a, b, c) };
            assert_eq!(rc.to_bits(), rr.to_bits(), "row14: ({a}, {b}, {c})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — negative `c`: `c % 10` truncates toward zero -> negative exponent
// ---------------------------------------------------------------------------

#[test]
fn err_calc_negative_exponent() {
    let p = apis();
    let mut rng = Rng::new(0x0E0F);
    for _ in 0..3000 {
        let c = -(1 + rng.below(1 << 30) as c_int);
        let a = rng.spicy_i32();
        let b = loop {
            let b = rng.spicy_i32();
            if b != 0 {
                break b;
            }
        };
        let rc = unsafe { (p.c.calculate_with_doubles)(a, b, c) };
        let rr = unsafe { (p.rust.calculate_with_doubles)(a, b, c) };
        assert_eq!(
            rc.to_bits(),
            rr.to_bits(),
            "row15: calculate_with_doubles({a}, {b}, {c}) C={rc:?} Rust={rr:?}"
        );
    }
    // The exponent really is negative (not the C99-illegal floor behaviour).
    let scaled = unsafe { (p.c.calculate_with_doubles)(1, 1, -1) };
    assert_eq!(scaled, 0.1f64, "row15: expected pow(10, -1), got {scaled:?}");
}

// ---------------------------------------------------------------------------
// Row 16 — c == INT_MIN / INT_MAX
// ---------------------------------------------------------------------------

#[test]
fn err_calc_c_extremes() {
    let p = apis();
    for c in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        for (a, b) in [(1, 1), (i32::MIN, -1), (i32::MAX, 3), (0, 7), (5, 0)] {
            let rc = unsafe { (p.c.calculate_with_doubles)(a, b, c) };
            let rr = unsafe { (p.rust.calculate_with_doubles)(a, b, c) };
            assert_eq!(
                rc.to_bits(),
                rr.to_bits(),
                "row16: calculate_with_doubles({a}, {b}, {c}) C={rc:?} Rust={rr:?}"
            );
        }
    }
    // INT_MIN % 10 == -8, INT_MAX % 10 == 7 (truncating remainder).
    let lo = unsafe { (p.c.calculate_with_doubles)(1, 1, i32::MIN) };
    let hi = unsafe { (p.c.calculate_with_doubles)(1, 1, i32::MAX) };
    assert_eq!(lo, 1e-8f64, "row16: INT_MIN exponent, got {lo:?}");
    assert_eq!(hi, 1e7f64, "row16: INT_MAX exponent, got {hi:?}");
}

// ---------------------------------------------------------------------------
// Rows 17-20 — convert_double_to_int's undefined-behaviour range
// ---------------------------------------------------------------------------

fn conv_pair(v: f64) -> (c_int, c_int) {
    let p = apis();
    unsafe {
        (
            (p.c.convert_double_to_int)(v),
            (p.rust.convert_double_to_int)(v),
        )
    }
}

#[test]
fn err_conv_out_of_range() {
    let mut rng = Rng::new(0x0E11);
    let mut values: Vec<f64> = vec![
        2147483648.0,
        -2147483649.0,
        1e10,
        -1e10,
        -1.0 * 2f64.powi(40),
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
    ];
    for _ in 0..3000 {
        let e = 31 + rng.below(990) as i32;
        let m = 1.0 + (rng.next_u64() as f64) / (u64::MAX as f64);
        let s = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        values.push(s * m * 2f64.powi(e));
    }
    for v in values {
        let (c, r) = conv_pair(v);
        assert_eq!(c, r, "row17: convert_double_to_int({v:?}) C={c} Rust={r}");
        assert_eq!(
            c,
            i32::MIN,
            "row17: C's out-of-range cast should yield INT_MIN for {v:?}"
        );
    }
}

#[test]
fn err_conv_infinities() {
    for v in [f64::INFINITY, f64::NEG_INFINITY] {
        let (c, r) = conv_pair(v);
        assert_eq!(c, r, "row18: convert_double_to_int({v:?}) C={c} Rust={r}");
        assert_eq!(c, i32::MIN, "row18: expected INT_MIN for {v:?}");
    }
}

#[test]
fn err_conv_nan() {
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0000), // quiet NaN
        f64::from_bits(0xfff8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7ff8_0000_dead_beef), // quiet NaN with payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xfff0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7fff_ffff_ffff_ffff),
        f64::from_bits(0xffff_ffff_ffff_ffff),
    ];
    for v in nans {
        assert!(v.is_nan(), "test bug: {:#018x} is not NaN", v.to_bits());
        let (c, r) = conv_pair(v);
        assert_eq!(
            c, r,
            "row19: convert_double_to_int(NaN {:#018x}) C={c} Rust={r}",
            v.to_bits()
        );
        assert_eq!(c, i32::MIN, "row19: expected INT_MIN for NaN");
    }
}

#[test]
fn err_conv_boundaries() {
    // Exactly one step past the valid range, and the last in-range values.
    let expectations: &[(f64, c_int)] = &[
        (2147483647.0, 2147483647),
        (2147483647.5, 2147483647),
        (2147483647.9999998, 2147483647),
        (2147483648.0, i32::MIN),
        (2147483649.0, i32::MIN),
        (-2147483648.0, i32::MIN),
        (-2147483648.5, i32::MIN),
        (-2147483648.9999998, i32::MIN),
        (-2147483649.0, i32::MIN),
        (-2147483650.0, i32::MIN),
    ];
    for &(v, expected) in expectations {
        let (c, r) = conv_pair(v);
        assert_eq!(c, r, "row20: convert_double_to_int({v:?}) C={c} Rust={r}");
        assert_eq!(c, expected, "row20: C returned {c} for {v:?}");
    }
    // Every representable double within one ulp of both bounds.
    for anchor in [2147483647.0f64, 2147483648.0, -2147483648.0, -2147483649.0] {
        let mut v = anchor;
        for _ in 0..8 {
            v = v.next_down();
        }
        for _ in 0..16 {
            let (c, r) = conv_pair(v);
            assert_eq!(c, r, "row20 ulp sweep: {v:?} ({:#018x})", v.to_bits());
            v = v.next_up();
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — the "out-of-range enum" analogue: the API declares no enum type, so
// the equivalent is sweeping arbitrary ints through every int parameter.
// ---------------------------------------------------------------------------

#[test]
fn err_int_parameter_sweep() {
    let p = apis();
    const PROBES: [c_int; 17] = [
        i32::MIN,
        i32::MIN + 1,
        -1000000,
        -257,
        -256,
        -255,
        -128,
        -1,
        0,
        1,
        42,
        127,
        128,
        255,
        256,
        1000000,
        i32::MAX,
    ];

    for &v in &PROBES {
        // process_negation
        let (c, r) = unsafe { ((p.c.process_negation)(v), (p.rust.process_negation)(v)) };
        assert_eq!(c, r, "row25 process_negation({v})");

        // find_value_in_buffer's search_val
        let buf: Vec<i8> = (0..256).map(|i| i as u8 as i8).collect();
        let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, v) };
        let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, 256, v) };
        assert_eq!(c, r, "row25 find_value_in_buffer(search_val={v})");

        // create_numeric_buffer's size (non-positive values must not write) and
        // seed.
        if v <= 0 {
            assert_create_leaves_buffer_alone(v, 12345);
        }
        let mut bc = vec![0i8; 300];
        let mut br = vec![0i8; 300];
        unsafe {
            (p.c.create_numeric_buffer)(bc.as_mut_ptr() as *mut c_char, 300, v);
            (p.rust.create_numeric_buffer)(br.as_mut_ptr() as *mut c_char, 300, v);
        }
        assert_eq!(bc, br, "row25 create_numeric_buffer(seed={v})");

        // calculate_with_doubles: each parameter position in turn.
        for &other in &PROBES {
            for (a, b, c_arg) in [(v, other, 3), (other, v, 3), (7, other, v)] {
                let rc = unsafe { (p.c.calculate_with_doubles)(a, b, c_arg) };
                let rr = unsafe { (p.rust.calculate_with_doubles)(a, b, c_arg) };
                assert_eq!(
                    rc.to_bits(),
                    rr.to_bits(),
                    "row25 calculate_with_doubles({a}, {b}, {c_arg})"
                );
            }
        }

        // find_value_in_buffer's size: only non-negative values are meaningful
        // for `size_t`, so sweep the small ones plus the byte boundaries.
        for size in [0usize, 1, 2, 128, 255, 256] {
            let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, v) };
            let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, v) };
            assert_eq!(c, r, "row25 find_value_in_buffer(size={size}, sv={v})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — process_negation has no rejection path at all
// ---------------------------------------------------------------------------

#[test]
fn err_process_negation_total() {
    let p = apis();
    let mut rng = Rng::new(0x0E1A);
    let mut values: Vec<c_int> = vec![0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..20_000 {
        values.push(rng.next_i32());
    }
    for v in values {
        let (c, r) = unsafe { ((p.c.process_negation)(v), (p.rust.process_negation)(v)) };
        assert_eq!(c, r, "row26 process_negation({v})");
        assert_eq!(
            c,
            i32::from(v != 0),
            "row26: C returned {c} for {v} (never an error code)"
        );
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary: oversized `size`.
//
// A `size` larger than the real allocation is UB in C, but `memchr` resolves
// before reading past the object whenever a match sits inside the allocation.
// The C returns that offset, so the Rust must too — this is the case a
// `slice::from_raw_parts`-based translation gets wrong (a length above
// `isize::MAX` is instant UB for the slice, while `memchr` just works).
// ---------------------------------------------------------------------------

#[test]
fn err_find_oversized_size_with_early_match() {
    let p = apis();
    // Match at index 0 so memchr stops immediately, whatever `size` claims.
    let mut buf = vec![0x11i8; 4096];
    buf[0] = 0x7A;
    for size in [
        buf.len(),
        buf.len() + 1,
        1usize << 20,
        1usize << 40,
        (isize::MAX as usize) - 1,
        isize::MAX as usize,
        usize::MAX / 2,
        usize::MAX - 1,
        usize::MAX,
    ] {
        let c = unsafe { (p.c.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, 0x7A) };
        let r = unsafe { (p.rust.find_value_in_buffer)(buf.as_ptr() as *const c_char, size, 0x7A) };
        assert_eq!(c, 0, "oversized size={size}: C should find index 0");
        assert_eq!(r, c, "oversized size={size} mismatch: C={c} Rust={r}");
    }

    // Match further in, still within the real allocation.
    for idx in [1usize, 7, 63, 1000, 4095] {
        let mut b = vec![0x11i8; 4096];
        b[idx] = 0x7A;
        for size in [1usize << 20, 1usize << 40, usize::MAX] {
            let c = unsafe { (p.c.find_value_in_buffer)(b.as_ptr() as *const c_char, size, 0x7A) };
            let r =
                unsafe { (p.rust.find_value_in_buffer)(b.as_ptr() as *const c_char, size, 0x7A) };
            assert_eq!(c, idx as c_int, "oversized size={size}, idx={idx}");
            assert_eq!(r, c, "oversized size={size}, idx={idx}: C={c} Rust={r}");
        }
    }
}
