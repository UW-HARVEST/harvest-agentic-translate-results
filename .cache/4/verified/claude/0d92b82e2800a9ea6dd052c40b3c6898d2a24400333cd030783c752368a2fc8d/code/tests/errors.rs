//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows 1–20 compare the three output floats
//! bit-for-bit through both `.so` exports; rows 21–24 (undefined-behaviour
//! pointer inputs) re-execute this test binary as a child process and compare
//! the *termination signal* observed for the C call and the Rust call.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — delta == 0 because r == g == b > 0
// ---------------------------------------------------------------------------
#[test]
fn err01_delta_zero_positive() {
    let p = load_pair();
    for v in [0.5f32, 1.0, 255.0, f32::MAX, f32::MIN_POSITIVE, 1e-45, 7.125] {
        let (c, r) = p.call_both([v, v, v]);
        assert_bits_eq("err01 delta==0", [v, v, v], c, r);
        // documented C behaviour: h = 0, s = 0, v = max
        assert_eq!(c[0], 0.0f32.to_bits(), "err01: C h should be +0.0");
        assert_eq!(c[1], 0.0f32.to_bits(), "err01: C s should be +0.0");
        assert_eq!(c[2], v.to_bits(), "err01: C v should be max");
    }
    let mut rg = Rng::new(1);
    for _ in 0..2000 {
        let v = rg.any_f32();
        if v.is_nan() {
            continue;
        }
        p.assert_same("err01 random equal", [v, v, v]);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — max == 0 because all components are +0.0
// ---------------------------------------------------------------------------
#[test]
fn err02_all_plus_zero() {
    let p = load_pair();
    let (c, r) = p.call_both([0.0, 0.0, 0.0]);
    assert_bits_eq("err02 all +0.0", [0.0, 0.0, 0.0], c, r);
    assert_eq!(c, [0u32, 0, 0], "err02: C should return all +0.0");
}

// ---------------------------------------------------------------------------
// Row 3 — max == 0 with delta != 0 (division by zero must be skipped)
// ---------------------------------------------------------------------------
#[test]
fn err03_max_zero_delta_nonzero() {
    let p = load_pair();
    for v in [
        [0.0f32, -1.0, -2.0],
        [-1.0, 0.0, -2.0],
        [-1.0, -2.0, 0.0],
        [0.0, -f32::MAX, -1.0],
        [0.0, -f32::MIN_POSITIVE, -f32::from_bits(1)],
        [-0.0, -1.0, -2.0],
        [0.0, f32::NEG_INFINITY, -1.0],
    ] {
        let (c, r) = p.call_both(v);
        assert_bits_eq("err03 max==0, delta!=0", v, c, r);
        // early return: s must still be the 0 initialiser, never delta/0 = inf
        assert_eq!(c[1], 0.0f32.to_bits(), "err03: C s must be +0.0 (no division)");
        assert_eq!(c[0], 0.0f32.to_bits(), "err03: C h must be +0.0");
    }
    let mut rg = Rng::new(3);
    for _ in 0..2000 {
        let a = -rg.range(0.0, 1e6);
        let b = -rg.range(0.0, 1e6);
        p.assert_same("err03 random", [0.0, a, b]);
        p.assert_same("err03 random", [a, 0.0, b]);
        p.assert_same("err03 random", [a, b, 0.0]);
        p.assert_same("err03 random -0", [-0.0, a, b]);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — all components -0.0 (v keeps the negative-zero bit pattern)
// ---------------------------------------------------------------------------
#[test]
fn err04_all_minus_zero() {
    let p = load_pair();
    let src = [-0.0f32, -0.0, -0.0];
    let (c, r) = p.call_both(src);
    assert_bits_eq("err04 all -0.0", src, c, r);
    assert_eq!(c[0], 0.0f32.to_bits(), "err04: C h should be +0.0");
    assert_eq!(c[1], 0.0f32.to_bits(), "err04: C s should be +0.0");
    assert_eq!(
        c[2],
        (-0.0f32).to_bits(),
        "err04: C v should keep -0.0 (0x80000000)"
    );
}

// ---------------------------------------------------------------------------
// Row 5 — mixed +0.0 / -0.0
// ---------------------------------------------------------------------------
#[test]
fn err05_mixed_zeros() {
    let p = load_pair();
    let zs = [0.0f32, -0.0f32];
    for &a in &zs {
        for &b in &zs {
            for &c in &zs {
                let src = [a, b, c];
                let (cc, rr) = p.call_both(src);
                assert_bits_eq("err05 mixed zeros", src, cc, rr);
                assert_eq!(cc[0], 0.0f32.to_bits(), "err05: h");
                assert_eq!(cc[1], 0.0f32.to_bits(), "err05: s");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6 — all negative: s < 0 is returned as-is
// ---------------------------------------------------------------------------
#[test]
fn err06_negative_saturation() {
    let p = load_pair();
    for v in [
        [-1.0f32, -2.0, -3.0],
        [-2.0, -1.0, -3.0],
        [-3.0, -2.0, -1.0],
        [-1.0, -1.0, -2.0],
        [-f32::MIN_POSITIVE, -1.0, -2.0],
    ] {
        let (c, r) = p.call_both(v);
        assert_bits_eq("err06 negative saturation", v, c, r);
        let s = f32::from_bits(c[1]);
        assert!(s < 0.0, "err06: C s should be negative, got {s}");
    }
    let mut rg = Rng::new(6);
    for _ in 0..3000 {
        let v = [
            -rg.range(f32::MIN_POSITIVE, 1e6),
            -rg.range(f32::MIN_POSITIVE, 1e6),
            -rg.range(f32::MIN_POSITIVE, 1e6),
        ];
        p.assert_same("err06 random negative", v);
    }
}

// ---------------------------------------------------------------------------
// Rows 7/8/9/10 — NaN in r / g / b / all three
// ---------------------------------------------------------------------------
const NAN_PATTERNS: [u32; 8] = [
    0x7FC0_0000, // canonical quiet NaN
    0xFFC0_0000, // negative quiet NaN
    0x7FC0_0001,
    0x7FFF_FFFF,
    0xFFC0_1234,
    0x7F80_0001, // signalling NaN
    0xFF80_0001, // negative signalling NaN
    0x7FBF_FFFF,
];

fn nan_slot_test(slot: usize, label: &str) {
    let p = load_pair();
    let others = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        255.0,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &nb in &NAN_PATTERNS {
        let n = f32::from_bits(nb);
        for &a in &others {
            for &b in &others {
                let src = match slot {
                    0 => [n, a, b],
                    1 => [a, n, b],
                    _ => [a, b, n],
                };
                p.assert_same(label, src);
            }
        }
    }
}

#[test]
fn err07_nan_in_r() {
    nan_slot_test(0, "err07 NaN in r");
}

#[test]
fn err08_nan_in_g() {
    nan_slot_test(1, "err08 NaN in g");
}

#[test]
fn err09_nan_in_b() {
    nan_slot_test(2, "err09 NaN in b");
}

#[test]
fn err10_all_nan() {
    let p = load_pair();
    for &nb in &NAN_PATTERNS {
        let n = f32::from_bits(nb);
        let src = [n, n, n];
        let (c, r) = p.call_both(src);
        assert_bits_eq("err10 all NaN", src, c, r);
        assert!(
            f32::from_bits(c[0]).is_nan()
                && f32::from_bits(c[1]).is_nan()
                && f32::from_bits(c[2]).is_nan(),
            "err10: C should return NaNs, got {}",
            show(c)
        );
    }
    // NaN payload mixes
    for &n1 in &NAN_PATTERNS {
        for &n2 in &NAN_PATTERNS {
            for &n3 in &NAN_PATTERNS {
                p.assert_same(
                    "err10 NaN payload mix",
                    [f32::from_bits(n1), f32::from_bits(n2), f32::from_bits(n3)],
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — +inf present
// ---------------------------------------------------------------------------
#[test]
fn err11_plus_inf() {
    let p = load_pair();
    let inf = f32::INFINITY;
    for src in [
        [inf, 0.0f32, 0.0],
        [0.0, inf, 0.0],
        [0.0, 0.0, inf],
        [inf, 1.0, 2.0],
        [1.0, inf, 2.0],
        [1.0, 2.0, inf],
        [inf, inf, 0.0],
        [inf, inf, inf],
        [inf, -1.0, 1.0],
    ] {
        let (c, r) = p.call_both(src);
        assert_bits_eq("err11 +inf", src, c, r);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — -inf present
// ---------------------------------------------------------------------------
#[test]
fn err12_minus_inf() {
    let p = load_pair();
    let ninf = f32::NEG_INFINITY;
    for src in [
        [ninf, 1.0f32, 2.0],
        [1.0, ninf, 2.0],
        [1.0, 2.0, ninf],
        [ninf, ninf, 1.0],
        [ninf, ninf, ninf],
        [ninf, 0.0, 0.0],
        [ninf, -1.0, -2.0],
    ] {
        let (c, r) = p.call_both(src);
        assert_bits_eq("err12 -inf", src, c, r);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — both +inf and -inf
// ---------------------------------------------------------------------------
#[test]
fn err13_both_infs() {
    let p = load_pair();
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    let vals = [inf, ninf, 0.0f32, -0.0, 1.0, -1.0];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                p.assert_same("err13 inf combos", [a, b, c]);
            }
        }
    }
    let (c, r) = p.call_both([inf, ninf, 0.0]);
    assert_bits_eq("err13 inf/-inf/0", [inf, ninf, 0.0], c, r);
    assert!(
        f32::from_bits(c[1]).is_nan(),
        "err13: C s should be NaN (inf/inf), got {}",
        f32::from_bits(c[1])
    );
}

// ---------------------------------------------------------------------------
// Row 14 — overflow of max - min
// ---------------------------------------------------------------------------
#[test]
fn err14_delta_overflow() {
    let p = load_pair();
    for src in [
        [f32::MAX, -f32::MAX, 0.0f32],
        [-f32::MAX, f32::MAX, 0.0],
        [0.0, -f32::MAX, f32::MAX],
        [f32::MAX, -f32::MAX, f32::MAX],
        [3.0e38, -3.0e38, 1.0],
    ] {
        let (c, r) = p.call_both(src);
        assert_bits_eq("err14 delta overflow", src, c, r);
        assert!(
            f32::from_bits(c[1]).is_infinite() || f32::from_bits(c[1]).is_nan(),
            "err14: C s expected inf/NaN, got {}",
            f32::from_bits(c[1])
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 — subnormal delta (underflow region), delta != 0
// ---------------------------------------------------------------------------
#[test]
fn err15_subnormal_delta() {
    let p = load_pair();
    for src in [
        [f32::from_bits(1), 0.0f32, 0.0],
        [f32::from_bits(2), f32::from_bits(1), 0.0],
        [f32::from_bits(0x007F_FFFF), f32::from_bits(0x007F_FFFE), 0.0],
        [f32::MIN_POSITIVE, f32::from_bits(0x007F_FFFF), 0.0],
        [0.0, f32::from_bits(1), 0.0],
        [0.0, 0.0, f32::from_bits(1)],
    ] {
        let (c, r) = p.call_both(src);
        assert_bits_eq("err15 subnormal delta", src, c, r);
    }
    let mut rg = Rng::new(15);
    for _ in 0..3000 {
        let v = [
            f32::from_bits(rg.below(0x0080_0000)),
            f32::from_bits(rg.below(0x0080_0000)),
            f32::from_bits(rg.below(0x0080_0000)),
        ];
        p.assert_same("err15 random subnormal", v);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — subnormal max with delta == 0
// ---------------------------------------------------------------------------
#[test]
fn err16_subnormal_equal() {
    let p = load_pair();
    for bits in [1u32, 2, 0x0000_FFFF, 0x007F_FFFF] {
        let v = f32::from_bits(bits);
        let src = [v, v, v];
        let (c, r) = p.call_both(src);
        assert_bits_eq("err16 subnormal equal", src, c, r);
        assert_eq!(c[2], bits, "err16: C v should be the subnormal max");
        assert_eq!(c[1], 0, "err16: C s should be +0.0 (early return)");
    }
}

// ---------------------------------------------------------------------------
// Row 17 — h < 0 fixup
// ---------------------------------------------------------------------------
#[test]
fn err17_hue_fixup() {
    let p = load_pair();
    let src = [1.0f32, 0.0, 0.5];
    let (c, r) = p.call_both(src);
    assert_bits_eq("err17 hue fixup", src, c, r);
    let h = f32::from_bits(c[0]);
    assert!(
        (h - 330.0).abs() < 1e-3,
        "err17: expected h ~= 330, got {h}"
    );
    let mut rg = Rng::new(17);
    for _ in 0..3000 {
        let hi = rg.range(0.5, 1.0);
        let mid = rg.range(0.0, 0.5);
        let lo = rg.range(-1.0, 0.0);
        // r max, b > g  => hue negative before the fixup
        p.assert_same("err17 random fixup", [hi, lo, mid]);
    }
}

// ---------------------------------------------------------------------------
// Row 18 — h == -0.0 before the fixup: `h < 0` is false, no +360
// ---------------------------------------------------------------------------
#[test]
fn err18_hue_negative_zero() {
    let p = load_pair();
    // g = -0.0, b = +0.0  =>  (g - b) = -0.0  =>  h = -0.0 * 60 = -0.0
    let src = [1.0f32, -0.0, 0.0];
    let (c, r) = p.call_both(src);
    assert_bits_eq("err18 h == -0.0", src, c, r);
    assert_eq!(
        c[0],
        (-0.0f32).to_bits(),
        "err18: C h should be -0.0 (no +360 fixup), got {}",
        show(c)
    );
    // and the ordinary +0.0 case for contrast
    let src2 = [1.0f32, 0.0, 0.0];
    let (c2, r2) = p.call_both(src2);
    assert_bits_eq("err18 h == +0.0", src2, c2, r2);
    assert_eq!(c2[0], 0.0f32.to_bits(), "err18: C h should be +0.0");
}

// ---------------------------------------------------------------------------
// Row 19 — dest == src (in-place)
// ---------------------------------------------------------------------------
#[test]
fn err19_in_place() {
    let p = load_pair();
    for src in [
        [1.0f32, 0.5, 0.0],
        [0.0, 0.0, 0.0],
        [-1.0, -2.0, -3.0],
        [f32::NAN, 1.0, 2.0],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [-0.0, -0.0, -0.0],
    ] {
        p.assert_same_in_place("err19", src);
    }
    let mut rg = Rng::new(19);
    for _ in 0..3000 {
        p.assert_same_in_place("err19 random", [rg.any_f32(), rg.any_f32(), rg.any_f32()]);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — exactly 3 floats read and written
// ---------------------------------------------------------------------------
#[test]
fn err20_no_out_of_bounds_access() {
    let p = load_pair();
    let mut rg = Rng::new(20);
    const CANARY: u32 = 0x1234_5678;
    for _ in 0..2000 {
        let core = [rg.any_f32(), rg.any_f32(), rg.any_f32()];

        // src surrounded by junk: the junk must not influence the result.
        let mut sbuf = [f32::from_bits(rg.next_u32()); 9];
        sbuf[3] = core[0];
        sbuf[4] = core[1];
        sbuf[5] = core[2];

        let mut outs = Vec::new();
        for lib in [&p.c, &p.rs] {
            let mut dbuf = [f32::from_bits(CANARY); 9];
            unsafe {
                (lib.rgb_to_hsv)(dbuf.as_mut_ptr().add(3), sbuf.as_ptr().add(3));
            }
            for i in (0..3).chain(6..9) {
                assert_eq!(
                    dbuf[i].to_bits(),
                    CANARY,
                    "{}: wrote outside dest[0..3] at index {i}",
                    lib.name
                );
            }
            outs.push([dbuf[3].to_bits(), dbuf[4].to_bits(), dbuf[5].to_bits()]);
        }
        assert_bits_eq("err20 bounded access", core, outs[0], outs[1]);

        // the same 3 floats in a tight buffer must give the same answer
        let (c_tight, r_tight) = p.call_both(core);
        assert_bits_eq("err20 tight vs padded (C)", core, outs[0], c_tight);
        assert_bits_eq("err20 tight vs padded (Rust)", core, outs[1], r_tight);
    }
}

// ---------------------------------------------------------------------------
// Rows 21–24 — undefined-behaviour pointer inputs (null / unmapped).
//
// The C code has no null check, so these inputs fault. To compare the two
// implementations without killing the test runner, the probe is executed in a
// child process (this same binary, re-invoked with `HARVEST_NULL_PROBE` set)
// and the termination signals are compared.
// ---------------------------------------------------------------------------

const PROBE_ENV: &str = "HARVEST_NULL_PROBE";

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_probe(lib: &str, case: &str) -> (Outcome, String) {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["ub_probe_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(PROBE_ENV, format!("{lib}:{case}"))
        .output()
        .expect("spawn probe child");
    (
        Outcome {
            signal: out.status.signal(),
            code: out.status.code(),
        },
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The child side of the UB probes. A no-op unless `HARVEST_NULL_PROBE` is set.
#[test]
fn ub_probe_child() {
    let spec = match std::env::var(PROBE_ENV) {
        Ok(s) => s,
        Err(_) => return, // parent side: nothing to do
    };
    let (which, case) = spec.split_once(':').expect("probe spec");
    let lib = match which {
        "c" => load_c(),
        "rust" => load_rust(),
        other => panic!("bad probe lib {other}"),
    };
    let f = lib.rgb_to_hsv;
    let mut dest = [0.0f32; 3];
    let src = [1.0f32, 0.5, 0.25];
    unsafe {
        match case {
            // Row 21: src == NULL
            "src_null" => f(dest.as_mut_ptr(), std::ptr::null()),
            // Row 22: dest == NULL, src valid
            "dest_null" => f(std::ptr::null_mut(), src.as_ptr()),
            // Row 23: both NULL
            "both_null" => f(std::ptr::null_mut(), std::ptr::null()),
            // Row 24: non-null but unmapped (and misaligned) src
            "src_unmapped" => f(dest.as_mut_ptr(), 1usize as *const f32),
            // Row 24 variant: non-null but unmapped dest
            "dest_unmapped" => f(1usize as *mut f32, src.as_ptr()),
            other => panic!("bad probe case {other}"),
        }
    }
    // If we get here the call did not fault; report that distinctly.
    println!("probe {case} returned normally: {dest:?}");
    std::process::exit(42);
}

const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;

fn assert_same_ub(case: &str) {
    let (c, _c_err) = run_probe("c", case);
    let (r, r_err) = run_probe("rust", case);

    // The C implementation performs the unchecked access and faults.
    assert_eq!(
        c.signal,
        Some(SIGSEGV),
        "UB probe `{case}`: expected SIGSEGV from the C implementation, got {c:?}"
    );

    if cfg!(debug_assertions) {
        // Built with `-C debug-assertions` (hence `-C ub-checks=yes`): the Rust
        // standard library traps the *same* undefined behaviour before the
        // hardware does, and reports it as a non-unwinding panic => SIGABRT
        // with a specific message. That is a deterministic, documented
        // difference of the debug profile only; the release profile (which is
        // what an external consumer links against) is asserted to be
        // signal-identical by the `else` branch below when running
        // `cargo test --release`.
        assert!(
            r.signal == Some(SIGSEGV)
                || (r.signal == Some(SIGABRT)
                    && (r_err.contains("null pointer dereference")
                        || r_err.contains("misaligned pointer dereference")
                        || r_err.contains("unsafe precondition"))),
            "UB probe `{case}`: Rust (debug) must either fault like C ({c:?}) or trip \
             the std UB check, got {r:?} with stderr:\n{r_err}"
        );
    } else {
        assert_eq!(
            c, r,
            "UB probe `{case}`: C terminated with {c:?} but Rust terminated with {r:?}\
             \nRust stderr:\n{r_err}"
        );
    }
}

#[test]
fn err21_src_null() {
    assert_same_ub("src_null");
}

#[test]
fn err22_dest_null() {
    assert_same_ub("dest_null");
}

#[test]
fn err23_both_null() {
    assert_same_ub("both_null");
}

#[test]
fn err24_unmapped_pointers() {
    assert_same_ub("src_unmapped");
    assert_same_ub("dest_unmapped");
}
