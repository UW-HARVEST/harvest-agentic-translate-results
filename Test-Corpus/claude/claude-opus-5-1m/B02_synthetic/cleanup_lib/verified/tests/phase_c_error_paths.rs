//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E4) plus the generic FFI-boundary rows
//! (G1..G9). Each constructs the exact invalid input/condition, calls BOTH the
//! C `.so` and the Rust `.so`, and asserts the SAME rejection — the same return
//! value and the same stdout bytes, not merely "both failed somehow".

mod common;

use common::*;
use std::ffi::c_int;

fn call_cleanup(lib: &Lib, a: i32, b: i32, c: i32, d: i32) -> i64 {
    unsafe { (lib.cleanup)(a as c_int, b as c_int, c as c_int, d as c_int) as i64 }
}

/// Reference model of the C `switch`, including both fallthroughs.
/// `case 10 -> +10 then falls into case 20 -> +20` = +30.
/// `case 30 -> +30 then falls into case 40 -> +40` = +70.
fn model(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut r: i32 = 0;
    for v in [a, b, c, d] {
        r = match v {
            10 => r.wrapping_add(10).wrapping_add(20),
            20 => r.wrapping_add(20),
            30 => r.wrapping_add(30).wrapping_add(40),
            40 => r.wrapping_add(40),
            other => r.wrapping_add(other),
        };
    }
    r
}

// ======================================================= E1: dead validation branch

/// E1 — the `strncmp(input_str, expected_str, strlen(expected_str)) != 0` guard.
/// Both operands are the same `"VALID"` literal, so the branch is statically
/// unreachable. Assert its side effect never occurs in EITHER implementation.
fn e1_validation_branch_is_dead_in_both() {
    const MARKER: &[u8] = b"Input string validation failed.";
    let inputs: [[i32; 4]; 10] = [
        [0, 0, 0, 0],
        [10, 20, 30, 40],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [-1, -1, -1, -1],
        [7, 7, 7, 7],
        [9, 11, 39, 41],
        [40, 40, 40, 40],
        [1, -1, i32::MAX, i32::MIN],
        [30, 30, 30, 30],
    ];
    for v in inputs {
        diff_once("E1", &format!("{v:?}"), |lib| call_cleanup(lib, v[0], v[1], v[2], v[3]));
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| call_cleanup(lib, v[0], v[1], v[2], v[3]));
            assert!(
                !out.windows(MARKER.len()).any(|w| w == MARKER),
                "[E1] {} reached the dead validation branch for {v:?}: \"{}\"",
                lib.name,
                show(&out)
            );
        }
    }
}

// ========================================================= E2: dead malloc branch

/// E2 — the `if (!dynamic_str)` malloc-failure guard. A 50-byte request never
/// fails and the library exposes no injection hook, so the branch is
/// unreachable. Assert that neither implementation takes it: the failure marker
/// never appears and the success line ALWAYS does.
fn e2_malloc_failure_branch_is_dead_in_both() {
    const MARKER: &[u8] = b"Memory allocation failed.";
    const SUCCESS: &[u8] = b"Processed numbers: numbers\n";
    for i in 0..200u64 {
        let v = [
            rnd_i32(0xE2, i, 0),
            rnd_i32(0xE2, i, 1),
            rnd_i32(0xE2, i, 2),
            rnd_i32(0xE2, i, 3),
        ];
        for lib in [c_lib(), rust_lib()] {
            let (ret, out) = capture(|| call_cleanup(lib, v[0], v[1], v[2], v[3]));
            assert!(
                !out.windows(MARKER.len()).any(|w| w == MARKER),
                "[E2] {} reported malloc failure for {v:?}",
                lib.name
            );
            assert_eq!(
                out,
                SUCCESS,
                "[E2] {} did not emit the success line for {v:?}: \"{}\"",
                lib.name,
                show(&out)
            );
            // The full accumulator must survive to the return (no early exit).
            assert_eq!(
                ret,
                model(v[0], v[1], v[2], v[3]) as i64,
                "[E2] {} returned a partial result for {v:?}",
                lib.name
            );
        }
    }
}

// ================================================= E3/E4: cleanup_resources guard

/// E3 — NULL rejected by `if (dynamic_str)`: silent no-op, no crash, no output.
fn e3_cleanup_resources_null_is_noop() {
    diff_once("E3", "cleanup_resources(NULL)", |lib| {
        unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) };
        0
    });
    for lib in [c_lib(), rust_lib()] {
        let (_, out) = capture(|| unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) });
        assert!(
            out.is_empty(),
            "[E3] {} produced output for NULL: \"{}\"",
            lib.name,
            show(&out)
        );
    }
}

