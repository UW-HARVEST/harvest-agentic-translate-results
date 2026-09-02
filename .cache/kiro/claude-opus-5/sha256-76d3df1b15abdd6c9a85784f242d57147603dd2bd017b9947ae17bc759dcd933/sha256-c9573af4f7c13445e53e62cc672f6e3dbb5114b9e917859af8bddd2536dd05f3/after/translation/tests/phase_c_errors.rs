//! Phase C, part 1 — `ERRORS.md` rows that do not involve `driver`'s stdout:
//! E1..E5, E12..E17.
//!
//! Each row constructs the exact invalid input the C code rejects (or the exact
//! UB condition it walks into) and asserts the C `.so` and the Rust `.so` reach
//! the SAME outcome — the same returned sentinel, or the same fatal signal, not
//! merely "both misbehaved".
//!
//! Rows E13..E16 are the ones where the C genuinely faults. They are run in a
//! forked child (`common::isolated`) so a `SIGSEGV` is observed and compared as
//! data rather than taking the test process down. That turns "documented as UB"
//! into an actual assertion that both libraries fault identically.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

/// Marker a probe child returns to prove it ran to completion without faulting.
const SURVIVED: i32 = 0x0C0F_FEE1u32 as i32;

// ===========================================================================
// E1..E5 — the defined, non-faulting rejections
// ===========================================================================

/// E1 — `call_fma(data, 0)` hits `if (len == 0) return 0;`. The guard runs
/// before any VLA is created, so nothing is allocated and `data` is not read.
#[test]
fn e1_call_fma_len_zero() {
    let mut rng = Rng::new(SEED ^ 0x101);
    // Randomised buffers, to be sure the 0 is the guard's and not a stale read.
    for it in 0..200 {
        let n = rng.range(1, 64);
        let data = if rng.bool() {
            random_vec(&mut rng, n)
        } else {
            extreme_vec(&mut rng, n)
        };
        let v = assert_call_fma_matches(&data, 0, &format!("E1 it={it}"));
        assert_eq!(v, 0, "E1 it={it}: len==0 must return exactly 0");
    }
    // And with an empty buffer.
    let empty: [i32; 0] = [];
    assert_eq!(assert_call_fma_matches(&empty, 0, "E1 empty"), 0);
}

/// E2 — `call_fma(NULL, 0)`: the `len == 0` guard means the null is never
/// dereferenced, so this must return 0 rather than fault. Run isolated so a
/// regression that *does* dereference is reported as a signal instead of
/// killing the run.
#[test]
fn e2_call_fma_len_zero_null_data() {
    let (c, r) = *apis();
    let c_out = isolated(|| unsafe { (c.call_fma)(std::ptr::null(), 0) });
    let r_out = isolated(|| unsafe { (r.call_fma)(std::ptr::null(), 0) });
    assert_eq!(
        c_out,
        Isolated::Value(0),
        "E2: C call_fma(NULL, 0) should return 0"
    );
    assert_eq!(c_out, r_out, "E2: call_fma(NULL, 0) outcome differs");
}

/// E3 — `fma_array(..., 0)`: `i < len` is false immediately, so not a single
/// element of `out` may be written. Verified by comparing the whole pre-filled
/// buffer afterwards.
#[test]
fn e3_fma_array_len_zero() {
    let mut rng = Rng::new(SEED ^ 0x103);
    for it in 0..200 {
        let n = rng.range(1, 64);
        let prefill = random_vec(&mut rng, n);
        let mul1 = random_vec(&mut rng, n);
        let mul2 = random_vec(&mut rng, n);
        let add = random_vec(&mut rng, n);
        let out = assert_fma_array_matches(&prefill, &mul1, &mul2, &add, 0, &format!("E3 it={it}"));
        assert_eq!(out, prefill, "E3 it={it}: len==0 must write nothing");
    }
}

/// E4 — negative `len`. `0 < len` is false, so the loop body never runs: this is
/// a silent no-op, NOT a crash and NOT a wrap-around to a huge unsigned count.
#[test]
fn e4_fma_array_negative_len() {
    let mut rng = Rng::new(SEED ^ 0x104);
    let lens: &[c_int] = &[-1, -2, -7, -100, -4096, i32::MIN + 1, i32::MIN];
    for &len in lens {
        for it in 0..30 {
            let n = rng.range(1, 64);
            let prefill = random_vec(&mut rng, n);
            let mul1 = random_vec(&mut rng, n);
            let mul2 = random_vec(&mut rng, n);
            let add = random_vec(&mut rng, n);
            let out = assert_fma_array_matches(
                &prefill,
                &mul1,
                &mul2,
                &add,
                len,
                &format!("E4 len={len} it={it}"),
            );
            assert_eq!(out, prefill, "E4 len={len} it={it}: must write nothing");
        }
    }
}

