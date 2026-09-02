//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. `hsv_to_rgb` has no error return, no
//! `assert` and no range check, so every row is a rejection-adjacent boundary:
//! the two branch points, the undefined float→int conversions, the unclamped
//! out-of-range inputs, aliasing, and the null-pointer faults.
//!
//! Both libraries are always driven through their exported symbol via
//! `libloading`; nothing is called directly.

mod common;

use common::{Libs, Rng, SEED, SPECIAL_F32, SPECIAL_HUE};

fn libs() -> Libs {
    Libs::load()
}

// ---------------------------------------------------------------------------
// E1 / E2 — the only early `return` in the function (`s == 0`, incl. `-0.0f`).
// ---------------------------------------------------------------------------

#[test]
fn e1_e2_zero_and_negative_zero_saturation() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for &s in &[0.0f32, -0.0f32] {
        // The guard must fire regardless of hue, and must copy `v` verbatim.
        for &h in SPECIAL_HUE {
            for &v in SPECIAL_F32 {
                let (c, r) = l.both([h, s, v]);
                common::assert_bits_eq("E1/E2", [h, s, v], c, r);
                // Additionally pin down the documented C behaviour: {v,v,v}.
                assert_eq!(
                    c,
                    [v.to_bits(), v.to_bits(), v.to_bits()],
                    "E1/E2: C must write {{v,v,v}} for s=={s:e}, got {c:08x?}"
                );
            }
        }
        for _ in 0..50_000 {
            l.check("E1/E2", [rng.any_f32(), s, rng.any_f32()]);
        }
    }
}

// ---------------------------------------------------------------------------
// E3 / E4 — the `default:` arm of `switch (i)`: index < 0 and index >= 5.
// ---------------------------------------------------------------------------

#[test]
fn e3_negative_sector_index() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE3);
    for _ in 0..50_000 {
        // h < 0 => floorf(h/60) < 0 => default arm, no clamping, no rejection.
        let h = -rng.range(1e-6, 1e9);
        let s = rng.range(f32::EPSILON, 1.0);
        let v = rng.range(0.0, 1.0);
        l.check("E3", [h, s, v]);
    }
    for &h in &[-1e-30f32, -1.0, -60.0, -0.000_001, -1e9, f32::MIN] {
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check("E3", [h, s, v]);
            }
        }
    }
}

#[test]
fn e4_sector_index_ge_5() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE4);
    for _ in 0..50_000 {
        // h >= 300 => i >= 5. Includes h >= 360, i.e. outside the documented
        // hue range: the C neither wraps nor rejects.
        let h = rng.range(300.0, 1e9);
        let s = rng.range(f32::EPSILON, 1.0);
        let v = rng.range(0.0, 1.0);
        l.check("E4", [h, s, v]);
    }
    for &h in &[300.0f32, 359.999_97, 360.0, 360.000_03, 420.0, 3600.0, f32::MAX] {
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check("E4", [h, s, v]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E5 / E6 / E7 / E8 / E9 / E10 — undefined float→int conversions.
// ---------------------------------------------------------------------------

fn hue_row(row: &str, salt: u64, hues: &[f32]) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    for &h in hues {
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check(row, [h, s, v]);
            }
        }
        for _ in 0..5_000 {
            l.check(row, [h, rng.any_f32(), rng.any_f32()]);
        }
    }
}

#[test]
fn e5_hue_nan() {
    hue_row(
        "E5",
        0xE5,
        &[
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7FC0_0000),
            f32::from_bits(0x7FC0_1234),
            f32::from_bits(0xFFC0_1234),
            f32::from_bits(0x7F80_0001), // signalling NaN
            f32::from_bits(0xFF80_0001),
            f32::from_bits(0x7FFF_FFFF),
        ],
    );
}

#[test]
fn e6_e7_hue_infinities() {
    hue_row("E6/E7", 0xE6, &[f32::INFINITY, f32::NEG_INFINITY]);
}

#[test]
fn e8_hue_huge_finite() {
    let mut hues = vec![
        1e30f32,
        -1e30,
        f32::MAX,
        f32::MIN,
        1.29e11,
        -1.29e11,
        1e20,
        -1e20,
    ];
    // Randomized huge magnitudes, all with |h/60| >= 2^31.
    let mut rng = Rng::new(SEED ^ 0x8E8);
    for _ in 0..40 {
        let mag = rng.range(1.29e11, 3.0e38);
        hues.push(mag);
        hues.push(-mag);
    }
    hue_row("E8", 0xE8, &hues);
}