/// E3 — repeated NULL calls stay a no-op (no double-free path is entered).
fn e3_cleanup_resources_null_repeated() {
    diff_batch("E3-repeat", 1000, |lib, _| {
        unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) };
        0
    });
}

/// E4 — non-NULL passes the guard and is freed. Independent allocation per
/// library (freeing one pointer twice would be UB, not a differential test).
fn e4_cleanup_resources_frees_valid_pointer() {
    for &size in &[1usize, 8, 50, 64, 4096, 65536] {
        let (_, c_out) = capture(|| unsafe { (c_lib().cleanup_resources)(libc_malloc(size)) });
        let (_, r_out) = capture(|| unsafe { (rust_lib().cleanup_resources)(libc_malloc(size)) });
        assert_eq!(
            c_out,
            r_out,
            "[E4] size {size}: stdout differs (C=\"{}\", Rust=\"{}\")",
            show(&c_out),
            show(&r_out)
        );
        assert!(c_out.is_empty(), "[E4] size {size}: unexpected output");
    }
}

/// E5 — `cleanup_resources` must genuinely CALL `free`, not merely stay silent.
///
/// Closes a blind spot that pure stdout comparison cannot see: a
/// `cleanup_resources` that skips `free` leaks but prints exactly the same
/// nothing. Verified via glibc's LIFO tcache address-recycling probe.
fn e5_cleanup_resources_actually_frees() {
    let mut conclusive = 0;
    for &size in &[24usize, 50, 100, 200] {
        if !tcache_probe_usable(size) {
            eprintln!(
                "        [E5] size {size}: probe INCONCLUSIVE (this process's allocator \
                 does not exhibit LIFO reuse for this size class) — skipping"
            );
            continue;
        }
        let c_recycled = probe_frees(c_lib(), size);
        let r_recycled = probe_frees(rust_lib(), size);
        assert!(
            c_recycled,
            "[E5] size {size}: the allocator recycles addresses (precondition holds) yet \
             the C library's free() did not recycle — probe logic is wrong"
        );
        assert_eq!(
            c_recycled, r_recycled,
            "[E5] size {size}: C freed the pointer (address recycled={c_recycled}) but \
             Rust did not (recycled={r_recycled}) — cleanup_resources is leaking"
        );
        conclusive += 1;
    }
    assert!(
        conclusive > 0 || cfg!(not(debug_assertions)),
        "[E5] no size class produced a conclusive probe in a debug build — the \
         leak-detection row would be vacuous"
    );
}

/// E5b — the NULL guard must not be inverted. `free(NULL)` is a legal libc no-op,
/// so inverting `if (dynamic_str)` produces identical stdout while leaking every
/// real pointer; only the pairing of E5 (non-NULL is freed) with a silent,
/// crash-free NULL call pins the guard's polarity.
fn e5b_null_guard_polarity() {
    for lib in [c_lib(), rust_lib()] {
        let (_, out) = capture(|| unsafe { (lib.cleanup_resources)(std::ptr::null_mut()) });
        assert!(out.is_empty(), "[E5b] {}: NULL produced output", lib.name);
    }
    if !tcache_probe_usable(50) {
        eprintln!(
            "        [E5b] probe INCONCLUSIVE (no LIFO reuse in this process) — \
             the NULL half of the row still ran"
        );
        return;
    }
    for lib in [c_lib(), rust_lib()] {
        assert!(
            probe_frees(lib, 50),
            "[E5b] {}: non-NULL pointer was NOT freed — the `if (dynamic_str)` guard \
             looks inverted (free(NULL) is a no-op, so stdout alone cannot reveal this)",
            lib.name
        );
    }
}

/// E6 — `cleanup` must free the 50-byte buffer it allocates internally. A leak
/// there is invisible to stdout and to the return value, so it needs the heap
/// balance probe.
fn e6_cleanup_frees_its_internal_buffer() {
    if !tcache_probe_usable(50) {
        eprintln!(
            "        [E6] probe INCONCLUSIVE (this process's allocator does not \
             exhibit LIFO reuse for the 50-byte class) — skipping"
        );
        return;
    }
    let cases: [[i32; 4]; 4] = [[0, 0, 0, 0], [10, 20, 30, 40], [7, -7, 41, 9], [1, 2, 3, 4]];
    for args in cases {
        let c_ok = probe_cleanup_balanced(c_lib(), args);
        let r_ok = probe_cleanup_balanced(rust_lib(), args);
        assert!(
            c_ok,
            "[E6] {args:?}: allocator recycles (precondition holds) yet the C library \
             did not return its buffer to the 50-byte bin — probe logic is wrong"
        );
        assert_eq!(
            c_ok, r_ok,
            "[E6] {args:?}: C returns its internal malloc(50) to the heap (balanced={c_ok}) \
             but Rust does not (balanced={r_ok}) — cleanup is leaking its buffer"
        );
    }
}

