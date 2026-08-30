//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! This library has NO error returns, no sentinels, no asserts and no
//! rejection branches (see `ERRORS.md` for the mechanical derivation). Both
//! public functions are `void`. So "same error/rejection" here means:
//!
//!   * neither library rejects the input (both return normally), AND
//!   * both produce byte-identical output, AND
//!   * the Rust does not panic/abort/trap where the C wraps.
//!
//! The last point is the real risk: `bedrooms += extra` and `floors++` are
//! signed `int` arithmetic that overflows. The C (built at `-O0`, no
//! `-ftrapv`/UBSan) wraps. These tests run against the DEBUG Rust cdylib,
//! which has `overflow-checks = on`, so a translation using `+` instead of
//! `wrapping_add` would abort here and be caught.

mod common;
use common::*;

const INT_MAX: i32 = i32::MAX;
const INT_MIN: i32 = i32::MIN;

/// ERRORS.md row 1 — `run(INT_MAX)`: maximum in-range `int`.
/// Expected C result: no rejection, wrapping add, 4 lines, returns void.
#[test]
fn row1_run_int_max() {
    let mut h = lock();
    let before = h.bedrooms();
    let out = h.run(INT_MAX, "ERRORS row1");
    assert_eq!(
        h.bedrooms(),
        before.wrapping_add(INT_MAX),
        "must wrap, not saturate and not trap"
    );
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 4);
}

/// ERRORS.md row 2 — `run(INT_MIN)`: minimum in-range `int`.
#[test]
fn row2_run_int_min() {
    let mut h = lock();
    let before = h.bedrooms();
    let out = h.run(INT_MIN, "ERRORS row2");
    assert_eq!(h.bedrooms(), before.wrapping_add(INT_MIN));
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 4);
}

/// ERRORS.md row 3 — arbitrary "out-of-range" bit patterns reinterpreted as
/// `int`. The header documents no valid range, so every bit pattern is a real
/// input the C accepts. This is the analogue of passing an out-of-range enum
/// value across the FFI boundary: there is no enum in this API, and `int`
/// has no invalid representation, so the correct behaviour is "accept all".
#[test]
fn row3_arbitrary_bit_patterns() {
    let mut h = lock();
    let patterns: [u32; 14] = [
        0x0000_0000,
        0x0000_0001,
        0x0000_00FF,
        0x0000_7FFF,
        0x0000_8000,
        0x7FFF_FFFF, // INT_MAX
        0x8000_0000, // INT_MIN
        0x8000_0001,
        0xFFFF_FFFF, // -1
        0xFFFF_FFFE, // -2
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0xAAAA_AAAA,
        0x5555_5555,
    ];
    for p in patterns {
        let v = p as i32; // reinterpret, exactly as C would receive it
        let before = h.bedrooms();
        h.run(v, &format!("ERRORS row3 pattern=0x{p:08X} as int={v}"));
        assert_eq!(
            h.bedrooms(),
            before.wrapping_add(v),
            "bit pattern 0x{p:08X} must be accepted verbatim"
        );
    }
}

/// ERRORS.md row 4 — signed OVERFLOW of the `bedrooms` accumulator.
/// C is UB here; observed behaviour is two's-complement wrap to negative.
/// The Rust must reproduce the wrap and must not panic.
#[test]
fn row4_bedrooms_overflow_wraps() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 0xE4);
    for i in 0..60 {
        // Park exactly at INT_MAX, then add a positive value -> must wrap.
        let to_max = INT_MAX.wrapping_sub(h.bedrooms());
        h.run(to_max, &format!("ERRORS row4 park i={i}"));
        assert_eq!(h.bedrooms(), INT_MAX);

        let add = rng.range_i32(1, 1_000_000);
        h.run(add, &format!("ERRORS row4 overflow i={i} add={add}"));
        assert_eq!(
            h.bedrooms(),
            INT_MAX.wrapping_add(add),
            "overflow must wrap two's-complement"
        );
        assert!(h.bedrooms() < 0, "INT_MAX + {add} must wrap negative");
    }
}

/// ERRORS.md row 5 — signed UNDERFLOW of the `bedrooms` accumulator.
#[test]
fn row5_bedrooms_underflow_wraps() {
    let mut h = lock();
    let mut rng = Rng::new(SEED ^ 0xE5);
    for i in 0..60 {
        let to_min = INT_MIN.wrapping_sub(h.bedrooms());
        h.run(to_min, &format!("ERRORS row5 park i={i}"));
        assert_eq!(h.bedrooms(), INT_MIN);

        let sub = -rng.range_i32(1, 1_000_000);
        h.run(sub, &format!("ERRORS row5 underflow i={i} sub={sub}"));
        assert_eq!(
            h.bedrooms(),
            INT_MIN.wrapping_add(sub),
            "underflow must wrap two's-complement"
        );
        assert!(h.bedrooms() > 0, "INT_MIN + {sub} must wrap positive");
    }
}

