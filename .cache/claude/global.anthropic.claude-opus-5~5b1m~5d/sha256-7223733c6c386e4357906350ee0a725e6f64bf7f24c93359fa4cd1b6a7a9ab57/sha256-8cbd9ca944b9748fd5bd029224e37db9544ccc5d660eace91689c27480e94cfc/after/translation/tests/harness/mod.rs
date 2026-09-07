//! Shared differential-test harness.
//!
//! Loads BOTH the C shared library (`c_src/build/libString_Slice.so`) and the
//! Rust `cdylib` (`target/{release,debug}/libString_Slice.so`) with
//! `libloading` and calls `slice` through the FFI boundary in both, so the
//! `#[no_mangle]` export wrappers are exercised exactly as an external caller
//! would exercise them.
//!
//! `slice()` reports its result on stdout via C `printf`, so the harness
//! redirects file descriptor 1 to a temporary file around each call and
//! compares the captured bytes as well as the returned `int`.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `int slice(char *mystr, int *start_ptr, int *stop_ptr)`
pub type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libString_Slice.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // An explicit override lets the verification script point the same suite at
    // the release cdylib, the debug cdylib, and each feature-combination build.
    if let Ok(p) = std::env::var("STRING_SLICE_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "STRING_SLICE_RUST_SO points at a missing file: {}",
            p.display()
        );
        return p;
    }

    // Prefer the release cdylib (that is the artifact a real consumer ships),
    // fall back to the debug one.
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libString_Slice.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it first: cargo build --release",
        base.display()
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        // SAFETY: both libraries are plain C ABI shared objects with no
        // constructors that run arbitrary user code.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

pub fn c_slice() -> Symbol<'static, SliceFn> {
    // SAFETY: the symbol has the declared C signature (see slicing.h).
    unsafe { libs().c.get(b"slice\0") }.expect("C .so exports `slice`")
}

pub fn rust_slice() -> Symbol<'static, SliceFn> {
    // SAFETY: same signature, exported via #[no_mangle] extern "C".
    unsafe { libs().rust.get(b"slice\0") }.expect("Rust .so exports `slice`")
}

// ---------------------------------------------------------------------------
// Sequential test runner (`harness = false`)
// ---------------------------------------------------------------------------
//
// These integration tests redirect file descriptor 1 — a PROCESS-GLOBAL
// resource — around each library call. libtest's default harness runs tests on
// several threads and writes its own progress lines to stdout, which would land
// inside the captured bytes. So every test target uses `harness = false` and
// this runner, which executes the cases strictly sequentially on one thread and
// reports exclusively on stderr.

/// Executes each named case in order, catching panics, and exits non-zero if any
/// case failed. Filters cases by substring when arguments are given, so
/// `cargo test --test phase_b_configs -- row18` still works.
pub fn run_all(suite: &str, cases: &[(&str, fn())]) {
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();

    let selected: Vec<&(&str, fn())> = cases
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} case(s) in {suite}", selected.len());
    let mut failed: Vec<&str> = Vec::new();
    for (name, f) in &selected {
        eprint!("test {name} ... ");
        // The runner is single-threaded, so `catch_unwind` here just keeps the
        // remaining cases running after a divergence is reported.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    let skipped = cases.len() - selected.len();
    if failed.is_empty() {
        eprintln!(
            "\n{suite} result: ok. {} passed; 0 failed; {skipped} filtered out\n",
            selected.len()
        );
    } else {
        eprintln!("\nfailures:");
        for name in &failed {
            eprintln!("    {name}");
        }
        eprintln!(
            "\n{suite} result: FAILED. {} passed; {} failed; {skipped} filtered out\n",
            selected.len() - failed.len(),
            failed.len()
        );
        std::process::exit(101);
    }
}

// ---------------------------------------------------------------------------
// stdout capture (process-global: must be serialised)
// ---------------------------------------------------------------------------

static IO_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn io_guard() -> MutexGuard<'static, ()> {
    IO_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 pointing at a fresh temporary file and
