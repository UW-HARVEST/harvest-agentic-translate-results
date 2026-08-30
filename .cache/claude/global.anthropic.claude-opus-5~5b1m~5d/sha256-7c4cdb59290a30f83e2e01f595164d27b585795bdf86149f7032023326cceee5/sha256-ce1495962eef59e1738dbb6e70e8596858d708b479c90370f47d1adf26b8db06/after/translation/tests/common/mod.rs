// Shared differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and calls `driver` only via
// the `dlsym`-resolved symbol, exactly as an external C consumer would. The Rust
// implementation is never called directly as a Rust function, so the
// `#[no_mangle] extern "C"` export wrapper is under test too.
//
// `driver` communicates entirely through `stdout`, so "compare the outputs"
// means capturing file descriptor 1 around each call and comparing the bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which saves us having
    /// to bind the `stdout` global itself.
    fn fflush(stream: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

pub const LC_ALL: c_int = 6; // glibc value
pub const LC_NUMERIC: c_int = 1; // glibc value

/// `void driver(double f)` as seen through the ABI.
pub type DriverFn = unsafe extern "C" fn(f64);

pub struct Impls {
    pub c: DriverFn,
    pub rust: DriverFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Resolve the Rust `.so` built by *this* cargo invocation, so the test always
/// exercises the profile/feature set it was compiled under. The test executable
/// lives in `target/<profile>/deps/`, so the cdylib sits one level up.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<test-bin>");
    let cand = profile_dir.join("libdriver.so");
    if cand.exists() {
        return cand;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for prof in ["release", "debug"] {
        let p = manifest.join("target").join(prof).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found (looked in {}). Run `cargo build` first.",
        profile_dir.display()
    );
}

pub fn impls() -> &'static Impls {
    static I: OnceLock<Impls> = OnceLock::new();
    I.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        // Leaked so the fn pointers stay valid for the whole process and the
        // libraries are never unloaded mid-test. Both are opened RTLD_LOCAL by
        // libloading, so the two identically named `driver` symbols cannot
        // collide -- each is resolved against its own handle.
        let c_lib: &'static Library = Box::leak(Box::new(
            Library::new(&c_path).expect("dlopen C libdriver.so"),
        ));
        let rust_lib: &'static Library = Box::leak(Box::new(
            Library::new(&rust_path).expect("dlopen Rust libdriver.so"),
        ));

        let c_sym: Symbol<DriverFn> = c_lib.get(b"driver\0").expect("dlsym C driver");
        let rust_sym: Symbol<DriverFn> = rust_lib.get(b"driver\0").expect("dlsym Rust driver");

        Impls {
            c: *c_sym,
            rust: *rust_sym,
            c_path,
            rust_path,
        }
    })
}

/// fd 1 is process-global state, so captures must not overlap. `cargo test`
/// runs test fns on parallel threads, hence the lock.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Redirect fd 1 to a temp file, run `f`, restore fd 1, return what was written.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let bytes = unsafe {
        // Don't let previously buffered output land in our capture file. Two
        // separate buffers sit in front of fd 1 and BOTH must be drained first:
        //   * Rust's `std::io::Stdout` LineWriter (holds partial lines), and
        //   * libc's stdio buffer for `stdout` (`fflush(NULL)` flushes all
        //     output streams, which saves binding the `stdout` global).
        // Skipping the Rust-side flush lets a buffered partial line be emitted
        // into the capture file later, corrupting the comparison.
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        fflush(std::ptr::null_mut());

        let file = std::fs::File::create(&path).expect("create capture file");
        let fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 onto fd 1 failed");

        f();

        // Flush the library's stdio buffer *before* putting fd 1 back.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        drop(file);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    };

    let _ = std::fs::remove_file(&path);
    drop(guard);
    bytes
}

/// Call one implementation once per value, capturing all output in one go.
fn run_all(driver: DriverFn, bits: &[u64]) -> Vec<u8> {
    capture_stdout(|| {
        for &b in bits {
            unsafe { driver(f64::from_bits(b)) };
        }
    })
}