#[test]
fn e9_e10_int_conversion_boundaries() {
    // h/60 lands exactly on 2^31 (first value past INT_MAX -> indefinite) and
    // on -2^31 (still representable as int -> NOT indefinite).
    let two31 = 2147483648.0f32;
    let hues: &[f32] = &[
        two31 * 60.0,
        -two31 * 60.0,
        // one ULP either side of both
        f32::from_bits((two31 * 60.0).to_bits() - 1),
        f32::from_bits((two31 * 60.0).to_bits() + 1),
        f32::from_bits((-two31 * 60.0).to_bits() - 1),
        f32::from_bits((-two31 * 60.0).to_bits() + 1),
        // and directly on the conversion boundary values themselves
        two31,
        -two31,
        two31 - 128.0,
        -(two31 - 128.0),
    ];
    hue_row("E9/E10", 0xE9, hues);

    // Also probe densely around |h/60| == 2^31 by constructing h from the
    // quotient side, since h/60 is where the conversion happens.
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xEA);
    for k in -64i32..64 {
        for sign in [1.0f32, -1.0] {
            let q = f32::from_bits(
                (two31.to_bits() as i64 + k as i64) as u32,
            ) * sign;
            let h = q * 60.0;
            for _ in 0..20 {
                l.check("E9/E10", [h, rng.range(f32::EPSILON, 1.0), rng.unit()]);
            }
            l.check("E9/E10", [h, 1.0, 1.0]);
        }
    }
}

// ---------------------------------------------------------------------------
// E11 / E12 / E13 — saturation outside the documented [0,1] range (no clamp).
// ---------------------------------------------------------------------------

fn sat_row(row: &str, salt: u64, sats: &[f32]) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    for &s in sats {
        for &h in SPECIAL_HUE {
            for &v in SPECIAL_F32 {
                l.check(row, [h, s, v]);
            }
        }
        for _ in 0..5_000 {
            l.check(row, [rng.range(-800.0, 800.0), s, rng.any_f32()]);
        }
    }
}

#[test]
fn e11_saturation_nan() {
    sat_row(
        "E11",
        0xE11,
        &[
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7FC0_1234),
            f32::from_bits(0x7F80_0001),
        ],
    );
}

#[test]
fn e12_saturation_infinities() {
    sat_row("E12", 0xE12, &[f32::INFINITY, f32::NEG_INFINITY]);
}

#[test]
fn e13_saturation_out_of_range() {
    sat_row(
        "E13",
        0xE13,
        &[
            -f32::from_bits(1),
            -1e-30,
            -0.5,
            -1.0,
            -2.0,
            -255.0,
            -1e30,
            f32::MIN,
            1.000_000_1,
            1.5,
            2.0,
            255.0,
            1e30,
            f32::MAX,
        ],
    );
}

// ---------------------------------------------------------------------------
// E14 — value outside the documented range.
// ---------------------------------------------------------------------------

#[test]
fn e14_value_out_of_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE14);
    let vals: &[f32] = &[
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        -0.5,
        -1e30,
        f32::MIN,
        1.000_000_1,
        2.0,
        255.0,
        1e30,
        f32::MAX,
    ];
    for &v in vals {
        for &h in SPECIAL_HUE {
            for &s in SPECIAL_F32 {
                l.check("E14", [h, s, v]);
            }
        }
        for _ in 0..5_000 {
            l.check("E14", [rng.range(-800.0, 800.0), rng.any_f32(), v]);
        }
    }
}

// ---------------------------------------------------------------------------
// E15 — subnormals (no flush-to-zero divergence).
// ---------------------------------------------------------------------------

