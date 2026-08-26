//! Differential test-suite: C `.so` vs. Rust `.so`, both loaded with
//! `libloading` and called only through their exported symbols.
//!
//! Four shared libraries are involved:
//!
//! | handle     | library                                     | purpose                                    |
//! |------------|---------------------------------------------|--------------------------------------------|
//! | `c_main`   | `c_src/build/libtranslated_rust.so`         | the shipped C library (`call_predict`)     |
//! | `r_main`   | `target/release/libcall_predict_lib.so`     | the shipped Rust library (`call_predict`)  |
//! | `c_aux`    | built from `tests/aux/aux_c.c`              | exposes lib.c's `static` internals         |
//! | `r_aux`    | built from `src/lib.rs` + `aux_rust_suffix` | exposes lib.rs's private internals         |
//!
//! Nothing under `c_src/` is modified: the C shim `#include`s the original
//! translation unit. Rust is *never* called directly — every Rust value in
//! this file comes back through `dlsym`.

use libloading::Library;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// C types mirrored for the FFI boundary
// ---------------------------------------------------------------------------

/// `struct btac1c_idxstate_s` (c_src/src/lib.c:7)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    fn zeroed() -> Self {
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

type FnCallPredict = unsafe extern "C" fn(i32) -> i32;
type FnPredSample = unsafe extern "C" fn(*mut i32, i32, i32, *mut IdxState) -> i32;
type FnSelPred = unsafe extern "C" fn(i32, *mut i32, i32, i32, *mut IdxState) -> i32;
type FnIntInt = unsafe extern "C" fn(i32) -> i32;
type FnLayout = unsafe extern "C" fn(*mut usize);

// ---------------------------------------------------------------------------
// Building / loading the four libraries
// ---------------------------------------------------------------------------

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn run(cmd: &mut Command) -> String {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd:?}: {e}"));
    if !out.status.success() {
        panic!(
            "command failed {cmd:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The shipped C library, built exactly as documented in the task.
fn build_c_main() -> PathBuf {
    let so = crate_dir().join("c_src/build/libtranslated_rust.so");
    if so.exists() {
        return so;
    }
    let build = crate_dir().join("c_src/build");
    std::fs::create_dir_all(&build).unwrap();
    run(Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON"));
    run(Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."]));
    assert!(so.exists(), "C .so not produced at {so:?}");
    so
}

/// The shipped Rust cdylib. Preferred: the artifact `cargo build` produced.
/// Fallback (so the suite is self-contained): compile `src/lib.rs` with
/// `rustc --crate-type cdylib`, which yields the same exported surface.
fn build_rust_main() -> PathBuf {
    fn newer_than_source(p: &Path) -> bool {
        let src = crate_dir().join("src/lib.rs");
        match (
            std::fs::metadata(p).and_then(|m| m.modified()),
            std::fs::metadata(&src).and_then(|m| m.modified()),
        ) {
            (Ok(a), Ok(b)) => a >= b,
            _ => false,
        }
    }
    if let Ok(p) = std::env::var("RUST_TARGET_SO") {
        let p = PathBuf::from(p);
        if p.exists() {
            return p;
        }
    }
    for prof in ["release", "debug"] {
        let p = crate_dir().join(format!("target/{prof}/libcall_predict_lib.so"));
        // never test a stale artifact
        if p.exists() && newer_than_source(&p) {
            return p;
        }
    }
    let so = out_dir().join("libcall_predict_lib_fallback.so");
    run(Command::new("rustc")
        .arg("--edition=2024")
        .arg("-O")
        .args(["--crate-type", "cdylib"])
        .arg("-Cdebug-assertions=off")
        .arg("-Coverflow-checks=off")
        .arg("-o")
        .arg(&so)
        .arg(crate_dir().join("src/lib.rs")));
    so
}

/// C shim exposing lib.c's `static` functions (built outside `c_src/`).
fn build_c_aux() -> PathBuf {
    let so = out_dir().join("libaux_c.so");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    run(Command::new(cc)
        .arg("-O2")
        .arg("-fPIC")
        .arg("-shared")
        .arg("-std=c99")
        .arg("-w")
        .arg(format!("-I{}", crate_dir().join("c_src/src").display()))
        .arg(format!("-I{}", crate_dir().join("c_src/include").display()))
        .arg(crate_dir().join("tests/aux/aux_c.c"))
        .arg("-o")
        .arg(&so));
    so
}

/// Rust shim exposing lib.rs's private functions: `src/lib.rs` verbatim +
/// `tests/aux/aux_rust_suffix.rs`, compiled as a cdylib.
fn build_rust_aux() -> PathBuf {
    let lib_rs = std::fs::read_to_string(crate_dir().join("src/lib.rs")).unwrap();
    let suffix = std::fs::read_to_string(crate_dir().join("tests/aux/aux_rust_suffix.rs")).unwrap();
    let src = out_dir().join("aux_rust.rs");
    std::fs::write(&src, format!("{lib_rs}\n{suffix}")).unwrap();
    // `AUX_OVERFLOW_CHECKS=on` builds the Rust shim with debug-assertions and
    // overflow-checks enabled: the translation must then still agree with C
    // (i.e. it must never rely on an *unchecked* overflow) and must not panic.
    let checks = std::env::var("AUX_OVERFLOW_CHECKS").unwrap_or_default() == "on";
    let flag = if checks { "on" } else { "off" };
    let so = out_dir().join(format!("libaux_rust_{flag}.so"));
    run(Command::new("rustc")
        .arg("--edition=2024")
        .arg("-O")
        .args(["--crate-type", "cdylib"])
        .arg(format!("-Cdebug-assertions={flag}"))
        .arg(format!("-Coverflow-checks={flag}"))
        .arg("-Awarnings")
        .arg("-o")
        .arg(&so)
        .arg(&src));
    so
}

struct Libs {
    c_main: Library,
    r_main: Library,
    c_aux: Library,
    r_aux: Library,
    c_main_path: PathBuf,
    r_main_path: PathBuf,
}

// SAFETY: only used for the process-lifetime `OnceLock` below; `Library` is
// `Send + Sync` already, this is just to keep the struct usable from tests.
fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_main_path = build_c_main();
        let r_main_path = build_rust_main();
        let c_aux_path = build_c_aux();
        let r_aux_path = build_rust_aux();
        unsafe {
            Libs {
                c_main: Library::new(&c_main_path).unwrap(),
                r_main: Library::new(&r_main_path).unwrap(),
                c_aux: Library::new(&c_aux_path).unwrap(),
                r_aux: Library::new(&r_aux_path).unwrap(),
                c_main_path,
                r_main_path,
            }
        }
    })
}

fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    unsafe {
        *lib.get::<T>(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)))
    }
}

