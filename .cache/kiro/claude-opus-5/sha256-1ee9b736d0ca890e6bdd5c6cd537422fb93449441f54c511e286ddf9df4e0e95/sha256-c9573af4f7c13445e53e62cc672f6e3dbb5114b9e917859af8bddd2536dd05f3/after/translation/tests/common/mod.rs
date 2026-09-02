//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! only through their exported symbols — the Rust crate is *never* called
//! directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//!
//! Two observable channels are compared for every call:
//!   * the return value, and
//!   * the exact bytes the call writes to file descriptor 1.
//!
//! stdout is captured by temporarily `dup2`-ing fd 1 onto a temp file. Both
//! libraries write through the *same* process-wide glibc `stdout` FILE, so a
//! `fflush(NULL)` before and after each capture window is enough to make the
//! window exact. Because fd 1 is process-global, all capture is serialised
//! behind `STDOUT_LOCK`.

#![allow(dead_code)]

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
}

/// Allocate `n` bytes with the *same* allocator the C library frees with.
pub fn c_malloc(n: usize) -> *mut c_char {
    unsafe { malloc(n) as *mut c_char }
}

// ---------------------------------------------------------------------------
// Function signatures under test
// ---------------------------------------------------------------------------

pub type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
pub type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub cleanup: CleanupFn,
    pub print_result: PrintResultFn,
    pub cleanup_resources: CleanupResourcesFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
            let cleanup: Symbol<CleanupFn> = lib
                .get(b"cleanup\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `cleanup`: {e}"));
            let print_result: Symbol<PrintResultFn> = lib
                .get(b"print_result\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `print_result`: {e}"));
            let cleanup_resources: Symbol<CleanupResourcesFn> = lib
                .get(b"cleanup_resources\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `cleanup_resources`: {e}"));
            let (cleanup, print_result, cleanup_resources) =
                (*cleanup, *print_result, *cleanup_resources);
            Impl {
                name,
                _lib: lib,
                cleanup,
                print_result,
                cleanup_resources,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // Prefer the profile the test itself was built with, then fall back.
    let target = workspace_root().join("translation").join("target");
    let mut probes: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // <target>/<profile>/deps/<test-bin>  ->  <target>/<profile>
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            probes.push(profile_dir.join("libcleanup_lib.so"));
        }
    }
    probes.push(target.join("release").join("libcleanup_lib.so"));
    probes.push(target.join("debug").join("libcleanup_lib.so"));
    for p in &probes {
        if p.exists() {
            return p.clone();
        }
    }
    panic!(
        "libcleanup_lib.so not found; tried: {:?}\nBuild it first: cargo build --release",
        probes
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Fail loudly if the Rust `.so` predates `src/lib.rs`.
///
/// `crate-type = ["cdylib"]` means `cargo test` does **not** necessarily
/// rebuild the shared object, so without this check an edited-but-not-rebuilt
/// translation would be silently verified against a stale artifact and every
/// test would "pass". Always run `cargo build --release` before `cargo test`.
fn assert_so_is_fresh(so: &PathBuf) {
    let src = workspace_root()
        .join("translation")
        .join("src")
        .join("lib.rs");
    let mtime = |p: &PathBuf| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    let (so_t, src_t) = (mtime(so), mtime(&src));
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {} is older than {}.\n\
         `crate-type = [\"cdylib\"]` is not rebuilt by `cargo test`; run\n\
           cd translation && cargo build --release\n\
         before testing, or the suite verifies an out-of-date library.",
        so.display(),
        src.display()
    );
}

