//! Shared differential-test harness.
//!
//! Both the C `.so` (built from `c_src/`) and the Rust `.so` (this crate's
//! `cdylib`) are loaded with `libloading` and driven **only** through their
//! exported symbols. Nothing in `tests/` links against the Rust crate directly,
//! which is deliberate: `Cargo.toml` declares `crate-type = ["cdylib"]` only, so
//! `use driver::...` would not even compile. Every call therefore also exercises
//! the `#[no_mangle]` / `extern "C"` export wrappers.
//!
//! Two execution modes are provided, because this library's whole point is an
//! uninitialised-pointer dereference:
//!
//! * [`run_in_process`] — loads both libraries into the test process, redirects
//!   fd 1 to a scratch file around each call, and returns the exact bytes
//!   written. Used for every configuration whose outcome is specified.
//! * [`run_isolated`] — re-executes the test binary as a child, which performs
//!   one operation and exits. Used for anything that can fault, so a `SIGSEGV`
//!   is an observation rather than a lost test run. Returns the child's exit
//!   status/signal *and* its output, so the two implementations can be compared
//!   on both.
//!
//! Because both implementations are exercised by the *same* harness code at the
//! *same* stack depth, residue-dependent behaviour is compared fairly.

#![allow(dead_code)]

use std::ffi::OsString;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// libc bits used by the capture plumbing (declared here to avoid adding a
// `libc` dev-dependency).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes every open stdio stream, which is what drains the
    /// `printf` output of whichever `.so` we just called.
    fn fflush(stream: *mut core::ffi::c_void) -> i32;
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut core::ffi::c_void;
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
}

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: usize = usize::MAX;

// ---------------------------------------------------------------------------
// Exported ABI of the library under test (from c_src/include/driver.h and the
// non-static functions in c_src/src/driver.c).
// ---------------------------------------------------------------------------
pub type PrintIntPtrLineFn = unsafe extern "C" fn(*const core::ffi::c_int);
pub type GoodFn = unsafe extern "C" fn();
pub type BadFn = unsafe extern "C" fn();
pub type DriverFn = unsafe extern "C" fn(core::ffi::c_int);

/// `driver` declared with a 64-bit parameter, so a test can deliberately put
/// garbage in the upper half of `rdi` (ERRORS.md row 9 / CONFIGS.md row 17).
pub type DriverWideFn = unsafe extern "C" fn(u64);

/// The four exported entry points of one loaded library.
pub struct Api {
    _lib: libloading::Library,
    pub print_int_ptr_line: PrintIntPtrLineFn,
    pub good: GoodFn,
    pub bad: BadFn,
    pub driver: DriverFn,
    pub driver_wide: DriverWideFn,
}