#[test]
fn e15_subnormals() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE15);
    let subs: &[f32] = &[
        f32::from_bits(1),
        f32::from_bits(2),
        f32::from_bits(0x0000_0FFF),
        f32::from_bits(0x007F_FFFF), // largest subnormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for &a in subs {
        for &b in subs {
            for &c in subs {
                l.check("E15", [a, b, c]);
            }
        }
        // Subnormal in one slot, ordinary values elsewhere.
        for &h in SPECIAL_HUE {
            l.check("E15", [h, a, 1.0]);
            l.check("E15", [h, 1.0, a]);
            l.check("E15", [a, 1.0, 1.0]);
        }
        for _ in 0..5_000 {
            l.check("E15", [a, rng.any_f32(), rng.any_f32()]);
            l.check("E15", [rng.any_f32(), a, rng.any_f32()]);
            l.check("E15", [rng.any_f32(), rng.any_f32(), a]);
        }
    }
    // Random subnormal bit patterns.
    for _ in 0..50_000 {
        let sub = |r: &mut Rng| f32::from_bits((r.next_u32() & 0x807F_FFFF) | 1);
        let a = sub(&mut rng);
        let b = sub(&mut rng);
        let c = sub(&mut rng);
        l.check("E15", [a, b, c]);
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — aliasing. The C reads all inputs into locals before storing, so
// in-place and partially overlapping calls are well defined and must match.
// ---------------------------------------------------------------------------

fn overlap_case(f: common::HsvToRgb, src: [f32; 3], dst_off: usize) -> Vec<u32> {
    const POISON: f32 = f32::from_bits(0xDEAD_BEEF);
    let mut buf = [POISON; 8];
    buf[0] = src[0];
    buf[1] = src[1];
    buf[2] = src[2];
    unsafe {
        let base = buf.as_mut_ptr();
        f(base.add(dst_off), base as *const f32);
    }
    buf.iter().map(|x| x.to_bits()).collect()
}

fn overlap_row(row: &str, salt: u64, dst_off: usize) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    let one = |src: [f32; 3]| {
        let c = overlap_case(l.c, src, dst_off);
        let r = overlap_case(l.rust, src, dst_off);
        assert_eq!(
            c, r,
            "[{row}] dest=src+{dst_off} diverged for src={src:?}\n  C    = {c:08x?}\n  Rust = {r:08x?}"
        );
    };
    for &h in SPECIAL_HUE {
        for &s in SPECIAL_F32 {
            one([h, s, 0.75]);
            one([h, s, f32::NAN]);
        }
    }
    for _ in 0..20_000 {
        one([rng.any_f32(), rng.any_f32(), rng.any_f32()]);
        // and with the achromatic guard armed
        one([rng.any_f32(), 0.0, rng.any_f32()]);
        one([rng.any_f32(), -0.0, rng.any_f32()]);
    }
}

#[test]
fn e16_full_aliasing_in_place() {
    overlap_row("E16", 0xE16, 0);
}

#[test]
fn e17_partial_overlap() {
    overlap_row("E17a", 0xE17, 1);
    overlap_row("E17b", 0xE18, 2);
}

// ---------------------------------------------------------------------------
// E18 / E19 / E20 — null pointers. The C has no null check, so the only
// observable behaviour is a fault. Each case runs in a forked child process
// (this same test binary re-executed) and the termination signals are compared.
// ---------------------------------------------------------------------------

const CHILD_LIB: &str = "HARVEST_NULL_LIB";
const CHILD_CASE: &str = "HARVEST_NULL_CASE";

/// Child worker: performs one null-pointer call and (if it somehow survives)
/// exits 0. Ignored so it never runs as part of the normal suite.
#[test]
#[ignore]
fn null_child() {
    let which = std::env::var(CHILD_LIB).expect("HARVEST_NULL_LIB");
    let case = std::env::var(CHILD_CASE).expect("HARVEST_NULL_CASE");

    // Load only the library under test, without the freshness assertions of
    // `Libs::load` (the parent already checked them).
    let path = match which.as_str() {
        "c" => common::c_so_path(),
        "rust" => common::rust_so_path(),
        other => panic!("bad lib {other}"),
    };
    let f: common::HsvToRgb = unsafe {
        let lib = libloading::Library::new(&path).unwrap();
        let sym: libloading::Symbol<common::HsvToRgb> = lib.get(b"hsv_to_rgb\0").unwrap();
        let raw = *sym;
        std::mem::forget(lib);
        raw
    };

    let mut dest = [0.0f32; 3];
    let src_chromatic = [123.0f32, 0.5, 0.75];
    let src_achromatic = [123.0f32, 0.0, 0.75];

    // Flush before faulting so the harness output is not lost.
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe {
        match case.as_str() {
            // E18: src == NULL (dereferenced at lib.c:7).
            "src_null" => f(dest.as_mut_ptr(), std::ptr::null()),
            // E19: dest == NULL, chromatic path (store at lib.c:51).
            "dest_null" => f(std::ptr::null_mut(), src_chromatic.as_ptr()),
            // E20: dest == NULL, achromatic path (store at lib.c:13).
            "dest_null_zero_sat" => f(std::ptr::null_mut(), src_achromatic.as_ptr()),
            // Both null.
            "both_null" => f(std::ptr::null_mut(), std::ptr::null()),
            other => panic!("bad case {other}"),
        }
    }
    // Should be unreachable.
    println!("SURVIVED");
    std::process::exit(0);
}

