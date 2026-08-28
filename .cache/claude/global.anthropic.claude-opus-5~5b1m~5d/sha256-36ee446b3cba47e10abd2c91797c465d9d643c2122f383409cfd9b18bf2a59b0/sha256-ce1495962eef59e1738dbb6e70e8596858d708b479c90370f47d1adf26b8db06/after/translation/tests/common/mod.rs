//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
//! only through their exported `encode_quant` symbol. The Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper and the C
//! calling convention are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// The exact ABI of `int encode_quant(int, int, int, int, int, int)`.
pub type EncodeQuantFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: EncodeQuantFn,
    pub rust: EncodeQuantFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// The raw fn pointers borrow from the leaked `Library` handles, which live for
// the whole process, so sharing across threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the single `.so` produced by the C CMake build.
///
/// The library name is derived from the parent directory name by
/// `c_src/CMakeLists.txt`, so it must be discovered rather than hard-coded.
fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} not found - build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut found: Vec<PathBuf> = Vec::new();
    let mut stack = vec![build_dir.clone()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Don't descend into CMake's own scratch directories.
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "CMakeFiles" {
                    stack.push(path);
                }
            } else if path.extension().and_then(|e| e.to_str()) == Some("so") {
                found.push(path);
            }
        }
    }

    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C .so under {}, found {:?}",
        build_dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust cdylib next to the currently running test executable
/// (`target/<profile>/deps/<test>` -> `target/<profile>/`), so this works for
/// both `cargo test` and `cargo test --release`.
fn find_rust_so() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>")
        .to_path_buf();

    let candidates = [
        profile_dir.join("libencode_quant_lib.so"),
        workspace_root()
            .join("translation/target/release/libencode_quant_lib.so"),
        workspace_root().join("translation/target/debug/libencode_quant_lib.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libencode_quant_lib.so not found; looked in {:?}",
        candidates
    );
}

/// Load both shared objects once per test binary.
pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));

            let c_sym: Symbol<EncodeQuantFn> = c_lib
                .get(b"encode_quant\0")
                .expect("C .so does not export encode_quant");
            let rust_sym: Symbol<EncodeQuantFn> = rust_lib
                .get(b"encode_quant\0")
                .expect("Rust .so does not export encode_quant");

            let c = *c_sym;
            let rust = *rust_sym;

            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    })
}

/// One `encode_quant` argument tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    pub uni: i32,
    pub step: i32,
    pub pred: i32,
    pub tgt: i32,
    pub tgt2: i32,
    pub lsbit: i32,
}

impl Args {
    pub fn new(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32, lsbit: i32) -> Self {
        Args { uni, step, pred, tgt, tgt2, lsbit }
    }
}

/// Call the C export.
pub fn call_c(a: Args) -> i32 {
    let f = libs().c;
    unsafe { f(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) }
}

/// Call the Rust export (through the `.so`, never directly).
pub fn call_rust(a: Args) -> i32 {
    let f = libs().rust;
    unsafe { f(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) }
}

/// Differential check for a single tuple. Returns the agreed-upon C result.
#[track_caller]
pub fn check(row: &str, a: Args) -> i32 {
    let c = call_c(a);
    let r = call_rust(a);
    assert_eq!(
        c, r,
        "\n[{row}] DIVERGENCE\n  encode_quant(uni={}, step={}, pred={}, tgt={}, tgt2={}, lsbit={})\
         \n    C    = {c} (0x{c:08x})\n    Rust = {r} (0x{r:08x})\n",
        a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit
    );
    c
}

