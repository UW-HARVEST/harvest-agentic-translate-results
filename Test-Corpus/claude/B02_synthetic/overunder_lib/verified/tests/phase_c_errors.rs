// Phase C -- error-path differential tests, one test per row of ERRORS.md.
//
// Rows E19-E24 and E26 (the ones that go through `overunder`, which prints)
// live in tests/phase_overunder.rs.
//
// Both implementations are reached only through `dlopen`/`dlsym`.

mod common;
use common::*;

const INT_MAX: i32 = 2147483647;
const INT_MIN: i32 = -2147483648;

/// Assert both libraries return exactly `want` (the same sentinel, not merely
/// "both failed somehow").
fn assert_sdti_eq(d: f64, want: i32, row: &str) {
    let (c, r) = both();
    let cv = unsafe { (c.safe_double_to_int)(d) };
    let rv = unsafe { (r.safe_double_to_int)(d) };
    assert_eq!(
        cv, want,
        "{row}: C returned {cv} for d={d:?} (bits={:#018x}), expected {want}",
        d.to_bits()
    );
    assert_eq!(
        rv, want,
        "{row}: Rust returned {rv} for d={d:?} (bits={:#018x}), expected {want}",
        d.to_bits()
    );
}

fn assert_process_eq(code: i32, base: i32, want: i32, row: &str) {
    let (c, r) = both();
    let cv = unsafe { (c.process_with_fallthrough)(code, base) };
    let rv = unsafe { (r.process_with_fallthrough)(code, base) };
    assert_eq!(cv, want, "{row}: C returned {cv} for code={code} base={base}");
    assert_eq!(
        rv, want,
        "{row}: Rust returned {rv} for code={code} base={base}"
    );
}

/// nextafter without depending on newer std APIs.
fn ulp_step(x: f64, up: bool) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return if up { f64::from_bits(1) } else { -f64::from_bits(1) };
    }
    let bits = x.to_bits();
    let newbits = if (x > 0.0) == up { bits + 1 } else { bits - 1 };
    f64::from_bits(newbits)
}

// ===========================================================================
// safe_double_to_int -- E1 .. E9
// ===========================================================================

/// E1 (`d > (double)INT_MAX`) and E3 (one ULP above the boundary).
#[test]
fn err_e1_e3_clamp_high() {
    // E3: exactly one ULP above (double)INT_MAX
    let one_ulp = ulp_step(2147483647.0, true);
    assert!(one_ulp > 2147483647.0);
    assert_sdti_eq(one_ulp, INT_MAX, "E3");

    // E1: a spread of magnitudes above the boundary
    for d in [
        2147483647.5,
        2147483648.0,
        2147483649.0,
        4294967296.0,
        1e15,
        1e300,
        f64::MAX,
    ] {
        assert_sdti_eq(d, INT_MAX, "E1");
    }
    let mut rng = Rng::for_test("E1");
    for _ in 0..5000 {
        let d = rng.range_f64(2147483647.0000005, 1e18);
        assert_sdti_eq(d, INT_MAX, "E1-random");
    }
}

/// E2: `+INFINITY` satisfies `d > INT_MAX`.
#[test]
fn err_e2_pos_infinity() {
    assert_sdti_eq(f64::INFINITY, INT_MAX, "E2");
    assert_sdti_eq(f64::from_bits(0x7FF0_0000_0000_0000), INT_MAX, "E2-bits");
}

/// E4 (`d < (double)INT_MIN`) and E6 (one ULP below the boundary).
#[test]
fn err_e4_e6_clamp_low() {
    // E6: exactly one ULP below (double)INT_MIN
    let one_ulp = ulp_step(-2147483648.0, false);
    assert!(one_ulp < -2147483648.0);
    assert_sdti_eq(one_ulp, INT_MIN, "E6");

    // E4
    for d in [
        -2147483648.5,
        -2147483649.0,
        -4294967296.0,
        -1e15,
        -1e300,
        f64::MIN,
    ] {
        assert_sdti_eq(d, INT_MIN, "E4");
    }
    let mut rng = Rng::for_test("E4");
    for _ in 0..5000 {
        let d = rng.range_f64(-1e18, -2147483648.0000005);
        assert_sdti_eq(d, INT_MIN, "E4-random");
    }
}