/// ERRORS.md row 6 — overflow of the `floors` counter (`house->floors++`).
///
/// STRUCTURAL: reaching `floors == INT_MAX` needs ~2^31 public calls, which is
/// not feasible in test time. So this asserts the *mechanism*: the C uses a
/// plain `++` (which wraps at `-O0`) and the Rust must use `wrapping_add(1)`
/// rather than `+ 1` / `checked_add` / `saturating_add`. A `+ 1` would abort
/// the DEBUG cdylib on overflow and a `saturating_add` would diverge.
#[test]
fn row6_floors_counter_wraps_structural() {
    let c = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/driver.c"),
    )
    .expect("read C source");
    let rs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read Rust source");

    assert!(
        c.contains("house->floors++"),
        "C increments floors with ++ (wraps at -O0)"
    );
    assert!(
        rs.contains("floors.wrapping_add(1)"),
        "Rust must use wrapping_add(1) for floors to match C's wrapping ++"
    );
    assert!(
        rs.contains("bedrooms.wrapping_add(extra_bedrooms)"),
        "Rust must use wrapping_add for bedrooms to match C's wrapping +="
    );
    // Neither saturating nor checked arithmetic may be used: they would diverge.
    for bad in ["saturating_add", "checked_add", "unwrap_or(i32::MAX)"] {
        assert!(
            !rs.contains(bad),
            "Rust must not use `{bad}` — C wraps, it does not saturate/reject"
        );
    }

    // And the reachable part of the same mechanism really does wrap: exercise a
    // wrapping ++ path indirectly by confirming many increments stay in lockstep.
    let mut h = lock();
    let f0 = h.floors();
    for i in 0..32 {
        h.run(0, &format!("ERRORS row6 floors i={i}"));
    }
    assert_eq!(h.floors(), f0.wrapping_add(32));
}

/// ERRORS.md row 7 — extremes through the `driver` wrapper, which applies the
/// value TWICE, so the accumulator overflow of rows 4/5 happens inside a
/// single public call.
#[test]
fn row7_driver_applies_value_twice() {
    let mut h = lock();
    for &v in &[INT_MAX, INT_MIN, -1, 1, 0, INT_MAX / 2, INT_MIN / 2] {
        let before = h.bedrooms();
        let out = h.driver(v, &format!("ERRORS row7 v={v}"));
        assert_eq!(
            h.bedrooms(),
            before.wrapping_add(v).wrapping_add(v),
            "driver({v}) must apply the value twice, wrapping"
        );
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            8,
            "driver prints 8 lines"
        );
    }

    // Park at INT_MAX so a single driver(1) overflows mid-call, between its
    // two internal run() invocations.
    let to_max = INT_MAX.wrapping_sub(h.bedrooms());
    h.run(to_max, "ERRORS row7 park at INT_MAX");
    assert_eq!(h.bedrooms(), INT_MAX);
    h.driver(1, "ERRORS row7 overflow inside driver");
    assert_eq!(h.bedrooms(), INT_MAX.wrapping_add(1).wrapping_add(1));
}

/// ERRORS.md row 8 — there is NO reset entry point, so "already-used global
/// state" is an unavoidable input condition. Neither library may
/// re-initialise; both must accumulate identically across mixed calls.
#[test]
fn row8_no_reset_state_accumulates() {
    let mut h = lock();

    // No exported reset/init/free symbol exists to restore pristine state.
    let c_lib = unsafe { libloading::Library::new(c_so_path()).unwrap() };
    let r_lib = unsafe { libloading::Library::new(rust_so_path()).unwrap() };
    for name in [
        b"reset\0".as_ref(),
        b"init\0".as_ref(),
        b"driver_init\0".as_ref(),
        b"driver_reset\0".as_ref(),
        b"the_house\0".as_ref(),
        b"add_floor\0".as_ref(),
        b"print_the_house\0".as_ref(),
    ] {
        let in_c = unsafe { c_lib.get::<*mut ()>(name) }.is_ok();
        let in_r = unsafe { r_lib.get::<*mut ()>(name) }.is_ok();
        assert_eq!(
            in_c,
            in_r,
            "symbol {:?} visibility must match (C={in_c}, Rust={in_r})",
            String::from_utf8_lossy(&name[..name.len() - 1])
        );
        assert!(
            !in_c,
            "{:?} must NOT be exported (it is `static` in the C)",
            String::from_utf8_lossy(&name[..name.len() - 1])
        );
    }

    // State must persist, not reset, across a mixed sequence.
    let f0 = h.floors();
    let b0 = h.bathrooms();
    h.run(3, "ERRORS row8 a");
    h.driver(4, "ERRORS row8 b");
    h.run(5, "ERRORS row8 c");
    assert_eq!(h.floors(), f0.wrapping_add(4), "4 floors added, no reset");
    assert_eq!(h.bathrooms(), b0 + 4.0, "4 bathrooms added, no reset");
    assert_ne!(
        (h.floors(), h.bathrooms()),
        (2, 2.5),
        "state must not have been re-initialised"
    );
}