// ============================================== G1..G6: print_result boundaries

/// G1 — NULL label forwarded to glibc `%s`, which prints `(null)`. The C never
/// null-checks, so the Rust must forward the null pointer unchanged.
fn g1_print_result_null_label() {
    for result in [0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
        diff_once("G1", &format!("print_result(NULL, {result})"), |lib| {
            unsafe { (lib.print_result)(std::ptr::null(), result as c_int) };
            0
        });
    }
    // Pin the absolute expectation too.
    for lib in [c_lib(), rust_lib()] {
        let (_, out) = capture(|| unsafe { (lib.print_result)(std::ptr::null(), 7) });
        assert_eq!(
            out,
            b"(null): 7\n",
            "[G1] {}: expected \"(null): 7\\n\", got \"{}\"",
            lib.name,
            show(&out)
        );
    }
}

/// G2 — zero-length label.
fn g2_print_result_empty_label() {
    for result in [0i32, -1, i32::MIN, i32::MAX] {
        diff_once("G2", &format!("empty label, {result}"), |lib| {
            let buf = cstr(b"");
            unsafe { (lib.print_result)(buf.as_ptr(), result as c_int) };
            0
        });
    }
    for lib in [c_lib(), rust_lib()] {
        let (_, out) = capture(|| {
            let buf = cstr(b"");
            unsafe { (lib.print_result)(buf.as_ptr(), 0) };
        });
        assert_eq!(out, b": 0\n", "[G2] {}: got \"{}\"", lib.name, show(&out));
    }
}

/// G3 — oversized labels (4 KiB / 64 KiB) crossing glibc's buffer boundary.
fn g3_print_result_oversized_label() {
    for &n in &[4096usize, 65536] {
        let label = vec![b'Z'; n];
        diff_once("G3", &format!("{n}-byte label"), |lib| {
            let buf = cstr(&label);
            unsafe { (lib.print_result)(buf.as_ptr(), -7) };
            0
        });
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| {
                let buf = cstr(&label);
                unsafe { (lib.print_result)(buf.as_ptr(), -7) };
            });
            let mut expect = label.clone();
            expect.extend_from_slice(b": -7\n");
            assert_eq!(out, expect, "[G3] {}: {n}-byte label mismatch", lib.name);
        }
    }
}

/// G4 — label holding conversion specifiers must be printed literally.
fn g4_print_result_label_with_format_specifiers() {
    let labels: [&[u8]; 6] = [b"%s", b"%d", b"%n", b"%%", b"%s%s%s%n", b"50% done"];
    for l in labels {
        diff_once("G4", &format!("label {:?}", show(l)), |lib| {
            let buf = cstr(l);
            unsafe { (lib.print_result)(buf.as_ptr(), 3) };
            0
        });
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| {
                let buf = cstr(l);
                unsafe { (lib.print_result)(buf.as_ptr(), 3) };
            });
            let mut expect = l.to_vec();
            expect.extend_from_slice(b": 3\n");
            assert_eq!(
                out,
                expect,
                "[G4] {}: label {} was interpreted, not printed literally: \"{}\"",
                lib.name,
                show(l),
                show(&out)
            );
        }
    }
}