impl Api {
    /// `dlopen`s `path` (RTLD_LOCAL, so the two libraries' identically named
    /// symbols cannot shadow one another) and resolves all four exports.
    pub fn load(path: &Path) -> Api {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
            let print_int_ptr_line: PrintIntPtrLineFn = *lib
                .get::<PrintIntPtrLineFn>(b"printIntPtrLine\0")
                .expect("printIntPtrLine not exported");
            let good: GoodFn = *lib.get::<GoodFn>(b"good\0").expect("good not exported");
            let bad: BadFn = *lib.get::<BadFn>(b"bad\0").expect("bad not exported");
            let driver: DriverFn = *lib.get::<DriverFn>(b"driver\0").expect("driver not exported");
            let driver_wide: DriverWideFn = *lib
                .get::<DriverWideFn>(b"driver\0")
                .expect("driver not exported");
            Api {
                _lib: lib,
                print_int_ptr_line,
                good,
                bad,
                driver,
                driver_wide,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

/// Path to the C shared library. Overridable with `DIFFTEST_C_SO`.
pub fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFFTEST_C_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib`. Overridable with `DIFFTEST_RUST_SO`.
///
/// Derived from the test executable's own location (`target/<profile>/deps/…`),
/// so it always picks the `.so` from the same profile that cargo just built for
/// this test run.
///
/// Freshness matters here and is checked rather than assumed: because the crate
/// is `crate-type = ["cdylib"]` and the tests load it with `dlopen` instead of
/// linking it, `cargo test` does **not** rebuild it. Without this guard the whole
/// suite can pass against a stale library from an earlier edit. If the `.so` is
/// missing or older than the sources, it is rebuilt and, failing that, the test
/// aborts loudly.
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DIFFTEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test-exe>");
    let so = profile_dir.join("libdriver.so");

    ensure_fresh(&so, profile_dir);
    assert!(so.exists(), "Rust shared library not found at {}", so.display());
    so
}

/// Newest modification time across everything that can change the `.so`.
fn newest_source_mtime(manifest: &Path) -> std::time::SystemTime {
    fn walk(dir: &Path, newest: &mut std::time::SystemTime) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => walk(&path, newest),
                Ok(_) => {
                    if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                        if m > *newest {
                            *newest = m;
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    walk(&manifest.join("src"), &mut newest);
    for f in ["Cargo.toml", ".cargo/config.toml"] {
        if let Ok(m) = std::fs::metadata(manifest.join(f)).and_then(|m| m.modified()) {
            if m > newest {
                newest = m;
            }
        }
    }
    newest
}

fn ensure_fresh(so: &Path, profile_dir: &Path) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let newest_src = newest_source_mtime(&manifest);
    let so_mtime = std::fs::metadata(so).and_then(|m| m.modified()).ok();

    let stale = match so_mtime {
        None => true,
        Some(t) => t < newest_src,
    };
    if !stale {
        return;
    }

    // If the crate ever grows a feature table, an auto-build here could pick a
    // different feature set than the one the tests were compiled with, which
    // would silently verify the wrong configuration. Refuse rather than guess.
    let manifest_text = std::fs::read_to_string(manifest.join("Cargo.toml")).unwrap_or_default();
    let has_features = manifest_text
        .lines()
        .any(|l| l.trim_start().starts_with("[features]"));

    let profile = profile_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug");

    assert!(
        !has_features,
        "{} is missing or stale, and this crate now declares [features], so the \
         harness will not guess which features to build with.\nBuild it \
         explicitly first, e.g.:\n  cargo build --{profile} <the same --features …>\n\
         or run ./check_features.sh, which builds every combination before testing.",
        so.display()
    );

    eprintln!(
        "note: {} is missing or older than src/; rebuilding (cargo test does not \
         rebuild a cdylib-only lib)",
        so.display()
    );
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.arg("build").current_dir(&manifest);
    if profile == "release" {
        cmd.arg("--release");
    }
    let status = cmd.status();

    let now_ok = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .map(|t| t >= newest_src)
        .unwrap_or(false);
    assert!(
        now_ok,
        "{} is missing or stale and could not be rebuilt (cargo exited {:?}).\n\
         Build it first:  cargo build --{profile}",
        so.display(),
        status.as_ref().map(|s| s.code())
    );
}

pub fn load_c() -> Api {
    Api::load(&c_so_path())
}

pub fn load_rust() -> Api {
    Api::load(&rust_so_path())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so only one capture may be in flight at a time.
/// libtest runs `#[test]`s on parallel threads by default, which without this
/// lock lets one test's `dup2` land in the middle of another's capture window.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const LOCK_EX: i32 = 2;
const LOCK_UN: i32 = 8;

unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

/// Cross-*process* companion to `CAPTURE_LOCK`: cargo may run the `configs` and
/// `errors` test binaries concurrently, and their fd-1 redirections would
/// otherwise interleave. Both binaries `flock` the same file.
fn capture_file_lock() -> std::fs::File {
    let path = std::env::temp_dir().join("difftest-driver-capture.lock");
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .expect("open capture lock file");
    assert_eq!(
        unsafe { flock(f.as_raw_fd(), LOCK_EX) },
        0,
        "flock capture lock"
    );
    f
}

/// Runs `f` with fd 1 redirected to a scratch file and returns everything the
/// callee wrote through libc `stdout`.
///
/// `&mut dyn FnMut()` rather than a generic parameter on purpose: there is
/// exactly one monomorphisation, so the C and the Rust library are called from
/// byte-identical harness code at an identical stack depth. That matters for the
/// residue-dependent rows in `CONFIGS.md`.
pub fn capture(f: &mut dyn FnMut()) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom, Write};

    // Held for the whole redirect window. `unwrap_or_else` so that a panic in an
    // earlier capture (poisoning the mutex) does not turn every later test into
    // a confusing secondary failure.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let lock_file = capture_file_lock();

    // Drain anything the Rust test harness itself has buffered, and anything
    // libc has buffered, so it cannot leak into the capture.
    std::io::stdout().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let mut tmp = tempfile();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    // Flush *before* restoring fd 1, otherwise buffered bytes land on the real
    // stdout instead of in the capture file.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    tmp.read_to_end(&mut out).expect("read capture");
    unsafe { flock(lock_file.as_raw_fd(), LOCK_UN) };
    out
}

fn tempfile() -> std::fs::File {
    let mut path = std::env::temp_dir();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("difftest-{}-{}.out", std::process::id(), n));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create scratch capture file");
    // Unlink immediately; the fd keeps it alive and nothing is left behind.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Operation specs, shared by in-process and isolated execution
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Stable call-sequence helpers
// ---------------------------------------------------------------------------
//
// `bad()` reads whatever is at `[rbp-8]`, so its output depends on the frame of
// whatever ran at the same stack depth just before it. To make the paired
// sequences reproducible, the two calls must happen from ONE frame at ONE `rsp`:
//
//  * `#[inline(never)]` keeps the helper from being folded into a caller whose
//    frame layout varies between call sites;
//  * the trailing `black_box` keeps LLVM from turning the final call into a tail
//    jump, which would move `bad`'s frame up and make it read a different slot.
//
// These helpers are called with the C's function pointers and then with the
// Rust's, so both implementations see an identical caller frame.

#[inline(never)]
fn seq_good_bad(good: GoodFn, bad: BadFn) {
    unsafe {
        good();
        bad();
    }
    std::hint::black_box(());
}

#[inline(never)]
fn seq_bad_bad(bad: BadFn) {
    unsafe {
        bad();
        bad();
    }
    std::hint::black_box(());
}

#[inline(never)]
fn seq_pipl_bad(pipl: PrintIntPtrLineFn, bad: BadFn, v: core::ffi::c_int) {
    unsafe {
        pipl(&v as *const core::ffi::c_int);
        bad();
    }
    std::hint::black_box(());
}

/// One thing to do to a loaded library. Encoded as a string so the parent can
/// hand it to a child process verbatim.
///
/// Grammar (`:`-separated):
///
/// * `pipl:<i32>`            — `printIntPtrLine(&v)`, `v` on the stack
/// * `pipl_heap:<i32>`       — same, pointer into a heap allocation
/// * `pipl_static:<i32>`     — same, pointer into a `static mut`
/// * `pipl_idx:<i32>:<n>:<i>`— pointer to element `i` of an `n`-element array
/// * `pipl_unaligned:<u32>:<off>` — pointer `off` bytes into a byte buffer
/// * `pipl_addr:<usize>`     — `printIntPtrLine(addr)` with a raw address
/// * `pipl_null`             — `printIntPtrLine(NULL)`
/// * `pipl_unmapped`         — pointer into a page that was mapped then unmapped
/// * `pipl_protnone`         — pointer into a `PROT_NONE` page
/// * `pipl_writeonly`        — pointer into a `PROT_WRITE`-only page
/// * `pipl_straddle`         — 4-byte read straddling the end of a mapping
/// * `good`                  — `good()`
/// * `bad`                   — `bad()`
/// * `bad_bad`               — `bad(); bad();`
/// * `good_bad`              — `good(); bad();`
/// * `pipl_bad:<i32>`        — `printIntPtrLine(&v); bad();`
/// * `driver:<i32>`          — `driver(v)`
/// * `driver_wide:<u64>`     — `driver` called with a 64-bit `rdi`
/// * `driver_seq:<bits>`     — one `driver(c)` per character of `bits`
///                             (`'1'` → 1, `'0'` → 0)
pub fn perform(api: &Api, spec: &str) {
    let mut it = spec.split(':');
    let op = it.next().unwrap_or("");
    let mut next_i64 = || -> i64 {
        it.next()
            .unwrap_or_else(|| panic!("spec {spec:?}: missing argument"))
            .parse::<i64>()
            .unwrap_or_else(|e| panic!("spec {spec:?}: bad integer: {e}"))
    };
    unsafe {
        match op {
            "pipl" => {
                let v = next_i64() as i32;
                (api.print_int_ptr_line)(&v as *const i32);
            }
            "pipl_heap" => {
                let v = Box::new(next_i64() as i32);
                (api.print_int_ptr_line)(&*v as *const i32);
            }
            "pipl_static" => {
                static CELL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
                CELL.store(next_i64() as i32, std::sync::atomic::Ordering::SeqCst);
                (api.print_int_ptr_line)(CELL.as_ptr() as *const i32);
            }
            "pipl_idx" => {
                let base = next_i64() as i32;
                let n = next_i64() as usize;
                let i = next_i64() as usize;
                let arr: Vec<i32> = (0..n).map(|k| base.wrapping_add(k as i32 * 7)).collect();
                (api.print_int_ptr_line)(arr.as_ptr().add(i));
            }
            "pipl_unaligned" => {
                let word = next_i64() as u32;
                let off = next_i64() as usize;
                // 8 bytes so that a 4-byte read at any offset in 0..=3 is
                // entirely inside the buffer.
                let mut buf = [0u8; 8];
                buf[..4].copy_from_slice(&word.to_le_bytes());
                buf[4..].copy_from_slice(&word.rotate_left(11).to_le_bytes());
                (api.print_int_ptr_line)(buf.as_ptr().add(off) as *const i32);
            }
            "pipl_addr" => {
                let addr = it
                    .next()
                    .expect("pipl_addr needs an address")
                    .parse::<usize>()
                    .expect("pipl_addr address");
                (api.print_int_ptr_line)(addr as *const i32);
            }
            "pipl_null" => (api.print_int_ptr_line)(std::ptr::null()),
            "pipl_unmapped" => {
                let len = 4096;
                let p = mmap(
                    std::ptr::null_mut(),
                    len,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                );
                assert_ne!(p as usize, MAP_FAILED, "mmap failed");
                munmap(p, len);
                (api.print_int_ptr_line)(p as *const i32);
            }
            "pipl_protnone" => {
                let p = mmap(
                    std::ptr::null_mut(),
                    4096,
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                );
                assert_ne!(p as usize, MAP_FAILED, "mmap failed");
                (api.print_int_ptr_line)(p as *const i32);
            }
            "pipl_writeonly" => {
                // x86_64 has no write-without-read page protection, so this is
                // requested as PROT_WRITE only and the kernel may widen it to
                // readable. Whatever it does, it must do the same for both libs.
                let p = mmap(
                    std::ptr::null_mut(),
                    4096,
                    PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                );
                assert_ne!(p as usize, MAP_FAILED, "mmap failed");
                (api.print_int_ptr_line)(p as *const i32);
            }
            "pipl_straddle" => {
                // Two pages, second one unmapped: a 4-byte read starting 2 bytes
                // before the boundary reads 2 valid and 2 faulting bytes.
                let len = 8192;
                let p = mmap(
                    std::ptr::null_mut(),
                    len,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                );
                assert_ne!(p as usize, MAP_FAILED, "mmap failed");
                munmap((p as *mut u8).add(4096) as *mut _, 4096);
                (api.print_int_ptr_line)((p as *mut u8).add(4094) as *const i32);
            }
            "good" => (api.good)(),
            "bad" => (api.bad)(),
            "bad_bad" => seq_bad_bad(api.bad),
            "good_bad" => seq_good_bad(api.good, api.bad),
            "pipl_bad" => {
                let v = next_i64() as i32;
                seq_pipl_bad(api.print_int_ptr_line, api.bad, v);
            }
            "driver" => (api.driver)(next_i64() as i32),
            "driver_wide" => {
                let v = it
                    .next()
                    .expect("driver_wide needs a value")
                    .parse::<u64>()
                    .expect("driver_wide value");
                (api.driver_wide)(v);
            }
            "driver_seq" => {
                let bits = it.next().expect("driver_seq needs a bit pattern");
                for ch in bits.chars() {
                    (api.driver)(if ch == '1' { 1 } else { 0 });
                }
            }
            other => panic!("unknown op spec {other:?}"),
        }
    }
}

/// Runs `spec` against both libraries **in this process** and returns
/// `(c_output, rust_output)`.
///
/// Only safe for specs whose outcome is specified (i.e. that cannot fault). Use
/// [`run_isolated`] for anything reaching `bad()` on an uncontrolled slot.
pub fn run_in_process(c: &Api, r: &Api, spec: &str) -> (Vec<u8>, Vec<u8>) {
    let out_c = capture(&mut || perform(c, spec));
    let out_r = capture(&mut || perform(r, spec));
    (out_c, out_r)
}

/// Asserts that both libraries produce byte-identical output for `spec`.
pub fn assert_same_in_process(c: &Api, r: &Api, spec: &str) {
    let (out_c, out_r) = run_in_process(c, r, spec);
    assert_eq!(
        out_c,
        out_r,
        "output diverged for spec {spec:?}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r)
    );
}

/// Like [`assert_same_in_process`], but for specs that reach `bad()` and would
/// therefore take the whole test runner down with a `SIGSEGV` if the translation
/// read the wrong stack slot.
///
/// Probes the spec in a child process first. If it faults there, the failure is
/// reported cleanly (and the two implementations are still required to fault
/// identically) instead of the in-process call killing the run.
pub fn assert_same_in_process_guarded(c: &Api, r: &Api, spec: &str) {
    let (oc, or) = run_isolated_both(spec);
    assert_eq!(
        (oc.code, oc.signal),
        (or.code, or.signal),
        "termination diverged for spec {spec:?}: C={oc:?} Rust={or:?}"
    );
    if oc.crashed() || or.crashed() {
        assert_eq!(
            oc.stdout, or.stdout,
            "output diverged for faulting spec {spec:?}"
        );
        panic!(
            "spec {spec:?} faults (signal {:?}) in BOTH implementations. This \
             configuration is supposed to read a stack slot a predecessor just \
             wrote, so a fault means the frame layout no longer matches the C.",
            oc.signal
        );
    }
    assert_same_in_process(c, r, spec);
}

// ---------------------------------------------------------------------------
// Isolated (subprocess) execution
// ---------------------------------------------------------------------------

pub const ENV_CHILD_SO: &str = "DIFFTEST_CHILD_SO";
pub const ENV_CHILD_SPEC: &str = "DIFFTEST_CHILD_SPEC";
pub const ENV_CHILD_OUT: &str = "DIFFTEST_CHILD_OUT";

// ---------------------------------------------------------------------------
// Sequential test runner (these targets are `harness = false`)
// ---------------------------------------------------------------------------

/// One named check.
pub struct Test {
    pub name: &'static str,
    pub f: fn(),
}

/// Declares a `&[Test]` from a list of function names.
#[macro_export]
macro_rules! tests {
    ($($f:ident),* $(,)?) => {
        &[$($crate::common::Test { name: stringify!($f), f: $f }),*]
    };
}

/// Entry point for a `harness = false` test binary.
///
/// * Serves the child-process protocol first, so [`run_isolated`] can re-execute
///   this binary to run one operation in isolation.
/// * Runs the checks strictly sequentially, on the main thread, so nothing else
///   can touch fd 1 while a capture is in flight.
/// * Accepts a substring filter as the first non-flag argument, so
///   `cargo test --test configs row14` still works, and ignores the flags cargo
///   passes through (`--test-threads`, `--nocapture`, …).
/// * Exits non-zero if any check failed.
pub fn run_tests(tests: &[Test]) -> ! {
    child_entry();

    let mut filter: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--test-threads" || a == "--skip" || a == "--format" {
            let _ = args.next(); // consume the flag's value
        } else if !a.starts_with('-') {
            filter = Some(a);
        }
    }

    let selected: Vec<&Test> = tests
        .iter()
        .filter(|t| filter.as_deref().is_none_or(|f| t.name.contains(f)))
        .collect();

    println!("\nrunning {} tests", selected.len());
    let mut failed: Vec<(&str, String)> = Vec::new();

    // Collect panic location/message ourselves and silence the default hook, so
    // the default hook's stderr writes cannot interleave with the runner's own
    // progress line.
    static LAST_PANIC: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown location>".into());
        *LAST_PANIC.lock().unwrap_or_else(|e| e.into_inner()) = Some(loc);
    }));

    for t in &selected {
        print!("test {} ... ", t.name);
        use std::io::Write;
        std::io::stdout().flush().ok();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(t.f));
        match result {
            Ok(()) => println!("ok"),
            Err(payload) => {
                println!("FAILED");
                let msg = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                let loc = LAST_PANIC
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                    .unwrap_or_else(|| "<unknown location>".into());
                failed.push((t.name, format!("panicked at {loc}:\n{msg}")));
            }
        }
    }
    let _ = std::panic::take_hook();

    if failed.is_empty() {
        println!(
            "\ntest result: ok. {} passed; 0 failed; 0 ignored; 0 measured; \
             {} filtered out\n",
            selected.len(),
            tests.len() - selected.len()
        );
        std::process::exit(0);
    } else {
        println!("\nfailures:");
        for (name, msg) in &failed {
            println!("---- {name} ----\n{msg}\n");
        }
        println!("failures:");
        for (name, _) in &failed {
            println!("    {name}");
        }
        println!(
            "\ntest result: FAILED. {} passed; {} failed; 0 ignored; 0 measured; \
             {} filtered out\n",
            selected.len() - failed.len(),
            failed.len(),
            tests.len() - selected.len()
        );
        std::process::exit(101);
    }
}

