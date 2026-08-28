//! Shared differential-test harness.
//!
//! BOTH implementations are reached only through `libloading`, i.e. through the exported
//! `hdr_compare` symbol of a shared object. The Rust crate is never linked directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what gets tested.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type HdrCompareFn = unsafe extern "C" fn(*const u8, *const u8) -> c_int;

pub struct Libs {
    // Keep the libraries alive for the whole process; the raw fn pointers below borrow them.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HdrCompareFn,
    pub rs: HdrCompareFn,
    pub c_so_path: PathBuf,
    pub rust_so_path: PathBuf,
}

unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Directory that contains `c_src/` and `translation/`.
pub fn work_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// `translation/target/<profile>` — the directory cargo drops the `cdylib` into.
pub fn rust_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-binary>
    let deps = exe.parent().expect("deps dir");
    if deps.file_name().map(|n| n == "deps").unwrap_or(false) {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps.to_path_buf()
    }
}

const SO_NAMES: [&str; 2] = ["libhdr_compare_lib.so", "libtranslation.so"];

fn so_in(dir: &Path) -> Option<PathBuf> {
    SO_NAMES
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.exists())
}

/// Locates the Rust `cdylib` **for the profile currently under test**, building it on demand.
///
/// `cargo test` does not necessarily build the `cdylib` (nothing links it), so the harness
/// makes sure it exists. It deliberately does NOT fall back to a different profile's artifact:
/// `debug` and `release` differ in `-C debug-assertions`, which changes the observable
/// behaviour on invalid pointers, so testing the wrong artifact would mask real divergences.
/// If the profile's `.so` is missing it is built into a private target dir (a private dir
/// avoids lock contention with the `cargo test` invocation that is running us).
pub fn rust_so_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let profile_dir = rust_profile_dir();

    if let Some(p) = so_in(&profile_dir) {
        return p;
    }
    let is_release = profile_dir
        .file_name()
        .map(|n| n == "release")
        .unwrap_or(false);

    // Build it ourselves into a private target directory, for THIS profile.
    let priv_target = manifest.join("target").join("cdylib-for-tests");
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&priv_target);
    if is_release {
        cmd.arg("--release");
    }
    // Do not inherit the outer cargo's environment knobs that would confuse the nested build.
    for k in ["RUSTC_WORKSPACE_WRAPPER", "RUSTC_WRAPPER", "CARGO_TARGET_DIR"] {
        cmd.env_remove(k);
    }
    let out = cmd.output().expect("spawn nested `cargo build --lib`");
    let sub = priv_target.join(if is_release { "release" } else { "debug" });
    if let Some(p) = so_in(&sub) {
        return p;
    }
    panic!(
        "Rust cdylib not found in {} and the nested build failed.\n\
         Build it with `cargo build --release` (or `cargo build`) and re-run.\n\
         stdout:\n{}\nstderr:\n{}",
        profile_dir.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Locates (building on demand) the C shared object produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    let c_src = work_root().join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so(&build) {
        return p;
    }

    // Build it: cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let cfg = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("run cmake configure");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .current_dir(&build)
        .arg("--build")
        .arg(".")
        .output()
        .expect("run cmake build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );

    find_so(&build).expect("C .so still missing after cmake --build")
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_so_path = c_so_path();
        let rust_so_path = rust_so_path();

        unsafe {
            let c_lib = Library::new(&c_so_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_so_path.display()));
            let rust_lib = Library::new(&rust_so_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_so_path.display()));

            let c_sym: Symbol<HdrCompareFn> = c_lib
                .get(b"hdr_compare\0")
                .expect("C .so must export hdr_compare");
            let rs_sym: Symbol<HdrCompareFn> = rust_lib
                .get(b"hdr_compare\0")
                .expect("Rust .so must export hdr_compare");

            let c = *c_sym;
            let rs = *rs_sym;

            eprintln!(
                "[harness] C   .so: {}\n[harness] Rust.so: {}\n[harness] stride: {}",
                c_so_path.display(),
                rust_so_path.display(),
                stride()
            );

            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rs,
                c_so_path,
                rust_so_path,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

/// Calls both `.so`s with the given 3-byte headers and asserts byte-identical results.
/// Returns the (agreed) result.
#[inline]
pub fn diff3(h1: &[u8; 3], h2: &[u8; 3]) -> c_int {
    let l = libs();
    unsafe { diff_ptr(l, h1.as_ptr(), h2.as_ptr(), || format!("h1={h1:02X?} h2={h2:02X?}")) }
}

/// Raw-pointer differential call. `ctx` is only invoked on failure.
#[inline]
pub unsafe fn diff_ptr<F: FnOnce() -> String>(
    l: &Libs,
    h1: *const u8,
    h2: *const u8,
    ctx: F,
) -> c_int {
    let a = (l.c)(h1, h2);
    let b = (l.rs)(h1, h2);
    if a != b {
        panic!("DIVERGENCE: C returned {a}, Rust returned {b} for {}", ctx());
    }
    assert!(
        a == 0 || a == 1,
        "C returned {a}, expected a C boolean (0 or 1) for {}",
        ctx()
    );
    a
}

/// Reference model of the C, used only as an *extra* cross-check on top of the
/// C-vs-Rust differential (never as a substitute for it).
#[inline]
pub fn model(h1: &[u8; 3], h2: &[u8; 3]) -> c_int {
    let valid = h2[0] == 0xff
        && ((h2[1] & 0xF0) == 0xf0 || (h2[1] & 0xFE) == 0xe2)
        && (((h2[1] >> 1) & 3) != 0)
        && ((h2[2] >> 4) != 15)
        && (((h2[2] >> 2) & 3) != 3);
    let r = valid
        && ((h1[1] ^ h2[1]) & 0xFE) == 0
        && ((h1[2] ^ h2[2]) & 0x0C) == 0
        && (((h1[2] & 0xF0) == 0) == ((h2[2] & 0xF0) == 0));
    r as c_int
}

// ---------------------------------------------------------------------------
// Workload scaling
// ---------------------------------------------------------------------------

/// Divisor applied to the heavy sweeps. Unoptimized (`debug`) builds run the same code
/// paths on a strided subset so that `cargo test` (no `--release`) still finishes quickly;
/// `--release` runs every row at full size. Override with `HDR_STRIDE=<n>`.
pub fn stride() -> usize {
    if let Ok(v) = std::env::var("HDR_STRIDE") {
        if let Ok(n) = v.parse::<usize>() {
            return n.max(1);
        }
    }
    if cfg!(debug_assertions) {
        29 // coprime with 2, 3, 5, 180 and 256 so strided subsets stay well spread
    } else {
        1
    }
}

/// Scales an iteration count for the current profile.
pub fn iters(n: u64) -> u64 {
    (n / stride() as u64).max(1)
}

/// `0..=255`, strided for the current profile (the full 256 values under `--release`).
///
/// When strided, the boundary alphabet and every *valid* `h[1]` / `h[2]` byte are unioned
/// back in, so a reduced run still reaches the accepting branches of the C.
pub fn byte_range() -> Vec<u8> {
    let s = stride();
    if s == 1 {
        return (0..=255u8).collect();
    }
    let mut seen = [false; 256];
    let mut out = Vec::new();
    let push = |v: u8, seen: &mut [bool; 256], out: &mut Vec<u8>| {
        if !seen[v as usize] {
            seen[v as usize] = true;
            out.push(v);
        }
    };
    for v in (0..=255u8).step_by(s) {
        push(v, &mut seen, &mut out);
    }
    for v in BOUNDARY_BYTES {
        push(v, &mut seen, &mut out);
    }
    for v in valid_byte1() {
        push(v, &mut seen, &mut out);
    }
    for v in valid_byte2().into_iter().step_by(s) {
        push(v, &mut seen, &mut out);
    }
    out.sort_unstable();
    out
}

/// True when the current profile runs every row at full size (no striding).
pub fn full_size() -> bool {
    stride() == 1
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — no external rand dependency, fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { Self::DEFAULT_SEED } else { seed })
    }
    pub fn seeded() -> Self {
        Rng::new(Self::DEFAULT_SEED)
    }
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
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    #[inline]
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.below(xs.len() as u64)) as usize]
    }
    #[inline]
    pub fn bytes3(&mut self) -> [u8; 3] {
        let v = self.next_u64();
        [v as u8, (v >> 8) as u8, (v >> 16) as u8]
    }
}

