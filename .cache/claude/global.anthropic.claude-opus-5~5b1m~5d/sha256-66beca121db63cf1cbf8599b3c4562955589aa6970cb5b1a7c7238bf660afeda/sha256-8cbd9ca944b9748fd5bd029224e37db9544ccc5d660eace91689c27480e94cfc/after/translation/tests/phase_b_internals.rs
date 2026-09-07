//! Phase B (extension) — differential tests for the functions that are `static`
//! in the C source and therefore invisible through the exported ABI.
//!
//! Both sides are loaded as shared libraries via `libloading`:
//!   * the C side from a probe TU that `#include`s the UNMODIFIED
//!     `c_src/src/lib.c` and re-exports the statics under `probe_*` names;
//!   * the Rust side from the crate's own cdylib built with
//!     `--features internal_probe`, which adds the matching `probe_*` exports.
//!
//! Run with:  cargo test --offline --features internal_probe --test phase_b_internals
//!
//! Input magnitudes are bounded so that no intermediate C expression can signed
//! overflow (which would be UB and could legitimately differ between the C
//! compiler and Rust). Worst case is the FIR arm: 8 * |firfx| * |sample|.

#![cfg(feature = "internal_probe")]

mod common;
use common::{Rng, SEED};

use std::path::PathBuf;
use std::process::Command;

#[repr(C)]
#[derive(Clone, Copy)]
struct IdxState {
    idx: u16,
    lpred: i16,
    rpred: i16,
    tag: u8,
    bcfcn: u8,
    bsfcn: u8,
    usefx: u8,
    firfx: [[i16; 8]; 4],
}

impl IdxState {
    fn zeroed() -> IdxState {
        IdxState {
            idx: 0,
            lpred: 0,
            rpred: 0,
            tag: 0,
            bcfcn: 0,
            bsfcn: 0,
            usefx: 0,
            firfx: [[0; 8]; 4],
        }
    }
}

type PredictFn = unsafe extern "C" fn(*mut i32, i32, i32, *mut IdxState) -> i32;
type IdxFn = unsafe extern "C" fn(i32) -> i32;
type SizeFn = unsafe extern "C" fn() -> usize;

/// Compile the C probe TU into a shared library (in the cargo target dir).
fn build_c_probe() -> PathBuf {
    let manifest = common::manifest_dir();
    let out_dir = manifest.join("target/cprobe");
    std::fs::create_dir_all(&out_dir).expect("create target/cprobe");
    let out = out_dir.join("libcprobe.so");

    let c_lib = common::repo_root().join("c_src/src/lib.c");
    assert!(c_lib.exists(), "missing {c_lib:?}");
    let include = common::repo_root().join("c_src/include");
    let probe = manifest.join("tests/cprobe/probe.c");

    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-std=c11"])
        // No -O flag: matches how CMake builds the real C .so (empty
        // CMAKE_BUILD_TYPE => no optimisation flags).
        .arg(format!("-DC_LIB_SOURCE=\"{}\"", c_lib.display()))
        .arg("-I")
        .arg(&include)
        .arg("-o")
        .arg(&out)
        .arg(&probe)
        .status()
        .expect("run cc");
    assert!(st.success(), "compiling the C probe failed");
    out
}

struct Probes {
    _c: libloading::Library,
    _r: libloading::Library,
    c_pfn: Vec<PredictFn>,
    r_pfn: Vec<PredictFn>,
    c_switch: PredictFn,
    r_switch: PredictFn,
    c_idx: IdxFn,
    r_idx: IdxFn,
    c_layout: (usize, usize, usize),
    r_layout: (usize, usize, usize),
}

/// The probe cdylib, built freshly into its own target dir with the
/// `internal_probe` feature enabled (never a stale `target/<profile>/*.so`).
fn rust_probe_so() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    common::rust_so_variant(profile, &["internal_probe"])
}

