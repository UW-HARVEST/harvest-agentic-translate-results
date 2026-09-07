//! Shared differential-test harness.
//!
//! Both implementations are loaded **as shared objects** with `libloading` and
//! called only through their exported C symbols, so the `#[no_mangle]` wrappers
//! of the Rust crate are exercised exactly as an external consumer would
//! exercise them. No Rust function is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, CString};
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for fd-level stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Signatures of the three exported symbols
// ---------------------------------------------------------------------------

pub type FmaArrayFn = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
pub type CallFmaFn = unsafe extern "C" fn(data: *const c_int, len: c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(in_: *const c_char);

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub fma_array: Symbol<'static, FmaArrayFn>,
    pub call_fma: Symbol<'static, CallFmaFn>,
    pub driver: Symbol<'static, DriverFn>,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        assert!(
            path.exists(),
            "{name} shared object not found at {}\n\
             build it first:\n  C:    cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
             Rust: cd translation && cargo build --release",
            path.display()
        );
        // Leaked on purpose: the library must outlive every `Symbol` we hand
        // out, and it lives for the whole test process anyway.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(&path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        }));
        unsafe {
            Lib {
                name,
                fma_array: lib
                    .get(b"fma_array\0")
                    .unwrap_or_else(|e| panic!("{name}: fma_array: {e}")),
                call_fma: lib
                    .get(b"call_fma\0")
                    .unwrap_or_else(|e| panic!("{name}: call_fma: {e}")),
                driver: lib
                    .get(b"driver\0")
                    .unwrap_or_else(|e| panic!("{name}: driver: {e}")),
                path,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Dedicated target directory for the `cdylib` the tests load.
///
/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artefact
/// (integration tests cannot link it), so `target/release/libdriver.so` can be
/// arbitrarily stale — which would silently turn every differential test into a
/// no-op. The harness therefore builds the shared object itself, into a
/// separate `--target-dir` so it cannot deadlock against the build lock held by
/// the outer `cargo test`.
fn ffi_target_dir() -> PathBuf {
    manifest_dir().join("target/ffi-so")
}

static BUILD_ONCE: OnceLock<()> = OnceLock::new();

fn build_rust_cdylib() -> PathBuf {
    let target_dir = ffi_target_dir();
    BUILD_ONCE.get_or_init(|| {
        let mut cmd = std::process::Command::new(
            std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()),
        );
        // Strip the outer cargo/rustc environment so the nested invocation is a
        // clean, independent build.
        for (k, _) in std::env::vars() {
            if (k.starts_with("CARGO") && k != "CARGO_HOME")
                || k == "RUSTC"
                || k == "RUSTC_WRAPPER"
                || k == "RUSTC_WORKSPACE_WRAPPER"
                || k == "RUSTDOC"
                || k == "LD_LIBRARY_PATH"
                || k == "DYLD_FALLBACK_LIBRARY_PATH"
            {
                cmd.env_remove(k);
            }
        }
        let out = cmd
            .current_dir(manifest_dir())
            .args([
                "build",
                "--release",
                "--lib",
                "--target-dir",
                target_dir.to_str().expect("utf-8 target dir"),
            ])
            .output()
            .expect("failed to spawn `cargo build --release --lib` for the cdylib");
        assert!(
            out.status.success(),
            "building the Rust cdylib failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    });
    target_dir.join("release/libdriver.so")
}

fn c_src_dir() -> PathBuf {
    manifest_dir().parent().expect("crate has a parent dir").join("c_src")
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let so = c_src_dir().join("build/libdriver.so");
    if !so.exists() {
        // Configure + build exactly as documented in the task instructions.
        let build_dir = c_src_dir().join("build");
        fs::create_dir_all(&build_dir).expect("create c_src/build");
        let cfg = std::process::Command::new("cmake")
            .current_dir(&build_dir)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .output()
            .expect("spawn cmake configure");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cfg.stderr)
        );
        let bld = std::process::Command::new("cmake")
            .current_dir(&build_dir)
            .args(["--build", "."])
            .output()
            .expect("spawn cmake build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&bld.stderr)
        );
    }
    so
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    build_rust_cdylib()
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open("C", c_so_path()))
}

pub fn rs() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::open("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture (process global -> serialised)
// ---------------------------------------------------------------------------

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: Mutex<u64> = Mutex::new(0);

/// Runs `f`, capturing everything written to **file descriptor 1** by any code
/// (including `printf` inside either `.so`) and returning the raw bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let seq = {
        let mut s = CAPTURE_SEQ.lock().unwrap_or_else(|e| e.into_inner());
        *s += 1;
        *s
    };
    let tmp = std::env::temp_dir().join(format!(
        "driver_diff_capture_{}_{}.bin",
        std::process::id(),
        seq
    ));

    let bytes = {
        let file = fs::File::create(&tmp).expect("create capture file");
        unsafe {
            // Flush anything already buffered so it is not misattributed.
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

            f();

            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }
        drop(file);

        let mut buf = Vec::new();
        fs::File::open(&tmp)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    };
    let _ = fs::remove_file(&tmp);
    bytes
}

