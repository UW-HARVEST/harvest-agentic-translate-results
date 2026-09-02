//! Phase C — error / rejection-path differential tests, one per `ERRORS.md` row.
//!
//! Both `.so`s are loaded with `libloading`; nothing is called directly.
//!
//! `colourblind` returns `void`, so "the same error" is asserted on the only
//! observable channel the C API has: the exact bit patterns left in the three
//! in/out floats (unchanged == rejected), or, for the unconditional-dereference
//! rows, the terminating signal of a child process.

mod common;

use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// ---------------------------------------------------------------------------
// Helper: assert an out-of-range Impairment is a complete no-op in both .so's
// ---------------------------------------------------------------------------

/// The C `switch` has no `default:`, so an out-of-range `Impairment` writes
/// nothing. Asserts that for the C `.so` (establishing the ground truth) and
/// then that every Rust `.so` agrees exactly.
#[track_caller]
fn assert_rejected_noop(row: &str, imp: c_int, input: [f32; 3]) {
    let c_out = c_impl().apply(imp, input);
    assert_eq!(
        bits(c_out),
        bits(input),
        "[{row}] premise failed: the C .so DID modify its arguments for \
         Impairment={imp}; input {} -> {}",
        show(input),
        show(c_out)
    );
    for r in rust_impls() {
        let got = r.apply(imp, input);
        assert_eq!(
            bits(got),
            bits(c_out),
            "\n[{row}] rejection divergence in {}\n  impairment : {imp} (out of range)\n  input      : {}\n  C   output : {} (unchanged)\n  Rust output: {}\n",
            r.label,
            show(input),
            show(c_out),
            show(got),
        );
    }
}

/// A spread of interesting payloads to reject with, so a row is not proven by a
/// single value.
fn rejection_payloads() -> Vec<[f32; 3]> {
    let mut rng = Rng::new(SEED ^ 0xDEAD_BEEF);
    let mut v = vec![
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [0.25, 0.5, 0.75],
        [1.0, 1.0, 1.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::MAX, f32::MIN, f32::MIN_POSITIVE],
        [f32::from_bits(0x7FC0_0000), f32::from_bits(0xFFC0_1234), f32::from_bits(0x7F80_0001)],
        [f32::from_bits(1), f32::from_bits(0x8000_0001), f32::from_bits(0x007F_FFFF)],
    ];
    for _ in 0..256 {
        v.push(draw_triple(&mut rng, VClass::Any));
    }
    v
}

// ---------------------------------------------------------------------------
// E1..E5 — individual out-of-range enum values
// ---------------------------------------------------------------------------

fn out_of_range_row(row: &str, imp: c_int) {
    for (i, p) in rejection_payloads().iter().enumerate() {
        assert_rejected_noop(&format!("{row} payload#{i}"), imp, *p);
    }
}

#[test]
fn err_e1_impairment_3() {
    // One past the last enumerator cbTritanopia.
    out_of_range_row("E1", 3);
}

#[test]
fn err_e2_impairment_neg1() {
    // One before the first enumerator cbProtanopia. Caught by gcc's unsigned
    // `ja` bound check.
    out_of_range_row("E2", -1);
}

#[test]
fn err_e3_impairment_int_max() {
    out_of_range_row("E3", c_int::MAX);
}

#[test]
fn err_e4_impairment_int_min() {
    out_of_range_row("E4", c_int::MIN);
}

#[test]
fn err_e5_impairment_4() {
    // A power of two just past the range: a naive bitmask test would admit it.
    out_of_range_row("E5", 4);
}

// ---------------------------------------------------------------------------
// E6 — dense + randomized sweep of out-of-range enum values
// ---------------------------------------------------------------------------

