//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! exclusively through their exported `extern "C"` symbols. Rust functions are
//! NEVER called directly, so the `#[unsafe(no_mangle)]` export wrappers are
//! part of what is under test.
//!
//! Observables compared for every call: the `int` return value AND the exact
//! stdout byte stream. stdout is captured by `dup2`-redirecting fd 1 around the
//! call and `fflush(NULL)`-ing; both `.so`s import `printf` from the same glibc
//! and share the one `stdout` FILE, so buffering/formatting is identical by
//! construction rather than by coincidence.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------- libc bits

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// Used to mint genuine libc-`malloc` pointers to hand to
    /// `cleanup_resources`, which frees them with libc `free`.
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
}

/// `malloc` a buffer of `n` bytes through libc (same allocator both `.so`s use).
pub fn libc_malloc(n: usize) -> *mut c_char {
    let p = unsafe { malloc(n) } as *mut c_char;
    assert!(!p.is_null(), "libc malloc({n}) failed");
    p
}

pub fn libc_free(p: *mut c_char) {
    unsafe { free(p as *mut c_void) };
}

/// Checks the precondition the `free`-observation probes rely on: that in THIS
/// process, for THIS size class, `free(p)` followed by `malloc(same_size)` hands
/// back the very same address (glibc tcache LIFO reuse).
///
/// This is a heuristic about the allocator, not a guarantee: it holds in the
/// debug test binary but not always under `--release`, where the surrounding
/// allocation pattern differs. Measuring it directly lets the leak-detection rows
/// report "inconclusive" instead of producing a bogus pass or a bogus failure.
pub fn tcache_probe_usable(size: usize) -> bool {
    for _ in 0..4 {
        let w = libc_malloc(size);
        libc_free(w);
    }
    let a = libc_malloc(size);
    libc_free(a);
    let b = libc_malloc(size);
    let ok = std::ptr::eq(a, b);
    libc_free(b);
    ok
}

/// Observes whether `cleanup_resources` ACTUALLY called `free` on the pointer.
///
/// stdout comparison cannot see a leak: a `cleanup_resources` that silently
/// skips `free` prints nothing and returns nothing, so it is byte-identical to a
/// correct one. This probe closes that blind spot by exploiting glibc's tcache,
/// which is strictly LIFO per size class: if the pointer was freed, the very
/// next same-size `malloc` pops it straight back and returns the SAME address.
///
/// Returns `true` if the address was recycled, i.e. the pointer really was freed.
pub fn probe_frees(lib: &Lib, size: usize) -> bool {
    // Settle the tcache bin for this size class so the probe is not measuring
    // first-touch arena growth.
    for _ in 0..4 {
        let w = libc_malloc(size);
        libc_free(w);
    }

    let p1 = libc_malloc(size);
    unsafe { (lib.cleanup_resources)(p1) };
    let p2 = libc_malloc(size);
    let recycled = std::ptr::eq(p1, p2);
    libc_free(p2);
    // p1 is deliberately NOT freed here. "Address not recycled" does NOT imply
    // "pointer was not freed" — the allocator may hand back a different chunk
    // for other reasons — so freeing p1 on that path is a genuine double free
    // and aborts the process (glibc: "free(): double free detected in tcache").
    // Leaking `size` bytes in the test process is the safe choice.
    recycled
}

// ------------------------------------------------------------ loaded library

pub type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
pub type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

pub struct Lib {
    pub name: &'static str,
    pub cleanup: CleanupFn,
    pub print_result: PrintResultFn,
    pub cleanup_resources: CleanupResourcesFn,
    _lib: Library,
}

// The three symbols are plain reentrant C functions; sharing them across the
// harness is sound and the capture lock serialises all actual calls.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    /// True if `name` (must be NUL-terminated) resolves via `dlsym` here.
    /// Used by Phase D to prove `nm`-visible symbols are really lookup-able.
    pub unsafe fn raw_symbol(&self, name: &[u8]) -> bool {
        unsafe { self._lib.get::<*mut c_void>(name).is_ok() }
    }
}

