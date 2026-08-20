//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! *Both* implementations are reached exclusively through `dlopen`/`dlsym`
//! (via `libloading`) so that the Rust `#[no_mangle] extern "C"` wrapper is
//! exercised the same way an external C consumer would exercise it.  No Rust
//! function is ever called directly.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags, int param1, int param2)`
pub type ProcessBufferFn =
    unsafe extern "C" fn(*mut u8, usize, u32, std::ffi::c_int, std::ffi::c_int) -> usize;

/// Path of the shared object built from the pristine `c_src/src/lib.c`
/// (produced by `build.rs`).
///
/// `DIFF_C_SO` overrides it, which lets the same suite be run against C objects
/// built with different optimisation levels.
pub fn c_so_path() -> PathBuf {
    match std::env::var_os("DIFF_C_SO") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env!("C_SO_PATH")),
    }
}

/// Path of the original C command line program (`main.c` + `lib.c`).
pub fn c_driver_path() -> PathBuf {
    match std::env::var_os("DIFF_C_DRIVER") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env!("C_DRIVER_PATH")),
    }
}

/// `target/<profile>` directory that holds the artefacts of this build.
pub fn profile_dir() -> PathBuf {
    // .../target/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Path of the Rust command line program.
///
/// `DIFF_RUST_DRIVER` overrides it (used to test the `release` artefacts from a
/// `dev`-profile harness, since `panic = "abort"` in `[profile.release]` makes
/// `cargo test --release` unable to link a cdylib into a test binary).
pub fn rust_driver_path() -> PathBuf {
    match std::env::var_os("DIFF_RUST_DRIVER") {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from(env!("CARGO_BIN_EXE_driver")),
    }
}

/// Path of the Rust `cdylib`.
///
/// `cargo test` does not build non-test crate types, so the shared object is
/// materialised on demand with a plain `cargo build --lib` (the build lock is
/// already released by the time integration tests execute).
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(p) = std::env::var_os("DIFF_RUST_SO").filter(|p| !p.is_empty()) {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DIFF_RUST_SO points at {}", p.display());
            return p;
        }
        let dir = profile_dir();
        let so = dir.join("libdriver.so");
        let release = dir.file_name().map(|n| n == "release").unwrap_or(false);

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo);
        cmd.current_dir(&manifest).arg("build").arg("--lib");
        if release {
            cmd.arg("--release");
        }
        // Keep the feature selection of the current test run.
        if let Ok(features) = std::env::var("DIFF_TEST_FEATURES") {
            cmd.arg("--no-default-features");
            if !features.is_empty() {
                cmd.arg("--features").arg(features);
            }
        }
        match cmd.status() {
            Ok(s) if s.success() => {}
            other => {
                if !so.exists() {
                    panic!("could not build the Rust cdylib ({other:?}); run `cargo build` first");
                }
            }
        }
        assert!(so.exists(), "missing Rust shared object at {}", so.display());
        so
    })
    .clone()
}

fn load(path: &Path) -> ProcessBufferFn {
    let lib = unsafe { libloading::Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let sym: libloading::Symbol<ProcessBufferFn> = unsafe { lib.get(b"process_buffer\0") }
        .unwrap_or_else(|e| panic!("dlsym(process_buffer) in {} failed: {e}", path.display()));
    let f = *sym;
    // Deliberately leak: the library must stay mapped for the whole process.
    std::mem::forget(lib);
    f
}

/// `process_buffer` as exported by the C shared object.
pub fn c_process_buffer() -> ProcessBufferFn {
    static F: OnceLock<ProcessBufferFn> = OnceLock::new();
    *F.get_or_init(|| load(&c_so_path()))
}

/// `process_buffer` as exported by the Rust shared object.
pub fn rust_process_buffer() -> ProcessBufferFn {
    static F: OnceLock<ProcessBufferFn> = OnceLock::new();
    *F.get_or_init(|| load(&rust_so_path()))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) - fixed seeds keep every failure reproducible
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
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
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

// ---------------------------------------------------------------------------
// Input generators - the "data shapes" axis of CONFIGS.md
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Uniformly random bytes over the whole `0..=255` range.
    Random,
    /// Random bytes drawn from a tiny alphabet -> lots of duplicates & runs.
    SmallAlphabet,
    /// Every byte identical -> one gigantic run.
    Constant,
    /// Strictly increasing, every value distinct -> no runs at all.
    AllDistinct,
    /// Concatenation of runs whose lengths are 1..=6 -> straddles thresholds.
    ShortRuns,
    /// Concatenation of runs whose lengths are 250..=260 -> straddles the 255 cap.
    LongRuns,
    /// Alternating two values -> every run has length exactly 1.
    Alternating,
    /// Two halves, each internally constant.
    TwoBlocks,
}

