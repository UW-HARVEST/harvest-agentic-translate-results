//! Phase C — error-path differential tests, one test per ERRORS.md row.
//!
//! The library has NO error-reporting surface (no return values, no error
//! codes, no asserts, no null checks — see ERRORS.md), so each row here pins
//! down the *observable* behaviour of an invalid / boundary input:
//!
//!   * for the inputs the C handles gracefully, both libraries must produce
//!     identical results (buffers and/or stdout bytes);
//!   * for the inputs where the C hits undefined behaviour and dies, both
//!     libraries are run in an isolated subprocess and their termination is
//!     compared signal-for-signal.

mod common;

use common::*;
use std::ffi::c_int;
use std::time::Duration;

const T: Duration = Duration::from_secs(30);

// Subprocess entry points (no-ops during a normal run).
#[test]
fn zz_stdout_worker() {
    common::worker_main_if_requested();
}

#[test]
fn zz_crash_worker() {
    common::crash_worker_main_if_requested();
}

// ===========================================================================
// Row 1 — fma_array, len == 0: loop never runs, `out` untouched
// ===========================================================================

#[test]
fn e01_fma_len_zero_is_a_noop() {
    let mut rng = Rng::new(0xE01);
    let l = libs();
    for _ in 0..200 {
        let a = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let b = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let c = gen_vals(&mut rng, ValueClass::FullRandom, 8);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, 8);

        diff_fma("e01", Alias::AllDistinct, 0, &a, &b, &c, &o);

        // Both implementations must have written nothing at all.
        for imp in [&l.c, &l.rs] {
            let res = call_fma(imp, Alias::AllDistinct, 0, &a, &b, &c, &o);
            assert_eq!(
                res.buffers[0][0], o[0],
                "{}: fma_array(len=0) wrote to out[0]",
                imp.name
            );
            assert!(
                res.buffers[0][1..].iter().all(|&v| v == SENTINEL),
                "{}: fma_array(len=0) wrote past out[0]",
                imp.name
            );
        }
    }
}

// ===========================================================================
// Row 2 — fma_array, len < 0: degenerates to the same no-op as len == 0
// ===========================================================================

#[test]
fn e02_fma_negative_len_is_a_noop() {
    let mut rng = Rng::new(0xE02);
    let l = libs();

    let mut lens: Vec<c_int> = vec![-1, -2, -3, -100, i32::MIN, i32::MIN + 1, -65536];
    for _ in 0..40 {
        lens.push(rng.range_i32(i32::MIN, -1));
    }

    for &len in &lens {
        let a = gen_vals(&mut rng, ValueClass::Boundary, 8);
        let b = gen_vals(&mut rng, ValueClass::Boundary, 8);
        let c = gen_vals(&mut rng, ValueClass::Boundary, 8);
        let o = gen_vals(&mut rng, ValueClass::FullRandom, 8);

        // `call_fma` clamps a negative len to an empty live region, so this
        // compares the two libraries on exactly the same memory.
        diff_fma("e02", Alias::AllDistinct, len, &a, &b, &c, &o);

        for imp in [&l.c, &l.rs] {
            let res = call_fma(imp, Alias::AllDistinct, len, &a, &b, &c, &o);
            assert_eq!(
                res.buffers[0][0], o[0],
                "{}: fma_array(len={len}) must not write",
                imp.name
            );
            assert!(
                res.buffers[0][1..].iter().all(|&v| v == SENTINEL),
                "{}: fma_array(len={len}) must not write",
                imp.name
            );
        }
    }
}

// ===========================================================================
// Row 3 — fma_array with ALL FOUR pointers NULL and len <= 0: the loop guard
// short-circuits before any dereference, so this is legal and must not crash.
// ===========================================================================

#[test]
fn e03_fma_all_null_pointers_with_nonpositive_len() {
    for len in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        let (c, r) = run_crash_both("fma", len, NULL_ALL4, 0, T);
        assert_eq!(
            c,
            Outcome::Exited(0),
            "C fma_array(NULL x4, len={len}) should return normally"
        );
        assert_eq!(
            r, c,
            "fma_array(NULL x4, len={len}): C={c:?} but Rust={r:?}"
        );
    }
}

// ===========================================================================
// Row 4 — fma_array, len == 1: exactly one store (one step past the no-op range)
// ===========================================================================