fn call_predict_pair() -> (FnCallPredict, FnCallPredict) {
    let l = libs();
    (
        sym(&l.c_main, b"call_predict\0"),
        sym(&l.r_main, b"call_predict\0"),
    )
}

fn aux_predict_sample_pair() -> (FnPredSample, FnPredSample) {
    let l = libs();
    (
        sym(&l.c_aux, b"aux_predict_sample\0"),
        sym(&l.r_aux, b"aux_predict_sample\0"),
    )
}

fn aux_pfn_pair() -> (FnSelPred, FnSelPred) {
    let l = libs();
    (sym(&l.c_aux, b"aux_pfn\0"), sym(&l.r_aux, b"aux_pfn\0"))
}

fn aux_dispatch_pair() -> (FnSelPred, FnSelPred) {
    let l = libs();
    (
        sym(&l.c_aux, b"aux_getpredict_call\0"),
        sym(&l.r_aux, b"aux_getpredict_call\0"),
    )
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

const SEED: u64 = 0x243F_6A88_85A3_08D3;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// uniform in `-cap ..= cap`
    fn ranged(&mut self, cap: i32) -> i32 {
        let span = 2u64 * cap as u64 + 1;
        (self.next_u64() % span) as i64 as i32 - cap
    }
}

// ---------------------------------------------------------------------------
// Input shapes (axes C / D / E of CONFIGS.md)
// ---------------------------------------------------------------------------

/// Caps chosen so every C accumulator stays inside `int`:
/// worst-case tap-weight sum in arms 0..11 is 120 (arm 8), 120 * 2^20 < 2^31.
const CAP_BIG: i32 = 1 << 20;
/// For the `firfx` arms: sum|fx| can reach 8 * 32768, and 8*32768*4096 == 2^30.
const CAP_FIRFX: i32 = 1 << 12;

const N_PSAMP_SHAPES: u32 = 10;

fn make_psamp(shape: u32, cap: i32, rng: &mut Rng) -> [i32; 8] {
    let mut v = [0i32; 8];
    match shape % N_PSAMP_SHAPES {
        0 => {} // all zeros
        1 => {
            let c = rng.ranged(cap);
            v = [c; 8]; // constant (DC)
        }
        2 => {
            let step = 1 + (rng.next_u64() % (cap as u64 / 8).max(1)) as i32;
            for (k, e) in v.iter_mut().enumerate() {
                *e = (k as i32 - 4) * step; // ascending ramp through zero
            }
        }
        3 => {
            let step = 1 + (rng.next_u64() % (cap as u64 / 8).max(1)) as i32;
            for (k, e) in v.iter_mut().enumerate() {
                *e = (4 - k as i32) * step; // descending ramp
            }
        }
        4 => {
            for (k, e) in v.iter_mut().enumerate() {
                *e = if k % 2 == 0 { cap } else { -cap }; // alternating extremes
            }
        }
        5 => {
            for e in v.iter_mut() {
                *e = -((rng.next_u64() % (cap as u64 + 1)) as i32); // all negative
            }
        }
        6 => {
            let c16 = cap.min(32767);
            for e in v.iter_mut() {
                *e = rng.ranged(c16); // 16-bit audio range
            }
        }
        7 => {
            for e in v.iter_mut() {
                *e = rng.ranged(cap); // full random in cap
            }
        }
        8 => {
            let hi = cap.min(32767);
            let lo = cap.min(32768);
            for (k, e) in v.iter_mut().enumerate() {
                *e = match k % 4 {
                    0 => hi,   // i16::MAX
                    1 => -lo,  // i16::MIN
                    2 => 0,
                    _ => -1,
                };
            }
        }
        _ => {
            for (k, e) in v.iter_mut().enumerate() {
                // small mixed-sign values around zero (rounding-sensitive)
                *e = (rng.next_u64() % 9) as i32 - 4 + if k == 3 { -1 } else { 0 };
            }
        }
    }
    v
}

const N_FIRFX_SHAPES: u32 = 8;

fn make_firfx(shape: u32, rng: &mut Rng) -> [[i16; 8]; 4] {
    let mut f = [[0i16; 8]; 4];
    match shape % N_FIRFX_SHAPES {
        0 => {} // zeros
        1 => {
            for row in f.iter_mut() {
                row[0] = 256; // unit gain
            }
        }
        2 => {
            for row in f.iter_mut() {
                for e in row.iter_mut() {
                    *e = rng.next_u64() as u16 as i16; // full-range s16
                }
            }
        }
        3 => {
            for row in f.iter_mut() {
                for (k, e) in row.iter_mut().enumerate() {
                    *e = if k % 2 == 0 { i16::MAX } else { i16::MIN };
                }
            }
        }
        4 => {
            // distinct pattern per row: detects a wrong `firfx[pfcn-12]` index
            for (r, row) in f.iter_mut().enumerate() {
                for (k, e) in row.iter_mut().enumerate() {
                    *e = ((r as i32 + 1) * 1000 + k as i32 * 7 - 3500) as i16;
                }
            }
        }
        5 => {
            for row in f.iter_mut() {
                for e in row.iter_mut() {
                    *e = rng.ranged(256) as i16; // small coefficients
                }
            }
        }
        6 => {
            for row in f.iter_mut() {
                for e in row.iter_mut() {
                    *e = -1;
                }
            }
        }
        _ => {
            for row in f.iter_mut() {
                for e in row.iter_mut() {
                    *e = i16::MIN; // 2's-complement edge
                }
            }
        }
    }
    f
}