pub const ALL_SHAPES: [Shape; 8] = [
    Shape::Random,
    Shape::SmallAlphabet,
    Shape::Constant,
    Shape::AllDistinct,
    Shape::ShortRuns,
    Shape::LongRuns,
    Shape::Alternating,
    Shape::TwoBlocks,
];

pub fn make_input(shape: Shape, len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    match shape {
        Shape::Random => {
            for _ in 0..len {
                v.push(rng.byte());
            }
        }
        Shape::SmallAlphabet => {
            let alphabet = 1 + rng.below(4); // 1..=4 distinct values
            let base = rng.byte();
            for _ in 0..len {
                v.push(base.wrapping_add(rng.below(alphabet) as u8));
            }
        }
        Shape::Constant => {
            let b = rng.byte();
            v.resize(len, b);
        }
        Shape::AllDistinct => {
            let start = rng.byte();
            for i in 0..len {
                v.push(start.wrapping_add((i % 256) as u8));
            }
        }
        Shape::ShortRuns => {
            while v.len() < len {
                let b = rng.byte();
                let n = 1 + rng.below(6);
                for _ in 0..n {
                    if v.len() == len {
                        break;
                    }
                    v.push(b);
                }
            }
        }
        Shape::LongRuns => {
            while v.len() < len {
                let b = rng.byte();
                let n = 250 + rng.below(11);
                for _ in 0..n {
                    if v.len() == len {
                        break;
                    }
                    v.push(b);
                }
            }
        }
        Shape::Alternating => {
            let a = rng.byte();
            let b = a.wrapping_add(1 + rng.byte() % 254);
            for i in 0..len {
                v.push(if i % 2 == 0 { a } else { b });
            }
        }
        Shape::TwoBlocks => {
            let a = rng.byte();
            let b = a.wrapping_add(1);
            let half = len / 2;
            for i in 0..len {
                v.push(if i < half { a } else { b });
            }
        }
    }
    debug_assert_eq!(v.len(), len);
    v
}

// ---------------------------------------------------------------------------
// The differential driver
// ---------------------------------------------------------------------------

/// Extra bytes appended behind the window the C code may legally touch.
/// They are initialised with a fixed pattern; any divergence there is reported
/// like any other difference.
pub const GUARD: usize = 96;

/// Number of bytes both implementations may touch for a given `length`.
///
/// `compact_runs()` (`flags & 0x02`) can grow the logical length up to
/// `2 * length`; every other operation stays inside the current length.
pub fn window(length: usize, flags: u32) -> usize {
    if flags & 0x02 != 0 {
        length.saturating_mul(2)
    } else {
        length
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ret: usize,
    pub buffer: Vec<u8>,
}

/// Runs `process_buffer` from `f` on a freshly prepared copy of `data`.
///
/// The scratch buffer is `window(length, flags) + GUARD` bytes long so that
/// neither implementation can corrupt memory it does not own even when the
/// (buggy) C code grows the logical length past `length`.
pub fn run_one(
    f: ProcessBufferFn,
    data: &[u8],
    length: usize,
    flags: u32,
    p1: i32,
    p2: i32,
    filler: u8,
) -> Outcome {
    let cap = window(length, flags).max(data.len()) + GUARD;
    let mut buf = vec![filler; cap];
    buf[..data.len()].copy_from_slice(data);
    let ret = unsafe { f(buf.as_mut_ptr(), length, flags, p1, p2) };
    // Global invariant of the C implementation, and the contract the FFI wrapper
    // relies on: the returned length never leaves the `window()` the caller is
    // required to provide.  If this ever tripped for the C `.so`, the window
    // computed by `src/ffi.rs` would be wrong.
    assert!(
        ret <= window(length, flags),
        "process_buffer returned {ret} > window({length}, {flags:#x}) = {}",
        window(length, flags)
    );
    Outcome { ret, buffer: buf }
}

/// Case description used in assertion messages.
pub fn describe(data: &[u8], length: usize, flags: u32, p1: i32, p2: i32) -> String {
    let preview: Vec<String> = data.iter().take(48).map(|b| b.to_string()).collect();
    format!(
        "length={length} flags={flags:#x} param1={p1} param2={p2}\n  data[{}] = [{}{}]",
        data.len(),
        preview.join(", "),
        if data.len() > 48 { ", ..." } else { "" }
    )
}

/// Filler byte for the bytes behind `data`.
///
/// It is derived from the case itself instead of being a constant so that the
/// padding differs from case to case: if one implementation read a byte past the
/// live region that the other did not, a constant filler could accidentally
/// match the other implementation's value and hide the difference.  Both
/// implementations always receive the *same* filler, so the comparison stays
/// deterministic and reproducible.
///
/// It deliberately depends only on `(data, length)` and **not** on
/// `flags`/`param1`/`param2`, so that a test may still compare the outcomes of
/// two *different* option sets applied to the *same* input (e.g. "`param1 <= 0`
/// must behave exactly like `param1 == 4`").
pub fn case_filler(data: &[u8], length: usize) -> u8 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x1000_0000_01b3);
    };
    mix(length as u64);
    mix(data.len() as u64);
    for &b in data.iter().take(64) {
        mix(b as u64);
    }
    // Never 0 and never a value the generators favour, just to be tidy.
    (h >> 33) as u8
}

