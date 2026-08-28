//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes their exported symbols behind identical wrappers.
//!
//! Nothing in here ever calls a Rust function directly — every call to the
//! translation goes through `dlsym` on `libsiphash_lib.so`, exactly as an
//! external C consumer would, so the `#[no_mangle] extern "C"` wrappers are
//! themselves under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type HashBytesFn = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
pub type SipHashFn = unsafe extern "C" fn(c_int);

pub struct Lib {
    pub which: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub hash_bytes: HashBytesFn,
    pub siphash: SipHashFn,
}

impl Lib {
    fn open(which: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("dlopen {} ({which}): {e}", path.display()))
        };
        // Resolve by exact exported name -- this is the symbol-parity check
        // executed at runtime for every test.
        let hash_bytes: HashBytesFn = unsafe {
            let s: Symbol<HashBytesFn> = lib
                .get(b"stbds_hash_bytes\0")
                .unwrap_or_else(|e| panic!("dlsym stbds_hash_bytes in {which}: {e}"));
            *s
        };
        let siphash: SipHashFn = unsafe {
            let s: Symbol<SipHashFn> = lib
                .get(b"siphash\0")
                .unwrap_or_else(|e| panic!("dlsym siphash in {which}: {e}"));
            *s
        };
        Lib { which, path, _lib: lib, hash_bytes, siphash }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object built by `c_src/CMakeLists.txt`. The library
/// name is derived from the parent directory name by the CMake script, so we
/// glob rather than hard-code it.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    collect_so(&build, &mut found, 0);
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so under {}. Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_so(&p, out, depth + 1);
        } else if p.extension().and_then(|s| s.to_str()) == Some("so") {
            out.push(p);
        }
    }
}

/// Locate the Rust cdylib for the profile the test harness itself was built
/// with, so `cargo test` and `cargo test --release` each pick up the matching
/// artifact.
///
/// IMPORTANT: `cargo test` does **not** build `crate-type = ["cdylib"]`
/// artifacts -- the integration tests never `use` the crate, so cargo has no
/// reason to link it. That means the `.so` on disk can easily be STALE, and a
/// stale `.so` makes every differential test silently vacuous (it would happily
/// compare the C against an old, correct build while the current source is
/// broken).
///
/// So this function is deliberately strict:
///   * NO cross-profile fallback -- loading the release `.so` while running the
///     debug harness (or vice versa) hides exactly this bug.
///   * The `.so` must be at least as new as every crate source file, or we
///     panic with instructions rather than reporting a green run.
///
/// Use `./run_all_tests.sh` (or `cargo build` before `cargo test`) so the
/// artifact is always current.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let p = manifest_dir().join("target").join(profile).join("libsiphash_lib.so");

    if !p.exists() {
        panic!(
            "{} does not exist.\n\n\
             `cargo test` does NOT build cdylib artifacts. Build it first:\n\
             \n    cargo build{}\n\n\
             or just run ./run_all_tests.sh",
            p.display(),
            if profile == "release" { " --release" } else { "" }
        );
    }

    assert_so_fresh(&p);
    p
}

/// Panic if the cdylib is older than any crate source file, so a stale artifact
/// can never masquerade as a passing differential run.
fn assert_so_fresh(so: &Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut srcs: Vec<PathBuf> = Vec::new();
    collect_sources(&manifest_dir().join("src"), &mut srcs);
    srcs.push(manifest_dir().join("Cargo.toml"));

    for s in srcs {
        if let Ok(t) = std::fs::metadata(&s).and_then(|m| m.modified()) {
            if newest.as_ref().map(|(nt, _)| t > *nt).unwrap_or(true) {
                newest = Some((t, s));
            }
        }
    }

    if let Some((t, path)) = newest {
        assert!(
            so_mtime >= t,
            "STALE ARTIFACT: {} is older than {}.\n\n\
             `cargo test` does not rebuild cdylibs, so this test run would be \
             comparing the C library against an out-of-date Rust build and \
             could report a false pass. Rebuild first:\n\n    cargo build{}\n\n\
             or just run ./run_all_tests.sh",
            so.display(),
            path.display(),
            if cfg!(debug_assertions) { "" } else { " --release" }
        );
    }
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_sources(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(p);
        }
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open("C", find_c_so()))
}

pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::open("RUST", find_rust_so()))
}