/// Interesting `idx` values (axis C). Every one keeps `idx - 8` representable,
/// so no signed overflow (UB) is triggered in the C code.
fn idx_values(rng: &mut Rng) -> Vec<i32> {
    let mut v: Vec<i32> = (0..=8).collect();
    v.extend([
        -1,
        -2,
        -7,
        -8,
        -9,
        -16,
        9,
        15,
        16,
        17,
        1_000,
        -1_000,
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 7,
        i32::MIN + 8,
        i32::MIN + 9,
        i32::MIN + 15,
    ]);
    for _ in 0..8 {
        v.push(rng.next_i32() / 2); // random, still far from overflow
    }
    v
}

/// Halve the FIR coefficients until `sum|fx| * max|psamp|` fits in `int`, so
/// that the `firfx` arms (12..15) stay inside the well-defined C domain for the
/// generated `psamp`. Shape (sign pattern, row distinctness, extremes) is
/// preserved; only the magnitude shrinks.
fn scale_firfx_to_fit(firfx: &mut [[i16; 8]; 4], psamp: &[i32; 8]) {
    let m = psamp
        .iter()
        .map(|&x| (x as i64).abs())
        .max()
        .unwrap()
        .max(1);
    loop {
        let worst = firfx
            .iter()
            .map(|r| r.iter().map(|&c| (c as i64).abs()).sum::<i64>())
            .max()
            .unwrap();
        if worst == 0 || worst.saturating_mul(m) <= i32::MAX as i64 {
            return;
        }
        for r in firfx.iter_mut() {
            for e in r.iter_mut() {
                *e = (*e as i32 / 2) as i16;
            }
        }
    }
}

/// Conservative check that no C `int` accumulator can overflow (C signed
/// overflow is UB, so the differential comparison must stay inside the
/// well-defined domain). Scales `firfx` down first if needed.
fn assert_no_overflow(psamp: &[i32; 8], firfx: &mut [[i16; 8]; 4]) {
    scale_firfx_to_fit(firfx, psamp);
    let m = psamp.iter().map(|&x| (x as i64).abs()).max().unwrap();
    assert!(
        m * 120 <= i32::MAX as i64,
        "psamp magnitude {m} can overflow int in arm 8"
    );
    for row in firfx.iter() {
        let s: i64 = row.iter().map(|&c| (c as i64).abs()).sum();
        assert!(
            s * m <= i32::MAX as i64,
            "firfx row sum {s} * psamp {m} can overflow int in arms 12..15"
        );
    }
}

// ---------------------------------------------------------------------------
// The core grid runner
// ---------------------------------------------------------------------------

/// Which entry point to drive.
#[derive(Clone, Copy, Debug)]
enum Ep {
    /// `BTAC1C2_PredictSample(psamp, idx, pfcn, ridx)`
    Generic,
    /// `BTAC1C2_PredictSample_Pfn<which>(psamp, idx, pfcn, ridx)`
    Pfn(i32),
    /// `((pred_fn)BTAC1C2_GetPredictFunc(sel))(psamp, idx, pfcn, ridx)`
    Dispatch(i32),
}

struct GridStats {
    cases: usize,
}

/// Runs the full axis-C × axis-D × axis-E grid for one entry point and one
/// `pfcn`, comparing the C and Rust libraries call-for-call.
fn run_grid(ep: Ep, pfcn: i32, iters: u32, firfx_shapes: &[u32], cap: i32) -> GridStats {
    let (c_generic, r_generic) = aux_predict_sample_pair();
    let (c_pfn, r_pfn) = aux_pfn_pair();
    let (c_disp, r_disp) = aux_dispatch_pair();

    let mut rng = Rng::new(SEED ^ ((pfcn as i64 as u64) << 8) ^ (iters as u64) << 40);
    let idxs = idx_values(&mut rng);
    let mut cases = 0usize;

    for &fx_shape in firfx_shapes {
        for shape in 0..N_PSAMP_SHAPES {
            for it in 0..iters {
                let mut firfx = make_firfx(fx_shape, &mut rng);
                let psamp = make_psamp(shape, cap, &mut rng);
                assert_no_overflow(&psamp, &mut firfx);

                let mut st = IdxState::zeroed();
                st.firfx = firfx;
                // the other struct fields are never read by lib.c, but vary
                // them anyway so that a bogus field offset would show up.
                st.idx = rng.next_u64() as u16;
                st.lpred = rng.next_u64() as u16 as i16;
                st.rpred = rng.next_u64() as u16 as i16;
                st.tag = rng.next_u64() as u8;
                st.bcfcn = rng.next_u64() as u8;
                st.bsfcn = rng.next_u64() as u8;
                st.usefx = rng.next_u64() as u8;

                for &idx in &idxs {
                    let mut c_samp = psamp;
                    let mut r_samp = psamp;
                    let mut c_st = st;
                    let mut r_st = st;

                    let (cv, rv) = unsafe {
                        match ep {
                            Ep::Generic => (
                                c_generic(c_samp.as_mut_ptr(), idx, pfcn, &mut c_st),
                                r_generic(r_samp.as_mut_ptr(), idx, pfcn, &mut r_st),
                            ),
                            Ep::Pfn(w) => (
                                c_pfn(w, c_samp.as_mut_ptr(), idx, pfcn, &mut c_st),
                                r_pfn(w, r_samp.as_mut_ptr(), idx, pfcn, &mut r_st),
                            ),
                            Ep::Dispatch(sel) => (
                                c_disp(sel, c_samp.as_mut_ptr(), idx, pfcn, &mut c_st),
                                r_disp(sel, r_samp.as_mut_ptr(), idx, pfcn, &mut r_st),
                            ),
                        }
                    };

                    assert_eq!(
                        cv, rv,
                        "MISMATCH ep={ep:?} pfcn={pfcn} idx={idx} shape={shape} \
                         fx_shape={fx_shape} it={it}\npsamp={psamp:?}\nfirfx={firfx:?}\n\
                         C={cv} Rust={rv}"
                    );
                    // neither implementation may write to its inputs
                    assert_eq!(c_samp, psamp, "C mutated psamp");
                    assert_eq!(r_samp, psamp, "Rust mutated psamp");
                    assert_eq!(c_st, st, "C mutated *ridx");
                    assert_eq!(r_st, st, "Rust mutated *ridx");
                    assert_eq!(c_st, r_st, "state diverged");
                    cases += 1;
                }
            }
        }
    }
    GridStats { cases }
}