/// Calls both shared objects and asserts identical return value *and* identical
/// scratch buffer (including the bytes past the returned length and the guard
/// area).
pub fn assert_same(data: &[u8], length: usize, flags: u32, p1: i32, p2: i32) {
    let filler = case_filler(data, length);
    let c = run_one(c_process_buffer(), data, length, flags, p1, p2, filler);
    let r = run_one(rust_process_buffer(), data, length, flags, p1, p2, filler);

    if c.ret != r.ret {
        panic!(
            "return value mismatch: C={} Rust={}\n  {}",
            c.ret,
            r.ret,
            describe(data, length, flags, p1, p2)
        );
    }
    if c.buffer != r.buffer {
        let idx = c
            .buffer
            .iter()
            .zip(r.buffer.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "buffer mismatch at index {idx}: C={} Rust={}\n  {}\n  C   ={:?}\n  Rust={:?}",
            c.buffer[idx],
            r.buffer[idx],
            describe(data, length, flags, p1, p2),
            &c.buffer[..c.buffer.len().min(160)],
            &r.buffer[..r.buffer.len().min(160)],
        );
    }
}

/// Runs the case through both shared objects and hands both outcomes back so a
/// test can additionally assert the *exact* sentinel the C code documents
/// (return value, "buffer untouched", …) instead of only C-vs-Rust equality.
pub fn run_both(data: &[u8], length: usize, flags: u32, p1: i32, p2: i32) -> (Outcome, Outcome) {
    let filler = case_filler(data, length);
    let c = run_one(c_process_buffer(), data, length, flags, p1, p2, filler);
    let r = run_one(rust_process_buffer(), data, length, flags, p1, p2, filler);
    assert_eq!(
        c.ret,
        r.ret,
        "return value mismatch: C={} Rust={}\n  {}",
        c.ret,
        r.ret,
        describe(data, length, flags, p1, p2)
    );
    assert_eq!(
        c.buffer,
        r.buffer,
        "buffer mismatch\n  {}",
        describe(data, length, flags, p1, p2)
    );
    (c, r)
}

/// Same as [`run_both`] but additionally asserts that *neither* implementation
/// modified the input region (used for every "guard rejected the request"
/// row of `ERRORS.md`).
pub fn assert_same_and_untouched(data: &[u8], length: usize, flags: u32, p1: i32, p2: i32) {
    let (c, r) = run_both(data, length, flags, p1, p2);
    assert_eq!(
        &c.buffer[..data.len()],
        data,
        "C modified the buffer although the guard should have rejected: {}",
        describe(data, length, flags, p1, p2)
    );
    assert_eq!(
        &r.buffer[..data.len()],
        data,
        "Rust modified the buffer although the guard should have rejected: {}",
        describe(data, length, flags, p1, p2)
    );
}

/// Calls both `.so`s with a raw pointer (possibly NULL) and asserts identical
/// return values.
pub fn assert_same_raw(ptr: *mut u8, length: usize, flags: u32, p1: i32, p2: i32) -> usize {
    let c = unsafe { c_process_buffer()(ptr, length, flags, p1, p2) };
    let r = unsafe { rust_process_buffer()(ptr, length, flags, p1, p2) };
    assert_eq!(
        c, r,
        "raw-call mismatch: C={c} Rust={r} (ptr={ptr:p} length={length} flags={flags:#x} p1={p1} p2={p2})"
    );
    c
}

/// Convenience wrapper: generate `iters` random cases for the given shape/length
/// pools and check every one of them.
pub struct Sweep {
    pub rng: Rng,
}

impl Sweep {
    pub fn new(seed: u64) -> Self {
        Sweep { rng: Rng::new(seed) }
    }

    pub fn check(&mut self, shapes: &[Shape], lengths: &[usize], flags: u32, p1: i32, p2: i32) {
        for &shape in shapes {
            for &len in lengths {
                let data = make_input(shape, len, &mut self.rng);
                assert_same(&data, len, flags, p1, p2);
            }
        }
    }
}