// ---------------------------------------------------------------------------
// Domain knowledge extracted from the C (used to build valid inputs)
// ---------------------------------------------------------------------------

/// Every `h[1]` byte for which the C's sync-class + layer checks pass.
pub fn valid_byte1() -> Vec<u8> {
    (0u16..=255)
        .map(|v| v as u8)
        .filter(|&v| ((v & 0xF0) == 0xF0 || (v & 0xFE) == 0xE2) && ((v >> 1) & 3) != 0)
        .collect()
}

/// Every `h[2]` byte for which the C's bitrate + sample-rate checks pass.
pub fn valid_byte2() -> Vec<u8> {
    (0u16..=255)
        .map(|v| v as u8)
        .filter(|&v| (v >> 4) != 15 && ((v >> 2) & 3) != 3)
        .collect()
}

pub const BOUNDARY_BYTES: [u8; 18] = [
    0x00, 0x01, 0x02, 0x03, 0x0C, 0x0F, 0x10, 0x7F, 0x80, 0xE0, 0xE2, 0xE3, 0xEF, 0xF0, 0xF1,
    0xFB, 0xFE, 0xFF,
];

/// A battery of fixed `h1` patterns, incl. real MPEG frame headers.
pub const H1_BATTERY: [[u8; 3]; 8] = [
    [0x00, 0x00, 0x00],
    [0xFF, 0xFF, 0xFF],
    [0xAA, 0x55, 0xAA],
    [0x55, 0xAA, 0x55],
    [0xFF, 0xFB, 0x90],
    [0xFF, 0xF3, 0x40],
    [0xFF, 0xE3, 0x00],
    [0xFF, 0xFF, 0xEF],
];

