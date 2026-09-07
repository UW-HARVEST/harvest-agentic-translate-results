//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` (built by `c_src/CMakeLists.txt`) and the Rust
//! `libdriver.so` (this crate's `cdylib`) are loaded with `libloading` and
//! called only through their exported `searchAndReplace` symbol, exactly as an
//! external C consumer would. No Rust function of the crate is ever called
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type SarFn = unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut c_char;

pub struct Libs {
    // Kept alive for the whole process so the raw function pointers stay valid.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: SarFn,
    pub rust: SarFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// The raw `extern "C"` pointers are plain code addresses; the library handles
// live for the lifetime of the process.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("crate has a parent dir");
    let p = root.join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // target/<profile>/deps/<test-bin> -> target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");
    let p = profile_dir.join("libdriver.so");
    if p.exists() {
        return p;
    }
    // `cargo test` alone does not build the `cdylib`, so fall back to a sibling
    // profile directory -- but say so loudly, because that means the tests are
    // exercising a differently-compiled artifact than the current profile.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for prof in ["release", "debug"] {
        let q = manifest.join("target").join(prof).join("libdriver.so");
        if q.exists() {
            eprintln!(
                "WARNING: {p:?} does not exist (run `cargo build` for this profile, \
                 or set RUST_DRIVER_SO); falling back to {q:?}"
            );
            return q;
        }
    }
    panic!("Rust cdylib libdriver.so not found (looked at {p:?} and target/{{release,debug}})");
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(&rust_path).expect("dlopen Rust libdriver.so");
            let c_sym: Symbol<SarFn> = c_lib
                .get(b"searchAndReplace\0")
                .expect("C libdriver.so exports searchAndReplace");
            let rust_sym: Symbol<SarFn> = rust_lib
                .get(b"searchAndReplace\0")
                .expect("Rust libdriver.so exports searchAndReplace");
            let c = *c_sym;
            let rust = *rust_sym;
            let libs = Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            };
            // Warm up: bind every PLT entry / lazy relocation both libraries need
            // on the happy path, so later forked children (which run under a
            // tight RLIMIT_AS) do not fail inside the dynamic loader instead of
            // inside the function under test.
            let (o, s, v) = (
                CString::new("warm-up").unwrap(),
                CString::new("m").unwrap(),
                CString::new("MM").unwrap(),
            );
            for f in [libs.c, libs.rust] {
                let p = f(o.as_ptr(), s.as_ptr(), v.as_ptr());
                assert!(!p.is_null());
                libc::free(p as *mut libc::c_void);
                // no-match path (strdup)
                let miss = CString::new("zzz").unwrap();
                let p = f(o.as_ptr(), miss.as_ptr(), v.as_ptr());
                assert!(!p.is_null());
                libc::free(p as *mut libc::c_void);
            }
            libs
        }
    })
}

/// One invocation's observable result: either `NULL` or the returned bytes
/// (without the NUL terminator, which is asserted to be present by `CStr`).
#[derive(PartialEq, Eq)]
pub enum Ret {
    Null,
    Str(Vec<u8>),
}

impl std::fmt::Debug for Ret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ret::Null => write!(f, "NULL"),
            Ret::Str(b) => write!(f, "{:?}", String::from_utf8_lossy(b)),
        }
    }
}

unsafe fn call(f: SarFn, orig: &CStr, search: &CStr, value: &CStr) -> Ret {
    let p = f(orig.as_ptr(), search.as_ptr(), value.as_ptr());
    if p.is_null() {
        return Ret::Null;
    }
    let bytes = CStr::from_ptr(p).to_bytes().to_vec();
    libc::free(p as *mut libc::c_void);
    Ret::Str(bytes)
}

/// Run one differential case: call the C export and the Rust export with the
/// same three C strings and assert the results are byte-identical.
pub fn diff(row: &str, orig: &[u8], search: &[u8], value: &[u8]) -> Ret {
    let o = CString::new(orig).expect("orig contains an interior NUL");
    let s = CString::new(search).expect("search contains an interior NUL");
    let v = CString::new(value).expect("value contains an interior NUL");
    let (rc, rr) = unsafe { (call(libs().c, &o, &s, &v), call(libs().rust, &o, &s, &v)) };
    assert!(
        rc == rr,
        "[{row}] DIVERGENCE\n  orig   = {:?}\n  search = {:?}\n  value  = {:?}\n  C    -> {:?}\n  Rust -> {:?}",
        String::from_utf8_lossy(orig),
        String::from_utf8_lossy(search),
        String::from_utf8_lossy(value),
        rc,
        rr
    );
    rc
}