/// E5: `-INFINITY` satisfies `d < INT_MIN`.
#[test]
fn err_e5_neg_infinity() {
    assert_sdti_eq(f64::NEG_INFINITY, INT_MIN, "E5");
    assert_sdti_eq(f64::from_bits(0xFFF0_0000_0000_0000), INT_MIN, "E5-bits");
}

/// E7: NaN reaches the `isnan` arm because every relational test is false.
#[test]
fn err_e7_nan_variants() {
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN, minimal payload
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FF7_FFFF_FFFF_FFFF), // signalling NaN, max payload
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // quiet NaN, all payload bits
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ];
    for d in nans {
        assert!(d.is_nan());
        assert_sdti_eq(d, 0, "E7");
    }
    // random NaN payloads
    let mut rng = Rng::for_test("E7");
    let mut n = 0;
    while n < 2000 {
        let bits = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 0x7FF0_0000_0000_0000;
        let d = f64::from_bits(bits | (rng.next_u64() & 0x8000_0000_0000_0000));
        if !d.is_nan() {
            continue; // was an infinity encoding
        }
        assert_sdti_eq(d, 0, "E7-random");
        n += 1;
    }
}

/// E8 / E9: the *inclusive* boundaries are NOT rejected -- they go through
/// `(int)d`, which happens to produce the same number as the clamp would.
#[test]
fn err_e8_e9_inrange_boundaries() {
    // (double)INT_MAX is exactly representable, so `d > (double)INT_MAX` is false
    assert_eq!(2147483647.0f64 as i64, 2147483647i64);
    assert_sdti_eq(2147483647.0, INT_MAX, "E8");
    assert_sdti_eq(-2147483648.0, INT_MIN, "E9");
    // one ULP *inside* each boundary
    assert_sdti_eq(ulp_step(2147483647.0, false), 2147483646, "E8-inside");
    assert_sdti_eq(ulp_step(-2147483648.0, true), -2147483647, "E9-inside");
    // and the neighbouring integers
    assert_sdti_eq(2147483646.0, 2147483646, "E8-neighbour");
    assert_sdti_eq(-2147483647.0, -2147483647, "E9-neighbour");
    // truncation direction at the extremes
    assert_sdti_eq(2147483646.9999998, 2147483646, "E8-trunc");
    assert_sdti_eq(-2147483647.9999998, -2147483647, "E9-trunc");
}

// ===========================================================================
// process_with_fallthrough -- E10 .. E15
// ===========================================================================

/// E10: negative `code` has no `case`, so `default:` returns the sentinel -1.
#[test]
fn err_e10_negative_code() {
    let mut rng = Rng::for_test("E10");
    for code in [-1i32, -2, -3, -4, -5, -6, -7, -100, INT_MIN, INT_MIN + 1] {
        for base in [0, 1, -1, INT_MAX, INT_MIN, 12345] {
            assert_process_eq(code, base, -1, "E10");
        }
        for _ in 0..200 {
            assert_process_eq(code, rng.next_i32(), -1, "E10-random");
        }
    }
    for _ in 0..3000 {
        assert_process_eq(rng.range_i32(INT_MIN, -1), rng.next_i32(), -1, "E10-sweep");
    }
}