/// G5 — control bytes and non-UTF-8 bytes must pass through byte-for-byte.
fn g5_print_result_non_utf8_and_control_bytes() {
    // All 128 high bytes individually.
    diff_batch("G5-high", 128, |lib, i| {
        let byte = 0x80u8 + i as u8;
        let label = [byte, byte, b'x', byte];
        let buf = cstr(&label);
        unsafe { (lib.print_result)(buf.as_ptr(), i as c_int) };
        0
    });
    // Control bytes 0x01..=0x1f individually.
    diff_batch("G5-ctrl", 31, |lib, i| {
        let byte = 0x01u8 + i as u8;
        let label = [b'a', byte, b'b'];
        let buf = cstr(&label);
        unsafe { (lib.print_result)(buf.as_ptr(), -(i as c_int)) };
        0
    });
    // Truncated / invalid UTF-8 multibyte sequences.
    let bad: [&[u8]; 6] = [
        b"\xc3",             // lone lead byte
        b"\xe2\x82",         // truncated 3-byte
        b"\xf0\x9f\x92",     // truncated 4-byte
        b"\xff\xfe\xfd",     // never-valid
        b"\xed\xa0\x80",     // surrogate
        b"a\x80b\xc0c\xf5d", // mixed
    ];
    for l in bad {
        diff_once("G5-bad", &show(l), |lib| {
            let buf = cstr(l);
            unsafe { (lib.print_result)(buf.as_ptr(), 1) };
            0
        });
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| {
                let buf = cstr(l);
                unsafe { (lib.print_result)(buf.as_ptr(), 1) };
            });
            let mut expect = l.to_vec();
            expect.extend_from_slice(b": 1\n");
            assert_eq!(out, expect, "[G5] {}: bytes {} corrupted", lib.name, show(l));
        }
    }
}

/// G6 — `result` at the signed extremes (one step past is unrepresentable).
fn g6_print_result_int_extremes() {
    let cases: [(i32, &[u8]); 6] = [
        (i32::MIN, b"L: -2147483648\n"),
        (i32::MIN + 1, b"L: -2147483647\n"),
        (-1, b"L: -1\n"),
        (0, b"L: 0\n"),
        (i32::MAX - 1, b"L: 2147483646\n"),
        (i32::MAX, b"L: 2147483647\n"),
    ];
    for (result, expect) in cases {
        diff_once("G6", &format!("result={result}"), |lib| {
            let buf = cstr(b"L");
            unsafe { (lib.print_result)(buf.as_ptr(), result as c_int) };
            0
        });
        for lib in [c_lib(), rust_lib()] {
            let (_, out) = capture(|| {
                let buf = cstr(b"L");
                unsafe { (lib.print_result)(buf.as_ptr(), result as c_int) };
            });
            assert_eq!(
                out,
                expect,
                "[G6] {}: result={result} expected \"{}\", got \"{}\"",
                lib.name,
                show(expect),
                show(&out)
            );
        }
    }
}

// ================================== G7..G9: cleanup selector / overflow / fallthrough

/// G7 — "out-of-range enum" values for the `switch`. A C `switch` accepts any
/// `int`, so every value with no matching case label is a real input that must
/// fall to `default`. One step past every case label, in both directions.
fn g7_cleanup_off_by_one_around_every_case_label() {
    // Each near-miss placed in each of the 4 slots, with the others neutral (0).
    let near = [
        -41, -40, -39, -31, -30, -29, -21, -20, -19, -11, -10, -9, -1, 0, 1, 9, 11, 19, 21, 29, 31,
        39, 41, 50, 100,
    ];
    diff_batch("G7", near.len() * 4, |lib, i| {
        let v = near[i % near.len()];
        let slot = i / near.len();
        let mut args = [0i32; 4];
        args[slot] = v;
        call_cleanup(lib, args[0], args[1], args[2], args[3])
    });

    // And confirm none of these accidentally hit a case label.
    for &v in &near {
        let (cr, _) = capture(|| call_cleanup(c_lib(), v, 0, 0, 0));
        assert_eq!(
            cr, v as i64,
            "[G7] C: value {v} did not take the default branch (got {cr})"
        );
        let (rr, _) = capture(|| call_cleanup(rust_lib(), v, 0, 0, 0));
        assert_eq!(cr, rr, "[G7] value {v}: C={cr} Rust={rr}");
    }
}

/// G7 — the signed extremes as switch selectors.
fn g7_cleanup_int_extremes() {
    let ext = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    diff_batch("G7-ext", ext.len() * 4, |lib, i| {
        let v = ext[i % ext.len()];
        let slot = i / ext.len();
        let mut args = [0i32; 4];
        args[slot] = v;
        call_cleanup(lib, args[0], args[1], args[2], args[3])
    });
}

