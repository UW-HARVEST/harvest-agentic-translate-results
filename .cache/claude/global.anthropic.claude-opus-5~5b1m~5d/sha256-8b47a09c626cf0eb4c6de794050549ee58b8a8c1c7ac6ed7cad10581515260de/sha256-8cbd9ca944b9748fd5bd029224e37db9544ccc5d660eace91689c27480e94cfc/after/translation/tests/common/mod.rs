// Shared differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
// call is made through the exported C ABI symbols, so the `#[no_mangle]`
// wrappers themselves are under test. Rust functions are never called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
pub type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    pub fn malloc(n: usize) -> *mut c_void;
}

/// Locate the C shared library built by `c_src/build`.
fn c_lib_path() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf();
    let build = root.join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  cd c_src && mkdir -p build && cd build && \
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Profile the integration-test binary was built under ("debug" / "release").
fn current_profile() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            // target/<profile>/deps/<test-bin>
            exe.parent()
                .and_then(|deps| deps.parent())
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "debug".to_string())
}

/// Locate the Rust cdylib, BUILDING IT FIRST.
///
/// `cargo test` does NOT produce the `cdylib` artifact — integration tests link
/// the library as an rlib, so `target/<profile>/libcleanup_lib.so` can be
/// missing or, far worse, left over from an older `cargo build` and therefore
/// stale. Loading a stale `.so` silently turns every differential test into a
/// no-op: a deliberate mutation of `src/lib.rs` would still "pass". So build the
/// cdylib explicitly here and then assert it is newer than the sources.
fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = current_profile();
    let so = manifest
        .join("target")
        .join(&profile)
        .join("libcleanup_lib.so");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(cargo);
    cmd.arg("build").arg("--offline").arg("--lib");
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.current_dir(&manifest);
    // Avoid inheriting the parent cargo's jobserver/target-dir surprises.
    cmd.env_remove("CARGO_MAKEFLAGS");
    match cmd.output() {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            if !so.exists() {
                panic!(
                    "failed to build the Rust cdylib and none exists at {}:\n{}",
                    so.display(),
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            eprintln!(
                "warning: `cargo build --lib` failed, using existing {}:\n{}",
                so.display(),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Err(e) => {
            if !so.exists() {
                panic!("could not run cargo to build the cdylib ({e}) and none exists");
            }
        }
    }

    assert!(
        so.exists(),
        "libcleanup_lib.so not found at {} after `cargo build --lib`",
        so.display()
    );

    // Freshness gate: the artifact under test must be at least as new as the
    // source it is supposed to represent.
    let src = manifest.join("src").join("lib.rs");
    if let (Ok(so_m), Ok(src_m)) = (
        std::fs::metadata(&so).and_then(|m| m.modified()),
        std::fs::metadata(&src).and_then(|m| m.modified()),
    ) {
        assert!(
            so_m >= src_m,
            "STALE ARTIFACT: {} is older than {}. The differential tests would be \
             comparing against out-of-date Rust code. Run `cargo build --lib` (or delete \
             target/) and re-run.",
            so.display(),
            src.display()
        );
    }
    so
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn c_cleanup(&self) -> Symbol<'_, CleanupFn> {
        unsafe { self.c.get(b"cleanup\0").expect("C cleanup") }
    }
    pub fn rust_cleanup(&self) -> Symbol<'_, CleanupFn> {
        unsafe { self.rust.get(b"cleanup\0").expect("Rust cleanup") }
    }
    pub fn c_print_result(&self) -> Symbol<'_, PrintResultFn> {
        unsafe { self.c.get(b"print_result\0").expect("C print_result") }
    }
    pub fn rust_print_result(&self) -> Symbol<'_, PrintResultFn> {
        unsafe { self.rust.get(b"print_result\0").expect("Rust print_result") }
    }
    pub fn c_cleanup_resources(&self) -> Symbol<'_, CleanupResourcesFn> {
        unsafe {
            self.c
                .get(b"cleanup_resources\0")
                .expect("C cleanup_resources")
        }
    }
    pub fn rust_cleanup_resources(&self) -> Symbol<'_, CleanupResourcesFn> {
        unsafe {
            self.rust
                .get(b"cleanup_resources\0")
                .expect("Rust cleanup_resources")
        }
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