/// Headers seen in the wild.
pub const REALWORLD: [[u8; 3]; 6] = [
    [0xFF, 0xFB, 0x90], // MPEG-1 Layer III, 128 kbps, 44.1 kHz
    [0xFF, 0xFD, 0x40], // MPEG-1 Layer II
    [0xFF, 0xFF, 0x10], // MPEG-1 Layer I
    [0xFF, 0xF3, 0x40], // MPEG-2 Layer III
    [0xFF, 0xE3, 0x40], // MPEG-2.5 Layer III
    [0xFF, 0xFB, 0x00], // free format
];

// ---------------------------------------------------------------------------
// Guarded (page-protected) buffers: prove nothing is read out of bounds
// ---------------------------------------------------------------------------

/// A mapping of `2 * page` bytes whose second page is `PROT_NONE`. `readable(n)` returns a
/// pointer such that exactly `n` bytes are readable starting there.
pub struct GuardedBuf {
    base: *mut u8,
    page: usize,
}

impl GuardedBuf {
    pub fn new() -> Self {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page * 2,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(base != libc::MAP_FAILED, "mmap failed");
        let base = base as *mut u8;
        let rc = unsafe { libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE) };
        assert_eq!(rc, 0, "mprotect failed");
        GuardedBuf { base, page }
    }

    /// Pointer with exactly `n` readable bytes before the guard page.
    pub fn tail(&self, n: usize) -> *mut u8 {
        assert!(n <= self.page);
        unsafe { self.base.add(self.page - n) }
    }

    /// Writes `data` so that its last byte is the last readable byte.
    pub fn put_tail(&self, data: &[u8]) -> *const u8 {
        let p = self.tail(data.len());
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len()) };
        p as *const u8
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut libc::c_void, self.page * 2);
        }
    }
}

// ---------------------------------------------------------------------------
// Fork-based crash comparison (for the null / truncated-buffer error rows)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// Returned normally with this value.
    Returned(c_int),
    /// Died from this signal.
    Signal(i32),
    /// Exited with an unexpected status.
    Exited(i32),
}

/// Runs `f` in a forked child and reports how the child terminated. `f` must be
/// crash-prone-safe: the child does nothing but call `f` and `_exit`.
///
/// Return value is smuggled out through the exit status: `_exit(200 + result)` for
/// results in `0..=54`, which covers the C boolean domain.
pub fn probe<F: FnOnce() -> c_int>(f: F) -> Outcome {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child. Make sure a fault is a plain fatal signal.
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = libc::SIG_DFL;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(libc::SIGSEGV, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGBUS, &sa, std::ptr::null_mut());

            let r = f();
            let code = if (0..=54).contains(&r) {
                200 + r
            } else {
                255
            };
            libc::_exit(code as libc::c_int);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else if libc::WIFEXITED(status) {
            let code = libc::WEXITSTATUS(status);
            if (200..=254).contains(&code) {
                Outcome::Returned(code - 200)
            } else {
                Outcome::Exited(code)
            }
        } else {
            Outcome::Exited(-1)
        }
    }
}

/// Asserts the C and the Rust `.so` behave identically (same return value, or death by the
/// same signal) for a raw-pointer call that may fault.
pub fn assert_same_outcome(label: &str, h1: *const u8, h2: *const u8) -> Outcome {
    let l = libs();
    let (c_fn, rs_fn) = (l.c, l.rs);
    let a = probe(move || unsafe { c_fn(h1, h2) });
    let b = probe(move || unsafe { rs_fn(h1, h2) });
    assert_eq!(a, b, "DIVERGENCE [{label}]: C = {a:?}, Rust = {b:?}");
    a
}