/// E11: `code >= 6` has no `case`, so `default:` returns the sentinel -1.
#[test]
fn err_e11_code_above_range() {
    let mut rng = Rng::for_test("E11");
    for code in [6i32, 7, 8, 9, 10, 100, 12345, INT_MAX - 1, INT_MAX] {
        for base in [0, 1, -1, INT_MAX, INT_MIN, 12345] {
            assert_process_eq(code, base, -1, "E11");
        }
        for _ in 0..200 {
            assert_process_eq(code, rng.next_i32(), -1, "E11-random");
        }
    }
    for _ in 0..3000 {
        assert_process_eq(rng.range_i32(6, INT_MAX), rng.next_i32(), -1, "E11-sweep");
    }
}

/// E12: out-of-range "enum" values crossing the FFI boundary. A C `switch` on
/// `int` accepts any `int`, including values with no valid variant, so these are
/// real inputs the Rust `match` must handle identically.
#[test]
fn err_e12_ffi_out_of_range_enum() {
    let no_variant = [
        INT_MIN,
        INT_MIN + 1,
        INT_MIN / 2,
        -1_000_000,
        -7,
        -6,
        -1,
        6,
        7,
        1_000_000,
        INT_MAX / 2,
        INT_MAX - 1,
        INT_MAX,
    ];
    for code in no_variant {
        assert_process_eq(code, 0, -1, "E12");
        assert_process_eq(code, INT_MAX, -1, "E12");
        assert_process_eq(code, INT_MIN, -1, "E12");
    }
    // Exhaustively confirm which `code` values DO have a variant: only 0..=5.
    // `base_value = 1000` is chosen so no valid arm can coincidentally produce
    // the -1 sentinel, making "hit the default arm" observable.
    let (c, _) = both();
    for code in -64..=64i32 {
        let got = unsafe { (c.process_with_fallthrough)(code, 1000) };
        let has_variant = (0..=5).contains(&code);
        if has_variant {
            let want = match code {
                5 => 1150,
                4 => 1100,
                3 => 1060,
                2 => 1030,
                1 => 1010,
                _ => 0, // case 0
            };
            assert_eq!(got, want, "E12: code={code} is a valid variant");
        } else {
            assert_eq!(
                got, -1,
                "E12: code={code} has no variant and must hit the default arm"
            );
        }
        // ...and the Rust side must agree for every one of them.
        diff_process(code, 1000, "E12-exhaustive");
    }
}

/// E13: `case 0` discards `base_value` entirely.
#[test]
fn err_e13_code_zero_discards_base() {
    let mut rng = Rng::for_test("E13");
    for base in [0, 1, -1, INT_MAX, INT_MIN, INT_MAX - 1, INT_MIN + 1] {
        assert_process_eq(0, base, 0, "E13");
    }
    for _ in 0..5000 {
        assert_process_eq(0, rng.next_i32(), 0, "E13-random");
    }
}

/// E14 / E15: signed overflow and underflow across the fall-through chain.
#[test]
fn err_e14_fallthrough_overflow() {
    // Deltas straight from the C switch.
    let deltas = [(5i32, 150i32), (4, 100), (3, 60), (2, 30), (1, 10)];
    for (code, delta) in deltas {
        // E14: every base_value within `delta` of INT_MAX overflows.
        for k in 0..=delta {
            let base = INT_MAX - k;
            assert_process_eq(code, base, base.wrapping_add(delta), "E14");
        }
        // E15: the underflow end (adding a positive delta cannot underflow, but
        // the extreme value must still match exactly).
        for k in 0..=delta {
            let base = INT_MIN + k;
            assert_process_eq(code, base, base.wrapping_add(delta), "E15");
        }
    }
    // explicit spot checks of the wrapped values
    assert_process_eq(5, INT_MAX, INT_MIN + 149, "E14-wrap5");
    assert_process_eq(1, INT_MAX, INT_MIN + 9, "E14-wrap1");
    assert_process_eq(1, INT_MIN, INT_MIN + 10, "E15-min1");
    assert_process_eq(5, INT_MIN, INT_MIN + 150, "E15-min5");
}

