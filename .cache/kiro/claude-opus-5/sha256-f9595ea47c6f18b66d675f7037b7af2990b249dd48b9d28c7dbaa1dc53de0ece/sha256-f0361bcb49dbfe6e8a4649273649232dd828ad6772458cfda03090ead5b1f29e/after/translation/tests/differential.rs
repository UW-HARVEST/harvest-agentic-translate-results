//! Differential test harness: loads BOTH the C `.so` and the Rust `.so` with
//! `libloading` and compares `encode_quant` outputs through the FFI boundary.
//!
//! The Rust implementation is NEVER called directly — only via the exported
//! `#[no_mangle] extern "C"` symbol in the built cdylib, exactly as an external
//! consumer would.
//!
//! Phases (see SYMBOLS.md / CONFIGS.md / ERRORS.md in the crate root):
//!   * Phase B — `phase_b_config_surface`, `phase_b_extreme_shapes`
//!   * Phase C — `phase_c_*`
//!   * Phase D — `phase_d_symbol_parity`

use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

pub type EncodeQuantFn = unsafe extern "C" fn(
    uni: i32,
    step: i32,
    pred: i32,
    tgt: i32,
    tgt2: i32,
    lsbit: i32,
) -> i32;

// ---------------------------------------------------------------------------
// Locating and building the two shared objects
// ---------------------------------------------------------------------------

/// Crate root (`translation/`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Working-directory root that contains both `c_src/` and `translation/`.
fn project_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate root must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<PathBuf> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            // Deterministic pick: lexicographically smallest name.
            match &best {
                None => best = Some(p),
                Some(b) if p.file_name() < b.file_name() => best = Some(p),
                _ => {}
            }
        }
    }
    best
}

/// Build (if needed) and return the path to the C shared object.
fn c_so_path() -> PathBuf {
    let build_dir = project_root().join("c_src/build");
    if let Some(p) = find_so(&build_dir) {
        return p;
    }
    std::fs::create_dir_all(&build_dir).expect("create c_src/build");
    let ok = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure")
        .success();
    assert!(ok, "cmake configure failed");
    let ok = Command::new("cmake")
        .current_dir(&build_dir)
        .args(["--build", "."])
        .status()
        .expect("run cmake build")
        .success();
    assert!(ok, "cmake build failed");
    find_so(&build_dir).expect("C .so not found after building c_src")
}

/// Build (if needed) and return the path to the Rust cdylib.
///
/// `cargo test` does not guarantee the cdylib artifact is fresh, so we build it
/// explicitly. The exact same feature set the test binary was compiled with is
/// forwarded so that Phase D feature-combination runs load a matching `.so`.
fn rust_so_path() -> PathBuf {
    let root = crate_root();
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir(&root).arg("build").arg("--release");
    // This crate declares no [features]; forwarding is a no-op today but keeps
    // the harness correct if features are ever added.
    if let Ok(feats) = std::env::var("DIFFTEST_FEATURES") {
        if feats == "--no-default-features" {
            cmd.arg("--no-default-features");
        } else if !feats.is_empty() {
            cmd.arg("--no-default-features")
                .arg("--features")
                .arg(feats);
        }
    }
    let ok = cmd.status().expect("run cargo build --release").success();
    assert!(ok, "cargo build --release failed");

    let p = root.join("target/release/libencode_quant_lib.so");
    assert!(p.exists(), "rust cdylib missing at {}", p.display());
    p
}

struct Pair {
    _c_lib: Library,
    _r_lib: Library,
    c: EncodeQuantFn,
    r: EncodeQuantFn,
}

impl Pair {
    fn load() -> Pair {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} : {e}", c_path.display()));
            let r_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {} : {e}", r_path.display()));
            let c_sym: Symbol<EncodeQuantFn> = c_lib
                .get(b"encode_quant\0")
                .expect("C .so must export encode_quant");
            let r_sym: Symbol<EncodeQuantFn> = r_lib
                .get(b"encode_quant\0")
                .expect("Rust .so must export encode_quant");
            let c = *c_sym;
            let r = *r_sym;
            Pair {
                _c_lib: c_lib,
                _r_lib: r_lib,
                c,
                r,
            }
        }
    }
}

