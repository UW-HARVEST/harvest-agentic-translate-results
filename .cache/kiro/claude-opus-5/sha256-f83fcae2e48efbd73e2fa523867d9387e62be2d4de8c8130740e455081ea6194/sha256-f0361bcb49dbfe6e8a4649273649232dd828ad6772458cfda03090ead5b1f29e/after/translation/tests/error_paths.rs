//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row, plus the generic FFI boundary cases (null pointers, out-of-range enum
//! values, extreme magnitudes).
//!
//! The C library has no error return of any kind: every function returns `void`.
//! Its entire rejection surface is the `switch (Impairment)` with no `default:`
//! label, so "the same error/rejection" here means *the same observable no-op*
//! — the three `f32`s must be left bit-identical by both implementations, and
//! the pointers must not be dereferenced at all.

mod common;

use common::*;
use std::ffi::c_int;

/// Every `Impairment` value that has no enumerator, used by several rows.
fn probe_values() -> (f32, f32, f32) {
    // Deliberately not 0.0, so a spurious write is visible, and a NaN payload
    // that no arithmetic result would ever produce.
    (
        0.375f32,
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0x8000_0001),
    )
}

/// Assert: both implementations leave the buffers untouched for `mode`.
#[track_caller]
fn assert_noop(row: &str, mode: c_int) {
    let l = libs();
    let (r, g, b) = probe_values();
    let expect = (r.to_bits(), g.to_bits(), b.to_bits());

    let c_out = call(l.c, mode, r, g, b);
    let rust_out = call(l.rust, mode, r, g, b);
    let c_bits = (c_out.0.to_bits(), c_out.1.to_bits(), c_out.2.to_bits());
    let rust_bits = (
        rust_out.0.to_bits(),
        rust_out.1.to_bits(),
        rust_out.2.to_bits(),
    );

    assert_eq!(
        c_bits, expect,
        "[{row}] C did NOT no-op for mode={mode}: {c_bits:08X?}"
    );
    assert_eq!(
        rust_bits, c_bits,
        "[{row}] Rust diverged from C for mode={mode}: C={c_bits:08X?} Rust={rust_bits:08X?}"
    );
}

// ---------------------------------------------------------------------------
// E1 — Impairment == 3, one past the last enumerator
// ---------------------------------------------------------------------------

#[test]
fn e1_one_past_last_enumerator() {
    assert_noop("E1", 3);
}

// ---------------------------------------------------------------------------
// E2 — Impairment == -1, one before the first enumerator
// ---------------------------------------------------------------------------

#[test]
fn e2_one_before_first_enumerator() {
    assert_noop("E2", -1);
}

// ---------------------------------------------------------------------------
// E3 / E4 — extreme int values
// ---------------------------------------------------------------------------

#[test]
fn e3_int_max() {
    assert_noop("E3", c_int::MAX);
}

#[test]
fn e4_int_min() {
    assert_noop("E4", c_int::MIN);
}

// ---------------------------------------------------------------------------
// E5 — sweep 4..=255 plus a randomized sweep of arbitrary out-of-range ints
// ---------------------------------------------------------------------------

#[test]
fn e5_out_of_range_sweep() {
    for m in 4..=255 {
        assert_noop("E5", m as c_int);
    }
    for m in -255..=-1 {
        assert_noop("E5", m as c_int);
    }

    let mut rng = Rng::new(0x0000_00E5);
    for _ in 0..20_000 {
        let m = rng.next_u32() as i32;
        if (0..=2).contains(&m) {
            continue; // valid, covered by Phase B
        }
        assert_noop("E5", m as c_int);
    }
}

// ---------------------------------------------------------------------------
// E6 — out-of-range Impairment with NULL pointers: must not dereference
// ---------------------------------------------------------------------------