fn run_null_child(which: &str, case: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", "null_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD_LIB, which)
        .env(CHILD_CASE, case)
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap_or_else(|e| panic!("spawn child: {e}"));
    (out.status.code(), out.status.signal())
}

#[test]
fn e18_e19_e20_null_pointers() {
    // Sanity: make sure the artifacts are fresh before forking.
    let _ = libs();
    for case in ["src_null", "dest_null", "dest_null_zero_sat", "both_null"] {
        let c = run_null_child("c", case);
        let r = run_null_child("rust", case);
        assert_eq!(
            c, r,
            "null-pointer case `{case}`: C exited with (code, signal) = {c:?} but Rust exited with {r:?}"
        );
        // And confirm it really is a fault, not a silent success, so the test
        // cannot pass by both libraries doing nothing.
        assert_eq!(
            c.1,
            Some(libc_sigsegv()),
            "null-pointer case `{case}` should fault with SIGSEGV in C, got {c:?}"
        );
    }
}

fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

// ---------------------------------------------------------------------------
// E21 — the "out-of-range enum value" class. This API declares no enum, flag
// or mode parameter, so the analogue is an arbitrary 32-bit pattern
// reinterpreted as `float`: every one of the 2^32 patterns is a legal argument
// and none of them is rejected.
// ---------------------------------------------------------------------------

#[test]
fn e21_arbitrary_bit_patterns() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xE21);
    for _ in 0..500_000 {
        l.check("E21", [rng.any_f32(), rng.any_f32(), rng.any_f32()]);
    }
    // Systematic sweep of the exponent/sign space with a fixed mantissa, so
    // every exponent (incl. all-zero = subnormal and all-ones = inf/NaN) is hit
    // in every argument slot.
    for e in 0u32..=255 {
        for sign in [0u32, 1] {
            for mant in [0u32, 1, 0x40_0000, 0x7F_FFFF] {
                let x = f32::from_bits((sign << 31) | (e << 23) | mant);
                for &other in &[0.0f32, -0.0, 1.0, -1.0, 0.5, f32::NAN, f32::INFINITY] {
                    l.check("E21", [x, other, other]);
                    l.check("E21", [other, x, other]);
                    l.check("E21", [other, other, x]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic API boundaries that are not `ERRORS.md` rows because the C API has
// no length or count parameter — recorded here so the omission is explicit.
// ---------------------------------------------------------------------------

#[test]
fn generic_boundaries_no_length_parameter() {
    // `hsv_to_rgb` takes no length/count argument: the element count is fixed
    // at 3 by the source (`src[0..2]`, `dest[0..2]`). There is therefore no
    // "zero length" or "oversized length" input to test. What CAN be checked is
    // that both implementations touch exactly three floats and no more, from
    // both directions.
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xB0);
    const POISON: f32 = f32::from_bits(0xDEAD_BEEF);
    for _ in 0..20_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        let run = |f: common::HsvToRgb| -> Vec<u32> {
            // src at the very end of its own buffer, dest in the middle of
            // another, so an over-read or over-write shows up as a difference.
            let sbuf = [src[0], src[1], src[2]];
            let mut dbuf = [POISON; 9];
            unsafe { f(dbuf.as_mut_ptr().add(3), sbuf.as_ptr()) };
            dbuf.iter().map(|x| x.to_bits()).collect()
        };
        let c = run(l.c);
        let r = run(l.rust);
        assert_eq!(c, r, "write-extent divergence for src={src:?}");
        for (i, &w) in c.iter().enumerate() {
            if !(3..6).contains(&i) {
                assert_eq!(w, POISON.to_bits(), "C wrote outside dest[0..3] at {i}");
            }
        }
    }
}