/// What a child process did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Normal exit code, if the child exited normally.
    pub code: Option<i32>,
    /// Terminating signal, if it was killed (11 = `SIGSEGV`).
    pub signal: Option<i32>,
    /// Bytes the library wrote through libc `stdout`.
    pub stdout: Vec<u8>,
}

impl Outcome {
    pub fn crashed(&self) -> bool {
        self.signal.is_some()
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// Child-side entry point, called first by [`run_tests`]. Returns immediately
/// when the process is not a child.
///
/// The child redirects fd 1 to the file named by `ENV_CHILD_OUT` before calling
/// into the library, so a `SIGSEGV` mid-`printf` loses exactly the same buffered
/// bytes it would lose in the C library. It then exits without running any
/// checks, so the output file holds only library output.
pub fn child_entry() {
    let (so, spec, out) = match (
        std::env::var_os(ENV_CHILD_SO),
        std::env::var_os(ENV_CHILD_SPEC),
        std::env::var_os(ENV_CHILD_OUT),
    ) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => return, // not a child; nothing to do
    };
    let spec = spec.to_string_lossy().into_owned();

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(PathBuf::from(&out))
        .expect("child: open output file");

    let api = Api::load(Path::new(&so));

    {
        use std::io::Write;
        std::io::stdout().flush().ok();
        unsafe {
            fflush(std::ptr::null_mut());
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "child: dup2 failed");
        }
    }