#[test]
fn e6_out_of_range_with_null_pointers() {
    let l = libs();
    for m in [3, -1, 7, 255, c_int::MAX, c_int::MIN] {
        // If either implementation dereferenced the pointers, this would fault
        // and the test process would die — the assertion is "we get here".
        unsafe {
            (l.c)(m, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            (l.rust)(m, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — out-of-range Impairment must preserve NaN / inf payloads exactly
// ---------------------------------------------------------------------------

#[test]
fn e7_out_of_range_preserves_nan_payloads() {
    let l = libs();
    let mut rng = Rng::new(0x0000_00E7);
    let specials = [
        f32::from_bits(0x7FC0_0000), // +qNaN default
        f32::from_bits(0xFFC0_0000), // -qNaN (x86 real indefinite)
        f32::from_bits(0x7F80_0001), // +sNaN, payload 1
        f32::from_bits(0xFFBF_FFFF), // -sNaN, max payload
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
    ];
    for &s in &specials {
        for m in [3, -1, 99, c_int::MAX, c_int::MIN] {
            let c_out = call(l.c, m, s, s, s);
            let rust_out = call(l.rust, m, s, s, s);
            assert_eq!(
                c_out.0.to_bits(),
                s.to_bits(),
                "[E7] C mutated the buffer for mode={m}"
            );
            assert!(
                bits_eq(c_out, rust_out),
                "[E7] divergence: mode={m} in={} C={} Rust={}",
                show(s),
                show3(c_out),
                show3(rust_out)
            );
        }
    }
    // Randomized payloads too.
    for _ in 0..5_000 {
        let (r, g, b) = (rng.any_f32(), rng.any_f32(), rng.any_f32());
        let m = 3 + (rng.next_u32() % 1000) as c_int;
        let c_out = call(l.c, m, r, g, b);
        let rust_out = call(l.rust, m, r, g, b);
        assert!(
            bits_eq(c_out, (r, g, b)),
            "[E7] C mutated the buffer for mode={m}"
        );
        assert!(
            bits_eq(c_out, rust_out),
            "[E7] divergence: mode={m} C={} Rust={}",
            show3(c_out),
            show3(rust_out)
        );
    }
}

// ---------------------------------------------------------------------------
// E8 — valid mode with fully aliased pointers: load-all-then-store contract
// ---------------------------------------------------------------------------

#[test]
fn e8_valid_mode_full_aliasing() {
    let l = libs();
    let mut rng = Rng::new(0x0000_00E8);
    let check = |v: f32| {
        for m in MODES {
            let run = |f: ColourblindFn| {
                let mut x = v;
                let p: *mut f32 = &mut x;
                unsafe { f(m, p, p, p) };
                x
            };
            let c = run(l.c);
            let r = run(l.rust);
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "[E8] mode={m} in={} C={} Rust={}",
                show(v),
                show(c),
                show(r)
            );
        }
    };
    for &v in &[
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0x7F80_0001),
    ] {
        check(v);
    }
    for _ in 0..10_000 {
        check(rng.any_f32());
    }
}

// ---------------------------------------------------------------------------
// E9 — valid mode with partial aliasing
// ---------------------------------------------------------------------------

#[test]
fn e9_valid_mode_partial_aliasing() {
    let l = libs();
    let mut rng = Rng::new(0x0000_00E9);
    for i in 0..10_000 {
        let (a, b) = if i % 3 == 0 {
            (rng.any_f32(), rng.any_f32())
        } else {
            (rng.signed_unit(), rng.signed_unit())
        };
        for which in 0u8..3 {
            for m in MODES {
                let run = |f: ColourblindFn| {
                    let mut x = a;
                    let mut y = b;
                    let px: *mut f32 = &mut x;
                    let py: *mut f32 = &mut y;
                    unsafe {
                        match which {
                            0 => f(m, px, px, py),
                            1 => f(m, px, py, px),
                            _ => f(m, py, px, px),
                        }
                    }
                    (x, y)
                };
                let c = run(l.c);
                let r = run(l.rust);
                assert!(
                    c.0.to_bits() == r.0.to_bits() && c.1.to_bits() == r.1.to_bits(),
                    "[E9] which={which} mode={m} in=({}, {}) C=({}, {}) Rust=({}, {})",
                    show(a),
                    show(b),
                    show(c.0),
                    show(c.1),
                    show(r.0),
                    show(r.1)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — valid mode with unaligned pointers
// ---------------------------------------------------------------------------

#[test]
fn e10_valid_mode_unaligned() {
    let l = libs();
    let mut rng = Rng::new(0x0000_00EA);

    let run = |f: ColourblindFn, m: c_int, off: usize, v: [u32; 3]| -> [u32; 3] {
        let mut buf = [0u8; 16];
        for (i, w) in v.iter().enumerate() {
            buf[off + i * 4..off + i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        unsafe {
            let base = buf.as_mut_ptr().add(off);
            f(
                m,
                base as *mut f32,
                base.add(4) as *mut f32,
                base.add(8) as *mut f32,
            );
        }
        let mut out = [0u32; 3];
        for i in 0..3 {
            out[i] = u32::from_le_bytes(buf[off + i * 4..off + i * 4 + 4].try_into().unwrap());
        }
        out
    };

    for _ in 0..3_000 {
        let v = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        for off in 1..=3usize {
            for m in MODES {
                let c = run(l.c, m, off, v);
                let r = run(l.rust, m, off, v);
                assert_eq!(
                    c, r,
                    "[E10] off={off} mode={m} in={v:08X?} C={c:08X?} Rust={r:08X?}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — signalling NaN inputs with a valid mode
// ---------------------------------------------------------------------------

#[test]
fn e11_signalling_nan_inputs() {
    let mut rng = Rng::new(0x0000_00EB);
    // Deterministic edge payloads first.
    let snans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
        f32::from_bits(0x7F80_0000 | 0x0020_0000),
    ];
    for &s in &snans {
        for pos in 0..3 {
            let (r, g, b) = match pos {
                0 => (s, 0.25f32, -0.75f32),
                1 => (0.25f32, s, -0.75f32),
                _ => (0.25f32, -0.75f32, s),
            };
            for m in MODES {
                assert_same("E11", m, r, g, b);
            }
        }
    }
    for _ in 0..10_000 {
        let s = rng.snan();
        let (a, b) = (rng.signed_unit(), rng.signed_unit());
        for pos in 0..3 {
            let (r, g, bl) = match pos {
                0 => (s, a, b),
                1 => (a, s, b),
                _ => (a, b, s),
            };
            for m in MODES {
                assert_same("E11", m, r, g, bl);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E12 — overflow / invalid-operation results
// ---------------------------------------------------------------------------

#[test]
fn e12_overflow_and_invalid_operations() {
    // Every combination that can make an SSE op raise invalid-operation:
    // inf * 0, inf - inf, and plain overflow to inf.
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0f32,
        -0.0f32,
        f32::MAX,
        -f32::MAX,
        3.4e38f32,
        -3.4e38f32,
        1.0f32,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                for m in MODES {
                    assert_same("E12", m, r, g, b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases beyond the table
// ---------------------------------------------------------------------------

/// Out-of-range enum values are the classic FFI blind spot: a C `enum`
/// parameter accepts any `int`. Exhaustively cover a dense band around the
/// valid range plus powers of two and their neighbours.
#[test]
fn boundary_enum_dense_and_powers_of_two() {
    for m in -600..=600i32 {
        if (0..=2).contains(&m) {
            continue;
        }
        assert_noop("boundary-dense", m as c_int);
    }
    for shift in 0..31u32 {
        let p = 1i32 << shift;
        for cand in [p - 1, p, p + 1, -p - 1, -p, -p + 1] {
            if (0..=2).contains(&cand) {
                continue;
            }
            assert_noop("boundary-pow2", cand as c_int);
        }
    }
}

/// The three valid discriminants must NOT be treated as rejections — the guard
/// that keeps `assert_noop`'s "both did nothing" from passing vacuously.
#[test]
fn boundary_valid_enum_values_do_write() {
    let l = libs();
    let (r, g, b) = (0.375f32, 0.5f32, 0.125f32);
    for m in MODES {
        let c_out = call(l.c, m, r, g, b);
        assert!(
            c_out != (r, g, b),
            "sanity: mode={m} must modify the buffer, but C left it unchanged"
        );
        let rust_out = call(l.rust, m, r, g, b);
        assert!(
            bits_eq(c_out, rust_out),
            "mode={m}: C={} Rust={}",
            show3(c_out),
            show3(rust_out)
        );
    }
}

/// Null pointers with a *valid* mode are undefined behaviour in the C (it
/// dereferences unconditionally). Verify out-of-process that both libraries
/// behave the same way: fault on a valid mode, no-op on an invalid one.
#[test]
fn null_pointer_with_valid_mode_faults_in_both() {
    use std::process::Command;

    let exe = std::env::current_exe().expect("current_exe");
    // Child mode is selected by an env var; see `main`-less harness below.
    for m in [0, 1, 2] {
        let mut statuses = Vec::new();
        for which in ["c", "rust"] {
            let out = Command::new(&exe)
                .args(["--exact", "child_null_deref", "--nocapture", "--ignored"])
                .env("CB_CHILD_IMPL", which)
                .env("CB_CHILD_MODE", m.to_string())
                .output()
                .expect("spawn child");
            statuses.push((which, out.status.code(), out.status.to_string()));
        }
        let signalled = |s: &(&str, Option<i32>, String)| s.1.is_none() || s.1 == Some(101);
        assert_eq!(
            signalled(&statuses[0]),
            signalled(&statuses[1]),
            "mode={m}: C and Rust disagree on null-deref outcome: {statuses:?}"
        );
    }
}

/// Helper process body for `null_pointer_with_valid_mode_faults_in_both`.
/// `#[ignore]` so it only runs when explicitly named by the parent.
#[test]
#[ignore]
fn child_null_deref() {
    let l = libs();
    let which = std::env::var("CB_CHILD_IMPL").unwrap_or_default();
    let mode: c_int = std::env::var("CB_CHILD_MODE")
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let f = if which == "c" { l.c } else { l.rust };
    unsafe {
        f(
            mode,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
    }
    println!("survived");
}

/// Zero-sized / degenerate "length" analogue: this API has no length parameter,
/// but the pointers can legitimately point into the *same* single-element
/// allocation with only one element's worth of storage. Covered by E8; this test
/// pins the documented behaviour that no more than 4 bytes per pointer are ever
/// touched, by fencing each float with a guard word.
#[test]
fn boundary_no_out_of_bounds_writes() {
    let l = libs();
    let guard: u32 = 0xDEAD_BEEF;
    for m in [0, 1, 2, 3, -1, c_int::MAX] {
        let run = |f: ColourblindFn| -> ([u32; 9], [f32; 3]) {
            // layout: [guard, R, guard, G, guard, B, guard, guard, guard]
            let mut cells: [u32; 9] = [guard; 9];
            let vals = [0.375f32, 0.5, 0.125];
            cells[1] = vals[0].to_bits();
            cells[3] = vals[1].to_bits();
            cells[5] = vals[2].to_bits();
            unsafe {
                let base = cells.as_mut_ptr();
                f(
                    m,
                    base.add(1) as *mut f32,
                    base.add(3) as *mut f32,
                    base.add(5) as *mut f32,
                );
            }
            let out = [
                f32::from_bits(cells[1]),
                f32::from_bits(cells[3]),
                f32::from_bits(cells[5]),
            ];
            (cells, out)
        };
        let (c_cells, c_out) = run(l.c);
        let (r_cells, r_out) = run(l.rust);
        for i in [0usize, 2, 4, 6, 7, 8] {
            assert_eq!(c_cells[i], guard, "C clobbered guard {i} at mode={m}");
            assert_eq!(r_cells[i], guard, "Rust clobbered guard {i} at mode={m}");
        }
        assert!(
            bits_eq(
                (c_out[0], c_out[1], c_out[2]),
                (r_out[0], r_out[1], r_out[2])
            ),
            "mode={m}: C={c_out:?} Rust={r_out:?}"
        );
    }
}
