//! Shared differential-test harness.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported C ABI symbol — the Rust side is never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

pub type GaussianKernelFn = unsafe extern "C" fn(*mut f32, c_int, f32);

/// Number of `f32` guard slots kept *after* the region the caller nominally
/// owns. The C writes `size + 1` elements when `size` is even, so the guard
/// must be big enough to absorb that and still leave untouched padding whose
/// bytes we can compare.
pub const GUARD: usize = 16;

/// Bit pattern used to pre-fill buffers: a NaN the implementation can never
/// produce (it never stores a NaN), so any surviving slot is provably untouched.
pub const SENTINEL: u32 = 0x7f81_2345;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Impl {
    name: &'static str,
    _lib: Library,
    pub gaussian_kernel: GaussianKernelFn,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        let f: GaussianKernelFn = unsafe {
            let sym: Symbol<GaussianKernelFn> = lib
                .get(b"gaussian_kernel\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `gaussian_kernel`: {e}"));
            *sym
        };
        Impl {
            name,
            _lib: lib,
            gaussian_kernel: f,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let is_so = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false);
            if is_so {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    let lib_name = "libgaussian_kernel_lib.so";

    // Explicit override (used by `run_all_tests.sh`), so we always know exactly
    // which artefact is under test.
    if let Some(p) = std::env::var_os("RUST_DIFF_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.is_file(),
            "RUST_DIFF_SO points at {} which is not a file",
            p.display()
        );
        return p;
    }

    // current_exe() is <target>/<profile>/deps/<test-bin>; the cdylib that
    // cargo built for this very invocation lives in <target>/<profile>/.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join(lib_name);
            if p.is_file() {
                return p;
            }
        }
    }

    for profile in ["release", "debug"] {
        let p = workspace_root()
            .join("translation")
            .join("target")
            .join(profile)
            .join(lib_name);
        if p.is_file() {
            return p;
        }
    }

    panic!("could not locate {lib_name}; run `cargo build` first");
}

/// The two artefacts under test: `(C .so, Rust .so)`.
pub fn so_paths() -> (PathBuf, PathBuf) {
    (find_c_so(), find_rust_so())
}

/// Loads both implementations. Held in a `OnceLock` so every test in a binary
/// shares one `dlopen`.
pub fn impls() -> &'static (Impl, Impl) {
    use std::sync::OnceLock;
    static PAIR: OnceLock<(Impl, Impl)> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c = Impl::open("C", &find_c_so());
        let r = Impl::open("Rust", &find_rust_so());
        (c, r)
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external crate, fully reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
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

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        (self.next_u64() % n as u64) as u32
    }

    /// Uniform in `[lo, hi]` (inclusive), `i32` domain.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// Uniform `f32` in `[0, 1)` with 24 bits of entropy.
    pub fn unit_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform `f32` in `[lo, hi]`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit_f32()
    }

    /// Log-uniform `f32` in `[lo, hi]`, both > 0.
    pub fn log_range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        let l = (lo as f64).ln();
        let h = (hi as f64).ln();
        (l + (h - l) * self.unit_f32() as f64).exp() as f32
    }

    /// A completely arbitrary `f32`: every one of the 2^32 bit patterns is
    /// reachable (NaNs, sNaNs, ±inf, subnormals, ±0).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

// ---------------------------------------------------------------------------
// Differential invocation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Zero,
    Sentinel,
    Random(u64),
}

/// One differential test case.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub size: c_int,
    pub radius: f32,
    /// How many `f32` slots sit *before* the pointer handed to the library.
    pub offset: usize,
    pub fill: Fill,
    /// Pass a null `dest`. Only valid when the C provably never dereferences.
    pub null_dest: bool,
}

impl Case {
    pub fn new(size: c_int, radius: f32) -> Case {
        Case {
            size,
            radius,
            offset: 0,
            fill: Fill::Sentinel,
            null_dest: false,
        }
    }
    pub fn offset(mut self, offset: usize) -> Case {
        self.offset = offset;
        self
    }
    pub fn fill(mut self, fill: Fill) -> Case {
        self.fill = fill;
        self
    }
    pub fn null_dest(mut self) -> Case {
        self.null_dest = true;
        self
    }

    /// Total number of `f32` slots the scratch buffer needs.
    fn buf_len(&self) -> usize {
        // The C stores `2*(size/2) + 1` elements when `size/2 >= 0`.
        let stores = if self.size / 2 >= 0 {
            2usize.saturating_mul((self.size / 2) as usize) + 1
        } else {
            0
        };
        let nominal = if self.size > 0 { self.size as usize } else { 0 };
        self.offset + stores.max(nominal) + GUARD
    }
}

fn make_buffer(case: &Case) -> Vec<u32> {
    let len = case.buf_len();
    match case.fill {
        Fill::Zero => vec![0u32; len],
        Fill::Sentinel => vec![SENTINEL; len],
        Fill::Random(seed) => {
            let mut rng = Rng::new(seed);
            (0..len).map(|_| rng.next_u32()).collect()
        }
    }
}

