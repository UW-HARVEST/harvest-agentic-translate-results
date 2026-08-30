// Shared differential-testing harness.
//
// Loads BOTH shared objects (the C `libdriver.so` and the Rust `libdriver.so`)
// with `libloading` and calls them only through their exported `driver` symbol,
// exactly as an external consumer would. Nothing in this crate is called
// directly as a Rust function, so the `#[no_mangle]`/`extern "C"` export
// wrapper is under test too.
//
// `driver` returns `void` and communicates solely by writing to stdout via
// libc `printf`, so the observable output is captured at the file-descriptor
// level (dup/dup2 on fd 1) around each call and compared byte-for-byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
    ];
    let so = first_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "C shared library not found. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             Looked in: {:?}",
            candidates
        )
    });
    assert_fresh(
        &so,
        &[root.join("c_src/src"), root.join("c_src/include")],
        "cd c_src/build && cmake --build .",
    );
    so
}

/// Path to the Rust `cdylib`. Prefers the profile the test itself was built
/// with, but accepts the other one so a `--release` .so can be tested from a
/// debug test binary and vice versa.
pub fn rust_so_path() -> PathBuf {
    let base = manifest_dir().join("target");
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    let candidates = [
        base.join(first).join("libdriver.so"),
        base.join(second).join("libdriver.so"),
    ];
    let so = first_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found. Build it with `cargo build` / `cargo build --release`.\n\
             Looked in: {:?}",
            candidates
        )
    });
    assert_fresh(
        &so,
        &[manifest_dir().join("src"), manifest_dir().join("Cargo.toml")],
        "cargo build   (or use ./run_all_feature_combos.sh)",
    );
    so
}

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

fn newest_mtime(path: &Path, newest: &mut std::time::SystemTime) {
    let Ok(md) = std::fs::metadata(path) else { return };
    if md.is_dir() {
        if let Ok(rd) = std::fs::read_dir(path) {
            for e in rd.flatten() {
                newest_mtime(&e.path(), newest);
            }
        }
    } else if let Ok(t) = md.modified() {
        if t > *newest {
            *newest = t;
        }
    }
}

/// Guards against the single most dangerous failure mode of this harness:
/// silently testing a STALE shared object.
///
/// `cargo test` does not rebuild the `cdylib`, because no test target links it —
/// the tests reach it through `dlopen`. Without this check, editing `src/lib.rs`
/// and re-running `cargo test` would re-test the previously built `.so` and every
/// case would pass no matter what the new source says (verified: a deliberate
/// `bedrooms = 4` mutant survived the whole suite before this guard existed).
fn assert_fresh(so: &Path, sources: &[PathBuf], rebuild_cmd: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH);
    let mut newest_src = std::time::UNIX_EPOCH;
    for s in sources {
        newest_mtime(s, &mut newest_src);
    }
    assert!(
        so_mtime >= newest_src,
        "STALE ARTIFACT: {:?} is older than its sources {:?}.\n\
         The differential test would be meaningless (it would exercise the previous build).\n\
         Rebuild first with:  {}",
        so,
        sources,
        rebuild_cmd
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: the libraries are dlopen'd once and never unloaded; `driver` holds no
// state, so concurrent use across test threads is sound. stdout capture is
// serialized separately (see CAPTURE_LOCK).
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so");
        Libs { c, rust }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .c
            .get(b"driver\0")
            .expect("symbol `driver` missing from the C .so")
    }
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so (missing #[no_mangle] export?)")
    }
}

/// Redirecting fd 1 is process-global, so only one capture may be in flight.
fn capture_lock() -> &'static std::sync::Mutex<()> {
    static L: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    L.get_or_init(|| std::sync::Mutex::new(()))
}

/// Runs `f` with fd 1 redirected to a temporary file and returns everything
/// written to it, including anything buffered inside libc's `stdout` FILE.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.bin",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));

    let out = {
        let file = std::fs::File::create(&path).expect("failed to create capture file");
        let file_fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        unsafe {
            // Flush anything already pending so it is not misattributed to us.
            // Rust's `stdout` has its own LineWriter buffer that can still hold a
            // newline-less fragment (e.g. a progress line); it must be drained
            // BEFORE fd 1 is redirected, or it would land in the capture file.
            {
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file_fd, 1) >= 0, "dup2 onto fd 1 failed");

            f();

            // The library writes with libc printf; once fd 1 is a regular file
            // the stream is fully buffered, so an explicit flush is required.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "failed to restore fd 1");
            close(saved);
        }

        drop(file);
        std::fs::read(&path).expect("failed to read capture file")
    };

    let _ = std::fs::remove_file(&path);
    drop(guard);
    out
}

/// Calls the C `driver(x)` through the C `.so` and returns its stdout bytes.
pub fn run_c(x: i32) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe { f(x as c_int) })
}

/// Calls the Rust `driver(x)` through the Rust `.so` and returns its stdout bytes.
pub fn run_rust(x: i32) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe { f(x as c_int) })
}