// ===========================================================================
// copy_data_block -- E16 .. E18 / C23
// ===========================================================================

/// The child half of E16: performs the unchecked NULL `memcpy` so the parent can
/// observe how the process dies. Marked `#[ignore]` so it only runs when the
/// parent re-execs this binary with `HARVEST_FAULT_CASE` set.
#[test]
#[ignore]
fn helper_child_null_fault() {
    let spec = match std::env::var("HARVEST_FAULT_CASE") {
        Ok(s) => s,
        Err(_) => return, // not the child; do nothing
    };
    let (which, case) = spec.split_once(':').expect("HARVEST_FAULT_CASE=<impl>:<case>");
    let im = match which {
        "c" => c_impl(),
        "rust" => rust_impl(),
        other => panic!("unknown impl {other}"),
    };
    let arena = Arena::new(DATABLOCK_SIZE);
    let (dest, src): (*mut u8, *const u8) = match case {
        "both" => (std::ptr::null_mut(), std::ptr::null()),
        "null_dest" => (std::ptr::null_mut(), arena.at(0) as *const u8),
        "null_src" => (arena.at(0), std::ptr::null()),
        other => panic!("unknown case {other}"),
    };
    eprintln!("CHILD_READY {spec}");
    unsafe { (im.copy_data_block)(dest, src) };
    // If we get here the call did NOT fault.
    println!("CHILD_SURVIVED {spec}");
}

struct ChildOutcome {
    code: Option<i32>,
    signal: Option<i32>,
    output: String,
}

fn run_null_fault_child(spec: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "helper_child_null_fault",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HARVEST_FAULT_CASE", spec)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    ChildOutcome {
        code: out.status.code(),
        signal: out.status.signal(),
        output: text,
    }
}

/// E16: `copy_data_block` has no NULL check (the C dereferences both pointers
/// unconditionally at line 78). The Rust must be equally unchecked: it must NOT
/// silently return, and it must NOT convert the fault into a Rust panic or a
/// library-precondition abort -- it has to die exactly the way the C does.
#[test]
fn err_e16_null_pointers_fault_identically() {
    for case in ["both", "null_dest", "null_src"] {
        let c = run_null_fault_child(&format!("c:{case}"));
        let r = run_null_fault_child(&format!("rust:{case}"));

        assert!(
            c.output.contains("CHILD_READY"),
            "E16[{case}]: C child never reached the call:\n{}",
            c.output
        );
        assert!(
            r.output.contains("CHILD_READY"),
            "E16[{case}]: Rust child never reached the call:\n{}",
            r.output
        );

        // Neither may survive the call.
        assert!(
            !c.output.contains("CHILD_SURVIVED"),
            "E16[{case}]: C survived the NULL memcpy"
        );
        assert!(
            !r.output.contains("CHILD_SURVIVED"),
            "E16[{case}]: Rust survived the NULL memcpy -- it must not silently \
             skip the copy when the C would fault.\n{}",
            r.output
        );

        // The Rust must fault, not panic / trip a std precondition assertion.
        for bad in [
            "panicked at",
            "unsafe precondition",
            "not implemented",
            "unimplemented",
            "attempt to",
        ] {
            assert!(
                !r.output.contains(bad),
                "E16[{case}]: Rust reported `{bad}` instead of faulting like the C:\n{}",
                r.output
            );
        }

        // Same termination mechanism, same signal number.
        assert_eq!(
            c.signal, r.signal,
            "E16[{case}]: different fatal signal -- C={:?}/{:?} Rust={:?}/{:?}\n--- C ---\n{}\n--- Rust ---\n{}",
            c.signal, c.code, r.signal, r.code, c.output, r.output
        );
        assert_eq!(
            c.code, r.code,
            "E16[{case}]: different exit code -- C={:?} Rust={:?}",
            c.code, r.code
        );
        assert_eq!(
            c.signal,
            Some(11),
            "E16[{case}]: expected SIGSEGV from the C, got signal={:?} code={:?}\n{}",
            c.signal,
            c.code,
            c.output
        );
    }
}