impl Probes {
    fn load() -> Probes {
        let cp = build_c_probe();
        let rp = rust_probe_so();
        unsafe {
            let c = libloading::Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}"));
            let r = libloading::Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}"));

            let mut c_pfn = Vec::new();
            let mut r_pfn = Vec::new();
            for n in 0..12 {
                let name = format!("probe_Pfn{n}\0");
                let cs: libloading::Symbol<PredictFn> = c
                    .get(name.as_bytes())
                    .unwrap_or_else(|e| panic!("C probe_Pfn{n}: {e}"));
                let rs: libloading::Symbol<PredictFn> = r
                    .get(name.as_bytes())
                    .unwrap_or_else(|e| panic!("Rust probe_Pfn{n}: {e}"));
                c_pfn.push(*cs);
                r_pfn.push(*rs);
            }

            let cs: libloading::Symbol<PredictFn> = c.get(b"probe_PredictSample\0").unwrap();
            let rs: libloading::Symbol<PredictFn> = r.get(b"probe_PredictSample\0").unwrap();
            let c_switch = *cs;
            let r_switch = *rs;

            let ci: libloading::Symbol<IdxFn> = c.get(b"probe_GetPredictFunc_index\0").unwrap();
            let ri: libloading::Symbol<IdxFn> = r.get(b"probe_GetPredictFunc_index\0").unwrap();
            let c_idx = *ci;
            let r_idx = *ri;

            let g = |lib: &libloading::Library| -> (usize, usize, usize) {
                let a: libloading::Symbol<SizeFn> = lib.get(b"probe_idxstate_size\0").unwrap();
                let b: libloading::Symbol<SizeFn> = lib.get(b"probe_idxstate_align\0").unwrap();
                let d: libloading::Symbol<SizeFn> =
                    lib.get(b"probe_idxstate_firfx_offset\0").unwrap();
                (a(), b(), d())
            };
            let c_layout = g(&c);
            let r_layout = g(&r);

            Probes {
                _c: c,
                _r: r,
                c_pfn,
                r_pfn,
                c_switch,
                r_switch,
                c_idx,
                r_idx,
                c_layout,
                r_layout,
            }
        }
    }
}

/// Random sample window. Bounded to +/-2047 so the FIR arm
/// (8 * 255 * 2047 ~= 4.2e6) and every other arm stay far from i32 overflow.
fn rand_samples(rng: &mut Rng) -> [i32; 8] {
    let mut s = [0i32; 8];
    for v in s.iter_mut() {
        *v = rng.range(-2047, 2047);
    }
    s
}

fn rand_state(rng: &mut Rng) -> IdxState {
    let mut st = IdxState::zeroed();
    st.idx = rng.next_u64() as u16;
    st.lpred = rng.next_u64() as i16;
    st.rpred = rng.next_u64() as i16;
    st.tag = rng.next_u64() as u8;
    st.bcfcn = rng.next_u64() as u8;
    st.bsfcn = rng.next_u64() as u8;
    st.usefx = rng.next_u64() as u8;
    for row in st.firfx.iter_mut() {
        for v in row.iter_mut() {
            *v = rng.range(-255, 255) as i16;
        }
    }
    st
}

/// The Rust `#[repr(C)]` struct must have byte-identical layout to the C one,
/// otherwise the FIR arm would read the wrong coefficients.
#[test]
fn idxstate_layout_matches() {
    let p = Probes::load();
    assert_eq!(
        p.c_layout, p.r_layout,
        "btac1c_idxstate layout differs: C (size, align, firfx offset) = {:?}, Rust = {:?}",
        p.c_layout, p.r_layout
    );
    // Cross-check the local test mirror too.
    assert_eq!(std::mem::size_of::<IdxState>(), p.c_layout.0);
    assert_eq!(std::mem::align_of::<IdxState>(), p.c_layout.1);
    assert_eq!(std::mem::offset_of!(IdxState, firfx), p.c_layout.2);
}