/// Differential check over an iterator of tuples; asserts at least one ran.
#[track_caller]
pub fn check_all(row: &str, it: impl IntoIterator<Item = Args>) -> usize {
    let mut n = 0usize;
    for a in it {
        check(row, a);
        n += 1;
    }
    assert!(n > 0, "[{row}] ran zero cases");
    n
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) - fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// Per-row seed so each row is reproducible independently of test ordering.
    pub fn for_row(row: &str) -> Self {
        // FNV-1a of the row label mixed into the global seed.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in row.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Rng(SEED ^ h)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform random `i32` over the whole range.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// Axis generators - these mirror CONFIGS.md exactly.
// ---------------------------------------------------------------------------

/// Signed extremes / one-step-past-range values used by V7 and by ERRORS.md.
pub const EXTREMES: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

/// Axis L class labels, in CONFIGS.md order (L0..L12).
pub const L_CLASSES: [&str; 13] = [
    "L0", "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "L9", "L10", "L11", "L12",
];

/// Axis U class labels (U0..U11).
pub const U_CLASSES: [&str; 12] = [
    "U0", "U1", "U2", "U3", "U4", "U5", "U6", "U7", "U8", "U9", "U10", "U11",
];

/// Axis V class labels (V0..V8).
pub const V_CLASSES: [&str; 9] = ["V0", "V1", "V2", "V3", "V4", "V5", "V6", "V7", "V8"];

/// Draw an `lsbit` for axis class L*.
pub fn gen_lsbit(class: &str, rng: &mut Rng) -> i32 {
    match class {
        "L0" => 0,
        "L1" => 4,
        "L2" => 1,
        "L3" => 3,
        "L4" => 5,
        "L5" => 2,
        "L6" => 6,
        "L7" => 8,
        "L8" => -1,
        "L9" => -4,
        "L10" => i32::MAX,
        "L11" => i32::MIN,
        // Fully random, but biased so the interesting small values stay frequent.
        "L12" => match rng.next_u64() % 4 {
            0 => rng.range_i32(-16, 16),
            1 => rng.pick(&EXTREMES),
            2 => rng.range_i32(0, 8),
            _ => rng.i32_any(),
        },
        other => panic!("unknown L class {other}"),
    }
}

/// Draw a `uni` for axis class U*.
pub fn gen_uni(class: &str, rng: &mut Rng) -> i32 {
    match class {
        "U0" => 0,
        "U1" => 8,
        "U2" => 7,
        "U3" => 15,
        "U4" => rng.range_i32(1, 6),
        "U5" => rng.range_i32(9, 14),
        "U6" => rng.range_i32(0, 15),
        // Positive with high bits set: keep low 4 bits arbitrary, force high bits.
        "U7" => {
            let high = rng.range_i32(1, 0x07FF_FFFF) << 4;
            high | rng.range_i32(0, 15)
        }
        "U8" => match rng.next_u64() % 3 {
            0 => rng.range_i32(-16, -1),
            1 => -(rng.range_i32(1, 0x0FFF_FFFF)),
            _ => (rng.next_u32() | 0x8000_0000) as i32,
        },
        "U9" => i32::MAX,
        "U10" => i32::MIN,
        "U11" => match rng.next_u64() % 4 {
            0 => rng.range_i32(-32, 32),
            1 => rng.pick(&EXTREMES),
            2 => rng.range_i32(0, 15),
            _ => rng.i32_any(),
        },
        other => panic!("unknown U class {other}"),
    }
}

/// Draw `(step, pred, tgt, tgt2)` for axis class V*.
pub fn gen_values(class: &str, rng: &mut Rng) -> (i32, i32, i32, i32) {
    match class {
        "V0" => (
            rng.range_i32(1, 255),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
        ),
        "V1" => (
            rng.range_i32(0, 7),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
        ),
        "V2" => (
            0,
            rng.range_i32(-1 << 20, 1 << 20),
            rng.range_i32(-1 << 20, 1 << 20),
            rng.range_i32(-1 << 20, 1 << 20),
        ),
        "V3" => (
            rng.range_i32(-255, -1),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
        ),
        "V4" => (
            rng.pick(&[
                i32::MAX,
                i32::MIN,
                0x1000_0000,
                0x7FFF_FFF8,
                i32::MIN + 1,
                -0x1000_0000,
            ]),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
            rng.range_i32(-32768, 32767),
        ),
        "V5" => {
            let step = rng.range_i32(1, 4096);
            let pred = rng.range_i32(-1 << 20, 1 << 20);
            let tgt = rng.range_i32(-1 << 20, 1 << 20);
            (step, pred, tgt, tgt) // tgt2 == tgt
        }
        "V6" => {
            let step = rng.range_i32(1, 4096);
            let pred = rng.range_i32(-1 << 20, 1 << 20);
            let tgt = rng.range_i32(-1 << 20, 1 << 20);
            // |tgt2 - pred| > 2^26 so that d3 >> 5 dominates.
            let far = rng.range_i32(1 << 26, i32::MAX);
            let tgt2 = if rng.bool() { far } else { far.wrapping_neg() };
            (step, pred, tgt, tgt2)
        }
        "V7" => (
            rng.pick(&EXTREMES),
            rng.pick(&EXTREMES),
            rng.pick(&EXTREMES),
            rng.pick(&EXTREMES),
        ),
        "V8" => (rng.i32_any(), rng.i32_any(), rng.i32_any(), rng.i32_any()),
        other => panic!("unknown V class {other}"),
    }
}

/// Build a full argument tuple from one class per axis.
pub fn gen_args(l: &str, u: &str, v: &str, rng: &mut Rng) -> Args {
    let lsbit = gen_lsbit(l, rng);
    let uni = gen_uni(u, rng);
    let (step, pred, tgt, tgt2) = gen_values(v, rng);
    Args::new(uni, step, pred, tgt, tgt2, lsbit)
}

/// Randomized sweep for one CONFIGS.md row: pins whichever axes are `Some` and
/// randomizes the rest across all of their classes.
pub fn sweep_row(
    row: &str,
    pin_l: Option<&str>,
    pin_u: Option<&str>,
    pin_v: Option<&str>,
    cases: usize,
) -> usize {
    let mut rng = Rng::for_row(row);
    let mut n = 0usize;
    for _ in 0..cases {
        let l = pin_l.unwrap_or_else(|| L_CLASSES[(rng.next_u64() % 13) as usize]);
        let u = pin_u.unwrap_or_else(|| U_CLASSES[(rng.next_u64() % 12) as usize]);
        let v = pin_v.unwrap_or_else(|| V_CLASSES[(rng.next_u64() % 9) as usize]);
        let a = gen_args(l, u, v, &mut rng);
        check(row, a);
        n += 1;
    }
    assert!(n > 0, "[{row}] ran zero cases");
    n
}