/// Serialises fd-1 redirection across all concurrently running tests.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Distinguishes capture temp files without relying on stack addresses.
static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c = unsafe { Library::new(c_lib_path()).expect("load C .so") };
        let rust = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
        Libs { c, rust }
    })
}

/// Run `f` with fd 1 redirected into a temporary file and return the bytes it
/// wrote. Captures output produced by libc `printf` inside the loaded `.so`s
/// (which `std`-level capture would miss entirely).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // fd 1 is process-global, so only one capture may be in flight at a time,
    // otherwise concurrently running tests would steal each other's output.
    // NOTE: the lock MUST live at module scope — a `static` declared inside a
    // generic function is monomorphised once per closure type, which would give
    // every caller its own independent mutex and no mutual exclusion at all.
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Drain BOTH buffering layers before stealing fd 1: libc's stdio (used
        // by the loaded .so's `printf`) and Rust's own `std::io::stdout`
        // buffer (used by the libtest harness's progress text). Anything left
        // sitting in either buffer would otherwise be flushed into our capture
        // file and be mistaken for library output.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        fflush(std::ptr::null_mut());
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "cleanup_diff_{}_{}.out",
            std::process::id(),
            CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .expect("open capture file");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore stdout failed");
        close(saved);

        let mut buf = Vec::new();
        let mut file = file;
        use std::io::Seek;
        file.rewind().expect("rewind capture file");
        file.read_to_end(&mut buf).expect("read capture file");
        drop(file);
        let _ = std::fs::remove_file(&tmp);
        buf
    }
}

/// Deterministic xorshift64* PRNG so every property-style sweep is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Differentially call `cleanup` in both libraries, comparing the return value
/// AND the stdout bytes.
pub fn diff_cleanup(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let l = libs();
    let cf = l.c_cleanup();
    let rf = l.rust_cleanup();

    let mut c_ret = 0;
    let c_out = capture_stdout(|| c_ret = unsafe { cf(a, b, c, d) });
    let mut r_ret = 0;
    let r_out = capture_stdout(|| r_ret = unsafe { rf(a, b, c, d) });

    assert_eq!(
        c_ret, r_ret,
        "cleanup({a},{b},{c},{d}) return mismatch: C={c_ret} Rust={r_ret}"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "cleanup({a},{b},{c},{d}) stdout mismatch"
    );
    assert_eq!(c_out, r_out, "cleanup({a},{b},{c},{d}) stdout byte mismatch");
    c_ret
}

/// Differentially call `print_result`, comparing stdout bytes.
pub fn diff_print_result(label: *const c_char, result: i32, ctx: &str) {
    let l = libs();
    let cf = l.c_print_result();
    let rf = l.rust_print_result();
    let c_out = capture_stdout(|| unsafe { cf(label, result) });
    let r_out = capture_stdout(|| unsafe { rf(label, result) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "print_result stdout mismatch [{ctx}]"
    );
    assert_eq!(c_out, r_out, "print_result stdout byte mismatch [{ctx}]");
}

/// The reference model of the C switch, used only as a cross-check that the
/// differential agreement is not "both wrong in the same trivial way".
pub fn model_cleanup(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut result: i32 = 0;
    for n in [a, b, c, d] {
        match n {
            10 => {
                result = result.wrapping_add(10);
                result = result.wrapping_add(20);
            }
            20 => result = result.wrapping_add(20),
            30 => {
                result = result.wrapping_add(30);
                result = result.wrapping_add(40);
            }
            40 => result = result.wrapping_add(40),
            other => result = result.wrapping_add(other),
        }
    }
    result
}