    perform(&api, &spec);

    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(0);
}

/// Runs `spec` against the library at `so` in a fresh child process.
///
/// The child is this same test binary, re-executed with the child protocol
/// env vars set; `run_tests` dispatches to `child_entry` before running any
/// checks.
pub fn run_isolated(so: &Path, spec: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out_path = {
        let mut p = std::env::temp_dir();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("difftest-child-{}-{}.out", std::process::id(), n));
        p
    };

    let status = Command::new(&exe)
        .env(ENV_CHILD_SO, OsString::from(so))
        .env(ENV_CHILD_SPEC, spec)
        .env(ENV_CHILD_OUT, &out_path)
        .env("RUST_BACKTRACE", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("spawn child test process");

    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);

    Outcome {
        code: status.code(),
        signal: status.signal(),
        stdout,
    }
}

/// Runs `spec` isolated against both libraries.
pub fn run_isolated_both(spec: &str) -> (Outcome, Outcome) {
    (
        run_isolated(&c_so_path(), spec),
        run_isolated(&rust_so_path(), spec),
    )
}

/// Asserts both libraries produce identical exit status *and* identical bytes.
pub fn assert_same_isolated(spec: &str) {
    let (c, r) = run_isolated_both(spec);
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "termination diverged for spec {spec:?}: C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "output diverged for spec {spec:?}\n  C   : {:?}\n  Rust: {:?}",
        c.text(),
        r.text()
    );
}