/// One differential call. Returns `Err(message)` on divergence.
fn diff_call(p: &Pair, a: [i32; 6]) -> Result<i32, String> {
    let [uni, step, pred, tgt, tgt2, lsbit] = a;
    let cv = unsafe { (p.c)(uni, step, pred, tgt, tgt2, lsbit) };
    let rv = unsafe { (p.r)(uni, step, pred, tgt, tgt2, lsbit) };
    if cv == rv {
        Ok(cv)
    } else {
        Err(format!(
            "DIVERGENCE encode_quant(uni={uni}, step={step}, pred={pred}, \
             tgt={tgt}, tgt2={tgt2}, lsbit={lsbit}) : C={cv} (0x{cv:08x}) \
             Rust={rv} (0x{rv:08x})"
        ))
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed per row for reproducibility
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Inclusive range.
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// A "spicy" i32: biased toward boundaries and small magnitudes, which is where
/// the wrapping / truncating-division behaviour lives, while still covering the
/// full 32-bit space.
fn spicy_i32(rng: &mut Rng) -> i32 {
    match rng.next_u64() % 8 {
        0 => i32::MIN,
        1 => i32::MAX,
        2 => rng.range_i32(-8, 8),
        3 => rng.range_i32(-1024, 1024),
        4 => i32::MIN + rng.range_i32(0, 64),
        5 => i32::MAX - rng.range_i32(0, 64),
        6 => rng.next_i32() >> rng.range_i32(0, 20),
        _ => rng.next_i32(),
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row model — must match the table's ordering exactly
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq)]
enum LsbitMode {
    Off,
    Four,
    Odd,
    Even,
}
const LSBIT_MODES: [LsbitMode; 4] = [
    LsbitMode::Off,
    LsbitMode::Four,
    LsbitMode::Odd,
    LsbitMode::Even,
];

#[derive(Copy, Clone, Debug)]
enum UniClass {
    Low0,
    Low7,
    MidB30,
    Low8,
    Low15,
    MidB31,
}
const UNI_CLASSES: [UniClass; 6] = [
    UniClass::Low0,
    UniClass::Low7,
    UniClass::MidB30,
    UniClass::Low8,
    UniClass::Low15,
    UniClass::MidB31,
];

#[derive(Copy, Clone, Debug)]
enum StepClass {
    Zero,
    PosSmall,
    NegSmall,
    PosHuge,
    NegHuge,
    Extreme,
}
const STEP_CLASSES: [StepClass; 6] = [
    StepClass::Zero,
    StepClass::PosSmall,
    StepClass::NegSmall,
    StepClass::PosHuge,
    StepClass::NegHuge,
    StepClass::Extreme,
];

fn gen_lsbit(rng: &mut Rng, m: LsbitMode) -> i32 {
    match m {
        LsbitMode::Off => 0,
        LsbitMode::Four => 4,
        // odd: includes negatives and INT_MAX
        LsbitMode::Odd => loop {
            let r = rng.next_i32() | 1;
            let v = rng.pick(&[1i32, 3, 5, 7, 9, -1, -3, -7, i32::MAX, i32::MIN + 1, r]);
            if v & 1 != 0 {
                return v;
            }
        },
        // even, non-zero, != 4 : includes negatives and INT_MIN
        LsbitMode::Even => loop {
            let r = rng.next_i32() & !1;
            let v = rng.pick(&[
                2i32,
                6,
                8,
                10,
                100,
                -2,
                -4,
                -6,
                i32::MIN,
                i32::MAX - 1,
                r,
            ]);
            if v != 0 && v != 4 && v & 1 == 0 {
                return v;
            }
        },
    }
}

/// Generate a `uni` whose low nibble lands in the requested class, with the
/// high bits randomized (including negative values and near-boundary values).
fn gen_uni(rng: &mut Rng, c: UniClass) -> i32 {
    let low: i32 = match c {
        UniClass::Low0 => 0x0,
        UniClass::Low7 => 0x7,
        UniClass::MidB30 => rng.range_i32(1, 6),
        UniClass::Low8 => 0x8,
        UniClass::Low15 => 0xF,
        UniClass::MidB31 => 0x8 | rng.range_i32(1, 6),
    };
    // High bits: 0, small +, small -, all-ones (negative), and near-boundary.
    let high: i32 = match rng.next_u64() % 6 {
        0 => 0,
        1 => rng.range_i32(0, 1 << 20) << 4,
        2 => -(rng.range_i32(0, 1 << 20) << 4),
        3 => !0i32 << 4,          // uni becomes negative with the chosen low nibble
        4 => i32::MIN,            // most-negative high bits
        _ => (rng.next_i32() >> 4) << 4,
    };
    high.wrapping_add(low)
}

fn gen_step(rng: &mut Rng, c: StepClass) -> i32 {
    match c {
        StepClass::Zero => 0,
        StepClass::PosSmall => rng.range_i32(1, 4096),
        StepClass::NegSmall => rng.range_i32(-4096, -1),
        StepClass::PosHuge => rng.range_i32(1 << 28, i32::MAX),
        StepClass::NegHuge => rng.range_i32(i32::MIN, -(1 << 28)),
        StepClass::Extreme => rng.pick(&[i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1]),
    }
}

const PER_ROW: usize = 512;

// ---------------------------------------------------------------------------
// Phase B
// ---------------------------------------------------------------------------

/// CONFIGS.md rows 1..=144: the pruned cross product
/// `lsbit mode (4) x uni class (6) x step class (6)`.
#[test]
fn phase_b_config_surface() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();
    let mut row = 0usize;
    let mut calls = 0usize;
    // Coverage counters for the selection branches (proves the rows actually
    // reach the `d1 < d0` / `d2 < d0` code, rather than only one outcome).
    let mut sel_none = 0usize;
    let mut sel_changed = 0usize;

    for &lm in LSBIT_MODES.iter() {
        for &uc in UNI_CLASSES.iter() {
            for &sc in STEP_CLASSES.iter() {
                row += 1;
                let mut rng = Rng::new(0xC0FF_EE00_0000_0000 ^ row as u64);
                let mut row_fail = 0usize;
                for _ in 0..PER_ROW {
                    let uni = gen_uni(&mut rng, uc);
                    let step = gen_step(&mut rng, sc);
                    let pred = spicy_i32(&mut rng);
                    let tgt = spicy_i32(&mut rng);
                    let tgt2 = spicy_i32(&mut rng);
                    let lsbit = gen_lsbit(&mut rng, lm);
                    calls += 1;
                    match diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                        Ok(v) => {
                            // `uni` after the lsbit fixups is not observable, so
                            // approximate: if the result differs from the raw
                            // input's low bits pattern we saw a candidate swap.
                            if v == uni {
                                sel_none += 1;
                            } else {
                                sel_changed += 1;
                            }
                        }
                        Err(m) => {
                            row_fail += 1;
                            if row_fail <= 3 {
                                failures.push(format!("[CONFIGS row {row}] {m}"));
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(row, 144, "must cover CONFIGS.md rows 1..=144");
    eprintln!(
        "phase_b_config_surface: {row} rows x {PER_ROW} inputs = {calls} differential calls; \
         result==uni: {sel_none}, result!=uni: {sel_changed}"
    );
    assert!(
        sel_changed > 0 && sel_none > 0,
        "both selection outcomes must be exercised (got none={sel_none} changed={sel_changed})"
    );
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// CONFIGS.md rows 145..=156: extreme value shapes and forced selection
/// outcomes.
#[test]
fn phase_b_extreme_shapes() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();

    // Rows 145..=148: extreme `uni`.
    let extreme_unis = [
        i32::MAX,
        i32::MIN,
        -1,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX - 7,
        i32::MIN + 7,
        i32::MAX - 8,
        i32::MIN + 8,
    ];
    // Rows 149..=154: extreme pred/tgt/tgt2.
    let extremes = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let lsbits = [0i32, 1, 2, 3, 4, 5, 6, -1, -2, -4, i32::MIN, i32::MAX];

    let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
    let mut calls = 0usize;

    // Rows 145-148 and 149-154, driven together with the other args randomized.
    for &uni in extreme_unis.iter() {
        for &lsbit in lsbits.iter() {
            for &ex in extremes.iter() {
                // Put the extreme in each of pred / tgt / tgt2 in turn.
                for slot in 0..3 {
                    for _ in 0..8 {
                        let mut pred = spicy_i32(&mut rng);
                        let mut tgt = spicy_i32(&mut rng);
                        let mut tgt2 = spicy_i32(&mut rng);
                        match slot {
                            0 => pred = ex,
                            1 => tgt = ex,
                            _ => tgt2 = ex,
                        }
                        let step = spicy_i32(&mut rng);
                        calls += 1;
                        if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                            if failures.len() < 20 {
                                failures.push(format!("[CONFIGS rows 145-154] {m}"));
                            }
                        }
                    }
                }
            }
        }
    }

    // Row 155: all of pred/tgt/tgt2 extreme simultaneously (compounded wrap).
    for &uni in extreme_unis.iter() {
        for &step in extremes.iter() {
            for &pred in extremes.iter() {
                for &tgt in extremes.iter() {
                    for &tgt2 in extremes.iter() {
                        for &lsbit in [0i32, 4, 3, 2].iter() {
                            calls += 1;
                            if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                                if failures.len() < 20 {
                                    failures.push(format!("[CONFIGS row 155] {m}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Row 156: forced selection outcomes. `tgt` is swept around `pred` so the
    // nearest of the three candidate reconstructions is sometimes uni,
    // sometimes uni1, sometimes uni2, and sometimes both branches fire.
    let mut outcomes = [0usize; 3]; // kept / +1 / -1 shape observed
    for uni in 0i32..16 {
        for &lsbit in [0i32, 4, 1, 2].iter() {
            for step in [8i32, 16, 64, 100, 255, 1024].iter().copied() {
                let pred = 0i32;
                let lo = -4 * step - 4;
                let hi = 4 * step + 4;
                let mut t = lo;
                while t <= hi {
                    for tgt2 in [t, 0, -t, t / 2, i32::MAX, i32::MIN].iter().copied() {
                        calls += 1;
                        match diff_call(&p, [uni, step, pred, t, tgt2, lsbit]) {
                            Ok(v) => {
                                let d = v.wrapping_sub(uni);
                                let idx = match d {
                                    0 => 0,
                                    1 => 1,
                                    _ => 2,
                                };
                                outcomes[idx] += 1;
                            }
                            Err(m) => {
                                if failures.len() < 20 {
                                    failures.push(format!("[CONFIGS row 156] {m}"));
                                }
                            }
                        }
                    }
                    t += step.max(1) / 3 + 1;
                }
            }
        }
    }
    eprintln!(
        "phase_b_extreme_shapes: {calls} differential calls; selection outcomes \
         kept={} plus={} other={}",
        outcomes[0], outcomes[1], outcomes[2]
    );
    assert!(
        outcomes[0] > 0 && outcomes[1] > 0 && outcomes[2] > 0,
        "all three selection outcomes must be observed: {outcomes:?}"
    );
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Exhaustive sweep of the whole branch-relevant neighbourhood: every
/// `uni` in -64..=64 crossed with every `lsbit` in -16..=16, with randomized
/// step/pred/tgt/tgt2. This covers every low-nibble x lsbit-branch pairing
/// exactly, not just sampled ones.
#[test]
fn phase_b_exhaustive_low_bits() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();
    let mut rng = Rng::new(0xABCD_EF01_2345_6789);
    let mut calls = 0usize;
    for uni in -64i32..=64 {
        for lsbit in -16i32..=16 {
            for _ in 0..24 {
                let step = spicy_i32(&mut rng);
                let pred = spicy_i32(&mut rng);
                let tgt = spicy_i32(&mut rng);
                let tgt2 = spicy_i32(&mut rng);
                calls += 1;
                if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                    if failures.len() < 20 {
                        failures.push(m);
                    }
                }
            }
        }
    }
    eprintln!("phase_b_exhaustive_low_bits: {calls} differential calls");
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Large fully-random fuzz over the whole `i32^6` domain, fixed seed.
#[test]
fn phase_b_random_fuzz() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();
    let mut rng = Rng::new(0x0BAD_C0DE_DEAD_BEEF);
    const N: usize = 300_000;
    for _ in 0..N {
        let a = [
            spicy_i32(&mut rng),
            spicy_i32(&mut rng),
            spicy_i32(&mut rng),
            spicy_i32(&mut rng),
            spicy_i32(&mut rng),
            spicy_i32(&mut rng),
        ];
        if let Err(m) = diff_call(&p, a) {
            if failures.len() < 20 {
                failures.push(m);
            }
        }
    }
    eprintln!("phase_b_random_fuzz: {N} differential calls");
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Phase C — one test per ERRORS.md row
// ---------------------------------------------------------------------------

/// Shared driver: run every `(uni, step, pred, tgt, tgt2)` combination from
/// small representative sets against the given `lsbit` values.
fn sweep_with_lsbits(p: &Pair, label: &str, lsbits: &[i32], failures: &mut Vec<String>) {
    let unis = [
        0i32,
        1,
        6,
        7,
        8,
        9,
        14,
        15,
        16,
        -1,
        -7,
        -8,
        -9,
        -16,
        i32::MIN,
        i32::MAX,
    ];
    let steps = [0i32, 1, 7, 8, 9, -1, -8, -9, 1 << 28, i32::MIN, i32::MAX];
    let vals = [0i32, 1, -1, 12345, -12345, i32::MIN, i32::MAX];
    for &lsbit in lsbits {
        for &uni in unis.iter() {
            for &step in steps.iter() {
                for &pred in vals.iter() {
                    for &tgt in vals.iter() {
                        for &tgt2 in [0i32, 1, -1, i32::MIN, i32::MAX].iter() {
                            if let Err(m) = diff_call(p, [uni, step, pred, tgt, tgt2, lsbit]) {
                                if failures.len() < 10 {
                                    failures.push(format!("[{label}] {m}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

macro_rules! err_row {
    ($name:ident, $label:expr, $lsbits:expr) => {
        #[test]
        fn $name() {
            let p = Pair::load();
            let mut failures = Vec::new();
            sweep_with_lsbits(&p, $label, &$lsbits, &mut failures);
            assert!(
                failures.is_empty(),
                "{} divergence(s):\n{}",
                failures.len(),
                failures.join("\n")
            );
        }
    };
}

// Row 1
err_row!(phase_c_row01_lsbit_zero, "ERRORS row 1: lsbit = 0", [0i32]);
// Row 2
err_row!(phase_c_row02_lsbit_even_two, "ERRORS row 2: lsbit = 2", [2i32]);
// Row 3
err_row!(phase_c_row03_lsbit_odd_three, "ERRORS row 3: lsbit = 3", [3i32]);
// Row 4
err_row!(
    phase_c_row04_lsbit_arbitrary_out_of_range,
    "ERRORS row 4: lsbit in {5,6,7,8,100}",
    [5i32, 6, 7, 8, 100]
);
// Row 5
err_row!(
    phase_c_row05_lsbit_negative_odd,
    "ERRORS row 5: lsbit = -1 (negative odd -> set-bit0 branch)",
    [-1i32, -3, -5, -12345]
);
// Row 6
err_row!(
    phase_c_row06_lsbit_negative_even,
    "ERRORS row 6: lsbit in {-2,-4,-6} (negative even -> clear-bit0)",
    [-2i32, -4, -6, -100]
);
// Row 7
err_row!(
    phase_c_row07_lsbit_int_max,
    "ERRORS row 7: lsbit = INT_MAX (odd)",
    [i32::MAX]
);
// Row 8
err_row!(
    phase_c_row08_lsbit_int_min,
    "ERRORS row 8: lsbit = INT_MIN (even, != 0, != 4)",
    [i32::MIN]
);
// Row 9
err_row!(
    phase_c_row09_lsbit_four_and_neighbours,
    "ERRORS row 9: lsbit in {3,4,5} (the only special variant and one step past)",
    [3i32, 4, 5]
);

/// Row 10 — exhaustive `lsbit` in -16..=16.
#[test]
fn phase_c_row10_lsbit_exhaustive_neighbourhood() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let all: Vec<i32> = (-16..=16).collect();
    sweep_with_lsbits(&p, "ERRORS row 10: lsbit in -16..=16", &all, &mut failures);
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Rows 11 & 12 — `uni = INT_MAX` (uni+1 overflows) and `uni = INT_MIN`
/// (uni-1 overflows).
#[test]
fn phase_c_row11_12_uni_overflow() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for &uni in [i32::MAX, i32::MIN].iter() {
        for lsbit in -8i32..=8 {
            for &step in [0i32, 1, 8, -8, 1 << 28, i32::MIN, i32::MAX].iter() {
                for _ in 0..64 {
                    let pred = spicy_i32(&mut rng);
                    let tgt = spicy_i32(&mut rng);
                    let tgt2 = spicy_i32(&mut rng);
                    if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                        if failures.len() < 10 {
                            failures.push(format!("[ERRORS rows 11/12] {m}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Rows 13-16 — `step` at INT_MAX / INT_MIN / 0 / negative.
#[test]
fn phase_c_row13_16_step_boundaries() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let mut rng = Rng::new(0x7777_8888_9999_AAAA);
    let steps = [
        i32::MAX,
        i32::MIN,
        0,
        -1,
        -7,
        -8,
        -9,
        -4096,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for &step in steps.iter() {
        for uni in 0i32..16 {
            for &lsbit in [0i32, 1, 2, 4, -1, -2].iter() {
                for _ in 0..64 {
                    let pred = spicy_i32(&mut rng);
                    let tgt = spicy_i32(&mut rng);
                    let tgt2 = spicy_i32(&mut rng);
                    if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                        if failures.len() < 10 {
                            failures.push(format!("[ERRORS rows 13-16] {m}"));
                        }
                    }
                }
            }
        }
    }
    // Explicitly pin the truncate-toward-zero division on negative products:
    // uni&7 = 1 -> (2*1+1)*step/8 = 3*step/8. For step = -1: 3*-1/8 == 0 in C
    // (truncation), not -1 (floor). Any Rust `div_euclid`/floor mistake shows up
    // as a divergence here.
    for step in -64i32..=64 {
        for uni in 0i32..16 {
            if let Err(m) = diff_call(&p, [uni, step, 0, 0, 0, 0]) {
                failures.push(format!("[ERRORS row 16: /8 truncation] {m}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Rows 17-22 — `pred` / `tgt` / `tgt2` overflow paths and the `d0 += d3>>5`
/// wrap.
#[test]
fn phase_c_row17_22_pred_tgt_overflow() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let boundary = [i32::MIN, i32::MIN + 1, i32::MIN + 7, -1, 0, 1, i32::MAX - 7, i32::MAX - 1, i32::MAX];
    let mut rng = Rng::new(0xBBBB_CCCC_DDDD_EEEE);
    for &pred in boundary.iter() {
        for &tgt in boundary.iter() {
            for &tgt2 in boundary.iter() {
                for &lsbit in [0i32, 4, 1, 2].iter() {
                    for _ in 0..4 {
                        let uni = spicy_i32(&mut rng);
                        let step = spicy_i32(&mut rng);
                        if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                            if failures.len() < 10 {
                                failures.push(format!("[ERRORS rows 17-22] {m}"));
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Row 23 — full 4^6 boundary cross-product over {INT_MIN, -1, 0, INT_MAX}
/// (plus 1 and INT_MIN+1 / INT_MAX-1 for a 7^6 sweep of the same idea).
#[test]
fn phase_c_row23_boundary_cross_product() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let b = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut calls = 0usize;
    for &a0 in b.iter() {
        for &a1 in b.iter() {
            for &a2 in b.iter() {
                for &a3 in b.iter() {
                    for &a4 in b.iter() {
                        for &a5 in b.iter() {
                            calls += 1;
                            if let Err(m) = diff_call(&p, [a0, a1, a2, a3, a4, a5]) {
                                if failures.len() < 20 {
                                    failures.push(format!("[ERRORS row 23] {m}"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("phase_c_row23_boundary_cross_product: {calls} differential calls");
    assert_eq!(calls, 7usize.pow(6));
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Row 24 — `uni = -1` (all bits set) and other negative `uni` with the
/// `lsbit == 4` sign-extending-shift path.
#[test]
fn phase_c_row24_uni_all_bits_set() {
    let p = Pair::load();
    let mut failures = Vec::new();
    let mut rng = Rng::new(0x2468_ACE0_1357_9BDF);
    for uni in -32i32..=0 {
        for &lsbit in [4i32, 0, 1, 2, -1, -4].iter() {
            for &step in [0i32, 1, 8, 100, -100, i32::MAX, i32::MIN].iter() {
                for _ in 0..32 {
                    let pred = spicy_i32(&mut rng);
                    let tgt = spicy_i32(&mut rng);
                    let tgt2 = spicy_i32(&mut rng);
                    if let Err(m) = diff_call(&p, [uni, step, pred, tgt, tgt2, lsbit]) {
                        if failures.len() < 10 {
                            failures.push(format!("[ERRORS row 24] {m}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity, enforced from inside the test suite
// ---------------------------------------------------------------------------

fn nm_defined(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm -D");
    assert!(out.status.success(), "nm failed on {}", so.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        // Rust-internal mangled symbols are not part of the C surface.
        .filter(|s| !s.starts_with("_ZN"))
        .collect()
}

#[test]
fn phase_d_symbol_parity() {
    let c = c_so_path();
    let r = rust_so_path();
    let mut cs = nm_defined(&c);
    let mut rs = nm_defined(&r);
    cs.sort();
    cs.dedup();
    rs.sort();
    rs.dedup();
    eprintln!("C  exports ({}): {cs:?}", cs.len());
    eprintln!("Rust exports ({}): {rs:?}", rs.len());
    let missing: Vec<&String> = cs.iter().filter(|s| !rs.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C-exported symbol(s): {missing:?}",
        missing.len()
    );
    assert!(
        cs.contains(&"encode_quant".to_string()),
        "sanity: C .so must export encode_quant"
    );

    // Undefined symbols in the Rust .so must all be libc / unwind imports.
    let out = Command::new("nm").args(["-D", "-u"]).arg(&r).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .filter(|s| !s.is_empty())
        .filter(|s| {
            // weak/glibc/unwind imports are fine
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("__gmon_start__")
                || s.starts_with("_Unwind_")
                || s.starts_with("__cxa_")
                || s.starts_with("__tls_get_addr")
                || s.starts_with("__errno_location"))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has non-libc undefined symbols: {bad:?}"
    );
}

/// Dense exhaustive grid: every `uni` low nibble x every `lsbit` branch x a
/// contiguous `step` range x a contiguous `tgt` range. This nails the
/// truncating-division and arithmetic-shift behaviour at every small magnitude
/// (where floor-vs-truncate and the `d3 >> 5` term actually differ) instead of
/// relying on sampling.
#[test]
fn phase_b_dense_exhaustive_grid() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();
    let mut calls = 0usize;
    for uni in 0i32..16 {
        for lsbit in -6i32..=6 {
            for step in -96i32..=96 {
                for tgt in -96i32..=96 {
                    for tgt2 in [-97i32, 0, 61, i32::MIN, i32::MAX].iter().copied() {
                        calls += 1;
                        if let Err(m) = diff_call(&p, [uni, step, 0, tgt, tgt2, lsbit]) {
                            if failures.len() < 20 {
                                failures.push(format!("[dense grid] {m}"));
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("phase_b_dense_exhaustive_grid: {calls} differential calls");
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Same dense grid with a non-zero `pred` and negative `uni` values, so the
/// sign-extending shifts in the `lsbit == 4` path and the `pred + diff` wrap are
/// swept exhaustively too.
#[test]
fn phase_b_dense_exhaustive_grid_negative_uni() {
    let p = Pair::load();
    let mut failures: Vec<String> = Vec::new();
    let mut calls = 0usize;
    for uni in -16i32..0 {
        for lsbit in [0i32, 1, 2, 3, 4, 5, 6, -1, -2, -4].iter().copied() {
            for step in -64i32..=64 {
                for pred in [-1000i32, -1, 0, 1, 1000, i32::MIN, i32::MAX]
                    .iter()
                    .copied()
                {
                    for tgt in -32i32..=32 {
                        calls += 1;
                        if let Err(m) = diff_call(&p, [uni, step, pred, tgt, -tgt, lsbit]) {
                            if failures.len() < 20 {
                                failures.push(format!("[dense grid neg uni] {m}"));
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("phase_b_dense_exhaustive_grid_negative_uni: {calls} differential calls");
    assert!(
        failures.is_empty(),
        "{} divergence(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