/// Runs `case` against a single implementation and returns the raw bit patterns
/// of the whole scratch buffer (prefix + payload + guard).
fn run_one(f: GaussianKernelFn, case: &Case) -> Vec<u32> {
    let mut buf = make_buffer(case);
    if case.null_dest {
        assert!(
            case.size / 2 < 0,
            "null dest is only safe when the C never dereferences (size/2 < 0), got size={}",
            case.size
        );
        unsafe { f(std::ptr::null_mut(), case.size, case.radius) };
    } else {
        let base = buf.as_mut_ptr() as *mut f32;
        let dest = unsafe { base.add(case.offset) };
        unsafe { f(dest, case.size, case.radius) };
    }
    buf
}

fn describe(case: &Case) -> String {
    format!(
        "size={} radius={:e} (bits 0x{:08x}) offset={} fill={:?} null_dest={}",
        case.size,
        case.radius,
        case.radius.to_bits(),
        case.offset,
        case.fill,
        case.null_dest
    )
}

/// Total number of differential comparisons performed by this test binary.
/// Lets the suite prove it really executed the advertised number of draws
/// (run with `-- --nocapture` to see the per-test totals).
pub static COMPARISONS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn comparisons() -> u64 {
    COMPARISONS.load(std::sync::atomic::Ordering::Relaxed)
}

/// The workhorse: run the case through both `.so`s and assert byte-identical
/// buffers. Returns the (shared) result buffer for further inspection.
pub fn assert_same(case: &Case) -> Vec<u32> {
    COMPARISONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (c, r) = impls();
    let out_c = run_one(c.gaussian_kernel, case);
    let out_r = run_one(r.gaussian_kernel, case);

    if out_c != out_r {
        let mut diffs = Vec::new();
        for (i, (a, b)) in out_c.iter().zip(out_r.iter()).enumerate() {
            if a != b {
                diffs.push(format!(
                    "  [{i}] C=0x{a:08x} ({}) Rust=0x{b:08x} ({})",
                    f32::from_bits(*a),
                    f32::from_bits(*b)
                ));
                if diffs.len() == 24 {
                    diffs.push("  ...".to_string());
                    break;
                }
            }
        }
        panic!(
            "DIVERGENCE for {}\nbuffer len = {}\n{}",
            describe(case),
            out_c.len(),
            diffs.join("\n")
        );
    }
    out_c
}

/// Same as [`assert_same`] but for a *sequence* of calls on one buffer, which
/// is how a real consumer reuses scratch space and which also proves there is
/// no hidden per-library state.
pub fn assert_same_sequence(cases: &[Case]) -> Vec<u32> {
    assert!(!cases.is_empty());
    let (c, r) = impls();
    let template = cases
        .iter()
        .max_by_key(|c| c.buf_len())
        .expect("non-empty")
        .to_owned();

    let run_all = |f: GaussianKernelFn| -> Vec<u32> {
        let mut buf = make_buffer(&template);
        for case in cases {
            let base = buf.as_mut_ptr() as *mut f32;
            let dest = unsafe { base.add(case.offset) };
            unsafe { f(dest, case.size, case.radius) };
        }
        buf
    };

    let out_c = run_all(c.gaussian_kernel);
    let out_r = run_all(r.gaussian_kernel);
    assert_eq!(
        out_c,
        out_r,
        "DIVERGENCE in call sequence {:?}",
        cases.iter().map(describe).collect::<Vec<_>>()
    );
    out_c
}

// ---------------------------------------------------------------------------
// Shared input vocabulary (mirrors the axes in CONFIGS.md)
// ---------------------------------------------------------------------------

/// `sigma` from the C source.
pub const SIGMA: f32 = 1.6;
/// `1.0f - 1.0f/expf(sigma*sigma*tetha)`, i.e. the value stored at `r == 0`.
pub const V0_BITS: u32 = 0x3f7f_317d;
/// `sqrtf(sigma*sigma*tetha)`: `|x| >= this` ⇒ the clamp fires.
pub const CLAMP_X: f32 = 2.400_000_1;

/// Every "special" radius the C distinguishes, as raw bit patterns.
pub fn special_radii() -> Vec<f32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,          // == -f32::MAX
        f32::MIN_POSITIVE, // smallest normal
        -f32::MIN_POSITIVE,
        SIGMA,
        -SIGMA,
    ];
    // NaNs: quiet, signalling, both signs, several payloads.
    for bits in [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7f80_0001,
        0xff80_0001,
        0x7fff_ffff,
        0xffff_ffff,
        0x7fc0_dead,
        0xffca_fe00,
    ] {
        v.push(f32::from_bits(bits));
    }
    // Subnormals (positive and negative), including ones that make
    // `sigma / radius` overflow to +/-inf.
    for bits in [
        0x0000_0001u32,
        0x0000_0002,
        0x0040_0000,
        0x007f_ffff,
        0x8000_0001,
        0x807f_ffff,
    ] {
        v.push(f32::from_bits(bits));
    }
    v
}

/// A representative spread of `size` values covering every shape class.
pub fn representative_sizes() -> Vec<c_int> {
    let mut v = vec![
        i32::MIN,
        i32::MIN + 1,
        -100_000,
        -12_345,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        15,
        16,
        17,
        31,
        32,
        63,
        64,
        65,
        127,
        128,
        255,
        256,
        511,
        512,
        1023,
        1024,
        1025,
    ];
    v.dedup();
    v
}
