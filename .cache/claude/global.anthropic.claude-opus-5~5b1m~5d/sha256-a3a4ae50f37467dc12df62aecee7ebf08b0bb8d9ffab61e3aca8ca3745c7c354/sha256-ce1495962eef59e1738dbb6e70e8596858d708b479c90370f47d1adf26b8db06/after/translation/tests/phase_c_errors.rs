//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library has no error-reporting channel: its only rejection is the
//! `default`-less `switch`, which silently leaves `*R`, `*G`, `*B` untouched.
//! "Same error" therefore means *the same sentinel*: the three floats must come
//! back **bit-identical to the input** in both libraries — not merely "both did
//! something harmless". Where the C's answer is a fault (NULL), the test
//! compares the actual termination signal out of process.

mod common;

use common::*;

/// Asserts that `imp` is a silent no-op in BOTH libraries: every one of the
/// three floats must be returned bit-identical to what was passed in.
#[track_caller]
fn assert_both_noop(c: &Lib, rust: &Lib, imp: u32, input: [f32; 3], ctx: &str) {
    let mut a = input;
    let mut b = input;
    c.call(imp, &mut a);
    rust.call(imp, &mut b);

    // Same as each other ...
    assert_same(ctx, imp, &input, &a, &b);
    // ... AND equal to the input, i.e. the rejection sentinel really is "no
    // write happened", which is what the C's fall-through does.
    assert_eq!(
        bits(&a),
        bits(&input),
        "{ctx}: C did NOT no-op for impairment {imp}: in {:08x?} out {:08x?}",
        bits(&input),
        bits(&a)
    );
    assert_eq!(
        bits(&b),
        bits(&input),
        "{ctx}: Rust did NOT no-op for impairment {imp}: in {:08x?} out {:08x?}",
        bits(&input),
        bits(&b)
    );
}

/// A spread of payloads used by every out-of-range-impairment row, so the
/// "untouched" claim is checked against exotic bit patterns too.
fn payloads() -> Vec<[f32; 3]> {
    let mut v = vec![
        [0.25f32, 0.5, 0.75],
        [0.0, -0.0, 1.0],
        [f32::INFINITY, f32::NEG_INFINITY, f32::MAX],
        [f32::from_bits(1), f32::MIN_POSITIVE, -f32::MAX],
        [
            f32::from_bits(0x7FC0_0001),
            f32::from_bits(0xFFC0_0002),
            f32::from_bits(0x7F80_0003),
        ],
        [
            f32::from_bits(0xDEAD_BEEF),
            f32::from_bits(0x0BAD_F00D),
            f32::from_bits(0xFFFF_FFFF),
        ],
    ];
    let mut rng = Rng::new(SEED ^ 0xE770_0000);
    for _ in 0..2_000 {
        v.push([rng.any_f32(), rng.any_f32(), rng.any_f32()]);
    }
    v
}

// ---------------------------------------------------------------------------
// Row 1: Impairment == 3, one step past cbTritanopia.
// ---------------------------------------------------------------------------

#[test]
fn err_row01_impairment_one_past_end() {
    let (c, rust) = both();
    for p in payloads() {
        assert_both_noop(&c, &rust, 3, p, "row01 impairment == 3");
    }
}

// ---------------------------------------------------------------------------
// Row 2: small out-of-range sweep, 4..=64.
// ---------------------------------------------------------------------------