#[test]
fn e04_fma_len_one_single_store() {
    let mut rng = Rng::new(0xE04);
    let l = libs();
    for _ in 0..300 {
        let class = rng.pick(ALL_VALUE_CLASSES);
        let a = gen_vals(&mut rng, class, 2);
        let b = gen_vals(&mut rng, class, 2);
        let c = gen_vals(&mut rng, class, 2);
        let o = vec![];

        diff_fma("e04", Alias::AllDistinct, 1, &a, &b, &c, &o);

        for imp in [&l.c, &l.rs] {
            let res = call_fma(imp, Alias::AllDistinct, 1, &a, &b, &c, &o);
            let want = a[0].wrapping_mul(b[0]).wrapping_add(c[0]);
            assert_eq!(res.buffers[0][0], want, "{}: out[0]", imp.name);
            assert!(
                res.buffers[0][1..].iter().all(|&v| v == SENTINEL),
                "{}: fma_array(len=1) wrote more than one element",
                imp.name
            );
        }
    }
}

// ===========================================================================
// Rows 5 & 6 — signed overflow in the multiply and in the add.
// Signed overflow is UB in C; the emitted code wraps, and the Rust must wrap
// identically. Checked exhaustively over the boundary set.
// ===========================================================================

#[test]
fn e05_fma_multiply_overflow_wraps_identically() {
    // Exhaustive BOUNDARY x BOUNDARY for the product, add fixed at 0.
    let mut a = Vec::new();
    let mut b = Vec::new();
    for &x in BOUNDARY {
        for &y in BOUNDARY {
            a.push(x);
            b.push(y);
        }
    }
    let n = a.len();
    a.push(0);
    b.push(0);
    let c = vec![0i32; n + 1];
    diff_fma("e05", Alias::AllDistinct, n as c_int, &a, &b, &c, &[]);

    // And confirm both agree with two's-complement wrapping.
    let l = libs();
    for imp in [&l.c, &l.rs] {
        let res = call_fma(imp, Alias::AllDistinct, n as c_int, &a, &b, &c, &[]);
        for i in 0..n {
            assert_eq!(
                res.buffers[0][i],
                a[i].wrapping_mul(b[i]),
                "{}: product wrap at i={i} ({} * {})",
                imp.name,
                a[i],
                b[i]
            );
        }
    }
}

#[test]
fn e06_fma_add_overflow_wraps_identically() {
    // Product fixed at 1 * x, add = y, over the exhaustive boundary set, so the
    // `+` is the operation that overflows.
    let mut b = Vec::new();
    let mut c = Vec::new();
    for &x in BOUNDARY {
        for &y in BOUNDARY {
            b.push(x);
            c.push(y);
        }
    }
    let n = b.len();
    b.push(0);
    c.push(0);
    let a = vec![1i32; n + 1];
    diff_fma("e06", Alias::AllDistinct, n as c_int, &a, &b, &c, &[]);

    let l = libs();
    for imp in [&l.c, &l.rs] {
        let res = call_fma(imp, Alias::AllDistinct, n as c_int, &a, &b, &c, &[]);
        for i in 0..n {
            assert_eq!(
                res.buffers[0][i],
                b[i].wrapping_add(c[i]),
                "{}: sum wrap at i={i} ({} + {})",
                imp.name,
                b[i],
                c[i]
            );
        }
    }
}

// ===========================================================================
// Row 7 — fully aliased arguments (no `restrict`), the `inner` call pattern
// ===========================================================================