/// Deterministic xorshift64* PRNG so every "randomized" row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A byte string of exactly `len` bytes drawn from `alphabet`
    /// (`alphabet` must not contain NUL, so the result never does either).
    pub fn bytes(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len())])
            .collect()
    }

    /// `bytes` with a randomized length in `lo..=hi`.
    pub fn bytes_range(&mut self, lo: usize, hi: usize, alphabet: &[u8]) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n, alphabet)
    }
}

pub const ALPHA_TINY: &[u8] = b"ab";
pub const ALPHA_SMALL: &[u8] = b"abcX";
pub const ALPHA_ASCII: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCXYZ0189 _-";

/// Bytes 0x80..=0xFF plus a couple of ASCII anchors: exercises the fact that
/// the C works on raw bytes, not UTF-8.
pub fn alpha_high() -> Vec<u8> {
    let mut v: Vec<u8> = (0x80u8..=0xFFu8).collect();
    v.extend_from_slice(b"aX");
    v
}

// ---------------------------------------------------------------------------
// Child-process helpers (Phase C: allocation failure, NULL derefs, hangs)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
    /// Still running when the deadline expired (killed by the harness).
    Timeout,
}

/// Current virtual size of this process, in bytes (`/proc/self/statm` field 1
/// is the VM size in pages).
pub fn vsize_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = s
        .split_whitespace()
        .next()
        .expect("statm field 0")
        .parse()
        .expect("statm field 0 is a number");
    pages * 4096
}

/// Fork, optionally cap the child's address space, run `f` and return how the
/// child terminated. `f` must only use async-signal-safe operations and must
/// finish by returning the exit code (0..=255).
pub fn run_in_child<F>(rlimit_as: Option<u64>, timeout: std::time::Duration, f: F) -> Outcome
where
    F: FnOnce() -> i32,
{
    // Flush so the child does not duplicate buffered output.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        if let Some(limit) = rlimit_as {
            let rl = libc::rlimit {
                rlim_cur: limit as libc::rlim_t,
                rlim_max: limit as libc::rlim_t,
            };
            unsafe { libc::setrlimit(libc::RLIMIT_AS, &rl) };
        }
        let code = f();
        unsafe { libc::_exit(code) };
    }

    // ---- parent ----
    let deadline = std::time::Instant::now() + timeout;
    let mut status: libc::c_int = 0;
    loop {
        let r = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if r == pid {
            return decode(status);
        }
        if std::time::Instant::now() >= deadline {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                libc::waitpid(pid, &mut status, 0);
            }
            return Outcome::Timeout;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

fn decode(status: libc::c_int) -> Outcome {
    if libc::WIFEXITED(status) {
        Outcome::Exited(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else {
        Outcome::Exited(-1)
    }
}

/// Exit code convention for the allocation-failure children:
/// `0` = returned non-NULL, `1` = returned NULL.
pub const CHILD_NONNULL: i32 = 0;
pub const CHILD_NULL: i32 = 1;

/// Run the same call in two separate children (one per implementation) under an
/// identical `RLIMIT_AS`, and assert both report the same outcome.
///
/// `build` produces the three NUL-terminated inputs; it runs in the parent
/// *before* the limit is applied so the inputs themselves always fit.
pub fn diff_under_rlimit(
    row: &str,
    slack_bytes: u64,
    orig: &CStr,
    search: &CStr,
    value: &CStr,
) -> Outcome {
    let limit = vsize_bytes() + slack_bytes;
    let l = libs();
    let (op, sp, vp) = (orig.as_ptr(), search.as_ptr(), value.as_ptr());

    let run = |f: SarFn| {
        run_in_child(
            Some(limit),
            std::time::Duration::from_secs(30),
            move || unsafe {
                let p = f(op, sp, vp);
                if p.is_null() {
                    CHILD_NULL
                } else {
                    CHILD_NONNULL
                }
            },
        )
    };

    let oc = run(l.c);
    let or_ = run(l.rust);
    assert_eq!(
        oc, or_,
        "[{row}] DIVERGENCE under RLIMIT_AS={limit}: C -> {oc:?}, Rust -> {or_:?}"
    );
    oc
}