/// The core differential assertion: for every bit pattern, the C `.so` and the
/// Rust `.so` must emit byte-identical output.
///
/// `driver` prints exactly one line per call, so the batched output can be split
/// on newlines and attributed back to individual inputs, which keeps failure
/// messages precise while staying fast.
pub fn assert_same(row: &str, bits: &[u64]) {
    let i = impls();
    for chunk in bits.chunks(4096) {
        let c_out = run_all(i.c, chunk);
        let r_out = run_all(i.rust, chunk);

        if c_out == r_out {
            continue;
        }

        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();

        for (idx, &b) in chunk.iter().enumerate() {
            let cl = c_lines.get(idx).copied().unwrap_or(b"<missing>");
            let rl = r_lines.get(idx).copied().unwrap_or(b"<missing>");
            if cl != rl {
                panic!(
                    "[{row}] output mismatch for input bits 0x{b:016x} (f64 = {:?}):\n  \
                     C   : {:?}\n  Rust: {:?}\n  (C .so: {}, Rust .so: {})",
                    f64::from_bits(b),
                    String::from_utf8_lossy(cl),
                    String::from_utf8_lossy(rl),
                    i.c_path.display(),
                    i.rust_path.display(),
                );
            }
        }
        panic!(
            "[{row}] outputs differ in total length but every line matched \
             (C {} bytes / {} lines vs Rust {} bytes / {} lines)",
            c_out.len(),
            c_lines.len(),
            r_out.len(),
            r_lines.len()
        );
    }

    // Sanity: one line of output per call, so a silently-dropped or duplicated
    // print would be caught even if C and Rust agreed with each other.
    let probe = run_all(i.c, &bits[..bits.len().min(64)]);
    let expected = bits.len().min(64);
    let got = probe.iter().filter(|&&b| b == b'\n').count();
    assert_eq!(
        got, expected,
        "[{row}] expected exactly one output line per call, got {got} for {expected} calls"
    );
}

/// Convenience for rows expressed as `f64` literals.
pub fn assert_same_f64(row: &str, vals: &[f64]) {
    let bits: Vec<u64> = vals.iter().map(|v| v.to_bits()).collect();
    assert_same(row, &bits);
}

/// Assert both implementations agree *and* that the shared output matches an
/// exact expected string. Used by the error-surface rows, where ERRORS.md
/// records the precise expected rendering.
pub fn assert_same_and_eq(row: &str, bits: u64, expected: &str) {
    assert_same(row, &[bits]);
    let i = impls();
    let c_out = run_all(i.c, &[bits]);
    let got = String::from_utf8_lossy(&c_out).to_string();
    assert_eq!(
        got, expected,
        "[{row}] C output for bits 0x{bits:016x} was {got:?}, expected {expected:?}"
    );
}

/// Write a marker line to `stdout` through libc, for the interleaving row.
pub fn libc_print(s: &str) {
    let cs = std::ffi::CString::new(s).unwrap();
    unsafe { printf(b"%s\0".as_ptr() as *const c_char, cs.as_ptr()) };
}

pub fn try_setlocale(category: c_int, name: &str) -> bool {
    let cs = std::ffi::CString::new(name).unwrap();
    unsafe { !setlocale(category, cs.as_ptr()).is_null() }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_D1FF_C0FF_EE01;

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A random 52-bit mantissa field.
    pub fn mantissa(&mut self) -> u64 {
        self.next_u64() & 0x000F_FFFF_FFFF_FFFF
    }
    pub fn sign(&mut self) -> u64 {
        (self.next_u64() & 1) << 63
    }
}

/// Assemble a double from its IEEE-754 fields.
pub fn compose(sign: u64, exp: u64, mantissa: u64) -> u64 {
    ((sign & 1) << 63) | ((exp & 0x7FF) << 52) | (mantissa & 0x000F_FFFF_FFFF_FFFF)
}