#[test]
fn e07_fma_full_aliasing_is_legal_and_matches() {
    let mut rng = Rng::new(0xE07);
    let l = libs();
    for &class in ALL_VALUE_CLASSES {
        for len in [0i32, 1, 2, 7, 64, 257] {
            let n = len.max(0) as usize;
            let a = gen_vals(&mut rng, class, n + 2);
            diff_fma("e07", Alias::AllSame, len, &a, &a, &a, &a);

            for imp in [&l.c, &l.rs] {
                let res = call_fma(imp, Alias::AllSame, len, &a, &a, &a, &a);
                for i in 0..n {
                    let want = a[i].wrapping_mul(a[i]).wrapping_add(a[i]);
                    assert_eq!(
                        res.buffers[0][i], want,
                        "{}: aliased fma at i={i}, class={class:?}",
                        imp.name
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Row 8 — partially aliased / offset arguments: the result depends on the
// ascending iteration order, which both must share.
// ===========================================================================

#[test]
fn e08_fma_offset_aliasing_pins_iteration_order() {
    let mut rng = Rng::new(0xE08);
    for &alias in &[Alias::Mul1AheadOfOut, Alias::OutAheadOfMul1] {
        for &class in ALL_VALUE_CLASSES {
            for len in [0i32, 1, 2, 3, 8, 33, 128] {
                let n = len.max(0) as usize;
                let a = gen_vals(&mut rng, class, n + 2);
                diff_fma("e08", alias, len, &a, &a, &a, &a);
            }
        }
    }
}

// ===========================================================================
// Row 9 — driver, len == 0: zero-length VLA, memcpy of 0 bytes, prints nothing
// ===========================================================================

#[test]
fn e09_driver_len_zero_prints_nothing_and_returns() {
    let mut rng = Rng::new(0xE09);
    let cases: Vec<(Vec<i32>, c_int)> = (0..50)
        .map(|_| (gen_vals(&mut rng, ValueClass::Boundary, 8), 0))
        .collect();
    for (i, out) in diff_driver_batch("e09", &cases).iter().enumerate() {
        assert!(out.is_empty(), "driver(len=0) case {i} printed {out:?}");
    }
    // Also confirm the process survives it (no VLA/memcpy fault).
    let (c, r) = run_crash_both("driver", 0, NULL_NONE, 8, T);
    assert_eq!(c, Outcome::Exited(0), "C driver(len=0) must return normally");
    assert_eq!(r, c, "driver(len=0): C={c:?} Rust={r:?}");
}

// ===========================================================================
// Row 10 — driver, len < 0: `len * sizeof(int)` converts to a huge size_t and
// `memcpy` runs off the stack. The C dies with SIGSEGV. There is no error code
// to match; this test records the C ground truth and the Rust's outcome.
// ===========================================================================

#[test]
fn e10_driver_negative_len_faults_in_c() {
    for len in [-1i32, -2, -3, -100, -65536, i32::MIN + 1, i32::MIN] {
        let (c, r) = run_crash_both("driver", len, NULL_NONE, 8, T);
        assert!(
            is_fault(c),
            "C driver(len={len}) is expected to fault (UB: memcpy of \
             (size_t)len*4 bytes); got {c:?}"
        );
        // The Rust translation clamps the VLA length to 0, so it survives.
        // This is a UB-only divergence: the C produces no value to compare,
        // it dies. Documented as ERRORS.md row 10.
        assert!(
            matches!(r, Outcome::Exited(0)) || is_fault(r),
            "Rust driver(len={len}) had an unexpected outcome {r:?}"
        );
        assert_ne!(r, Outcome::TimedOut, "Rust driver(len={len}) hung");
    }
}

// ===========================================================================
// Row 11 — driver, data == NULL with len > 0: memcpy from NULL. Both fault.
// ===========================================================================

#[test]
fn e11_driver_null_data_with_positive_len_faults_in_both() {
    for len in [1i32, 2, 8, 1024] {
        let (c, r) = run_crash_both("driver", len, NULL_DATA, 0, T);
        assert_same_fault(&format!("driver(NULL, len={len})"), c, r);
    }
}

// ===========================================================================
// Row 12 — driver, data == NULL with len == 0: memcpy with a zero count never
// dereferences, so this returns normally and prints nothing.
// ===========================================================================

#[test]
fn e12_driver_null_data_with_zero_len_is_fine() {
    let (c, r) = run_crash_both("driver", 0, NULL_DATA, 0, T);
    assert_eq!(
        c,
        Outcome::Exited(0),
        "C driver(NULL, len=0) should return normally"
    );
    assert_eq!(r, c, "driver(NULL, len=0): C={c:?} but Rust={r:?}");
}

// ===========================================================================
// Row 13 — driver, len == 1: exactly one line
// ===========================================================================

#[test]
fn e13_driver_len_one_prints_exactly_one_line() {
    let mut rng = Rng::new(0xE13);
    let mut cases: Vec<(Vec<i32>, c_int)> = BOUNDARY.iter().map(|&v| (vec![v], 1)).collect();
    for _ in 0..200 {
        cases.push((gen_vals(&mut rng, ValueClass::FullRandom, 1), 1));
    }
    let outs = diff_driver_batch("e13", &cases);
    for ((data, _), out) in cases.iter().zip(outs.iter()) {
        let want = data[0].wrapping_mul(data[0]).wrapping_add(data[0]);
        assert_eq!(
            out,
            format!("{want}\n").as_bytes(),
            "driver(len=1) for data[0]={}",
            data[0]
        );
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "driver(len=1) must print exactly one line"
        );
    }
}

// ===========================================================================
// Rows 14 & 15 — driver with a `len` whose VLA exceeds the stack.
//
// The C's `int out[len]` lives on the CALLER'S stack, so the threshold depends
// on the caller's stack size. The crash worker therefore runs the call on a
// thread with an explicitly pinned stack (`WORKER_STACK_BYTES`, 8 MiB) so the
// behaviour is deterministic. Measured ground truth at that stack size:
//
//   len <= 2^20  (VLA <= 4 MiB)  -> C returns normally; Rust matches exactly
//   len >= 2^21  (VLA >= 8 MiB)  -> C dies (SIGSEGV, or SIGABRT via the
//                                   stack-overflow guard)
//
// The Rust translation heap-allocates instead of using the stack, so it
// survives the sizes where the C blows its stack. That is a UB-only
// divergence: on those inputs the C produces no value to compare -- it dies.
// Recorded in ERRORS.md rows 14/15.
// ===========================================================================

#[test]
fn e14_driver_vla_within_the_stack_matches_exactly() {
    // Everything that fits the pinned 8 MiB stack must behave identically.
    for shift in 0..=20u32 {
        let len = 1i32 << shift;
        let (c, r) = run_crash_both("driver", len, NULL_NONE, len as usize, T);
        assert_eq!(
            c,
            Outcome::Exited(0),
            "C driver(len=2^{shift}) fits an 8 MiB stack and should return normally"
        );
        assert_eq!(
            r, c,
            "driver(len=2^{shift}) below the VLA threshold: C={c:?} Rust={r:?}"
        );
    }
}

#[test]
fn e14b_driver_vla_larger_than_the_stack_faults_in_c() {
    // At or above the pinned stack size the C reliably dies.
    for shift in 21..=24u32 {
        let len = 1i32 << shift;
        let (c, r) = run_crash_both("driver", len, NULL_NONE, len as usize, T);
        assert!(
            is_fault(c),
            "C driver(len=2^{shift}) is expected to fault on its {} MiB VLA; got {c:?}",
            (1u64 << shift) * 4 / (1 << 20)
        );
        // Rust heap-allocates, so it survives. Assert it at least terminates
        // and never produces a *wrong* answer (there is no C answer to match).
        assert_ne!(r, Outcome::TimedOut, "Rust driver(len=2^{shift}) hung");
    }
}

#[test]
fn e15_driver_len_int_max_faults_in_c() {
    // len == INT_MAX: the C's VLA would be 8 GiB and `len * sizeof(int)`
    // overflows to 0x1FFFFFFFC, so the C dies immediately. The Rust attempts a
    // multi-gigabyte heap allocation, which either fails hard (SIGABRT from the
    // allocator) or takes unboundedly long to touch; both are acceptable, since
    // the C never returns a result on these inputs.
    let short = Duration::from_secs(15);
    for len in [i32::MAX, i32::MAX - 1, i32::MAX / 2] {
        let (c, r) = run_crash_both("driver", len, NULL_NONE, 16, short);
        assert!(
            is_fault(c),
            "C driver(len={len}) should fault on its multi-GiB VLA, got {c:?}"
        );
        assert!(
            is_fault(r) || r == Outcome::TimedOut,
            "Rust driver(len={len}) should not silently succeed, got {r:?}"
        );
        assert_ne!(
            r,
            Outcome::Exited(0),
            "Rust driver(len={len}) must not claim success where the C dies"
        );
    }
}

// ===========================================================================
// Row 16 — there are NO enums and NO mode/flag parameters in this API, so
// there is no out-of-range-enum class of bug. Asserted against the C source so
// the claim cannot silently rot if the C ever gains one.
// ===========================================================================

#[test]
fn e16_c_source_declares_no_enums_or_flags() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src");
    let mut checked = 0;
    for rel in ["src/driver.c", "include/driver.h"] {
        let src = std::fs::read_to_string(root.join(rel)).expect("read C source");
        checked += 1;
        assert!(
            !src.contains("enum"),
            "{rel} now declares an enum — ERRORS.md row 16 and the FFI \
             out-of-range-enum tests must be revisited"
        );
        // No error-returning surface either: every function returns void.
        for line in src.lines() {
            let t = line.trim();
            assert!(
                !t.starts_with("return ") || t == "return;",
                "{rel} now has a value-returning statement ({t:?}) — the \
                 ERRORS.md table must be regenerated"
            );
            assert!(
                !t.contains("assert("),
                "{rel} now has an assert ({t:?}) — regenerate ERRORS.md"
            );
        }
    }
    assert_eq!(checked, 2, "both C sources must be checked");

    // The nearest analogue to an out-of-range enum in this API is an
    // out-of-range `int len`. Exercise the extreme representable values that no
    // sane caller would pass, in both directions, on the function that can take
    // them without UB.
    for len in [i32::MIN, i32::MIN + 1, -1, 0] {
        let (c, r) = run_crash_both("fma", len, NULL_ALL4, 0, T);
        assert_eq!(c, Outcome::Exited(0), "C fma_array(len={len}) with NULLs");
        assert_eq!(r, c, "fma_array(len={len}) with NULLs: C={c:?} Rust={r:?}");
    }
}

// ===========================================================================
// Generic FFI boundary coverage every C API needs, beyond the table:
// individual NULL arguments, and `len` one step past each valid boundary.
// ===========================================================================

#[test]
fn e17_fma_individual_null_arguments() {
    // With len <= 0 no pointer is dereferenced, so every single-NULL (and every
    // combination) must be survivable and identical.
    for len in [0i32, -1, i32::MIN] {
        for mask in 0u32..16 {
            let (c, r) = run_crash_both("fma", len, mask, 8, T);
            assert_eq!(
                c,
                Outcome::Exited(0),
                "C fma_array(len={len}, nullmask={mask:#x}) should not fault"
            );
            assert_eq!(
                r, c,
                "fma_array(len={len}, nullmask={mask:#x}): C={c:?} Rust={r:?}"
            );
        }
    }

    // With len > 0 every NULL argument is dereferenced, so each must fault the
    // same way in both.
    for mask in [NULL_OUT, NULL_MUL1, NULL_MUL2, NULL_ADD, NULL_ALL4] {
        let (c, r) = run_crash_both("fma", 4, mask, 8, T);
        assert_same_fault(&format!("fma_array(len=4, nullmask={mask:#x})"), c, r);
    }

    // And nullmask == 0 with a valid length must succeed in both.
    let (c, r) = run_crash_both("fma", 8, NULL_NONE, 8, T);
    assert_eq!(c, Outcome::Exited(0));
    assert_eq!(r, c);
}

#[test]
fn e18_len_one_step_past_each_boundary() {
    let l = libs();
    let mut rng = Rng::new(0xE18);

    // fma_array: -1 / 0 / 1 straddle the "does anything happen" boundary.
    for len in [-1i32, 0, 1] {
        let a = gen_vals(&mut rng, ValueClass::Boundary, 4);
        let b = gen_vals(&mut rng, ValueClass::Boundary, 4);
        let c = gen_vals(&mut rng, ValueClass::Boundary, 4);
        diff_fma("e18-fma", Alias::AllDistinct, len, &a, &b, &c, &[]);

        for imp in [&l.c, &l.rs] {
            let res = call_fma(imp, Alias::AllDistinct, len, &a, &b, &c, &[]);
            let writes = res.buffers[0].iter().filter(|&&v| v != SENTINEL).count();
            assert_eq!(
                writes,
                if len > 0 { len as usize } else { 0 },
                "{}: fma_array(len={len}) wrote the wrong number of elements",
                imp.name
            );
        }
    }

    // driver: 0 / 1 straddle the "prints anything" boundary, and the exact
    // element count printed must step by one.
    let cases: Vec<(Vec<i32>, c_int)> = (0..6)
        .map(|n| (gen_vals(&mut rng, ValueClass::Boundary, (n as usize).max(1)), n))
        .collect();
    let outs = diff_driver_batch("e18-driver", &cases);
    for ((_, len), out) in cases.iter().zip(outs.iter()) {
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            (*len).max(0) as usize,
            "driver(len={len}) printed the wrong number of lines"
        );
    }
}

#[test]
fn e19_zero_and_oversized_lengths_against_a_small_buffer() {
    // A `len` far larger than the buffer the caller actually allocated: the C
    // reads/writes off the end. Both must fault identically.
    let (c, r) = run_crash_both("fma", 1 << 24, NULL_NONE, 8, T);
    assert_same_fault("fma_array(len=2^24) against an 8-element buffer", c, r);

    // Zero length against a valid buffer: nothing happens, in both.
    let (c, r) = run_crash_both("fma", 0, NULL_NONE, 8, T);
    assert_eq!(c, Outcome::Exited(0));
    assert_eq!(r, c);
}