/// returns every byte written to it.
///
/// The caller must already hold [`io_guard`] — fd 1 is process-global.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let n = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "string_slice_diff_{}_{}.out",
        std::process::id(),
        n
    ));

    // Make sure nothing already buffered leaks into the capture.
    std::io::stdout().flush().ok();
    // SAFETY: fflush(NULL) flushes all open C output streams.
    unsafe { fflush(std::ptr::null_mut()) };

    let file = std::fs::File::create(&path)
        .unwrap_or_else(|e| panic!("create {}: {e}", path.display()));

    // SAFETY: plain POSIX fd juggling; `saved` is restored before returning.
    let (saved, result) = unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        let result = f();

        // Flush whatever the library buffered before we take fd 1 back.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restoring stdout failed");
        close(saved);
        (saved, result)
    };
    let _ = saved;
    drop(file);

    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);
    (result, bytes)
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// One `slice()` invocation: the subject string plus the two optional bounds.
#[derive(Clone, Debug)]
pub struct Call {
    pub s: Vec<u8>,
    pub start: Option<i32>,
    pub stop: Option<i32>,
}

impl Call {
    pub fn new(s: impl AsRef<[u8]>, start: Option<i32>, stop: Option<i32>) -> Self {
        let s = s.as_ref().to_vec();
        assert!(
            !s.contains(&0),
            "test strings must not contain interior NUL bytes"
        );
        Call { s, start, stop }
    }
}

fn invoke(f: &SliceFn, call: &Call) -> c_int {
    // Fresh, independently owned buffers per invocation so neither library can
    // observe the other's writes.
    let mut buf: Vec<u8> = call.s.clone();
    buf.push(0);
    let mut start_val: c_int = call.start.unwrap_or(0);
    let mut stop_val: c_int = call.stop.unwrap_or(0);

    let start_ptr = if call.start.is_some() {
        &mut start_val as *mut c_int
    } else {
        std::ptr::null_mut()
    };
    let stop_ptr = if call.stop.is_some() {
        &mut stop_val as *mut c_int
    } else {
        std::ptr::null_mut()
    };

    // SAFETY: `buf` is NUL-terminated and the bound pointers are either null
    // or point at live `c_int`s — exactly slicing.h's contract.
    unsafe { f(buf.as_mut_ptr() as *mut c_char, start_ptr, stop_ptr) }
}

fn describe(call: &Call) -> String {
    format!(
        "s={:?} (len {}), start={:?}, stop={:?}",
        String::from_utf8_lossy(&call.s),
        call.s.len(),
        call.start,
        call.stop
    )
}