/// E17: `dest == src` (aliasing). The C `memcpy` is called with identical
/// pointers; contents must be unchanged.
#[test]
fn err_e17_dest_equals_src() {
    let (c, r) = both();
    let mut rng = Rng::for_test("E17");

    let run = |f: FnCopyDataBlock, bytes: &[u8]| -> Vec<u8> {
        let a = Arena::new(DATABLOCK_SIZE + 16);
        a.fill(0x5A);
        a.write(0, bytes);
        let p = a.at(0);
        unsafe { f(p, p as *const u8) };
        a.read()
    };

    for i in 0..500 {
        let mut bytes = [0u8; DATABLOCK_SIZE];
        rng.fill(&mut bytes);
        let cv = run(c.copy_data_block, &bytes);
        let rv = run(r.copy_data_block, &bytes);
        assert_eq!(cv, rv, "E17 divergence at iteration {i}");
        assert_eq!(&cv[..DATABLOCK_SIZE], &bytes[..], "E17: contents changed");
        assert!(
            cv[DATABLOCK_SIZE..].iter().all(|&b| b == 0x5A),
            "E17: wrote past the struct"
        );
    }
    // and the all-zero / all-ones extremes
    for fill in [0x00u8, 0xFFu8] {
        let bytes = [fill; DATABLOCK_SIZE];
        assert_eq!(
            run(c.copy_data_block, &bytes),
            run(r.copy_data_block, &bytes),
            "E17 divergence for fill {fill:#04x}"
        );
    }
}

/// E18 / C23: exactly `sizeof(DataBlock) == 40` bytes are copied -- byte 39 must
/// be written and byte 40 must not.
#[test]
fn err_e18_copies_exactly_40_bytes() {
    let (c, r) = both();
    let mut rng = Rng::for_test("E18");
    const ARENA: usize = 96;

    let run = |f: FnCopyDataBlock, payload: &[u8], sentinel: u8| -> Vec<u8> {
        let dst = Arena::new(ARENA);
        dst.fill(sentinel);
        let src = Arena::new(DATABLOCK_SIZE);
        src.write(0, payload);
        unsafe { f(dst.at(0), src.at(0) as *const u8) };
        dst.read()
    };

    for i in 0..300 {
        // A payload with no byte equal to the sentinel, so "changed" is exact.
        let sentinel = 0x00u8;
        let mut payload = [0u8; DATABLOCK_SIZE];
        for b in payload.iter_mut() {
            *b = loop {
                let v = rng.next_u8();
                if v != sentinel {
                    break v;
                }
            };
        }
        let cv = run(c.copy_data_block, &payload, sentinel);
        let rv = run(r.copy_data_block, &payload, sentinel);
        assert_eq!(cv, rv, "E18 divergence at iteration {i}");

        let changed = cv.iter().position(|&b| b == sentinel).unwrap_or(ARENA);
        assert_eq!(changed, DATABLOCK_SIZE, "E18: copied {changed} bytes, not 40");
        assert_eq!(&cv[..DATABLOCK_SIZE], &payload[..], "E18: payload mismatch");
        assert!(
            cv[DATABLOCK_SIZE..].iter().all(|&b| b == sentinel),
            "E18: bytes past offset 40 were modified"
        );
    }
    // Also verify a zero-payload copy still touches exactly 40 bytes.
    let cv = run(c.copy_data_block, &[0u8; DATABLOCK_SIZE], 0xFF);
    let rv = run(r.copy_data_block, &[0u8; DATABLOCK_SIZE], 0xFF);
    assert_eq!(cv, rv, "E18 divergence (zero payload)");
    assert!(cv[..DATABLOCK_SIZE].iter().all(|&b| b == 0));
    assert!(cv[DATABLOCK_SIZE..].iter().all(|&b| b == 0xFF));
}

