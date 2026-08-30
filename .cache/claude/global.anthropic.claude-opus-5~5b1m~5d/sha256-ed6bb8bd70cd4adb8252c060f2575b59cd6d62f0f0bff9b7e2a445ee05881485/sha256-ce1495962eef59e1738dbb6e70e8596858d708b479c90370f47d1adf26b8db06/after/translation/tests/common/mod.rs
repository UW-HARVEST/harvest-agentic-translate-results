// Shared differential-test harness.
//
// This module is compiled into several test binaries, each of which uses only a
// subset of the helpers, so unused-item warnings here are expected noise.
#![allow(dead_code)]
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and called
// only through their exported `driver` symbol -- never by calling Rust code
// directly -- so the `#[no_mangle]` export wrapper is under test too.
//
// `driver` returns `void` and communicates purely through `stdout`, so the
// observable output is captured by temporarily redirecting file descriptor 1
// to a temp file. Both shared objects route their output through the same libc
// `stdout` FILE stream, so the same redirect + `fflush` works for both.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::ffi::c_void;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int);

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    /// `fflush(NULL)` flushes every open output stream.
    fn fflush(stream: *mut c_void) -> i32;
}

/// fd 1 is process-wide state, so captures must be serialized even though
/// cargo runs tests on multiple threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn c_so_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so")
}

/// Locate the Rust cdylib under test.
///
/// IMPORTANT: `cargo test` does **not** build the `cdylib` artifact (this crate
/// declares `crate-type = ["cdylib"]`, and the test profile builds an rlib+test
/// harness instead). So the `.so` must be produced by a separate `cargo build`.
/// An earlier version of this harness silently fell back to whatever
/// `libdriver.so` happened to exist, which meant a *stale* artifact could be
/// tested and a real divergence could pass unnoticed. `assert_so_is_fresh`
/// below now makes that failure mode impossible.
///
/// `DRIVER_RUST_SO` overrides the path (used by `run_tests.sh`).
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test binary lives in target/<profile>/deps");
    let direct = profile_dir.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    let target_dir = profile_dir.parent().expect("target dir");
    for p in ["debug", "release"] {
        let cand = target_dir.join(p).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    direct
}

/// Refuse to run against a `.so` older than the sources it was built from.
/// Without this, a mutation to `src/lib.rs` can appear to "pass" simply because
/// the loaded artifact predates the edit.
fn assert_so_is_fresh(so: &std::path::Path) {
    let mtime = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {p:?}: {e}"))
    };
    let so_time = mtime(so);
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest_src = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                let t = mtime(&p);
                if newest_src.map_or(true, |cur| t > cur) {
                    newest_src = Some(t);
                }
            }
        }
    }
    if let Some(src_time) = newest_src {
        assert!(
            so_time >= src_time,
            "STALE ARTIFACT: {so:?} is older than the Rust sources it should have \
             been built from.\nTesting it would validate old code and could hide a real \
             divergence.\nRebuild with `cargo build` (or use ./run_tests.sh) before `cargo test`."
        );
    }
}

struct Libs {
    _c: Library,
    _rust: Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

// The raw fn pointers borrow from the leaked-for-process-lifetime Libraries.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {c_path:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {rust_path:?}. Build it with `cargo build`."
        );
        assert_so_is_fresh(&rust_path);
        unsafe {
            let c = Library::new(&c_path).expect("dlopen C .so");
            let rust = Library::new(&rust_path).expect("dlopen Rust .so");
            let c_sym: Symbol<DriverFn> =
                c.get(b"driver\0").expect("C .so must export `driver`");
            let rust_sym: Symbol<DriverFn> = rust
                .get(b"driver\0")
                .expect("Rust .so must export `driver`");
            let c_driver = *c_sym;
            let rust_driver = *rust_sym;
            Libs {
                _c: c,
                _rust: rust,
                c_driver,
                rust_driver,
            }
        }
    })
}

pub fn c_driver() -> DriverFn {
    libs().c_driver
}

pub fn rust_driver() -> DriverFn {
    libs().rust_driver
}

/// Run `f`, capturing everything it writes to fd 1, and return the raw bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Force dlopen + the freshness check to happen BEFORE fd 1 is redirected,
    // so any setup panic (e.g. "STALE ARTIFACT") is actually visible to the user
    // instead of being written into the capture file.
    libs();

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let id = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}_{}.bin",
        std::process::id(),
        id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    // Drain *both* buffers layered over fd 1 before redirecting, or their
    // pending contents (e.g. libtest's "test foo ... " progress text, which has
    // no trailing newline and so sits in Rust's LineWriter) would be flushed
    // into the capture file and misattributed to `f`.
    let _ = std::io::Write::flush(&mut std::io::stdout());

    unsafe {
        // Push out anything already buffered so it is not misattributed to `f`.
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        // Restore fd 1 even if `f` panics. Without this, a panic inside the
        // capture window leaves fd 1 pointing at the (already unlinked) temp
        // file, and every later message from libtest -- including the panic
        // report that explains the failure -- is silently discarded.
        struct FdRestore(i32);
        impl Drop for FdRestore {
            fn drop(&mut self) {
                unsafe {
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    fflush(std::ptr::null_mut());
                    dup2(self.0, 1);
                    close(self.0);
                }
            }
        }
        let _restore = FdRestore(saved);

        {
            let file = File::create(&path).expect("create capture temp file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
            // `file` closes here; fd 1 keeps the description alive.
        }

        f();

        // `_restore` flushes and restores fd 1 here.
    }

    let data = std::fs::read(&path).expect("read capture temp file");
    let _ = std::fs::remove_file(&path);
    data
}

/// Sanity guard: a capture harness that silently produced nothing would make
/// every differential assertion pass vacuously. Every comparison routes through
/// here so that "both empty" can never be mistaken for "both agree".
pub fn assert_wellformed_c_output(bytes: &[u8], calls: usize, ctx: &str) {
    assert!(
        !bytes.is_empty(),
        "capture harness produced no output for {ctx}; the differential comparison \
         would have been vacuous"
    );
    // sizeof(int) == 4 bytes -> 8 hex digits, plus '\n', per call.
    assert_eq!(
        bytes.len(),
        9 * calls,
        "unexpected C output length for {ctx}: {:?}",
        String::from_utf8_lossy(bytes)
    );
    for (i, line) in bytes.split(|&b| b == b'\n').enumerate() {
        if i == calls {
            assert!(line.is_empty(), "trailing bytes after final newline for {ctx}");
            continue;
        }
        assert_eq!(line.len(), 8, "line {i} not 8 hex digits for {ctx}");
        assert!(
            line.iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c)),
            "line {i} is not lowercase hex for {ctx}: {:?}",
            String::from_utf8_lossy(line)
        );
    }
}

/// The core differential assertion for a single `driver(x)` call.
pub fn assert_same(x: i32) {
    let c_out = capture_stdout(|| unsafe { c_driver()(x) });
    let rust_out = capture_stdout(|| unsafe { rust_driver()(x) });

    let ctx = format!("x={x} (0x{:08x})", x as u32);
    assert_wellformed_c_output(&c_out, 1, &ctx);

    assert_eq!(
        c_out,
        rust_out,
        "DIVERGENCE for {ctx}:\n  C   = {:?}\n  Rust= {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}

pub fn assert_same_many<I: IntoIterator<Item = i32>>(xs: I) {
    for x in xs {
        assert_same(x);
    }
}

/// Deterministic SplitMix64 so randomized rows are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
}