#[test]
fn err_row02_impairment_small_out_of_range_sweep() {
    let (c, rust) = both();
    let ps = payloads();
    for imp in 4u32..=64 {
        for p in &ps {
            assert_both_noop(&c, &rust, imp, *p, "row02 small out-of-range");
        }
    }
    // Values just past a plausible jump-table size, and past every power of two
    // up to 2^31, where a mis-translated bound check would show up.
    for shift in 2u32..32 {
        for imp in [1u32 << shift, (1u32 << shift) - 1, (1u32 << shift) + 1] {
            if imp <= 2 {
                continue;
            }
            for p in &ps[..8] {
                assert_both_noop(&c, &rust, imp, *p, "row02 power-of-two neighbourhood");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3: extreme unsigned values.
// ---------------------------------------------------------------------------

#[test]
fn err_row03_impairment_extreme_unsigned() {
    let (c, rust) = both();
    let ps = payloads();
    for imp in [
        u32::MAX,
        u32::MAX - 1,
        0x8000_0000,
        0x8000_0001,
        0x7FFF_FFFF,
        0x7FFF_FFFE,
    ] {
        for p in &ps {
            assert_both_noop(&c, &rust, imp, *p, "row03 extreme unsigned");
        }
    }
    // Randomised out-of-range values, to cover the whole 2^32-3 invalid space.
    let mut rng = Rng::new(SEED ^ 0x3333);
    for _ in 0..20_000 {
        let imp = rng.next_u32();
        if imp <= 2 {
            continue;
        }
        assert_both_noop(&c, &rust, imp, ps[0], "row03 random out-of-range");
    }
}

// ---------------------------------------------------------------------------
// Row 4: negative int impairment (C enums accept any int).
// ---------------------------------------------------------------------------

#[test]
fn err_row04_impairment_negative_int() {
    let (c, rust) = both();
    let ps = payloads();
    for imp in [-1i32, -2, -3, -128, -32768, i32::MIN, i32::MIN + 1] {
        for p in &ps {
            let mut a = *p;
            let mut b = *p;
            c.call_i32(imp, &mut a);
            rust.call_i32(imp, &mut b);
            assert_same("row04 negative int", imp as u32, p, &a, &b);
            assert_eq!(
                bits(&a),
                bits(p),
                "row04: C did not no-op for negative impairment {imp}"
            );
            assert_eq!(
                bits(&b),
                bits(p),
                "row04: Rust did not no-op for negative impairment {imp}"
            );
        }
    }
    // The same values also reach the callee sign-extended in the full 64-bit
    // register; the callee must still look only at the low half.
    for imp in [-1i64, -2, -3, i32::MIN as i64] {
        for p in &ps[..8] {
            let mut a = *p;
            let mut b = *p;
            unsafe {
                let q = a.as_mut_ptr();
                c.call_raw64(imp as u64, q, q.add(1), q.add(2));
                let q = b.as_mut_ptr();
                rust.call_raw64(imp as u64, q, q.add(1), q.add(2));
            }
            assert_same("row04 sign-extended", imp as u32, p, &a, &b);
            assert_eq!(bits(&a), bits(p), "row04: C wrote for sign-extended {imp}");
            assert_eq!(bits(&b), bits(p), "row04: Rust wrote for sign-extended {imp}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5: dirty upper 32 bits of the argument register.
// ---------------------------------------------------------------------------

#[test]
fn err_row05_impairment_dirty_high_bits() {
    let (c, rust) = both();
    let base = [0.3f32, 0.6, 0.9];
    let highs = [
        0x0000_0001u64,
        0xDEAD_BEEFu64,
        0xFFFF_FFFFu64,
        0x8000_0000u64,
        0x1234_5678u64,
    ];
    for high in highs {
        for low in [0u32, 1, 2, 3, 7, u32::MAX] {
            let imp = (high << 32) | u64::from(low);
            let mut a = base;
            let mut b = base;
            unsafe {
                let p = a.as_mut_ptr();
                c.call_raw64(imp, p, p.add(1), p.add(2));
                let p = b.as_mut_ptr();
                rust.call_raw64(imp, p, p.add(1), p.add(2));
            }
            assert_same(
                &format!("row05 dirty high bits, rdi = {imp:#018x}"),
                low,
                &base,
                &a,
                &b,
            );
            // Cross-check against the honest 32-bit call: the low half alone
            // must decide, so the results have to be identical.
            let mut want = base;
            c.call(low, &mut want);
            assert_eq!(
                bits(&a),
                bits(&want),
                "row05: C behaved differently with dirty high bits {imp:#018x}"
            );
            assert_eq!(
                bits(&b),
                bits(&want),
                "row05: Rust behaved differently with dirty high bits {imp:#018x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: out-of-range impairment must not perturb exotic bit patterns at all,
// in particular must not quieten an sNaN.
// ---------------------------------------------------------------------------

#[test]
fn err_row06_out_of_range_preserves_exotic_bits() {
    let (c, rust) = both();
    let exotic = [
        [
            f32::from_bits(0x7F80_0001),
            f32::from_bits(0xFF80_0001),
            f32::from_bits(0x7FBF_FFFF),
        ],
        [
            f32::from_bits(0x7F80_0000),
            f32::from_bits(0xFF80_0000),
            f32::from_bits(0x8000_0000),
        ],
        [f32::from_bits(1), f32::from_bits(0x8000_0001), f32::from_bits(0x007F_FFFF)],
    ];
    for imp in [3u32, 5, 99, 0xFFFF_FFFF] {
        for p in exotic {
            assert_both_noop(&c, &rust, imp, p, "row06 exotic bits preserved");
        }
        let mut rng = Rng::new(SEED ^ 0x6666 ^ u64::from(imp));
        for _ in 0..5_000 {
            let p = [rng.snan(), rng.qnan(), rng.any_f32()];
            assert_both_noop(&c, &rust, imp, p, "row06 exotic bits preserved (random)");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7 & 8: NULL pointers.
//
// Row 8 (NULL + out-of-range impairment) is safe in process: the fall-through
// never dereferences. Row 7 (NULL + valid impairment) is a genuine fault, so it
// runs in a child process and the termination signals are compared.
// ---------------------------------------------------------------------------

#[test]
fn err_row08_null_pointers_with_invalid_impairment_are_safe() {
    let (c, rust) = both();
    let mut live = [1.0f32, 2.0, 3.0];
    let p = live.as_mut_ptr();
    let n: *mut f32 = std::ptr::null_mut();
    for imp in [3u32, 4, 17, u32::MAX, 0x8000_0000] {
        // Every combination of NULL / live pointers, including all three NULL.
        for mask in 0u8..8 {
            let a = if mask & 1 != 0 { n } else { p };
            let b = if mask & 2 != 0 { n } else { unsafe { p.add(1) } };
            let d = if mask & 4 != 0 { n } else { unsafe { p.add(2) } };
            let before = live;
            // SAFETY: the impairment is out of range, so the C (and the Rust)
            // return without touching any pointer. That is precisely the
            // property under test; if it were false this would fault, which the
            // test would report as a crash rather than silently passing.
            unsafe {
                c.call_ptrs(imp, a, b, d);
                rust.call_ptrs(imp, a, b, d);
            }
            assert_eq!(
                bits(&live),
                bits(&before),
                "row08: buffer changed for out-of-range impairment {imp}, mask {mask:#b}"
            );
        }
    }
}

/// Names of the child-process crash scenarios for row 7.
const NULL_CASES: [(&str, u8); 4] = [
    ("r-null", 0b001),
    ("g-null", 0b010),
    ("b-null", 0b100),
    ("all-null", 0b111),
];

#[test]
#[cfg(unix)]
fn err_row07_null_pointers_fault_identically() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    // Re-exec this very test binary, running only the ignored child helper.
    let exe = std::env::current_exe().expect("current_exe");
    for (name, mask) in NULL_CASES {
        for &imp in &VALID {
            let mut outcome = Vec::new();
            for which in ["C", "RUST"] {
                let status = Command::new(&exe)
                    .args(["zz_null_child", "--exact", "--ignored", "--test-threads=1"])
                    .env("CB_NULL_CASE", format!("{mask}"))
                    .env("CB_NULL_IMP", imp.to_string())
                    .env("CB_NULL_LIB", which)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .expect("failed to spawn child");
                outcome.push((which, status.signal(), status.code()));
            }
            let (_, c_sig, c_code) = outcome[0];
            let (_, r_sig, r_code) = outcome[1];
            assert_eq!(
                (c_sig, c_code),
                (r_sig, r_code),
                "row07 {name}, impairment {} ({imp}): C terminated with \
                 signal={c_sig:?} code={c_code:?} but Rust with signal={r_sig:?} code={r_code:?}",
                impairment_name(imp)
            );
            // And it really must be a fault, not a quiet success, otherwise the
            // row would be vacuous.
            assert_eq!(
                c_sig,
                Some(11),
                "row07 {name}: expected the C to die with SIGSEGV, got signal={c_sig:?} code={c_code:?}"
            );
        }
    }
}

/// The child half of row 7. Ignored so it never runs as part of the suite; the
/// parent invokes it explicitly with `--ignored` and the `CB_NULL_*` env vars.
#[test]
#[ignore = "child process helper for err_row07_null_pointers_fault_identically"]
fn zz_null_child() {
    let mask: u8 = std::env::var("CB_NULL_CASE")
        .expect("CB_NULL_CASE")
        .parse()
        .unwrap();
    let imp: u32 = std::env::var("CB_NULL_IMP").expect("CB_NULL_IMP").parse().unwrap();
    let which = std::env::var("CB_NULL_LIB").expect("CB_NULL_LIB");

    let (c, rust) = both();
    let lib = if which == "C" { &c } else { &rust };

    let mut live = [0.5f32, 0.5, 0.5];
    let p = live.as_mut_ptr();
    let n: *mut f32 = std::ptr::null_mut();
    let a = if mask & 1 != 0 { n } else { p };
    let b = if mask & 2 != 0 { n } else { unsafe { p.add(1) } };
    let d = if mask & 4 != 0 { n } else { unsafe { p.add(2) } };

    // SAFETY: intentionally not safe — this is the crash under test, isolated in
    // a dedicated child process by the parent test.
    unsafe { lib.call_ptrs(imp, a, b, d) };

    // Reached only if the platform tolerated the NULL dereference; report it so
    // the parent's exit-code comparison still lines up.
    println!("survived: {live:?}");
}

// ---------------------------------------------------------------------------
// Row 9: misaligned pointers must not be rejected.
// ---------------------------------------------------------------------------

#[test]
fn err_row09_misaligned_pointers() {
    let (c, rust) = both();
    for &imp in &VALID {
        for offset in 1usize..=3 {
            let mut rng = Rng::new(SEED ^ 0x9999 ^ u64::from(imp) ^ offset as u64);
            for _ in 0..2_000 {
                let input = [rng.wide_normal(), rng.qnan(), rng.subnormal()];
                let mut cbuf = [0u8; 20];
                let mut rbuf = [0u8; 20];
                for i in 0..3 {
                    let le = input[i].to_bits().to_le_bytes();
                    cbuf[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&le);
                    rbuf[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&le);
                }
                unsafe {
                    let p = cbuf.as_mut_ptr().add(offset).cast::<f32>();
                    c.call_ptrs(imp, p, p.byte_add(4), p.byte_add(8));
                    let p = rbuf.as_mut_ptr().add(offset).cast::<f32>();
                    rust.call_ptrs(imp, p, p.byte_add(4), p.byte_add(8));
                }
                assert_eq!(
                    cbuf, rbuf,
                    "row09: misaligned result diverged at offset {offset}, imp {imp}, input {input:?}"
                );
                // Not rejected: the transform actually ran (the bytes changed
                // for at least one of these inputs) — asserted in aggregate
                // below rather than per-iteration, since some inputs are fixed
                // points.
            }
            // Sanity: a plainly-transforming input does change under a
            // misaligned call, so the row is not vacuous.
            let mut cbuf = [0u8; 20];
            for (i, v) in [0.25f32, 0.5, 0.75].iter().enumerate() {
                cbuf[offset + 4 * i..offset + 4 * i + 4]
                    .copy_from_slice(&v.to_bits().to_le_bytes());
            }
            let before = cbuf;
            unsafe {
                let p = cbuf.as_mut_ptr().add(offset).cast::<f32>();
                c.call_ptrs(imp, p, p.byte_add(4), p.byte_add(8));
            }
            assert_ne!(
                cbuf, before,
                "row09: expected the misaligned call to transform the data"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10: aliased pointers are not rejected; last store wins.
// ---------------------------------------------------------------------------

#[test]
fn err_row10_aliased_pointers() {
    let (c, rust) = both();
    // All 27 aliasing maps over a 3-slot buffer, which includes every
    // pair-alias and the fully-collapsed case.
    for m0 in 0usize..3 {
        for m1 in 0usize..3 {
            for m2 in 0usize..3 {
                for &imp in &VALID {
                    let mut rng =
                        Rng::new(SEED ^ 0xA10A ^ u64::from(imp) ^ ((m0 * 9 + m1 * 3 + m2) as u64));
                    for _ in 0..400 {
                        let input = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
                        let mut ca = input;
                        let mut ra = input;
                        unsafe {
                            let p = ca.as_mut_ptr();
                            c.call_ptrs(imp, p.add(m0), p.add(m1), p.add(m2));
                            let p = ra.as_mut_ptr();
                            rust.call_ptrs(imp, p.add(m0), p.add(m1), p.add(m2));
                        }
                        assert_same(
                            &format!("row10 aliasing map [{m0},{m1},{m2}]"),
                            imp,
                            &input,
                            &ca,
                            &ra,
                        );
                    }
                }
            }
        }
    }
    // Fully collapsed: with R==G==B, only the Blue store survives, and it is
    // computed from the ORIGINAL value in all three positions. Check that the
    // observable answer matches what the C produces, for both libraries.
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0xBEEF ^ u64::from(imp));
        for _ in 0..2_000 {
            let v = rng.wide_normal();
            let mut cs = [v; 1];
            let mut rs = [v; 1];
            unsafe {
                let p = cs.as_mut_ptr();
                c.call_ptrs(imp, p, p, p);
                let p = rs.as_mut_ptr();
                rust.call_ptrs(imp, p, p, p);
            }
            assert_eq!(
                cs[0].to_bits(),
                rs[0].to_bits(),
                "row10 collapsed alias, imp {imp}, v = {:08x}: C {:08x} vs Rust {:08x}",
                v.to_bits(),
                cs[0].to_bits(),
                rs[0].to_bits()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11: values one step past every float boundary.
// ---------------------------------------------------------------------------

#[test]
fn err_row11_float_boundary_values() {
    let (c, rust) = both();
    let boundaries: [f32; 20] = [
        0.0,
        -0.0,
        f32::from_bits(1),                  // smallest positive subnormal
        f32::from_bits(0x8000_0001),        // smallest negative subnormal
        f32::from_bits(0x007F_FFFF),        // largest subnormal
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,                  // smallest normal
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0080_0001),        // one step past smallest normal
        1.0,
        -1.0,
        f32::from_bits(0x3F80_0001),        // one ULP above 1.0
        f32::from_bits(0x3F7F_FFFF),        // one ULP below 1.0
        f32::MAX,
        -f32::MAX,
        f32::from_bits(0x7F7F_FFFE),        // one ULP below MAX
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_0000),        // canonical qNaN
        f32::from_bits(0x7F80_0001),        // sNaN
    ];
    for &imp in &VALID {
        for &r in &boundaries {
            for &g in &boundaries {
                for &b in &boundaries {
                    diff(&c, &rust, "row11 float boundaries", imp, [r, g, b]);
                }
            }
        }
    }
    // No clamping/saturation: prove the C really does produce INF and NaN here,
    // so the row is not vacuous.
    //
    // Tritanopia's red is `R + 0.127*G - 0.127*B`, so R = G = MAX with
    // B = -MAX drives it past MAX and it overflows. (Protanopia's and
    // Deuteranopia's coefficients each sum to ~1.0, so MAX alone does not
    // overflow there — hence the specific choice of impairment and inputs.)
    let mut out = [f32::MAX, f32::MAX, -f32::MAX];
    c.call(CB_TRITANOPIA, &mut out);
    assert!(
        out.iter().any(|v| v.is_infinite()),
        "row11: expected overflow to infinity, got {out:?}"
    );
    let mut out = [f32::INFINITY, f32::NEG_INFINITY, 0.0];
    c.call(CB_PROTANOPIA, &mut out);
    assert!(
        out.iter().any(|v| v.is_nan()),
        "row11: expected INF + (-INF) to yield NaN, got {out:?}"
    );
}