/// The two implementations, loaded once per test process.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let rust_path = find_rust_so();
        assert_so_is_fresh(&rust_path);
        Pair {
            c: Impl::load("C", &find_c_so()),
            rust: Impl::load("Rust", &rust_path),
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

pub fn stdout_guard() -> MutexGuard<'static, ()> {
    STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `f` with fd 1 redirected to a temp file; return everything it wrote.
///
/// The caller must already hold [`stdout_guard`].
///
/// The temp file is created once per thread and reused (truncated) on every
/// call — the suite performs ~10^5 captures, so re-creating and unlinking a
/// file each time would dominate the runtime.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    CAPTURE_FILE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let file = slot.get_or_insert_with(new_capture_file);
        file.set_len(0).expect("truncate capture file");
        file.seek(SeekFrom::Start(0)).expect("rewind capture file");

        let out;
        let mut buf = Vec::new();
        unsafe {
            fflush(std::ptr::null_mut()); // flush all streams before swapping fd 1
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

            out = f();

            fflush(std::ptr::null_mut()); // flush what f() wrote, still on fd 1
            assert!(dup2(saved, 1) >= 0, "dup2 restore of fd 1 failed");
            close(saved);
        }
        file.seek(SeekFrom::Start(0)).expect("rewind capture file");
        file.read_to_end(&mut buf).expect("read capture file");
        (out, buf)
    })
}

thread_local! {
    static CAPTURE_FILE: RefCell<Option<std::fs::File>> = const { RefCell::new(None) };
}

/// An unlinked-but-open temp file: no path collisions, no cleanup needed.
fn new_capture_file() -> std::fs::File {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "cdiff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create capture temp file");
    // Unlink immediately; the open handle keeps it alive for this process only.
    let _ = std::fs::remove_file(&path);
    file
}

/// The full observation of one call: return value + stdout bytes.
#[derive(PartialEq, Eq, Clone)]
pub struct Obs<T> {
    pub ret: T,
    pub out: Vec<u8>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Obs<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Obs {{ ret: {:?}, out: {:?} }}",
            self.ret,
            String::from_utf8_lossy(&self.out)
        )
    }
}

/// Observe the same action on both libraries and assert full equality.
pub fn assert_same<T, F>(ctx: impl std::fmt::Display, f: F)
where
    T: PartialEq + std::fmt::Debug,
    F: Fn(&Impl) -> T,
{
    let p = pair();
    let _g = stdout_guard();
    let (c_ret, c_out) = capture(|| f(&p.c));
    let (r_ret, r_out) = capture(|| f(&p.rust));
    if c_ret != r_ret || c_out != r_out {
        panic!(
            "DIVERGENCE [{ctx}]\n  C   : ret={:?} stdout={:?}\n  Rust: ret={:?} stdout={:?}",
            c_ret,
            String::from_utf8_lossy(&c_out),
            r_ret,
            String::from_utf8_lossy(&r_out),
        );
    }
}

/// `assert_same` for `cleanup(a, b, c, d)`.
pub fn assert_cleanup(a: c_int, b: c_int, c: c_int, d: c_int) {
    assert_same(
        format!("cleanup({a}, {b}, {c}, {d})"),
        |imp| unsafe { (imp.cleanup)(a, b, c, d) },
    );
}

/// `assert_same` for `print_result(label, result)` with raw (possibly
/// non-UTF-8) label bytes. `label` must be NUL-terminated by the caller.
pub fn assert_print_result_raw(label: &[u8], result: c_int) {
    assert_eq!(
        label.last(),
        Some(&0u8),
        "label must be NUL-terminated for print_result"
    );
    assert_same(
        format!("print_result(<{} bytes>, {result})", label.len() - 1),
        |imp| unsafe {
            (imp.print_result)(label.as_ptr() as *const c_char, result);
        },
    );
}

pub fn assert_print_result(label: &str, result: c_int) {
    let cs = CString::new(label).expect("label contains interior NUL");
    assert_print_result_raw(cs.as_bytes_with_nul(), result);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed, reproducible failures
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

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
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The four `switch` labels the C special-cases.
pub const LABELS: [c_int; 4] = [10, 20, 30, 40];

/// A value drawn from an "interesting" distribution: half the time a switch
/// label, otherwise an arbitrary `i32` (which lands in `default`).
pub fn biased_i32(rng: &mut Rng) -> c_int {
    if rng.bool() {
        LABELS[rng.range_usize(0, 3)]
    } else {
        rng.next_i32()
    }
}