/// Calls `driver(text)` in `lib` and returns the bytes it printed.
pub fn run_driver(lib: &Lib, text: &str) -> Vec<u8> {
    let cs = CString::new(text).expect("test input must not contain an interior NUL");
    capture_stdout(|| unsafe { (lib.driver)(cs.as_ptr()) })
}

/// Same, but for input that deliberately contains interior NUL bytes: the
/// buffer is passed verbatim and must already be NUL terminated.
pub fn run_driver_raw(lib: &Lib, nul_terminated: &[u8]) -> Vec<u8> {
    assert_eq!(nul_terminated.last(), Some(&0), "buffer must be NUL terminated");
    capture_stdout(|| unsafe { (lib.driver)(nul_terminated.as_ptr() as *const c_char) })
}

/// Differential `driver`: runs both `.so`s on the same text and asserts the
/// captured stdout matches byte-for-byte. Returns the shared output.
pub fn diff_driver(text: &str, ctx: &str) -> Vec<u8> {
    let c_out = run_driver(c(), text);
    let r_out = run_driver(rs(), text);
    assert_eq!(
        c_out,
        r_out,
        "driver stdout mismatch [{ctx}]\n  input  = {:?}\n  C      = {:?}\n  Rust   = {:?}",
        Trunc(text),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
    c_out
}

/// Differential `call_fma`.
pub fn diff_call_fma(data: &[c_int], len: c_int, ctx: &str) -> c_int {
    let cv = unsafe { (c().call_fma)(data.as_ptr(), len) };
    let rv = unsafe { (rs().call_fma)(data.as_ptr(), len) };
    assert_eq!(
        cv, rv,
        "call_fma mismatch [{ctx}] len={len} data(first 16)={:?}",
        &data[..data.len().min(16)]
    );
    cv
}

/// Differential `fma_array`: both write into their own `out` buffer (prefilled
/// with the same sentinel pattern so writes *past* `len` would be caught too).
pub fn diff_fma_array(
    mul1: &[c_int],
    mul2: &[c_int],
    add: &[c_int],
    len: c_int,
    out_cap: usize,
    ctx: &str,
) -> Vec<c_int> {
    let sentinel: Vec<c_int> = (0..out_cap).map(|i| 0x5A5A_0000u32 as i32 ^ i as i32).collect();
    let mut c_out = sentinel.clone();
    let mut r_out = sentinel.clone();
    unsafe {
        (c().fma_array)(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
        (rs().fma_array)(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), len);
    }
    assert_eq!(
        c_out, r_out,
        "fma_array mismatch [{ctx}] len={len} out_cap={out_cap}"
    );
    c_out
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn usize_range(&mut self, lo: usize, hi: usize) -> usize {
        self.range(lo as i64, hi as i64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.usize_range(0, xs.len() - 1)]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub const EXTREMES: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];

/// Truncating `Debug` so failure messages stay readable for huge inputs.
pub struct Trunc<'a>(pub &'a str);

impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() <= 200 {
            write!(f, "{:?}", self.0)
        } else {
            write!(
                f,
                "{:?}… ({} bytes total)",
                &self.0[..200.min(self.0.len())],
                self.0.len()
            )
        }
    }
}