fn load(name: &'static str, path: &PathBuf) -> Lib {
    unsafe {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        let cleanup = {
            let s: Symbol<CleanupFn> = lib
                .get(b"cleanup\0")
                .unwrap_or_else(|e| panic!("{name}: symbol `cleanup` missing: {e}"));
            *s
        };
        let print_result = {
            let s: Symbol<PrintResultFn> = lib
                .get(b"print_result\0")
                .unwrap_or_else(|e| panic!("{name}: symbol `print_result` missing: {e}"));
            *s
        };
        let cleanup_resources = {
            let s: Symbol<CleanupResourcesFn> = lib
                .get(b"cleanup_resources\0")
                .unwrap_or_else(|e| panic!("{name}: symbol `cleanup_resources` missing: {e}"));
            *s
        };
        Lib { name, cleanup, print_result, cleanup_resources, _lib: lib }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe is <target>/<profile>/deps/<testbin>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for cand in [deps.join("libcleanup_lib.so"), profile.join("libcleanup_lib.so")] {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libcleanup_lib.so not found near {} — run `cargo build` first",
        deps.display()
    );
}

/// Guards against the failure mode that silently invalidated an entire earlier
/// test run: `cargo test` running against a STALE `libcleanup_lib.so`.
///
/// With `crate-type = ["cdylib"]` alone, nothing in `tests/` depends on the lib
/// target, so cargo never rebuilds the `.so` and every differential assertion
/// compares the C library against a months-old artifact — mutations to
/// `src/lib.rs` go completely undetected. `crate-type` now includes `"lib"` to
/// fix the cause; this check fails loudly if it ever regresses.
fn assert_so_is_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(_, n)| t > *n) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }

    if let Some((src, src_mtime)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             The differential tests would be comparing the C library against an \
             out-of-date Rust build, so they could not detect ANY divergence.\n\
             Rebuild with `cargo build --no-default-features` (and keep \
             crate-type = [\"lib\", \"cdylib\"] so cargo does this automatically).",
            so.display(),
            src.display()
        );
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| load("C", &c_so_path()))
}

pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| {
        let p = rust_so_path();
        assert_so_is_fresh(&p);
        load("Rust", &p)
    })
}

// -------------------------------------------------------------- stdout capture

/// fd 1 is process-global, so every capture must be serialised even though
/// `cargo test` runs test fns on parallel threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