// Both libraries are stateless pure code paths for `stbds_hash_bytes`, but
// `siphash` writes to the *process-wide* libc stdout, so stdout-capturing
// tests must not run concurrently.
pub static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

/// Call `stbds_hash_bytes` in both libraries and assert byte-identical results.
#[track_caller]
pub fn diff_hash(buf: &[u8], len: usize, seed: usize, ctx: &str) -> usize {
    // `void *p` is non-const in C; hand both implementations the same bytes.
    let mut a = buf.to_vec();
    let mut b = buf.to_vec();
    let cv = unsafe { (c_lib().hash_bytes)(a.as_mut_ptr() as *mut c_void, len, seed) };
    let rv = unsafe { (rust_lib().hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed) };
    assert_eq!(
        cv, rv,
        "stbds_hash_bytes divergence [{ctx}]\n  len={len} seed={seed:#018x}\n  \
         C   = {cv:#018x}\n  RUST= {rv:#018x}\n  bytes={:02x?}",
        &buf[..len.min(buf.len()).min(80)]
    );
    // The input buffer must not be mutated by either implementation.
    assert_eq!(a, b, "input buffer mutated differently [{ctx}]");
    assert_eq!(&a[..], buf, "C mutated its input buffer [{ctx}]");
    cv
}

/// Call `stbds_hash_bytes` with a raw pointer (for null / garbage-pointer
/// cases) in both libraries and assert identical results.
#[track_caller]
pub fn diff_hash_raw(p: *mut c_void, len: usize, seed: usize, ctx: &str) -> usize {
    let cv = unsafe { (c_lib().hash_bytes)(p, len, seed) };
    let rv = unsafe { (rust_lib().hash_bytes)(p, len, seed) };
    assert_eq!(
        cv, rv,
        "stbds_hash_bytes divergence [{ctx}]\n  p={p:?} len={len} seed={seed:#018x}\n  \
         C   = {cv:#018x}\n  RUST= {rv:#018x}"
    );
    cv
}

// ---------------------------------------------------------------------------
// stdout capture (for the `siphash` printf differential)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Redirect the process's fd 1 into a temp file, run `f`, flush libc's stdout,
/// and return the captured bytes.
///
/// Both `.so`s print through the *same* libc `stdout` FILE, so flushing
/// (`fflush(NULL)` flushes all streams) before reading is what makes the
/// comparison exact rather than buffering-dependent.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};

    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "siphash_capture_{}_{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    // Flush BOTH buffering layers that sit above fd 1 before we hijack it,
    // otherwise pending libtest progress output ("test foo ... ") can land in
    // our capture file and be mistaken for library output.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = as_raw_fd(&file);
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");
        saved
    };

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

fn as_raw_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

/// Capture `siphash(init)` from the C `.so` and from the Rust `.so` and assert
/// the emitted bytes are identical.
#[track_caller]
pub fn diff_siphash_stdout(init: c_int) {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let c_out = capture_stdout(|| unsafe { (c_lib().siphash)(init) });
    let r_out = capture_stdout(|| unsafe { (rust_lib().siphash)(init) });

    if c_out != r_out {
        let c_s = String::from_utf8_lossy(&c_out);
        let r_s = String::from_utf8_lossy(&r_out);
        let first_diff = c_s
            .lines()
            .zip(r_s.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("line {i}:\n    C   = {a:?}\n    RUST= {b:?}"))
            .unwrap_or_else(|| {
                format!("length differs: C={} bytes RUST={} bytes", c_out.len(), r_out.len())
            });
        panic!("siphash({init}) stdout divergence\n  {first_diff}");
    }
    assert!(!c_out.is_empty(), "siphash({init}) produced no output at all");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed, reproducible property tests
// ---------------------------------------------------------------------------

pub const PRNG_SEED: u64 = 0x5150_5CA1_AB1E_D00D;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_usize(&mut self) -> usize {
        self.next_u64() as usize
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() >> 1) as usize % n }
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
    /// A seed drawn from the interesting edges as well as the bulk of the range.
    pub fn seed_value(&mut self) -> usize {
        match self.below(8) {
            0 => 0,
            1 => usize::MAX,
            2 => 1,
            3 => usize::MAX - 1,
            4 => usize::MAX / 2,
            5 => 1usize << 63,
            _ => self.next_usize(),
        }
    }
}