/// G8 — accumulator overflow past `INT_MAX` and below `INT_MIN` must wrap
/// identically (C is built at `-O0`; the Rust uses `wrapping_add`).
fn g8_cleanup_overflow_wraps_identically() {
    let cases: [[i32; 4]; 12] = [
        [i32::MAX, i32::MAX, 0, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, 0, 0],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, 1, 0, 0],
        [i32::MIN, -1, 0, 0],
        [i32::MAX, 10, 0, 0],   // case 10 adds 30 on top of INT_MAX
        [i32::MIN, 30, 0, 0],   // case 30 adds 70
        [i32::MAX, 40, 40, 40], // repeated case 40
        [2_000_000_000, 2_000_000_000, 0, 0],
        [-2_000_000_000, -2_000_000_000, 0, 0],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
    ];
    for v in cases {
        diff_once("G8", &format!("{v:?}"), |lib| call_cleanup(lib, v[0], v[1], v[2], v[3]));
        // Cross-check against the reference model of the C semantics.
        let (cr, _) = capture(|| call_cleanup(c_lib(), v[0], v[1], v[2], v[3]));
        assert_eq!(
            cr,
            model(v[0], v[1], v[2], v[3]) as i64,
            "[G8] C result for {v:?} disagrees with the reference model"
        );
    }

    // Randomized full-width overflow sweep.
    diff_batch("G8-random", 3000, |lib, i| {
        let i = i as u64;
        call_cleanup(
            lib,
            rnd_i32(0x51, i, 0),
            rnd_i32(0x51, i, 1),
            rnd_i32(0x51, i, 2),
            rnd_i32(0x51, i, 3),
        )
    });
}

/// G9 — fallthrough vs. break at every case label, isolated per slot.
/// `10 -> +30`, `20 -> +20`, `30 -> +70`, `40 -> +40`.
fn g9_cleanup_fallthrough_semantics_per_case() {
    let expected: [(i32, i64); 4] = [(10, 30), (20, 20), (30, 70), (40, 40)];
    for (label, delta) in expected {
        for slot in 0..4usize {
            let mut args = [0i32; 4];
            args[slot] = label;
            diff_once("G9", &format!("case {label} in slot {slot}"), |lib| {
                call_cleanup(lib, args[0], args[1], args[2], args[3])
            });
            let (cr, _) = capture(|| call_cleanup(c_lib(), args[0], args[1], args[2], args[3]));
            assert_eq!(
                cr, delta,
                "[G9] C: case {label} in slot {slot} contributed {cr}, expected {delta}"
            );
            let (rr, _) = capture(|| call_cleanup(rust_lib(), args[0], args[1], args[2], args[3]));
            assert_eq!(rr, delta, "[G9] Rust: case {label} in slot {slot} contributed {rr}");
        }
    }

    // All four labels at once: 30 + 20 + 70 + 40 = 160.
    diff_once("G9", "all four case labels", |lib| call_cleanup(lib, 10, 20, 30, 40));
    let (cr, _) = capture(|| call_cleanup(c_lib(), 10, 20, 30, 40));
    assert_eq!(cr, 160, "[G9] C: cleanup(10,20,30,40) should be 160, got {cr}");
}

// ==================================================================== driver

/// Single `#[test]` entry point — see `common::run_rows` for why the rows are
/// not separate `#[test]`s (fd 1 is process-global during captures).
#[test]
fn phase_c_all_error_rows() {
    let rows: &[(&str, fn())] = &[
        ("E1 dead string-validation branch", e1_validation_branch_is_dead_in_both),
        ("E2 dead malloc-failure branch", e2_malloc_failure_branch_is_dead_in_both),
        ("E3 cleanup_resources(NULL) no-op", e3_cleanup_resources_null_is_noop),
        ("E3 cleanup_resources(NULL) repeated", e3_cleanup_resources_null_repeated),
        ("E4 cleanup_resources frees valid pointer", e4_cleanup_resources_frees_valid_pointer),
        ("E5 cleanup_resources actually frees", e5_cleanup_resources_actually_frees),
        ("E5b NULL guard polarity", e5b_null_guard_polarity),
        ("E6 cleanup frees internal buffer", e6_cleanup_frees_its_internal_buffer),
        ("G1 print_result NULL label", g1_print_result_null_label),
        ("G2 print_result empty label", g2_print_result_empty_label),
        ("G3 print_result oversized label", g3_print_result_oversized_label),
        ("G4 print_result format specifiers literal", g4_print_result_label_with_format_specifiers),
        ("G5 print_result non-UTF8 / control bytes", g5_print_result_non_utf8_and_control_bytes),
        ("G6 print_result int extremes", g6_print_result_int_extremes),
        ("G7 cleanup off-by-one around case labels", g7_cleanup_off_by_one_around_every_case_label),
        ("G7 cleanup int extremes as selectors", g7_cleanup_int_extremes),
        ("G8 cleanup overflow wraps identically", g8_cleanup_overflow_wraps_identically),
        ("G9 cleanup fallthrough per case", g9_cleanup_fallthrough_semantics_per_case),
    ];
    run_rows("Phase C (ERRORS.md)", rows);
}
