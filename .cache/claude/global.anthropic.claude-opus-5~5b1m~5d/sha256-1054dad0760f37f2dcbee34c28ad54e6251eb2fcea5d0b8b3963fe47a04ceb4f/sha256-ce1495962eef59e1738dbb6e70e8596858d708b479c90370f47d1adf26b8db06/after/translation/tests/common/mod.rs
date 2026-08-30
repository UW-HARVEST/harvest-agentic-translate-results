// Shared differential-test harness.
//
// Loads BOTH shared objects through `libloading` and calls the exported
// `driver` symbol via FFI only -- the Rust implementation is NEVER called
// directly, so the `#[no_mangle] extern "C"` wrapper is exercised too.
//
// `driver` communicates purely through side effects on `stdout`, so the
// harness redirects fd 1 to a temp file around each call and returns the
// captured bytes.

#![allow(dead_code)]

use std::ffi::c_int;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    // Flushing with a NULL argument flushes *all* open output streams,
    // including the single process-wide libc `stdout` that both the C `.so`
    // and the Rust `.so` write through.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int);

/// fd-1 redirection is process-global state, and `cargo test` runs the tests
/// inside one binary on multiple threads, so every capture must be serialized.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built from `c_src/`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so")
}

/// Path to the Rust `cdylib`. Prefer an explicit override, then the release
/// artifact, then the debug artifact that `cargo test` itself produced.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let cand = base.join(profile).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib not found under {}; run `cargo build --release` first",
        base.display()
    );
}

fn c_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared library missing at {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

fn rust_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let p = rust_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

/// `driver` as exported by the C `.so`.
pub fn c_driver() -> DriverFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| {
        let sym: Symbol<DriverFn> = unsafe { c_lib().get(b"driver\0") }
            .expect("C .so must export `driver`");
        unsafe { *sym.into_raw() as usize }
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

/// `driver` as exported by the Rust `.so` (goes through the `#[no_mangle]`
/// `extern "C"` wrapper, exactly like any external C consumer).
pub fn rust_driver() -> DriverFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| {
        let sym: Symbol<DriverFn> = unsafe { rust_lib().get(b"driver\0") }
            .expect("Rust .so must export `driver`");
        unsafe { *sym.into_raw() as usize }
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

/// Returns true iff `name` is a resolvable dynamic symbol in the C `.so`.
pub fn c_has_symbol(name: &[u8]) -> bool {
    let mut buf = name.to_vec();
    buf.push(0);
    unsafe { c_lib().get::<*const ()>(&buf) }.is_ok()
}

/// Returns true iff `name` is a resolvable dynamic symbol in the Rust `.so`.
pub fn rust_has_symbol(name: &[u8]) -> bool {
    let mut buf = name.to_vec();
    buf.push(0);
    unsafe { rust_lib().get::<*const ()>(&buf) }.is_ok()
}

/// Run `f` with fd 1 redirected to a temp file; return everything written to
/// it, including bytes written by libc `printf`/`putchar` inside either `.so`.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Make sure nothing already buffered leaks into our capture window: drain
    // both Rust's own `stdout` buffer and every libc stream.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{:?}.bin",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture temp file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Push libc's stdout buffer out to the (redirected) fd 1 before restoring.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe { close(saved) };
    drop(file);

    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture temp file")
        .read_to_end(&mut bytes)
        .expect("read capture temp file");
    let _ = std::fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    bytes
}

/// Output of the C `driver` for one argument.
pub fn c_out(x: i32) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe { f(x) })
}

/// Output of the Rust `driver` for one argument.
pub fn rust_out(x: i32) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe { f(x) })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Core differential assertion: byte-identical output from both `.so`s.
pub fn assert_same(x: i32, row: &str) {
    let c = c_out(x);
    let r = rust_out(x);
    assert_eq!(
        c,
        r,
        "[{row}] divergence for driver({x}) (0x{x:08x}):\n  C   : {}\n  Rust: {}",
        show(&c),
        show(&r)
    );
    // Shape invariants from CONFIGS.md row C23, checked on the C output (the
    // ground truth) and therefore transitively on the Rust output.
    assert_eq!(c.len(), 33, "[{row}] driver({x}) should emit 33 bytes");
    assert_eq!(c[32], b'\n', "[{row}] driver({x}) must end with a newline");
    assert!(
        c[..32].iter().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
        "[{row}] driver({x}) must emit 32 lowercase hex digits, got {}",
        show(&c)
    );
    // bedrooms == 3, bathrooms == 2.0 (IEEE-754 little endian)
    assert_eq!(&c[8..16], b"03000000", "[{row}] bedrooms field changed");
    assert_eq!(
        &c[16..32],
        b"0000000000000040",
        "[{row}] bathrooms field changed"
    );
    // The floors field must be the little-endian encoding of x.
    let expected: String = x
        .to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        &c[..8],
        expected.as_bytes(),
        "[{row}] floors field mismatch for {x}"
    );
}

/// Assert both `.so`s agree over a whole batch of arguments.
pub fn assert_same_all<I: IntoIterator<Item = i32>>(xs: I, row: &str) {
    for x in xs {
        assert_same(x, row);
    }
}

/// Deterministic SplitMix64 PRNG so every randomized row is reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform-ish value in `lo..=hi`.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i32 {
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// Minimal single-threaded test runner (`harness = false`).
//
// libtest would run these cases on several threads and print its own progress
// text to fd 1, which lands inside the stdout capture windows above and
// corrupts every comparison. This runner executes cases strictly sequentially
// and reports on STDERR, leaving fd 1 untouched except inside `capture_stdout`.
// ---------------------------------------------------------------------------

pub type Case = (&'static str, fn());

/// Run `cases` sequentially and exit with a non-zero status if any fails.
/// An optional substring filter can be passed on the command line, mirroring
/// `cargo test -- <filter>`.
pub fn run_suite(suite: &str, cases: &[Case]) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let selected: Vec<&Case> = cases
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();

    eprintln!("\nrunning {} case(s) in `{suite}`", selected.len());
    let mut passed = 0usize;
    let mut failed: Vec<&str> = Vec::new();

    for (name, f) in &selected {
        eprint!("  {name} ... ");
        let _ = std::io::stderr().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f)) {
            Ok(()) => {
                eprintln!("ok");
                passed += 1;
            }
            Err(_) => {
                // The panic message was already printed by the default hook.
                eprintln!("FAILED");
                failed.push(name);
            }
        }
    }

    eprintln!(
        "\n{suite} result: {}. {passed} passed; {} failed\n",
        if failed.is_empty() { "ok" } else { "FAILED" },
        failed.len()
    );
    if !failed.is_empty() {
        eprintln!("failures:");
        for name in &failed {
            eprintln!("    {name}");
        }
        std::process::exit(1);
    }
}