/// E5 — all four pointers null with a non-positive `len`. Because the loop body
/// never executes, no dereference happens and both libraries must return
/// normally. Isolated so a faulting regression is visible as a signal.
#[test]
fn e5_fma_array_null_ptrs_nonpositive_len() {
    let (c, r) = *apis();
    for &len in &[0 as c_int, -1, -1000, i32::MIN] {
        let c_out = isolated(|| {
            unsafe {
                (c.fma_array)(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                )
            };
            SURVIVED
        });
        let r_out = isolated(|| {
            unsafe {
                (r.fma_array)(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                )
            };
            SURVIVED
        });
        assert_eq!(
            c_out,
            Isolated::Value(SURVIVED),
            "E5 len={len}: C should not fault with null pointers and len<=0"
        );
        assert_eq!(c_out, r_out, "E5 len={len}: outcome differs");
    }
}

// ===========================================================================
// E12, E13 — negative / oversized `len` on `call_fma`: genuine UB
// ===========================================================================

/// E12 — `call_fma(data, len)` with `len < 0` declares negative-size VLAs and
/// then reads `out[len-1]`, i.e. off the front of the object. There is no fixed
/// C behaviour here to match: this test PROVES that by re-running the probe in
/// genuinely independent processes and showing the result is not reproducible
/// across them (or that it faults).
///
/// The value the C returns turns out to be a fragment of the process's own
/// stack layout, so `fork` is the wrong tool here — a forked child inherits a
/// byte-identical stack and therefore reproduces the parent's value exactly.
/// Only a fresh `execve`, with fresh ASLR, varies it. Hence the re-exec of this
/// test binary into the `e12_probe_child` helper below. The fork-based leg is
/// kept as well, because "constant within one address space, different across
/// address spaces" is precisely the signature of an uninitialised read.
///
/// The Rust deliberately returns a deterministic 0 instead of reading out of
/// bounds. That diverges from the C only in a region where the C has no defined
/// value at all, and it is recorded explicitly rather than papered over.
#[test]
fn e12_call_fma_negative_len_is_ub() {
    let (c, r) = *apis();
    let data: Vec<i32> = (0..64).map(|i| 1000 + i).collect();

    // Leg 1: same address space (fork). Expected to be self-consistent.
    let forked: Vec<Isolated> = (0..6)
        .map(|_| isolated(|| unsafe { (c.call_fma)(data.as_ptr(), -1) }))
        .collect();

    // Leg 2: fresh processes via re-exec, so ASLR is re-rolled each time.
    let mut fresh: Vec<Option<i32>> = Vec::new();
    for _ in 0..12 {
        fresh.push(run_e12_probe_child());
    }
    let distinct: std::collections::BTreeSet<Option<i32>> = fresh.iter().copied().collect();

    eprintln!("E12: forked (same address space) : {forked:?}");
    eprintln!("E12: fresh processes (new ASLR)  : {fresh:?}");

    assert!(
        distinct.len() > 1,
        "E12: the C returned the SAME value ({distinct:?}) from every FRESH process. \
         That would make negative len a de-facto defined behaviour worth matching, \
         so this row needs re-deriving rather than documenting."
    );

    // Whatever the C does, the Rust must at least be safe and deterministic.
    for _ in 0..6 {
        assert_eq!(
            isolated(|| unsafe { (r.call_fma)(data.as_ptr(), -1) }),
            Isolated::Value(0),
            "E12: Rust call_fma(data, -1) must be a deterministic, non-faulting 0"
        );
    }
}

/// Re-executes this test binary to run just `e12_probe_child`, and returns the
/// value it printed (or `None` if the child died).
fn run_e12_probe_child() -> Option<i32> {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--ignored", "--exact", "--nocapture", "e12_probe_child"])
        .output()
        .expect("re-exec test binary");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find_map(|l| l.strip_prefix("E12PROBE="))
        .and_then(|v| v.trim().parse::<i32>().ok())
}

/// Helper for `e12_call_fma_negative_len_is_ub`: performs one negative-`len`
/// call in a fresh process and prints the result. `#[ignore]`d so it only runs
/// when named explicitly.
#[test]
#[ignore = "helper process for e12_call_fma_negative_len_is_ub"]
fn e12_probe_child() {
    let (c, _r) = *apis();
    let data: Vec<i32> = (0..64).map(|i| 1000 + i).collect();
    let v = unsafe { (c.call_fma)(data.as_ptr(), -1) };
    println!("E12PROBE={v}");
}