/// Every specialised `_PfnN` (0..=11), over randomized sample windows and every
/// `idx` residue class, including negative and extreme `idx` values.
#[test]
fn pfn_variants_match() {
    let p = Probes::load();
    let mut rng = Rng::new(SEED);

    let idx_cases: Vec<i32> = {
        let mut v: Vec<i32> = (-20..=20).collect();
        v.extend([i32::MIN + 9, -1000, 1000, i32::MAX]);
        v
    };

    for n in 0..12usize {
        for _ in 0..400 {
            let mut samples = rand_samples(&mut rng);
            let mut st = rand_state(&mut rng);
            for &idx in &idx_cases {
                let a = unsafe {
                    (p.c_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st as *mut IdxState)
                };
                let b = unsafe {
                    (p.r_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st as *mut IdxState)
                };
                assert_eq!(
                    a, b,
                    "Pfn{n} diverges: idx={idx}, samples={samples:?} -> C={a} Rust={b}"
                );
            }
        }
    }
}

/// The big `switch`-based `BTAC1C2_PredictSample`, over every arm it has:
/// 0..=11, the shared 12..=15 FIR band, and the `default` arm.
#[test]
fn switch_predictsample_matches_all_arms() {
    let p = Probes::load();
    let mut rng = Rng::new(SEED ^ 0xDEAD_BEEF);

    let mut pfcns: Vec<i32> = (-4..=20).collect();
    pfcns.extend([i32::MIN, -1000, 1000, i32::MAX]);

    for _ in 0..2000 {
        let mut samples = rand_samples(&mut rng);
        let mut st = rand_state(&mut rng);
        let idx = rng.range(-64, 64);
        for &pfcn in &pfcns {
            let a = unsafe {
                (p.c_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            let b = unsafe {
                (p.r_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            assert_eq!(
                a, b,
                "PredictSample diverges: pfcn={pfcn}, idx={idx}, samples={samples:?}, \
                 firfx={:?} -> C={a} Rust={b}",
                st.firfx
            );
        }
    }
}

/// The FIR band (pfcn 12..=15) must select firfx row `pfcn - 12`. Coefficient
/// rows are made distinguishable so a wrong-row bug cannot hide.
#[test]
fn fir_band_selects_correct_firfx_row() {
    let p = Probes::load();
    let mut rng = Rng::new(SEED ^ 0x0F1E_2D3C);

    for _ in 0..2000 {
        let mut samples = rand_samples(&mut rng);
        let mut st = IdxState::zeroed();
        // Each row gets a distinct, non-degenerate coefficient pattern.
        for (r, row) in st.firfx.iter_mut().enumerate() {
            for (k, v) in row.iter_mut().enumerate() {
                *v = (((r as i32) * 61 + (k as i32) * 7 + 3) % 511 - 255) as i16;
            }
        }
        for pfcn in 12..=15 {
            let idx = rng.range(-16, 16);
            let a = unsafe {
                (p.c_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            let b = unsafe {
                (p.r_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            assert_eq!(a, b, "FIR row selection diverges at pfcn={pfcn}, idx={idx}");
        }
    }
}

/// Negative-value behaviour: C's `>>` on negative ints (arithmetic shift under
/// gcc) and `/` (truncation toward zero) must be reproduced exactly. Driven with
/// all-negative and mixed-sign windows, which is where the two differ.
#[test]
fn negative_values_shift_and_divide_identically() {
    let p = Probes::load();
    let mut rng = Rng::new(SEED ^ 0xABCD_1234);

    for _ in 0..3000 {
        let mut samples = [0i32; 8];
        let mode = rng.next_u64() % 3;
        for v in samples.iter_mut() {
            *v = match mode {
                0 => rng.range(-2047, -1),  // all negative
                1 => rng.range(1, 2047),    // all positive
                _ => rng.range(-2047, 2047) // mixed
            };
        }
        // Also hit small magnitudes where truncation-vs-floor differs most.
        if rng.next_u64() % 2 == 0 {
            for v in samples.iter_mut() {
                *v = rng.range(-9, 9);
            }
        }
        let mut st = rand_state(&mut rng);
        let idx = rng.range(-32, 32);
        for pfcn in 0..16 {
            let a = unsafe {
                (p.c_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            let b = unsafe {
                (p.r_switch)(samples.as_mut_ptr(), idx, pfcn, &mut st as *mut IdxState)
            };
            assert_eq!(
                a, b,
                "sign handling diverges: pfcn={pfcn}, idx={idx}, samples={samples:?}"
            );
        }
        for n in 0..12usize {
            let a = unsafe {
                (p.c_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st as *mut IdxState)
            };
            let b = unsafe {
                (p.r_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st as *mut IdxState)
            };
            assert_eq!(a, b, "Pfn{n} sign handling diverges: idx={idx}, samples={samples:?}");
        }
    }
}

/// `BTAC1C2_GetPredictFunc`'s dispatch decision, observed as an index.
/// Also confirms the documented divergence: the `_PfnN` variants for N = 10 and
/// 11 are NOT arithmetically equal to arms 10 and 11 of the big switch (the C
/// source shifts by different amounts), so a translation that "fixed" the C
/// would be caught here.
#[test]
fn getpredictfunc_dispatch_index_matches() {
    let p = Probes::load();
    for pfcn in -2000..=2000 {
        let a = unsafe { (p.c_idx)(pfcn) };
        let b = unsafe { (p.r_idx)(pfcn) };
        assert_eq!(a, b, "dispatch index diverges at pfcn={pfcn}: C={a} Rust={b}");
        let expect = if (0..=11).contains(&pfcn) { pfcn } else { -1 };
        assert_eq!(a, expect, "unexpected dispatch index for pfcn={pfcn}");
    }
    for pfcn in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        let a = unsafe { (p.c_idx)(pfcn) };
        let b = unsafe { (p.r_idx)(pfcn) };
        assert_eq!(a, b);
        assert_eq!(a, -1);
    }
}

/// Guards the C source's deliberate inconsistency: `_Pfn10` uses `>> 3` while
/// switch arm 10 uses `>> 4`, and `_Pfn11` uses `>> 1` while switch arm 11 uses
/// `>> 3`. Both libraries must reproduce that asymmetry identically.
#[test]
fn pfn10_pfn11_differ_from_switch_arms_in_both_libraries() {
    let p = Probes::load();
    let mut rng = Rng::new(SEED ^ 0x5151_5151);
    let mut c_diff10 = 0;
    let mut c_diff11 = 0;

    for _ in 0..2000 {
        let mut samples = rand_samples(&mut rng);
        let mut st = rand_state(&mut rng);
        let idx = rng.range(-32, 32);
        for n in [10usize, 11usize] {
            let c_pfn =
                unsafe { (p.c_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st) };
            let r_pfn =
                unsafe { (p.r_pfn[n])(samples.as_mut_ptr(), idx, n as i32, &mut st) };
            let c_sw =
                unsafe { (p.c_switch)(samples.as_mut_ptr(), idx, n as i32, &mut st) };
            let r_sw =
                unsafe { (p.r_switch)(samples.as_mut_ptr(), idx, n as i32, &mut st) };
            assert_eq!(c_pfn, r_pfn, "Pfn{n} diverges");
            assert_eq!(c_sw, r_sw, "switch arm {n} diverges");
            // The C/Rust *relationship* between the two must also match.
            assert_eq!(
                c_pfn == c_sw,
                r_pfn == r_sw,
                "Pfn{n} vs switch-arm-{n} relationship differs between C and Rust"
            );
            if c_pfn != c_sw {
                if n == 10 {
                    c_diff10 += 1;
                } else {
                    c_diff11 += 1;
                }
            }
        }
    }
    assert!(
        c_diff10 > 0 && c_diff11 > 0,
        "expected the C source's Pfn10/Pfn11 vs switch asymmetry to be observable \
         (saw {c_diff10} / {c_diff11} differing cases)"
    );
}