/// ERRORS.md row 9 — `%.1f` formatting parity for ALL `bathrooms` magnitudes,
/// including ones not reachable in test time.
///
/// Rather than trying to grow the double to 2^53, this asserts the mechanism:
/// both libraries pass a `double` to the SAME libc `printf` with the SAME
/// format-string bytes, so formatting is identical by construction for every
/// possible value. Verified by comparing the literal embedded in each `.so`
/// and each library's `printf` import.
#[test]
fn row9_bathrooms_large_magnitude_formatting() {
    let c_bytes = std::fs::read(c_so_path()).expect("read C .so");
    let r_bytes = std::fs::read(rust_so_path()).expect("read Rust .so");

    let needle = FMT.as_bytes();
    let find = |hay: &[u8]| hay.windows(needle.len()).any(|w| w == needle);

    assert!(
        find(&c_bytes),
        "C .so must embed the format string {FMT:?}"
    );
    assert!(
        find(&r_bytes),
        "Rust .so must embed the IDENTICAL format string {FMT:?} — \
         a re-implemented formatter (e.g. println!(\"{{:.1}}\")) would diverge \
         for values printf rounds differently"
    );

    // Both must delegate to libc printf rather than formatting themselves.
    let nm = |p: std::path::PathBuf| {
        let out = std::process::Command::new("nm")
            .args(["-D", "--undefined-only"])
            .arg(p)
            .output()
            .expect("run nm");
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    assert!(
        nm(c_so_path()).contains("printf"),
        "C .so imports printf"
    );
    assert!(
        nm(rust_so_path()).contains("printf"),
        "Rust .so must import libc printf so formatting is identical by construction"
    );

    // Sanity: drive bathrooms across a decimal-width change and compare bytes.
    let mut h = lock();
    while h.bathrooms() < 105.0 {
        h.run(0, "ERRORS row9 grow bathrooms");
    }
    let out = h.run(0, "ERRORS row9 3-digit bathrooms");
    let s = String::from_utf8_lossy(&out);
    assert!(
        s.contains(".5 bathrooms"),
        "bathrooms stays .5-exact: {s:?}"
    );
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries required by Phase C even though the table
// has no row for them (this API takes no pointers and no enums).
// ---------------------------------------------------------------------------

/// The public API takes NO pointers, so there is no null-pointer case to test
/// at the boundary — assert that mechanically against the C header so this
/// stays true if the surface ever changes.
#[test]
fn generic_no_pointer_or_enum_in_public_api() {
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read C header");
    let decls: Vec<&str> = hdr
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .filter(|l| l.contains('(') && l.contains(';'))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "header declares exactly one function, got {decls:?}"
    );
    assert!(decls[0].contains("void driver(int x)"));
    assert!(
        !decls[0].contains('*'),
        "no pointer parameter => no null-pointer error path"
    );

    let c = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/driver.c"),
    )
    .expect("read C source");
    assert!(!c.contains("enum "), "no enum => no out-of-range variant path");
    // And the exported `run` also takes a single plain int.
    assert!(c.contains("void run(int extra_bedrooms)"));
}

/// "One step past the valid range": for `int` there is no representable step
/// past INT_MAX/INT_MIN, so the boundary is the wrap itself. Sweep the full
/// neighbourhood of both extremes through BOTH entry points.
#[test]
fn generic_one_step_past_int_extremes() {
    let mut h = lock();
    let neighbourhood = [
        INT_MAX,
        INT_MAX - 1,
        INT_MAX - 2,
        INT_MIN,
        INT_MIN + 1,
        INT_MIN + 2,
        0,
        -1,
        1,
    ];
    for &v in &neighbourhood {
        let before = h.bedrooms();
        h.run(v, &format!("generic extremes run({v})"));
        assert_eq!(h.bedrooms(), before.wrapping_add(v));

        let before = h.bedrooms();
        h.driver(v, &format!("generic extremes driver({v})"));
        assert_eq!(h.bedrooms(), before.wrapping_add(v).wrapping_add(v));
    }
}

/// Zero and "oversized" argument values: `run(0)` is the zero-length analogue
/// and the int extremes are the oversized analogue. Confirm repeated zero is
/// idempotent on `bedrooms` in BOTH libraries.
#[test]
fn generic_zero_and_oversized_values() {
    let mut h = lock();
    let before = h.bedrooms();
    for i in 0..16 {
        h.run(0, &format!("generic zero run i={i}"));
        h.driver(0, &format!("generic zero driver i={i}"));
    }
    assert_eq!(h.bedrooms(), before, "zero must be idempotent on bedrooms");

    // Oversized: the largest-magnitude values an `int` can carry.
    for &v in &[INT_MAX, INT_MIN] {
        let before = h.bedrooms();
        h.run(v, &format!("generic oversized {v}"));
        assert_eq!(h.bedrooms(), before.wrapping_add(v));
    }
}