/// Calls `slice` in both libraries with `call` and asserts the return value and
/// the stdout bytes are identical. Returns the (shared) C result.
pub fn assert_same(row: &str, call: &Call) -> (c_int, Vec<u8>) {
    let _g = io_guard();
    let cf = c_slice();
    let rf = rust_slice();

    let (c_ret, c_out) = capture_stdout(|| invoke(&cf, call));
    let (r_ret, r_out) = capture_stdout(|| invoke(&rf, call));

    assert_eq!(
        c_ret,
        r_ret,
        "[{row}] return value diverged: C={c_ret} Rust={r_ret}\n  input: {}\n  C stdout:    {:?}\n  Rust stdout: {:?}",
        describe(call),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    assert_eq!(
        c_out,
        r_out,
        "[{row}] stdout diverged\n  input: {}\n  C stdout    ({} bytes): {:?}\n  Rust stdout ({} bytes): {:?}",
        describe(call),
        c_out.len(),
        String::from_utf8_lossy(&c_out),
        r_out.len(),
        String::from_utf8_lossy(&r_out),
    );
    (c_ret, c_out)
}

/// Like [`assert_same`] but also pins down the absolute expected behaviour, so
/// a bug that happens to be present in *both* implementations cannot pass.
pub fn assert_same_and(row: &str, call: &Call, expect_ret: c_int, expect_out: &[u8]) {
    let (ret, out) = assert_same(row, call);
    assert_eq!(
        ret,
        expect_ret,
        "[{row}] both libraries returned {ret}, but the C source mandates {expect_ret}\n  input: {}",
        describe(call)
    );
    assert_eq!(
        out,
        expect_out,
        "[{row}] both libraries printed {:?}, but the C source mandates {:?}\n  input: {}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expect_out),
        describe(call)
    );
}

// ---------------------------------------------------------------------------
// Expected-output model, transcribed directly from c_src/src/slicing.c
// ---------------------------------------------------------------------------

pub const ERR_START: &[u8] = b"Error: start is off the end of the string!\n";
pub const ERR_STOP_OFF: &[u8] = b"Error: stop is off the end of the string!\n";
pub const ERR_STOP_ORDER: &[u8] = b"Error: stop must come after start!\n";

/// Reference implementation of `slice()`'s observable behaviour.
///
/// Mirrors the C control flow line for line, including the `int`-vs-`size_t`
/// comparisons (`start > len` / `stop > len` are *unsigned* comparisons, so
/// negative bounds are rejected there) and the signed `stop <= start` check.
pub fn model(call: &Call) -> (c_int, Vec<u8>) {
    let len: u64 = call.s.len() as u64;
    let start: i32;

    match call.start {
        Some(v) => {
            // C: `start > len` with start promoted to size_t.
            if (v as i64 as u64) > len {
                return (1, ERR_START.to_vec());
            }
            start = v;
        }
        None => start = 0,
    }

    let stop: i32 = match call.stop {
        Some(v) => {
            if (v as i64 as u64) > len {
                return (1, ERR_STOP_OFF.to_vec());
            }
            if v <= start {
                return (1, ERR_STOP_ORDER.to_vec());
            }
            v
        }
        None => len as i32,
    };

    // printf("%.*s\n", stop - start, mystr + start)
    let width = stop.wrapping_sub(start).max(0) as usize;
    let from = start as usize;
    let mut out = call.s[from..(from + width).min(call.s.len())].to_vec();
    out.push(b'\n');
    (0, out)
}

/// Full check: C vs Rust vs the transcribed model.
pub fn check(row: &str, call: &Call) {
    let (ret, out) = model(call);
    assert_same_and(row, call, ret, &out);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }

    pub fn bool_p(&mut self, num: u64, den: u64) -> bool {
        self.next_u64() % den < num
    }
}

/// Byte-content flavours the tests draw strings from (CONFIGS axis G).
#[derive(Copy, Clone, Debug)]
pub enum Flavor {
    /// Printable ASCII, `0x20..=0x7E`.
    Ascii,
    /// Any non-NUL byte, `0x01..=0xFF` (includes invalid UTF-8).
    AnyByte,
    /// Printable ASCII sprinkled with `%s`, `%n`, `%d` and `%%`.
    FormatSpecifiers,
    /// Printable ASCII sprinkled with `\n`, `\t`, `\r` and spaces.
    Whitespace,
}

pub fn gen_string(rng: &mut Rng, len: usize, flavor: Flavor) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 4);
    while out.len() < len {
        match flavor {
            Flavor::Ascii => out.push(0x20 + (rng.next_u64() % 95) as u8),
            Flavor::AnyByte => out.push(1 + (rng.next_u64() % 255) as u8),
            Flavor::FormatSpecifiers => {
                if rng.bool_p(1, 4) {
                    let tok: &[u8] = match rng.next_u64() % 5 {
                        0 => b"%s",
                        1 => b"%n",
                        2 => b"%d",
                        3 => b"%%",
                        _ => b"%1000d",
                    };
                    out.extend_from_slice(tok);
                } else {
                    out.push(0x20 + (rng.next_u64() % 95) as u8);
                }
            }
            Flavor::Whitespace => {
                if rng.bool_p(1, 3) {
                    out.push(*b" \t\r\n".get((rng.next_u64() % 4) as usize).unwrap());
                } else {
                    out.push(0x41 + (rng.next_u64() % 26) as u8);
                }
            }
        }
    }
    out.truncate(len);
    debug_assert_eq!(out.len(), len);
    out
}

/// Number of randomized inputs per property-style row.
pub const ITERS: usize = 200;