/// Asserts both libraries terminate the same way, without constraining the
/// bytes. For the two unspecified rows (`ERRORS.md` 6/7) whose output is a
/// leaked stack address that differs on every run of the C library itself.
pub fn assert_same_termination(spec: &str) -> (Outcome, Outcome) {
    let (c, r) = run_isolated_both(spec);
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "termination diverged for spec {spec:?}: C={c:?} Rust={r:?}"
    );
    (c, r)
}

/// True if `bytes` is exactly one line holding one decimal `int`, i.e. what
/// `printf("%d\n", …)` produces.
pub fn is_one_int_line(bytes: &[u8]) -> bool {
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match s.strip_suffix('\n') {
        None => false,
        Some(body) => {
            let digits = body.strip_prefix('-').unwrap_or(body);
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && body.parse::<i64>().is_ok()
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
}

/// Boundary `i32` values worth testing: signed extremes, digit-count changes,
/// sign changes and every power of two with its neighbours.
pub fn boundary_i32s() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        5,
        9,
        10,
        -9,
        -10,
        99,
        100,
        -99,
        -100,
        999_999_999,
        1_000_000_000,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ];
    for bit in 0..32u32 {
        let p = 1i32.wrapping_shl(bit);
        v.push(p);
        v.push(p.wrapping_sub(1));
        v.push(p.wrapping_add(1));
        v.push(p.wrapping_neg());
    }
    v.sort_unstable();
    v.dedup();
    v
}