const PLAIN_FX: &[u32] = &[2, 5]; // firfx is ignored by arms 0..11 — vary it anyway
const ALL_FX: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7];

// ===========================================================================
// PHASE B — valid-path differential tests (CONFIGS.md rows)
// ===========================================================================

/// CONFIGS row 1 (+ ERRORS E13): every valid dispatch value.
#[test]
fn cfg01_call_predict_valid_range() {
    let (c, r) = call_predict_pair();
    for pfcn in 0..=11 {
        let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
        assert_eq!(cv, rv, "call_predict({pfcn}): C={cv} Rust={rv}");
        assert_eq!(
            cv, 1,
            "C itself must report a distinct predictor for pfcn={pfcn}"
        );
    }
}

/// CONFIGS row 2 / ERRORS E3,E4: 12..15 are `BTAC1C2_PredictSample` arms but
/// invalid for the dispatcher.
#[test]
fn cfg02_call_predict_predictsample_only_arms() {
    let (c, r) = call_predict_pair();
    for pfcn in 12..=15 {
        let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
        assert_eq!(cv, rv, "call_predict({pfcn}): C={cv} Rust={rv}");
        assert_eq!(cv, 0);
    }
}

/// CONFIGS row 4: repeated calls — function-pointer identity must be stable.
#[test]
fn cfg04_call_predict_repeated() {
    let (c, r) = call_predict_pair();
    for _ in 0..16 {
        for pfcn in -4..=19 {
            let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
            assert_eq!(cv, rv, "call_predict({pfcn}) not stable across calls");
        }
    }
}

macro_rules! generic_arm_test {
    ($name:ident, $pfcn:expr, $fx:expr, $cap:expr) => {
        #[test]
        fn $name() {
            let s = run_grid(Ep::Generic, $pfcn, 16, $fx, $cap);
            assert!(s.cases > 1000, "too few cases: {}", s.cases);
        }
    };
}

// CONFIGS rows 5..16 — BTAC1C2_PredictSample arms 0..11
generic_arm_test!(cfg05_generic_arm0, 0, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg06_generic_arm1, 1, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg07_generic_arm2, 2, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg08_generic_arm3, 3, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg09_generic_arm4, 4, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg10_generic_arm5, 5, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg11_generic_arm6, 6, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg12_generic_arm7, 7, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg13_generic_arm8, 8, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg14_generic_arm9, 9, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg15_generic_arm10, 10, PLAIN_FX, CAP_BIG);
generic_arm_test!(cfg16_generic_arm11, 11, PLAIN_FX, CAP_BIG);

// CONFIGS rows 17..20 — the firfx arms, all firfx shapes
generic_arm_test!(cfg17_generic_arm12_firfx0, 12, ALL_FX, CAP_FIRFX);
generic_arm_test!(cfg18_generic_arm13_firfx1, 13, ALL_FX, CAP_FIRFX);
generic_arm_test!(cfg19_generic_arm14_firfx2, 14, ALL_FX, CAP_FIRFX);
generic_arm_test!(cfg20_generic_arm15_firfx3, 15, ALL_FX, CAP_FIRFX);

macro_rules! pfn_test {
    ($name:ident, $which:expr) => {
        #[test]
        fn $name() {
            // `pfcn` is ignored by the specialised predictors: feed it garbage
            // (including out-of-range values) to prove it is ignored.
            for pfcn in [$which, 0, 15, -1, 99, i32::MIN, i32::MAX] {
                let s = run_grid(Ep::Pfn($which), pfcn, 4, PLAIN_FX, CAP_BIG);
                assert!(s.cases > 200, "too few cases: {}", s.cases);
            }
        }
    };
}

// CONFIGS rows 21..32 — the 12 specialised predictors
pfn_test!(cfg21_pfn0, 0);
pfn_test!(cfg22_pfn1, 1);
pfn_test!(cfg23_pfn2, 2);
pfn_test!(cfg24_pfn3, 3);
pfn_test!(cfg25_pfn4, 4);
pfn_test!(cfg26_pfn5, 5);
pfn_test!(cfg27_pfn6, 6);
pfn_test!(cfg28_pfn7, 7);
pfn_test!(cfg29_pfn8, 8);
pfn_test!(cfg30_pfn9, 9);
pfn_test!(cfg31_pfn10, 10);
pfn_test!(cfg32_pfn11, 11);

/// CONFIGS row 33: dispatcher maps each `pfcn` to the *right* predictor.
#[test]
fn cfg33_dispatch_valid_range() {
    for sel in 0..=11 {
        let s = run_grid(Ep::Dispatch(sel), sel, 4, PLAIN_FX, CAP_BIG);
        assert!(s.cases > 200);
    }
}

/// CONFIGS row 34: `pfcn` 12..15 fall through `default:` to the generic
/// predictor, which then takes its own `firfx` arms.
#[test]
fn cfg34_dispatch_firfx_arms() {
    for sel in 12..=15 {
        let s = run_grid(Ep::Dispatch(sel), sel, 4, ALL_FX, CAP_FIRFX);
        assert!(s.cases > 200);
    }
}