static CAP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Restores fd 1 even if the captured closure panics — otherwise a single failed
/// assertion inside a capture would leave fd 1 pointing at a deleted temp file
/// and silently corrupt every later capture.
struct FdRestore {
    saved: c_int,
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        unsafe {
            // Push the library's buffered output out while fd 1 is still the
            // capture file.
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

/// Run `f`, returning its value plus everything it wrote to fd 1.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = capture_lock();

    // Flush anything already pending so it is not attributed to this capture.
    unsafe { fflush(std::ptr::null_mut()) };

    let path = std::env::temp_dir().join(format!(
        "cleanup_cap_{}_{}.bin",
        std::process::id(),
        CAP_SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let file = File::create(&path).expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    let r = {
        let _restore = FdRestore { saved };
        f()
        // `_restore` drops here: fflush + restore fd 1.
    };

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (r, bytes)
}

// ------------------------------------------------------------- row driver

/// Runs every `CONFIGS.md` / `ERRORS.md` row sequentially inside ONE `#[test]`.
///
/// This is deliberate: `capture()` redirects the process-global fd 1, so no
/// other thread may write to stdout during a capture window. `libtest` itself
/// writes its `test NAME ... ok` progress lines straight to fd 1, so running
/// rows as separate parallel `#[test]`s lets that progress text land inside a
/// capture and produce bogus mismatches. One test per binary removes the race
/// without depending on the caller passing `--test-threads=1`.
///
/// Every row still runs even if an earlier one fails; all failures are reported
/// together. Progress goes to stderr, which is never redirected.
pub fn run_rows(suite: &str, rows: &[(&str, fn())]) {
    let mut failures: Vec<String> = Vec::new();

    eprintln!("\n=== {suite}: {} rows ===", rows.len());
    for (name, f) in rows {
        eprint!("  {name:<52} ... ");
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match outcome {
            Ok(()) => eprintln!("ok"),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    (*s).to_string()
                } else {
                    "<non-string panic payload>".to_string()
                };
                eprintln!("FAILED");
                failures.push(format!("{name}: {msg}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{suite}: {}/{} row(s) FAILED:\n\n{}",
            failures.len(),
            rows.len(),
            failures.join("\n\n")
        );
    }
    eprintln!("=== {suite}: all {} rows passed ===", rows.len());
}

/// Observes whether `cleanup` frees the 50-byte buffer it allocates internally.
///
/// Same blind spot as `probe_frees`, one level up: if `cleanup` leaked its
/// `malloc(50)` the stdout and the return value would be completely unchanged.
///
/// Method: prime glibc's 50-byte tcache bin so its head is a known address `p0`,
/// run `cleanup` (which pops `p0` for its own buffer and must push it back when
/// it frees), then allocate again — a balanced `cleanup` hands `p0` straight back.
/// No Rust-side heap allocation happens between the two probe allocations, since
/// that would perturb the same size class.
pub fn probe_cleanup_balanced(lib: &Lib, args: [i32; 4]) -> bool {
    let _guard = capture_lock();

    // Allocate everything the redirection needs BEFORE the measurement window.
    let path = std::env::temp_dir().join(format!(
        "cleanup_probe_{}_{}.bin",
        std::process::id(),
        CAP_SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    let sink = File::create(&path).expect("create probe sink");

    unsafe { fflush(std::ptr::null_mut()) };
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(sink.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let balanced = {
        let _restore = FdRestore { saved };

        // Warm-up call: establishes the stdio buffer and arena so the measured
        // call allocates nothing but its own 50-byte buffer.
        unsafe { (lib.cleanup)(args[0], args[1], args[2], args[3]) };
        for _ in 0..4 {
            let w = libc_malloc(50);
            libc_free(w);
        }

        let p0 = libc_malloc(50);
        libc_free(p0);
        unsafe { (lib.cleanup)(args[0], args[1], args[2], args[3]) };
        let p1 = libc_malloc(50);
        let ok = std::ptr::eq(p0, p1);
        libc_free(p1);
        ok
    };

    drop(sink);
    let _ = std::fs::remove_file(&path);
    balanced
}

// ------------------------------------------------------------------ reporting

pub fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            b'\r' => s.push_str("\\r"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s
}

fn truncated(b: &[u8], max: usize) -> String {
    if b.len() <= max {
        show(b)
    } else {
        format!("{}... [{} bytes total]", show(&b[..max]), b.len())
    }
}

// ------------------------------------------------------- differential drivers

/// Run `body` for every index `0..n` against the C `.so` (one capture), then
/// against the Rust `.so` (one capture), and require both the per-index return
/// values and the whole stdout byte stream to match.
///
/// `body` MUST be deterministic in `i` (derive inputs from `mix()`), because it
/// is replayed once per library.
///
/// Batching makes the composed multi-call stream part of what is compared. On
/// any mismatch the failing index is localised by replaying indices one at a
/// time.
pub fn diff_batch<F>(row: &str, n: usize, mut body: F)
where
    F: FnMut(&Lib, usize) -> i64,
{
    let (c_rets, c_out) = capture(|| (0..n).map(|i| body(c_lib(), i)).collect::<Vec<_>>());
    let (r_rets, r_out) = capture(|| (0..n).map(|i| body(rust_lib(), i)).collect::<Vec<_>>());

    if c_rets == r_rets && c_out == r_out {
        return;
    }

    // Localise: replay each index separately to find the first divergence.
    for i in 0..n {
        let (cr, co) = capture(|| body(c_lib(), i));
        let (rr, ro) = capture(|| body(rust_lib(), i));
        assert!(
            cr == rr && co == ro,
            "[{row}] DIVERGENCE at index {i}\n  C    ret={cr} stdout=\"{}\"\n  Rust ret={rr} stdout=\"{}\"",
            truncated(&co, 400),
            truncated(&ro, 400),
        );
    }

    // Per-index replay agreed, so the difference is in the composed stream.
    panic!(
        "[{row}] batched streams diverge though every index matches in isolation\n  \
         C rets == Rust rets: {}\n  C stdout ({} bytes): \"{}\"\n  Rust stdout ({} bytes): \"{}\"",
        c_rets == r_rets,
        c_out.len(),
        truncated(&c_out, 800),
        r_out.len(),
        truncated(&r_out, 800),
    );
}

/// Single-shot differential call with a caller-supplied description.
pub fn diff_once<F>(row: &str, desc: &str, mut body: F)
where
    F: FnMut(&Lib) -> i64,
{
    let (cr, co) = capture(|| body(c_lib()));
    let (rr, ro) = capture(|| body(rust_lib()));
    assert_eq!(
        cr,
        rr,
        "[{row}] {desc}: return value differs (C={cr}, Rust={rr})\n  C stdout=\"{}\"\n  Rust stdout=\"{}\"",
        truncated(&co, 400),
        truncated(&ro, 400)
    );
    assert!(
        co == ro,
        "[{row}] {desc}: stdout differs\n  C    ({} bytes): \"{}\"\n  Rust ({} bytes): \"{}\"",
        co.len(),
        truncated(&co, 800),
        ro.len(),
        truncated(&ro, 800)
    );
}

/// Alternate C / Rust call ordering inside a single capture pair, to surface any
/// cross-library state or allocator interference.
pub fn diff_interleaved<F>(row: &str, n: usize, mut body: F)
where
    F: FnMut(&Lib, usize) -> i64,
{
    for i in 0..n {
        let (a, ao, b, bo) = if i % 2 == 0 {
            let (a, ao) = capture(|| body(c_lib(), i));
            let (b, bo) = capture(|| body(rust_lib(), i));
            (a, ao, b, bo)
        } else {
            // Rust first this time.
            let (b, bo) = capture(|| body(rust_lib(), i));
            let (a, ao) = capture(|| body(c_lib(), i));
            (a, ao, b, bo)
        };
        assert!(
            a == b && ao == bo,
            "[{row}] interleaved divergence at i={i} (rust_first={})\n  C    ret={a} stdout=\"{}\"\n  Rust ret={b} stdout=\"{}\"",
            i % 2 == 1,
            truncated(&ao, 400),
            truncated(&bo, 400)
        );
    }
}

// ---------------------------------------------------------------------- PRNG

/// splitmix64 — deterministic, index-addressable so `body(i)` stays pure.
pub fn mix(seed: u64, i: u64) -> u64 {
    let mut z = seed
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_mul(i.wrapping_add(0xD1B5_4A32_D192_ED03) | 1);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Full-width random `i32`.
pub fn rnd_i32(seed: u64, i: u64, k: u64) -> i32 {
    (mix(seed ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15), i) >> 17) as u32 as i32
}

/// Random `i32` in `lo..=hi` (inclusive).
pub fn rnd_range(seed: u64, i: u64, k: u64, lo: i32, hi: i32) -> i32 {
    debug_assert!(lo <= hi);
    let span = (hi as i64 - lo as i64 + 1) as u64;
    let r = mix(seed ^ k.wrapping_mul(0xC2B2_AE3D_27D4_EB4F), i);
    (lo as i64 + (r % span) as i64) as i32
}

/// Random `usize` in `lo..=hi`.
pub fn rnd_len(seed: u64, i: u64, k: u64, lo: usize, hi: usize) -> usize {
    let span = (hi - lo + 1) as u64;
    lo + (mix(seed ^ k.wrapping_mul(0x1656_67B1_9E37_79F9), i) % span) as usize
}

// ------------------------------------------------------------- label helpers

/// NUL-terminated C string buffer from raw bytes (bytes are passed through
/// verbatim — no UTF-8 validation, so non-UTF-8 labels stay intact).
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// A deterministic pseudo-random label of random length with arbitrary
/// (possibly non-UTF-8, never NUL) bytes.
pub fn rnd_label(seed: u64, i: u64, max_len: usize) -> Vec<u8> {
    let n = rnd_len(seed, i, 900, 0, max_len);
    (0..n)
        .map(|j| {
            let b = (mix(seed ^ (j as u64).wrapping_mul(0x9E37_79B9), i) >> 13) as u8;
            if b == 0 { 1 } else { b } // keep interior NULs out of this generator
        })
        .collect()
}