/// E13 — `len == INT_MAX` asks the C for three 8 GiB stack VLAs. Not run: the C
/// is certain to exhaust the stack, and making the Rust side attempt the
/// matching 24 GiB of allocation would thrash the machine rather than teach us
/// anything. Recorded here so the row is visibly accounted for.
#[test]
#[ignore = "INT_MAX len asks for 3 x 8 GiB; C stack-overflows and the Rust side would thrash the host"]
fn e13_call_fma_int_max_len() {
    let (c, _r) = *apis();
    let data = vec![1i32; 16];
    let out = isolated(|| unsafe { (c.call_fma)(data.as_ptr(), i32::MAX) });
    eprintln!("E13: C call_fma(data, INT_MAX) -> {out:?}");
}

// ===========================================================================
// E14..E16 — the faulting null-pointer paths, compared as signals
// ===========================================================================

/// E14 — `fma_array` with `len > 0` and a null pointer in each argument
/// position in turn. The C has no null check and dereferences on iteration 0,
/// so every case must fault, and the Rust must fault the same way.
#[test]
fn e14_fma_array_faulting_nulls() {
    let (c, r) = *apis();
    let n = 8usize;
    let buf: Vec<i32> = (0..n as i32).collect();

    // Which argument to null out: 0=out, 1=mul1, 2=mul2, 3=add.
    for which in 0..4 {
        let run = |api: Api| {
            let mut out = vec![0i32; n];
            let o = if which == 0 {
                std::ptr::null_mut()
            } else {
                out.as_mut_ptr()
            };
            let p = |k: usize| {
                if which == k {
                    std::ptr::null()
                } else {
                    buf.as_ptr()
                }
            };
            unsafe { (api.fma_array)(o, p(1), p(2), p(3), n as c_int) };
            SURVIVED
        };
        let c_out = isolated(|| run(c));
        let r_out = isolated(|| run(r));
        assert_eq!(
            c_out, r_out,
            "E14 which={which}: C and Rust must fault identically \
             (C={c_out:?} Rust={r_out:?})"
        );
        assert!(
            matches!(c_out, Isolated::Signal(_)),
            "E14 which={which}: expected the C to fault, got {c_out:?}"
        );
    }
}

/// E15 — `call_fma(NULL, len)` with `len > 0`: the `len == 0` guard does not
/// apply, so `fma_array` dereferences `mul2 == NULL` and faults.
#[test]
fn e15_call_fma_faulting_null_data() {
    let (c, r) = *apis();
    for &len in &[1 as c_int, 2, 8, 100] {
        let c_out = isolated(|| unsafe { (c.call_fma)(std::ptr::null(), len) });
        let r_out = isolated(|| unsafe { (r.call_fma)(std::ptr::null(), len) });
        assert_eq!(
            c_out, r_out,
            "E15 len={len}: outcome differs (C={c_out:?} Rust={r_out:?})"
        );
        assert!(
            matches!(c_out, Isolated::Signal(_)),
            "E15 len={len}: expected the C to fault, got {c_out:?}"
        );
    }
}

/// E16 — `driver(NULL)`: the C passes the null straight to `sscanf`, which
/// faults. No null check exists anywhere in `driver`.
#[test]
fn e16_driver_faulting_null_input() {
    let (c, r) = *apis();
    let c_out = isolated(|| {
        unsafe { (c.driver)(std::ptr::null()) };
        SURVIVED
    });
    let r_out = isolated(|| {
        unsafe { (r.driver)(std::ptr::null()) };
        SURVIVED
    });
    assert_eq!(
        c_out, r_out,
        "E16: outcome differs (C={c_out:?} Rust={r_out:?})"
    );
    assert!(
        matches!(c_out, Isolated::Signal(_)),
        "E16: expected the C to fault on driver(NULL), got {c_out:?}"
    );
}

// ===========================================================================
// E17 — signed overflow in the multiply-add
// ===========================================================================