/// CONFIGS row 36: struct layout parity across the FFI boundary.
#[test]
fn cfg36_struct_layout() {
    let l = libs();
    let c: FnLayout = sym(&l.c_aux, b"aux_layout\0");
    let r: FnLayout = sym(&l.r_aux, b"aux_layout\0");
    let mut cv = [0usize; 14];
    let mut rv = [0usize; 14];
    unsafe {
        c(cv.as_mut_ptr());
        r(rv.as_mut_ptr());
    }
    let names = [
        "sizeof(btac1c_idxstate)",
        "alignof(btac1c_idxstate)",
        "offsetof idx",
        "offsetof lpred",
        "offsetof rpred",
        "offsetof tag",
        "offsetof bcfcn",
        "offsetof bsfcn",
        "offsetof usefx",
        "offsetof firfx",
        "sizeof(btac1c_u16)",
        "sizeof(btac1c_s16)",
        "sizeof(btac1c_byte)",
        "sizeof(int)",
    ];
    for i in 0..14 {
        assert_eq!(cv[i], rv[i], "{}: C={} Rust={}", names[i], cv[i], rv[i]);
    }
    // sanity: the test's own mirror struct must agree too
    assert_eq!(cv[0], std::mem::size_of::<IdxState>());
    assert_eq!(cv[9], std::mem::offset_of!(IdxState, firfx));
}