// ===========================================================================
// handle_pointer_operations -- E25
// ===========================================================================

/// E25: `value * 2` and `+ 100` overflow with no guard whatsoever.
#[test]
fn err_e25_hpo_extremes() {
    let (c, r) = both();
    let interesting = [
        INT_MAX,
        INT_MAX - 1,
        INT_MAX - 49,
        INT_MAX - 50,
        INT_MAX / 2,
        INT_MAX / 2 + 1,
        1_073_741_773, // value*2 + 100 == INT_MAX + 1 exactly
        1_073_741_774,
        1_073_741_823,
        1_073_741_824,
        INT_MIN,
        INT_MIN + 1,
        INT_MIN / 2,
        INT_MIN / 2 - 1,
        -1_073_741_824,
        -1_073_741_825,
        0,
        -50,
        -51,
    ];
    for v in interesting {
        let cv = unsafe { (c.handle_pointer_operations)(v) };
        let rv = unsafe { (r.handle_pointer_operations)(v) };
        assert_eq!(cv, rv, "E25: divergence for value={v} (C={cv} Rust={rv})");
        assert_eq!(
            cv,
            v.wrapping_mul(2).wrapping_add(100),
            "E25: C is not two's-complement wrapping for value={v}"
        );
    }
    let mut rng = Rng::for_test("E25");
    for _ in 0..20000 {
        diff_hpo(rng.next_i32(), "E25-sweep");
    }
}

// ===========================================================================
// Generic FFI boundary checks (required even though not in ERRORS.md)
// ===========================================================================

/// Every exported symbol must exist in BOTH `.so`s -- i.e. `dlsym` succeeds.
/// (`common::load` panics on a missing symbol, so merely getting here proves it,
/// but assert explicitly for the record.)
#[test]
fn generic_all_symbols_resolvable_in_both() {
    let (c, r) = both();
    for im in [c, r] {
        // Non-null function pointers, exercised once each.
        let _ = unsafe { (im.safe_double_to_int)(1.0) };
        let _ = unsafe { (im.process_with_fallthrough)(1, 1) };
        let _ = unsafe { (im.handle_pointer_operations)(1) };
        let a = Arena::new(DATABLOCK_SIZE);
        let b = Arena::new(DATABLOCK_SIZE);
        unsafe { (im.copy_data_block)(a.at(0), b.at(0) as *const u8) };
    }
}

/// Zero and "oversized" length analogues: this API takes no length parameter,
/// so the equivalent boundaries are the zero value and the extreme values of
/// every scalar argument. Sweep all of them through all four leaf functions.
#[test]
fn generic_zero_and_extreme_scalars() {
    for v in [0i32, 1, -1, INT_MAX, INT_MIN] {
        diff_process(v, 0, "generic");
        diff_process(0, v, "generic");
        diff_process(v, v, "generic");
        diff_hpo(v, "generic");
        diff_safe_double_to_int(v as f64, "generic");
    }
    for d in [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
    ] {
        diff_safe_double_to_int(d, "generic-f64");
    }
}

/// One step past each documented valid range, for every ranged parameter.
#[test]
fn generic_one_past_valid_ranges() {
    // process_with_fallthrough: valid `code` range is 0..=5
    for code in [-1i32, 0, 5, 6] {
        for base in [INT_MIN, -1, 0, 1, INT_MAX] {
            diff_process(code, base, "generic-past-range");
        }
    }
    // safe_double_to_int: valid range is [(double)INT_MIN, (double)INT_MAX]
    for d in [
        ulp_step(-2147483648.0, false),
        -2147483648.0,
        ulp_step(-2147483648.0, true),
        ulp_step(2147483647.0, false),
        2147483647.0,
        ulp_step(2147483647.0, true),
    ] {
        diff_safe_double_to_int(d, "generic-past-range");
    }
}