#[test]
fn err_e6_impairment_out_of_range_sweep() {
    let payload = [0.3125f32, -2.5, 7.75];

    // Dense sweep around the valid range.
    for imp in -4096i32..=4096 {
        if (0..=2).contains(&imp) {
            continue;
        }
        assert_rejected_noop("E6 dense", imp, payload);
    }

    // Randomized sweep over the whole i32 space.
    let mut rng = Rng::new(SEED ^ 0x0FF0_0FF0);
    for _ in 0..4096 {
        let imp = rng.next_u32() as i32;
        if (0..=2).contains(&imp) {
            continue;
        }
        let v = draw_triple(&mut rng, VClass::Any);
        assert_rejected_noop("E6 random", imp, v);
    }

    // And the boundary values, exactly.
    for imp in [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        3,
        4,
        i32::MAX - 1,
        i32::MAX,
        0x8000_0000u32 as i32,
        0x7FFF_FFFF,
    ] {
        assert_rejected_noop("E6 boundary", imp, payload);
    }
}

// ---------------------------------------------------------------------------
// E7 — only the low 32 bits of the enum argument are read
// ---------------------------------------------------------------------------

#[test]
fn err_e7_high_half_ignored() {
    // `mov %edi,-0x4(%rbp)`: the high half of rdi is ignored, so a 64-bit value
    // whose low half is a valid enumerator takes the corresponding case.
    for (wide, expect_imp) in [
        (0x0000_0001_0000_0000i64, CB_PROTANOPIA),
        (0x0000_0001_0000_0001i64, CB_DEUTERANOPIA),
        (0x7FFF_FFFF_0000_0002i64, CB_TRITANOPIA),
        (-0x1_0000_0000i64, CB_PROTANOPIA), // low half == 0
    ] {
        let truncated = wide as i32;
        assert_eq!(truncated, expect_imp, "test vector is inconsistent");

        for v in [
            [0.25f32, 0.5, 0.75],
            [1.0, 0.0, 0.0],
            [f32::INFINITY, 1.0, f32::from_bits(0x7FC0_1234)],
        ] {
            // Passed as a plain c_int, which is what the C prototype declares;
            // the point of the row is that the *value* is out of enum range in
            // 64-bit terms yet valid once truncated.
            assert_same(&format!("E7 wide={wide:#018x}"), truncated, v);

            // And it must not be treated as a rejection.
            let out = c_impl().apply(truncated, v);
            assert_ne!(
                bits(out),
                bits(v),
                "E7: expected the {} case to run for {wide:#018x}",
                impairment_name(expect_imp)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E8..E11 — null pointers with a VALID impairment must fault identically
// ---------------------------------------------------------------------------

/// Set when this process is a crash child. Format: `<so-index>:<impairment>:<mask>`
/// where `so-index` 0 is the C `.so` and 1.. index into `rust_impls()`, and
/// `mask` bit 0/1/2 nulls out R/G/B respectively.
const CRASH_ENV: &str = "CB_CRASH_SPEC";

/// The child entry point. Under normal test runs `CB_CRASH_SPEC` is unset and
/// this returns immediately.
#[test]
fn err_crash_child_entrypoint() {
    let Ok(spec) = std::env::var(CRASH_ENV) else {
        return;
    };
    let parts: Vec<&str> = spec.split(':').collect();
    assert_eq!(parts.len(), 3, "bad {CRASH_ENV}: {spec}");
    let so_index: usize = parts[0].parse().unwrap();
    let imp: c_int = parts[1].parse().unwrap();
    let mask: u8 = parts[2].parse().unwrap();

    let im: &Impl = if so_index == 0 {
        c_impl()
    } else {
        &rust_impls()[so_index - 1]
    };

    let mut r = 0.25f32;
    let mut g = 0.5f32;
    let mut b = 0.75f32;
    let pr = if mask & 1 != 0 { std::ptr::null_mut() } else { &mut r as *mut f32 };
    let pg = if mask & 2 != 0 { std::ptr::null_mut() } else { &mut g as *mut f32 };
    let pb = if mask & 4 != 0 { std::ptr::null_mut() } else { &mut b as *mut f32 };

    // Make sure the call is not optimised away and flush before faulting.
    eprintln!("crash child: so={so_index} imp={imp} mask={mask}");
    unsafe { (im.call)(imp, pr, pg, pb) };
    // If we get here the call did NOT fault; report that distinctly.
    println!("SURVIVED {r} {g} {b}");
    std::process::exit(42);
}

/// How a child terminated, as a comparable value.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn run_crash_child(so_index: usize, imp: c_int, mask: u8) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "err_crash_child_entrypoint",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CRASH_ENV, format!("{so_index}:{imp}:{mask}"))
        .output()
        .expect("spawn crash child");
    match out.status.signal() {
        Some(s) => Outcome::Signal(s),
        None => Outcome::Exit(out.status.code().unwrap_or(-1)),
    }
}

/// A null `float*` reaching a taken `case` is dereferenced unconditionally by
/// the C code. Ground truth is therefore "killed by SIGSEGV"; the Rust `.so`
/// must terminate the same way rather than, for instance, returning quietly or
/// aborting with a different signal.
///
/// Gating applies to the **release** cdylib, which is the artifact that
/// corresponds to the C shared object. The debug cdylib is additionally built
/// with `debug-assertions`, and `rustc` emits an explicit null/misaligned
/// pointer-dereference check there: it aborts (SIGABRT) instead of faulting
/// (SIGSEGV). That is `rustc` deliberately reporting the caller's UB, not a
/// behavioural difference in the translation, so for the debug build the
/// assertion is only that it also dies abnormally — never that it survives.
#[test]
fn err_e8_null_ptr_faults() {
    // (impairment, mask, ERRORS.md row)
    let cases: [(c_int, u8, &str); 8] = [
        (CB_PROTANOPIA, 0b001, "E8  R=NULL, imp=0"),
        (CB_DEUTERANOPIA, 0b010, "E9  G=NULL, imp=1"),
        (CB_TRITANOPIA, 0b100, "E10 B=NULL, imp=2"),
        (CB_PROTANOPIA, 0b111, "E11 all NULL, imp=0"),
        (CB_DEUTERANOPIA, 0b111, "E11 all NULL, imp=1"),
        (CB_TRITANOPIA, 0b111, "E11 all NULL, imp=2"),
        (CB_PROTANOPIA, 0b011, "E8/E9 R,G=NULL, imp=0"),
        (CB_TRITANOPIA, 0b101, "E8/E10 R,B=NULL, imp=2"),
    ];

    for (imp, mask, row) in cases {
        let c_outcome = run_crash_child(0, imp, mask);
        assert_eq!(
            c_outcome,
            Outcome::Signal(SIGSEGV),
            "[{row}] premise failed: the C .so did not die on SIGSEGV, got {c_outcome:?}"
        );

        for (i, r) in rust_impls().iter().enumerate() {
            let got = run_crash_child(i + 1, imp, mask);

            // Never acceptable: surviving the null dereference, or exiting
            // normally, in any profile.
            assert!(
                matches!(got, Outcome::Signal(_)),
                "[{row}] {} did NOT die on a null dereference: {got:?} \
                 (the C .so dies with {c_outcome:?})",
                r.label
            );

            if r.label == "rust-release" || r.label == "env" {
                assert_eq!(
                    got, c_outcome,
                    "[{row}] {} terminated as {got:?} but the C .so terminated as {c_outcome:?}",
                    r.label
                );
            } else {
                // Debug profile: SIGSEGV (same as C) or SIGABRT (rustc's
                // debug-assertion for a null pointer dereference).
                assert!(
                    got == Outcome::Signal(SIGSEGV) || got == Outcome::Signal(SIGABRT),
                    "[{row}] {} terminated as {got:?}; expected SIGSEGV (like C) or \
                     SIGABRT (rustc debug UB check)",
                    r.label
                );
                if got != c_outcome {
                    println!(
                        "[{row}] note: {} died with {got:?} rather than C's {c_outcome:?} \
                         (rustc debug-assertion for null pointer dereference)",
                        r.label
                    );
                }
            }
        }
    }
}

const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;

// ---------------------------------------------------------------------------
// E12/E13 — an invalid impairment must NOT dereference, even a bad pointer
// ---------------------------------------------------------------------------

/// With no `case` taken, the pointers are never touched, so null pointers are
/// harmless. This is the row that catches a translation which validates or
/// dereferences its pointers before dispatching on the impairment.
#[test]
fn err_e12_null_ptrs_invalid_impairment_is_safe() {
    for imp in [3, 4, -1, 17, c_int::MAX, c_int::MIN] {
        // In-process: if either .so dereferenced, the test process would die,
        // which the harness reports as a failure of this test.
        unsafe {
            (c_impl().call)(imp, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        for r in rust_impls() {
            unsafe {
                (r.call)(imp, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
            };
        }
    }

    // Mixed: some null, some valid, still out of range -> still a no-op.
    for imp in [3, -1, c_int::MIN] {
        let mut a = 1.0f32;
        let mut b = 2.0f32;
        unsafe { (c_impl().call)(imp, &mut a, std::ptr::null_mut(), &mut b) };
        assert_eq!(
            (a.to_bits(), b.to_bits()),
            (1.0f32.to_bits(), 2.0f32.to_bits()),
            "E12 premise failed: C wrote through a valid pointer for imp={imp}"
        );
        for r in rust_impls() {
            let mut ra = 1.0f32;
            let mut rb = 2.0f32;
            unsafe { (r.call)(imp, &mut ra, std::ptr::null_mut(), &mut rb) };
            assert_eq!(
                (ra.to_bits(), rb.to_bits()),
                (a.to_bits(), b.to_bits()),
                "E12 divergence in {} for imp={imp}",
                r.label
            );
        }
    }
}

/// Same, with non-null but deliberately unmapped addresses: proves the pointers
/// are not read merely because they are non-null.
#[test]
fn err_e13_bogus_ptr_invalid_impairment_is_safe() {
    let bogus = [1usize, 3, 0xDEAD, 0xFFFF_FFFF_FFF0];
    for imp in [3, 4, -1, c_int::MAX, c_int::MIN] {
        for &addr in &bogus {
            let p = addr as *mut f32;
            unsafe { (c_impl().call)(imp, p, p, p) };
            for r in rust_impls() {
                unsafe { (r.call)(imp, p, p, p) };
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E14 — signalling NaNs are not rejected; they are quieted
// ---------------------------------------------------------------------------

#[test]
fn err_e14_signaling_nan_not_rejected() {
    // Hand-picked sNaN encodings: minimum payload, maximum payload, both signs,
    // and the payloads that alias the default QNaN once quieted.
    let snans: [u32; 10] = [
        0x7F80_0001,
        0xFF80_0001,
        0x7FBF_FFFF,
        0xFFBF_FFFF,
        0x7F80_0002,
        0xFF80_1234,
        0x7F9F_FFFF,
        0xFFA5_5555,
        0x7F80_00FF,
        0xFF80_8000,
    ];

    for &imp in &VALID_IMPAIRMENTS {
        for &a in &snans {
            for &b in &snans {
                // sNaN in each position, with ordinary values elsewhere, and
                // with two sNaNs at once (which is what pins dst/src priority).
                let vectors: [[f32; 3]; 6] = [
                    [f32::from_bits(a), 0.5, 0.25],
                    [0.5, f32::from_bits(a), 0.25],
                    [0.5, 0.25, f32::from_bits(a)],
                    [f32::from_bits(a), f32::from_bits(b), 0.25],
                    [f32::from_bits(a), 0.25, f32::from_bits(b)],
                    [0.25, f32::from_bits(a), f32::from_bits(b)],
                ];
                for v in vectors {
                    // Ground truth: an sNaN is NOT rejected; the call proceeds.
                    let out = c_impl().apply(imp, v);
                    assert!(
                        out.iter().any(|x| x.is_nan()),
                        "E14 premise failed: no NaN in the C output {} for input {}",
                        show(out),
                        show(v)
                    );
                    assert_same("E14", imp, v);
                }
            }
        }
    }

    // Plus a randomized sweep over the whole sNaN encoding space.
    let mut rng = Rng::new(SEED ^ 0x5A11_1A0F_u64);
    for i in 0..20_000 {
        let v = draw_triple(&mut rng, VClass::SignallingNan);
        assert_same(&format!("E14 random#{i}"), VALID_IMPAIRMENTS[i % 3], v);
    }
}

// ---------------------------------------------------------------------------
// E15 — arithmetic invalid-operation produces the x86 default QNaN
// ---------------------------------------------------------------------------

#[test]
fn err_e15_invalid_op_default_qnan() {
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;

    // inf - inf, 0 * inf and mixed-sign infinities inside one expression.
    let vectors: [[f32; 3]; 14] = [
        [inf, inf, inf],
        [inf, ninf, 0.0],
        [ninf, inf, 0.0],
        [0.0, inf, ninf],
        [inf, 0.0, ninf],
        [inf, ninf, inf],
        [ninf, inf, ninf],
        [0.0, 0.0, inf],
        [inf, 0.0, 0.0],
        [0.0, inf, 0.0],
        [-0.0, inf, -0.0],
        [inf, inf, ninf],
        [ninf, ninf, inf],
        [f32::MAX, f32::MAX, ninf],
    ];

    for &imp in &VALID_IMPAIRMENTS {
        for v in vectors {
            assert_same("E15", imp, v);
        }
    }

    // At least one of these must actually produce the x86 "real indefinite"
    // QNaN, otherwise the row is not exercising what it claims.
    let mut saw_indefinite = false;
    for &imp in &VALID_IMPAIRMENTS {
        for v in vectors {
            let out = c_impl().apply(imp, v);
            if out.iter().any(|x| x.to_bits() == 0xFFC0_0000) {
                saw_indefinite = true;
            }
        }
    }
    assert!(
        saw_indefinite,
        "E15 premise failed: none of the invalid-operation vectors produced 0xFFC00000 in the C .so"
    );
}

// ---------------------------------------------------------------------------
// E16 — overflow / underflow are neither rejected nor clamped
// ---------------------------------------------------------------------------

#[test]
fn err_e16_overflow_underflow_not_clamped() {
    let interesting: [f32; 14] = [
        f32::MAX,
        -f32::MAX,
        f32::from_bits(0x7F7F_FFFE), // MAX - 1 ulp
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),  // smallest positive subnormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // largest subnormal
        f32::from_bits(0x0080_0000), // smallest normal
        1.0e38,
        -1.0e38,
        1.0e-38,
        f32::EPSILON,
        0.0,
    ];

    for &imp in &VALID_IMPAIRMENTS {
        for &a in &interesting {
            for &b in &interesting {
                for &c in &interesting {
                    assert_same("E16", imp, [a, b, c]);
                }
            }
        }
    }

    // Overflow really does happen (nothing is clamped): assert the C .so
    // produced an infinity from finite inputs at least once.
    let mut saw_overflow = false;
    for &imp in &VALID_IMPAIRMENTS {
        let out = c_impl().apply(imp, [f32::MAX, f32::MAX, f32::MAX]);
        if out.iter().any(|x| x.is_infinite()) {
            saw_overflow = true;
        }
    }
    // Note: whether overflow occurs depends on the coefficients; if it does not,
    // the row still holds (the point is only that no clamping is applied).
    println!("E16: overflow observed from finite inputs = {saw_overflow}");
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries beyond the table
// ---------------------------------------------------------------------------

/// Every out-of-range enum value the FFI boundary can actually carry, checked
/// against the valid ones, in one place. C enums accept any `int`.
#[test]
fn err_generic_enum_boundary_exhaustive_small_range() {
    let payload = [0.1f32, 0.2, 0.3];
    for imp in -1000i32..=1000 {
        if (0..=2).contains(&imp) {
            // Valid: must transform, and must match.
            assert_same("enum-valid", imp, payload);
            let out = c_impl().apply(imp, payload);
            assert_ne!(
                bits(out),
                bits(payload),
                "premise: imp={imp} should transform"
            );
        } else {
            assert_rejected_noop("enum-invalid", imp, payload);
        }
    }
}