/// CONFIGS row 37: `idx` extremes for every entry point.
#[test]
fn cfg37_idx_extremes() {
    let (c_generic, r_generic) = aux_predict_sample_pair();
    let (c_pfn, r_pfn) = aux_pfn_pair();
    let (c_disp, r_disp) = aux_dispatch_pair();
    let mut rng = Rng::new(SEED ^ 0xAAAA);
    let extremes = [
        i32::MIN + 8,
        i32::MIN + 9,
        i32::MIN + 15,
        i32::MIN + 16,
        -1,
        0,
        1,
        7,
        8,
        i32::MAX - 8,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut cases = 0;
    for it in 0..64 {
        let psamp = make_psamp(it % N_PSAMP_SHAPES, CAP_FIRFX, &mut rng);
        let mut firfx = make_firfx(it % N_FIRFX_SHAPES, &mut rng);
        assert_no_overflow(&psamp, &mut firfx);
        let mut st = IdxState::zeroed();
        st.firfx = firfx;
        for &idx in &extremes {
            for pfcn in -2..=17 {
                let mut a = psamp;
                let mut b = psamp;
                let mut sa = st;
                let mut sb = st;
                unsafe {
                    assert_eq!(
                        c_generic(a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r_generic(b.as_mut_ptr(), idx, pfcn, &mut sb),
                        "generic idx={idx} pfcn={pfcn} psamp={psamp:?}"
                    );
                    assert_eq!(
                        c_disp(pfcn, a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r_disp(pfcn, b.as_mut_ptr(), idx, pfcn, &mut sb),
                        "dispatch sel={pfcn} idx={idx}"
                    );
                    if (0..12).contains(&pfcn) {
                        assert_eq!(
                            c_pfn(pfcn, a.as_mut_ptr(), idx, pfcn, &mut sa),
                            r_pfn(pfcn, b.as_mut_ptr(), idx, pfcn, &mut sb),
                            "pfn{pfcn} idx={idx}"
                        );
                    }
                }
                cases += 1;
            }
        }
    }
    assert!(cases > 10_000, "cases={cases}");
}

/// CONFIGS rows 38 + 39: value-dependent paths — negative / mixed-sign inputs
/// (arithmetic `>>` vs. truncating `/`) and near-overflow magnitudes.
#[test]
fn cfg38_39_value_shapes() {
    let (c_generic, r_generic) = aux_predict_sample_pair();
    let (c_pfn, r_pfn) = aux_pfn_pair();
    let mut rng = Rng::new(SEED ^ 0xBBBB);
    let mut cases = 0;
    // shapes 5 (all negative), 8 (s16 extremes), 9 (small mixed sign), 4 (±cap)
    for &shape in &[5u32, 8, 9, 4, 2, 3] {
        for &cap in &[CAP_FIRFX, 32768, 4096, 255, 7, 1] {
            for it in 0..8 {
                let psamp = make_psamp(shape, cap, &mut rng);
                let mut firfx = make_firfx(it % N_FIRFX_SHAPES, &mut rng);
                assert_no_overflow(&psamp, &mut firfx);
                let mut st = IdxState::zeroed();
                st.firfx = firfx;
                for idx in 0..8 {
                    for pfcn in -1..=16 {
                        let mut a = psamp;
                        let mut b = psamp;
                        let mut sa = st;
                        let mut sb = st;
                        unsafe {
                            assert_eq!(
                                c_generic(a.as_mut_ptr(), idx, pfcn, &mut sa),
                                r_generic(b.as_mut_ptr(), idx, pfcn, &mut sb),
                                "generic pfcn={pfcn} idx={idx} shape={shape} cap={cap} \
                                 psamp={psamp:?} firfx={firfx:?}"
                            );
                            if (0..12).contains(&pfcn) {
                                assert_eq!(
                                    c_pfn(pfcn, a.as_mut_ptr(), idx, pfcn, &mut sa),
                                    r_pfn(pfcn, b.as_mut_ptr(), idx, pfcn, &mut sb),
                                    "pfn{pfcn} idx={idx} shape={shape} cap={cap} \
                                     psamp={psamp:?}"
                                );
                            }
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    assert!(cases > 10_000, "cases={cases}");
}

/// CONFIGS row 40: `firfx` row selection (`[pfcn-12]`) and `/256` on negative
/// accumulators.
#[test]
fn cfg40_firfx_row_selection() {
    let (c_generic, r_generic) = aux_predict_sample_pair();
    let mut rng = Rng::new(SEED ^ 0xCCCC);
    let mut cases = 0;
    for it in 0..64 {
        // every row distinct so a wrong row index cannot pass
        let mut firfx = [[0i16; 8]; 4];
        for (r, row) in firfx.iter_mut().enumerate() {
            for (k, e) in row.iter_mut().enumerate() {
                *e = match it % 4 {
                    0 => ((r as i32 + 1) * 4096 + k as i32) as i16,
                    1 => if r % 2 == 0 { i16::MAX } else { i16::MIN },
                    2 => (rng.next_u64() as u16 as i16) & (!0i16 ^ (r as i16)),
                    _ => (-256 * (r as i32 + 1) + k as i32 * 3) as i16,
                };
            }
        }
        let psamp = make_psamp(it % N_PSAMP_SHAPES, CAP_FIRFX, &mut rng);
        assert_no_overflow(&psamp, &mut firfx);
        let mut st = IdxState::zeroed();
        st.firfx = firfx;
        for idx in -8..=16 {
            for pfcn in 11..=16 {
                let mut a = psamp;
                let mut b = psamp;
                let mut sa = st;
                let mut sb = st;
                unsafe {
                    assert_eq!(
                        c_generic(a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r_generic(b.as_mut_ptr(), idx, pfcn, &mut sb),
                        "pfcn={pfcn} idx={idx} firfx={firfx:?} psamp={psamp:?}"
                    );
                }
                cases += 1;
            }
        }
    }
    assert!(cases > 1000, "cases={cases}");
}

/// CONFIGS row 41: internal consistency of each library, then cross-check.
/// `GetPredictFunc(k)` must behave exactly like `Pfn<k>` — in BOTH libraries —
/// and the identity bitmap must be the same on both sides.
#[test]
fn cfg41_dispatch_identity() {
    let l = libs();
    let c_id: FnIntInt = sym(&l.c_aux, b"aux_getpredict_identity\0");
    let r_id: FnIntInt = sym(&l.r_aux, b"aux_getpredict_identity\0");
    let c_null: FnIntInt = sym(&l.c_aux, b"aux_getpredict_is_null\0");
    let r_null: FnIntInt = sym(&l.r_aux, b"aux_getpredict_is_null\0");

    for sel in -8..=20 {
        let (cv, rv) = unsafe { (c_id(sel), r_id(sel)) };
        assert_eq!(cv, rv, "identity bitmap for sel={sel}: C={cv:#x} Rust={rv:#x}");
        let expect = if (0..12).contains(&sel) {
            1 << sel
        } else {
            1 << 12 // the generic BTAC1C2_PredictSample
        };
        assert_eq!(cv, expect, "C dispatch table entry {sel} is {cv:#x}");
        let (cn, rn) = unsafe { (c_null(sel), r_null(sel)) };
        assert_eq!(cn, rn);
        assert_eq!(cn, 0, "GetPredictFunc must never return NULL");
    }
    for sel in [i32::MIN, i32::MIN + 1, -100, 100, 4096, i32::MAX - 1, i32::MAX] {
        let (cv, rv) = unsafe { (c_id(sel), r_id(sel)) };
        assert_eq!(cv, rv, "identity bitmap for sel={sel}");
        assert_eq!(cv, 1 << 12);
    }

    // behavioural cross-check: dispatch(k) == Pfn(k) for k in 0..12
    let (c_pfn, r_pfn) = aux_pfn_pair();
    let (c_disp, r_disp) = aux_dispatch_pair();
    let mut rng = Rng::new(SEED ^ 0xDDDD);
    for it in 0..32 {
        let psamp = make_psamp(it % N_PSAMP_SHAPES, CAP_BIG, &mut rng);
        let mut firfx = make_firfx(it % 2 + 5, &mut rng);
        assert_no_overflow(&psamp, &mut firfx);
        let mut st = IdxState::zeroed();
        st.firfx = firfx;
        for idx in [-3i32, 0, 1, 5, 7, 8, 13] {
            for k in 0..12 {
                let mut a = psamp;
                let mut b = psamp;
                let mut sa = st;
                let mut sb = st;
                unsafe {
                    let cd = c_disp(k, a.as_mut_ptr(), idx, k, &mut sa);
                    let cp = c_pfn(k, a.as_mut_ptr(), idx, k, &mut sa);
                    let rd = r_disp(k, b.as_mut_ptr(), idx, k, &mut sb);
                    let rp = r_pfn(k, b.as_mut_ptr(), idx, k, &mut sb);
                    assert_eq!(cd, cp, "C: dispatch({k}) != Pfn{k}");
                    assert_eq!(rd, rp, "Rust: dispatch({k}) != Pfn{k}");
                    assert_eq!(cd, rd, "dispatch({k}) C={cd} Rust={rd}");
                }
            }
        }
    }
}

/// CONFIGS row 42 / ERRORS E14: the exported ABI really is a single
/// `int(int)`, reached through `dlsym` on both shipped `.so`s.
#[test]
fn cfg42_exported_surface_is_int_int() {
    let l = libs();
    let c: FnCallPredict = sym(&l.c_main, b"call_predict\0");
    let r: FnCallPredict = sym(&l.r_main, b"call_predict\0");
    // Only `call_predict` is exported, so no exported entry point can be
    // handed a NULL pointer (ERRORS E14).
    let c_syms = defined_symbols(&l.c_main_path);
    assert_eq!(c_syms, vec!["call_predict".to_string()]);
    let r_syms = defined_symbols(&l.r_main_path);
    assert_eq!(r_syms, vec!["call_predict".to_string()]);
    unsafe {
        assert_eq!(c(3), r(3));
    }
}

// ===========================================================================
// PHASE C — error-path differential tests (ERRORS.md rows)
// ===========================================================================

/// ERRORS E1, E2, E3, E5, E6, E7, E8: every out-of-range `pfcn` for the
/// exported entry point.
#[test]
fn err_call_predict_out_of_range() {
    let (c, r) = call_predict_pair();
    let mut fixed: Vec<i32> = (-64..=64).collect();
    fixed.extend([
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 11,
        -1,
        12,
        13,
        14,
        15,
        16,
        17,
        127,
        128,
        255,
        256,
        257,
        32767,
        32768,
        65535,
        65536,
        65539,
        0x0100_0000,
        0x7FFF_FFF0,
        i32::MAX - 1,
        i32::MAX,
    ]);
    for pfcn in fixed {
        let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
        assert_eq!(cv, rv, "call_predict({pfcn}): C={cv} Rust={rv}");
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        assert_eq!(cv, expect, "C: call_predict({pfcn}) = {cv}");
    }
    // randomized sweep
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..4096 {
        let pfcn = rng.next_i32();
        let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
        assert_eq!(cv, rv, "call_predict({pfcn}): C={cv} Rust={rv}");
    }
    // and a sweep of "enum-like" small values, including negatives
    for pfcn in -32..=48 {
        let (cv, rv) = unsafe { (c(pfcn), r(pfcn)) };
        assert_eq!(cv, rv);
    }
}

/// ERRORS E1..E8 taken to the limit: `call_predict` branches on nothing but
/// `pfcn`, so *every* `int` input can be checked. Opt-in because it is a 2^32
/// sweep: `cargo test --release -- --ignored`.
#[test]
#[ignore = "exhaustive 2^32 sweep; run with `cargo test --release -- --ignored`"]
fn err_call_predict_exhaustive_every_i32() {
    let (c, r) = call_predict_pair();
    let mut checked: u64 = 0;
    let mut u: u32 = 0;
    loop {
        let pfcn = u as i32;
        let cv = unsafe { c(pfcn) };
        let rv = unsafe { r(pfcn) };
        if cv != rv {
            panic!("call_predict({pfcn}): C={cv} Rust={rv}");
        }
        let expect = if (0..=11).contains(&pfcn) { 1 } else { 0 };
        if cv != expect {
            panic!("C: call_predict({pfcn}) = {cv}, expected {expect}");
        }
        checked += 1;
        if u == u32::MAX {
            break;
        }
        u += 1;
    }
    assert_eq!(checked, 1u64 << 32);
}

/// Same idea for the internal generic predictor: every `pfcn` in the whole
/// `int` range against a fixed input buffer.
#[test]
#[ignore = "exhaustive 2^32 sweep; run with `cargo test --release -- --ignored`"]
fn err_generic_predict_exhaustive_every_pfcn() {
    let (c, r) = aux_predict_sample_pair();
    let mut rng = Rng::new(SEED ^ 0xFFFF);
    let psamp = make_psamp(7, CAP_FIRFX, &mut rng);
    let mut st = IdxState::zeroed();
    st.firfx = make_firfx(2, &mut rng);
    assert_no_overflow(&psamp, &mut st.firfx);
    let mut a = psamp;
    let mut b = psamp;
    let mut sa = st;
    let mut sb = st;
    let mut u: u32 = 0;
    loop {
        let pfcn = u as i32;
        let cv = unsafe { c(a.as_mut_ptr(), 3, pfcn, &mut sa) };
        let rv = unsafe { r(b.as_mut_ptr(), 3, pfcn, &mut sb) };
        if cv != rv {
            panic!("aux_predict_sample(pfcn={pfcn}): C={cv} Rust={rv}");
        }
        if !(0..=15).contains(&pfcn) && cv != 0 {
            panic!("C default arm for pfcn={pfcn} returned {cv}");
        }
        if u == u32::MAX {
            break;
        }
        u += 1;
    }
}

/// ERRORS E9: `BTAC1C2_GetPredictFunc` `default:` → generic predictor, never NULL.
#[test]
fn err_dispatch_default_arm() {
    let l = libs();
    let c_null: FnIntInt = sym(&l.c_aux, b"aux_getpredict_is_null\0");
    let r_null: FnIntInt = sym(&l.r_aux, b"aux_getpredict_is_null\0");
    let (c_disp, r_disp) = aux_dispatch_pair();
    let mut rng = Rng::new(SEED ^ 0xE9);
    let mut psamp = make_psamp(7, CAP_FIRFX, &mut rng);
    let mut firfx = make_firfx(2, &mut rng);
    assert_no_overflow(&psamp, &mut firfx);
    let mut st = IdxState::zeroed();
    st.firfx = firfx;
    let mut psamp2 = psamp;
    let mut st2 = st;

    for sel in [
        -1,
        -2,
        12,
        13,
        14,
        15,
        16,
        17,
        1000,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ] {
        unsafe {
            assert_eq!(c_null(sel), r_null(sel), "is_null({sel})");
            assert_eq!(c_null(sel), 0, "C never returns NULL");
            for idx in -8..=9 {
                for pfcn in -2..=17 {
                    assert_eq!(
                        c_disp(sel, psamp.as_mut_ptr(), idx, pfcn, &mut st),
                        r_disp(sel, psamp2.as_mut_ptr(), idx, pfcn, &mut st2),
                        "dispatch sel={sel} idx={idx} pfcn={pfcn}"
                    );
                }
            }
        }
    }
}

/// ERRORS E10: `BTAC1C2_PredictSample` `default:` arm → 0 for any `pfcn`
/// outside 0..=15, whatever the buffers contain.
#[test]
fn err_generic_predict_default_arm() {
    let (c, r) = aux_predict_sample_pair();
    let mut rng = Rng::new(SEED ^ 0xE10);
    let bad: Vec<i32> = vec![
        -1,
        -2,
        -16,
        16,
        17,
        18,
        31,
        32,
        255,
        256,
        4096,
        65536,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
    ];
    for it in 0..16 {
        let psamp = make_psamp(it % N_PSAMP_SHAPES, CAP_FIRFX, &mut rng);
        let mut firfx = make_firfx(it % N_FIRFX_SHAPES, &mut rng);
        assert_no_overflow(&psamp, &mut firfx);
        let mut st = IdxState::zeroed();
        st.firfx = firfx;
        for &pfcn in &bad {
            for idx in [i32::MIN + 8, -5, 0, 3, 8, 9, i32::MAX] {
                let mut a = psamp;
                let mut b = psamp;
                let mut sa = st;
                let mut sb = st;
                let (cv, rv) = unsafe {
                    (
                        c(a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r(b.as_mut_ptr(), idx, pfcn, &mut sb),
                    )
                };
                assert_eq!(cv, rv, "generic default arm pfcn={pfcn} idx={idx}");
                assert_eq!(cv, 0, "C default arm must yield 0 (pfcn={pfcn})");
            }
        }
    }
    // randomized out-of-range pfcn
    for _ in 0..2048 {
        let pfcn = rng.next_i32();
        if (0..=15).contains(&pfcn) {
            continue;
        }
        let psamp = make_psamp(7, CAP_FIRFX, &mut rng);
        let mut st = IdxState::zeroed();
        st.firfx = make_firfx(2, &mut rng);
        assert_no_overflow(&psamp, &mut st.firfx);
        let mut a = psamp;
        let mut b = psamp;
        let mut sa = st;
        let mut sb = st;
        let idx = rng.next_i32() / 2;
        let (cv, rv) = unsafe {
            (
                c(a.as_mut_ptr(), idx, pfcn, &mut sa),
                r(b.as_mut_ptr(), idx, pfcn, &mut sb),
            )
        };
        assert_eq!(cv, rv, "generic default arm pfcn={pfcn}");
        assert_eq!(cv, 0);
    }
}

/// ERRORS E11: the `firfx[pfcn-12]` boundary — 11 and 16 must NOT index the
/// array; 12..15 must index rows 0..3. A poisoned `firfx` makes an off-by-one
/// row index observable.
#[test]
fn err_firfx_index_boundary() {
    let (c, r) = aux_predict_sample_pair();
    let mut rng = Rng::new(SEED ^ 0xE11);
    for it in 0..32 {
        let mut st = IdxState::zeroed();
        // strongly row-dependent coefficients
        for (row, dst) in st.firfx.iter_mut().enumerate() {
            for (k, e) in dst.iter_mut().enumerate() {
                *e = ((row as i32 + 1) * 8192 + (k as i32) * 129 - 16384) as i16;
            }
        }
        // neighbouring struct fields poisoned to catch an offset error
        st.idx = 0xFFFF;
        st.lpred = i16::MIN;
        st.rpred = i16::MAX;
        st.tag = 0xFF;
        st.bcfcn = 0xFF;
        st.bsfcn = 0xFF;
        st.usefx = 0xFF;
        let psamp = make_psamp(it % N_PSAMP_SHAPES, CAP_FIRFX, &mut rng);
        assert_no_overflow(&psamp, &mut st.firfx);
        for pfcn in [10, 11, 12, 13, 14, 15, 16, 17] {
            for idx in -9..=17 {
                let mut a = psamp;
                let mut b = psamp;
                let mut sa = st;
                let mut sb = st;
                let (cv, rv) = unsafe {
                    (
                        c(a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r(b.as_mut_ptr(), idx, pfcn, &mut sb),
                    )
                };
                assert_eq!(
                    cv, rv,
                    "firfx boundary pfcn={pfcn} idx={idx} psamp={psamp:?} firfx={:?}",
                    st.firfx
                );
                if pfcn == 16 || pfcn == 17 {
                    assert_eq!(cv, 0, "pfcn={pfcn} must hit the default arm");
                }
            }
        }
    }
}

/// ERRORS E12: `(idx - n) & 7` is the only bound check; it must keep every
/// access inside `psamp[0..8]` for arbitrary `idx`, including negatives and
/// `INT_MAX`. Guard pages around the 8-element buffer would fault on an OOB
/// access; here we detect it by making every element unique and checking the
/// result against the C reference.
#[test]
fn err_idx_mask_is_the_bound_check() {
    let (c_generic, r_generic) = aux_predict_sample_pair();
    let (c_pfn, r_pfn) = aux_pfn_pair();
    let mut rng = Rng::new(SEED ^ 0xE12);
    // unique, widely spaced values so any index confusion changes the result
    let psamp: [i32; 8] = [1, 3, 9, 27, 81, 243, 729, 2187];
    let mut st = IdxState::zeroed();
    st.firfx = make_firfx(5, &mut rng);
    assert_no_overflow(&psamp, &mut st.firfx);
    let mut idxs: Vec<i32> = (-40..=40).collect();
    idxs.extend([i32::MIN + 8, i32::MIN + 9, i32::MAX, i32::MAX - 1]);
    for _ in 0..256 {
        idxs.push(rng.next_i32() / 2);
    }
    for &idx in &idxs {
        for pfcn in -1..=16 {
            let mut a = psamp;
            let mut b = psamp;
            let mut sa = st;
            let mut sb = st;
            unsafe {
                assert_eq!(
                    c_generic(a.as_mut_ptr(), idx, pfcn, &mut sa),
                    r_generic(b.as_mut_ptr(), idx, pfcn, &mut sb),
                    "generic idx={idx} pfcn={pfcn}"
                );
                if (0..12).contains(&pfcn) {
                    assert_eq!(
                        c_pfn(pfcn, a.as_mut_ptr(), idx, pfcn, &mut sa),
                        r_pfn(pfcn, b.as_mut_ptr(), idx, pfcn, &mut sb),
                        "pfn{pfcn} idx={idx}"
                    );
                }
            }
        }
    }
}

// ===========================================================================
// PHASE D — symbol parity
// ===========================================================================

fn defined_symbols(so: &Path) -> Vec<String> {
    let out = run(Command::new("nm").arg("-D").arg("--defined-only").arg(so));
    let mut v: Vec<String> = out
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn phase_d_symbol_parity() {
    let l = libs();
    let c = defined_symbols(&l.c_main_path);
    let r = defined_symbols(&l.r_main_path);
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(
        extra.is_empty(),
        "symbols exported by the Rust .so that the C .so does not export: {extra:?}"
    );
    assert!(c.contains(&"call_predict".to_string()), "syms: {c:?}");
    // the header declares get_predict_func, but no TU defines it: neither
    // library may export it.
    assert!(!c.iter().any(|s| s == "get_predict_func"));
    assert!(!r.iter().any(|s| s == "get_predict_func"));
}
