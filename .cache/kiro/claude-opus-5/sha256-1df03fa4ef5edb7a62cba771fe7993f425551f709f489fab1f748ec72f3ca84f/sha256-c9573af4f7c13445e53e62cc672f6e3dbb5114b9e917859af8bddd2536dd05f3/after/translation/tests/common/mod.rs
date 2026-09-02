//! Shared harness: loads BOTH shared objects (the reference C one and the Rust
//! one) with `libloading` and exposes them behind one identical interface.
//!
//! Nothing in here calls a Rust function directly — every invocation goes
//! through `dlsym` on `libtritanopia_lib.so`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// Mirror of `cb_rgb_255` from `c_src/include/lib.h`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CbRgb255 {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub type TritanopiaFn = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

/// A `tritanopia` entry point reached through `dlsym`.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    // Field order matters: `func` must be dropped before `lib`.
    func: TritanopiaFn,
    _lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: PathBuf) -> Self {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let func = unsafe {
            let sym: Symbol<TritanopiaFn> = lib
                .get(b"tritanopia\0")
                .unwrap_or_else(|e| panic!("dlsym(tritanopia) in {}: {e}", path.display()));
            *sym
        };
        Self {
            name,
            path,
            func,
            _lib: lib,
        }
    }

    #[inline]
    pub fn call(&self, input: CbRgb255) -> CbRgb255 {
        unsafe { (self.func)(input) }
    }

    /// Calls through a signature that widens the argument to a full eightbyte so
    /// the caller controls the *unused* bytes 3..7 of the register. Used by the
    /// `ERRORS.md` E6 / `CONFIGS.md` R17 rows.
    #[inline]
    pub fn call_with_padding(&self, input: CbRgb255, junk: [u8; 5]) -> CbRgb255 {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct Padded {
            r: u8,
            g: u8,
            b: u8,
            junk: [u8; 5],
        }
        type PaddedFn = unsafe extern "C" fn(Padded) -> CbRgb255;
        let f: PaddedFn = unsafe { std::mem::transmute::<TritanopiaFn, PaddedFn>(self.func) };
        unsafe {
            f(Padded {
                r: input.r,
                g: input.g,
                b: input.b,
                junk,
            })
        }
    }
}

/// The pair under comparison.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

impl Pair {
    /// Asserts the two implementations agree on `input`, and returns the value.
    #[inline]
    #[track_caller]
    pub fn agree(&self, input: CbRgb255) -> CbRgb255 {
        let expected = self.c.call(input);
        let actual = self.rust.call(input);
        assert_eq!(
            expected, actual,
            "divergence for input {input:?}: C={expected:?} Rust={actual:?}"
        );
        expected
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    candidates.sort();
    match candidates.len() {
        0 => panic!("no .so found in {}", build.display()),
        _ => candidates.remove(0),
    }
}

fn find_rust_so() -> PathBuf {
    // Integration tests live in target/{debug,release}/deps/<name>-<hash>, so the
    // cdylib sits two directories up from the test executable.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf();

    let name = "libtritanopia_lib.so";
    let direct = profile_dir.join(name);
    if direct.is_file() {
        return direct;
    }
    // Fall back to whichever profile directory has it.
    for profile in ["release", "debug"] {
        let p = repo_root().join("translation/target").join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "{name} not found (looked in {} and target/{{release,debug}}). \
         Run `cargo build --release` in translation/ first.",
        profile_dir.display()
    );
}

/// Opens both shared objects. Panics with an actionable message if either is
/// missing, rather than silently skipping the comparison.
pub fn load() -> Pair {
    Pair {
        c: Impl::open("C", find_c_so()),
        rust: Impl::open("Rust", find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG: fixed seed so every row is reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // 0 is a fixed point of xorshift64*, so avoid it.
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    /// xorshift64* — tiny, dependency-free, and fully reproducible.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }

    /// Uniform in `0..=hi` (inclusive), unbiased enough for `u8` ranges.
    #[inline]
    pub fn u8_upto(&mut self, hi: u8) -> u8 {
        let span = hi as u64 + 1;
        ((self.next_u64() >> 32) % span) as u8
    }

    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn u8_in(&mut self, lo: u8, hi: u8) -> u8 {
        assert!(lo <= hi);
        lo + self.u8_upto(hi - lo)
    }

    #[inline]
    pub fn pick(&mut self, from: &[u8]) -> u8 {
        from[((self.next_u64() >> 32) as usize) % from.len()]
    }
}

/// Number of randomized inputs used per `CONFIGS.md` row.
pub const ROW_SAMPLES: usize = 4096;

/// Runs `gen` `ROW_SAMPLES` times and asserts C and Rust agree every time.
/// The row name is threaded through so a failure names the `CONFIGS.md` row.
#[track_caller]
pub fn check_row(pair: &Pair, row: &str, mut make: impl FnMut(&mut Rng) -> CbRgb255) {
    let mut rng = Rng::new(SEED ^ fnv1a(row.as_bytes()));
    for i in 0..ROW_SAMPLES {
        let input = make(&mut rng);
        let expected = pair.c.call(input);
        let actual = pair.rust.call(input);
        assert_eq!(
            expected, actual,
            "{row}: divergence on sample {i} for input {input:?}: C={expected:?} Rust={actual:?}"
        );
    }
}

/// Stable per-row seed derivation, so rows do not all draw the same sequence.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Byte values that straddle the `cbRemoveGammaRGB` threshold (`> 0.04045`),
/// measured on the C: linear for 0..=10, `pow` for 11..=255.
pub const GAMMA_LINEAR_MAX: u8 = 10;
pub const GAMMA_POW_MIN: u8 = 11;

/// The extreme / boundary set used by `CONFIGS.md` row R16.
pub const EXTREMES: [u8; 8] = [0, 1, 10, 11, 127, 128, 254, 255];