/// Calls `driver(x)` for every `x` in `xs`, in order, in a single capture.
pub fn run_c_seq(xs: &[i32]) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe {
        for &x in xs {
            f(x as c_int);
        }
    })
}

/// Calls `driver(x)` for every `x` in `xs`, in order, in a single capture.
pub fn run_rust_seq(xs: &[i32]) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe {
        for &x in xs {
            f(x as c_int);
        }
    })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// The core differential assertion: C and Rust must emit identical bytes.
pub fn assert_same(x: i32) -> Vec<u8> {
    let c = run_c(x);
    let r = run_rust(x);
    assert_eq!(
        c,
        r,
        "DIVERGENCE for driver({x}) (0x{:08x}):\n  C   : \"{}\"\n  Rust: \"{}\"",
        x as u32,
        show(&c),
        show(&r)
    );
    check_shape(x, &c);
    c
}

pub fn assert_same_all(label: &str, xs: &[i32]) {
    for &x in xs {
        let c = run_c(x);
        let r = run_rust(x);
        assert_eq!(
            c,
            r,
            "[{label}] DIVERGENCE for driver({x}) (0x{:08x}):\n  C   : \"{}\"\n  Rust: \"{}\"",
            x as u32,
            show(&c),
            show(&r)
        );
        check_shape(x, &c);
    }
}

/// Independent model of what the C must print, from the struct layout:
/// `{ int floors; int bedrooms = 3; double bathrooms = 2.0; }`, 16 bytes,
/// offsets 0/4/8, little-endian, hex-encoded with `%02x`, then `'\n'`.
pub fn expected_output(x: i32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&x.to_le_bytes());
    bytes.extend_from_slice(&3i32.to_le_bytes());
    bytes.extend_from_slice(&2.0f64.to_le_bytes());
    let mut s: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    s.push('\n');
    s.into_bytes()
}

/// Invariants the C holds for every input (CONFIGS.md row C16).
pub fn check_shape(x: i32, out: &[u8]) {
    assert_eq!(
        out.len(),
        33,
        "driver({x}) output should be 32 hex chars + newline, got {} bytes: {:?}",
        out.len(),
        show(out)
    );
    assert_eq!(out[32], b'\n', "driver({x}) output must end with a newline");
    let hex = std::str::from_utf8(&out[..32]).expect("output must be ASCII hex");
    assert!(
        hex.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "driver({x}) must emit lowercase hex only, got {hex:?}"
    );
    assert_eq!(&hex[8..16], "03000000", "bedrooms field must be 3 (LE)");
    assert_eq!(
        &hex[16..32], "0000000000000040",
        "bathrooms field must be IEEE-754 2.0 (LE)"
    );
    let expect = expected_output(x);
    assert_eq!(
        out,
        expect.as_slice(),
        "driver({x}) disagrees with the struct-layout model:\n  got   : \"{}\"\n  model : \"{}\"",
        show(out),
        show(&expect)
    );
}

/// SplitMix64 — deterministic, seeded, dependency-free PRNG for property tests.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive, computed over the unsigned range so it
    /// works across the full `i32` span without overflow.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Minimal sequential test runner for `harness = false` test targets.
///
/// Redirecting fd 1 is process-global, so libtest's default multi-threaded
/// harness (whose own progress lines are written from other threads) would
/// interleave its output into the capture file. These suites therefore run
/// without libtest: strictly one case at a time, with nothing else writing to
/// stdout while a capture is in flight.
pub fn run_suite(suite: &str, tests: &[(&str, fn())]) {
    use std::io::Write;
    println!("\nrunning {} {} case(s)", tests.len(), suite);
    let mut failed: Vec<&str> = Vec::new();
    for (name, f) in tests {
        print!("  {suite}::{name} ... ");
        let _ = std::io::stdout().flush();
        let prev = std::panic::take_hook();
        let msgs = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let sink = std::sync::Arc::clone(&msgs);
        std::panic::set_hook(Box::new(move |info| {
            sink.lock().unwrap().push(info.to_string());
        }));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::panic::set_hook(prev);
        match result {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                for m in msgs.lock().unwrap().iter() {
                    eprintln!("---- {suite}::{name} ----\n{m}\n");
                }
                failed.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }
    println!(
        "{suite} result: {} passed; {} failed",
        tests.len() - failed.len(),
        failed.len()
    );
    let _ = std::io::stdout().flush();
    if !failed.is_empty() {
        eprintln!("{suite} failures: {failed:?}");
        std::process::exit(1);
    }
}

/// Reads the exported dynamic symbols of an object with `nm -D --defined-only`.
pub fn dynamic_symbols(so: &Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .expect("failed to run `nm` (is binutils installed?)");
    assert!(
        out.status.success(),
        "nm failed on {:?}: {}",
        so,
        String::from_utf8_lossy(&out.stderr)
    );
    let mut syms: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    syms.sort();
    syms.dedup();
    syms
}