/// E17 — `mul1[i] * mul2[i] + add[i]` overflowing `int`. Signed overflow is UB
/// per the standard, but `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the C
/// is built unoptimised and gcc emits a plain two's-complement `imul`/`add` that
/// wraps. The Rust uses `wrapping_mul`/`wrapping_add` to match. This row pins
/// that down with the exact worst-case operand pairs rather than relying on the
/// randomised C6 row to stumble onto them.
#[test]
fn e17_fma_array_signed_overflow_wraps() {
    // (mul1, mul2, add) triples chosen so the product overflows, the sum
    // overflows, or both.
    let cases: &[(i32, i32, i32)] = &[
        (i32::MAX, i32::MAX, 0),
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, 0),
        (i32::MIN, -1, 0),
        (-1, i32::MIN, 0),
        (i32::MIN, 1, i32::MIN),
        (i32::MAX, 1, 1),
        (i32::MAX, 1, i32::MAX),
        (i32::MIN, 1, -1),
        (i32::MAX, 2, 0),
        (i32::MIN, 2, 0),
        (46341, 46341, 0),
        (-46341, 46341, 0),
        (46341, -46341, i32::MIN),
        (65536, 65536, 0),
        (65536, 65537, -1),
        (i32::MAX, i32::MIN, i32::MIN),
        (i32::MAX / 2, 3, i32::MAX),
        (1 << 30, 4, 0),
        (-(1 << 30), 4, -1),
        (0, i32::MIN, i32::MIN),
        (1, i32::MIN, i32::MIN),
    ];

    let mul1: Vec<i32> = cases.iter().map(|c| c.0).collect();
    let mul2: Vec<i32> = cases.iter().map(|c| c.1).collect();
    let add: Vec<i32> = cases.iter().map(|c| c.2).collect();
    let prefill = vec![0x5A5A_5A5Au32 as i32; cases.len()];

    let out = assert_fma_array_matches(
        &prefill,
        &mul1,
        &mul2,
        &add,
        cases.len() as c_int,
        "E17 worst-case triples",
    );

    // Both agreed; confirm the shared answer really is the wrapping one, so a
    // hypothetical future C build that traps or saturates would be noticed.
    for (i, &(a, b, d)) in cases.iter().enumerate() {
        assert_eq!(
            out[i],
            a.wrapping_mul(b).wrapping_add(d),
            "E17 case {i}: ({a} * {b} + {d}) is not two's-complement wrapping"
        );
    }

    // Randomised sweep over the extremes as well.
    let mut rng = Rng::new(SEED ^ 0x117);
    for it in 0..200 {
        let n = rng.range(1, 96);
        let m1 = extreme_vec(&mut rng, n);
        let m2 = extreme_vec(&mut rng, n);
        let ad = extreme_vec(&mut rng, n);
        let pf = vec![0i32; n];
        let out = assert_fma_array_matches(&pf, &m1, &m2, &ad, n as c_int, &format!("E17 it={it}"));
        for i in 0..n {
            assert_eq!(
                out[i],
                m1[i].wrapping_mul(m2[i]).wrapping_add(ad[i]),
                "E17 it={it} i={i}"
            );
        }
    }
}

// ===========================================================================
// Supporting evidence: E6 and E7 really are distinct rejections
// ===========================================================================

unsafe extern "C" {
    #[link_name = "__isoc99_sscanf"]
    fn libc_sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
}

/// `ERRORS.md` splits `driver`'s single `if (sscanf(...) != 1) break;` into two
/// rows because `sscanf` fails there in two genuinely different ways: `EOF`
/// (`-1`, input failure — nothing left to read) and `0` (matching failure — the
/// next character cannot start an `int`). This test confirms the split is real
/// against the same libc both `.so`s call, so E6 and E7 are not cosmetic
/// duplicates of one another.
#[test]
fn e6_e7_sscanf_failure_modes_are_distinct() {
    let fmt = b"%d%zn\0";
    let probe = |s: &str| -> c_int {
        let mut cs = s.as_bytes().to_vec();
        cs.push(0);
        let mut v: c_int = 0;
        let mut nb: usize = 0;
        unsafe {
            libc_sscanf(
                cs.as_ptr() as *const c_char,
                fmt.as_ptr() as *const c_char,
                &mut v as *mut c_int,
                &mut nb as *mut usize,
            )
        }
    };

    // E6 — input failure: no non-whitespace character at all.
    for s in ["", " ", "\t", "\n", "   \t\n\r ", "\u{b}\u{c}"] {
        assert_eq!(probe(s), -1, "E6: sscanf({s:?}) should report EOF");
    }
    // E7 — matching failure: a character is present but cannot begin an int.
    for s in ["abc", "-", "+", ".", ".5", "--5", ",", "  xyz", "\n-", "+ 1"] {
        assert_eq!(probe(s), 0, "E7: sscanf({s:?}) should report a match failure");
    }
    // Success, for contrast.
    for s in ["0", "5", "-7", "+9", "  12x", "007"] {
        assert_eq!(probe(s), 1, "sscanf({s:?}) should succeed");
    }
}
