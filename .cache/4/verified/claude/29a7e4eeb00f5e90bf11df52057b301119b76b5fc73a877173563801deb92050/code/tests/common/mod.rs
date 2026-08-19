//! Shared differential-test harness.
//!
//! Loads BOTH shared objects (the C reference and the Rust translation) with
//! `libloading` and calls every function only through its exported C ABI
//! symbol — never by calling Rust code directly — so the `#[no_mangle]`
//! `extern "C"` wrappers are part of what is under test.
//!
//! Every function in this library returns `void` and communicates solely by
//! writing to `stdout`, so "compare the outputs" means: redirect file
//! descriptor 1 to a temporary file, invoke the symbol, flush the C stdio
//! stream, and compare the captured bytes byte-for-byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open C output stream, which is what makes
    /// the captured bytes deterministic for both libraries (they share this
    /// process's single libc `stdout` FILE object).
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes fd-1 juggling across concurrently running tests.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test executable so it works
/// for debug and release alike.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-binary>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn scratch_dir() -> PathBuf {
    let d = target_profile_dir().join("difftest-scratch");
    fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// Newest modification time among `paths` (recursing into directories).
fn newest_mtime(paths: &[PathBuf]) -> std::time::SystemTime {
    fn walk(p: &std::path::Path, newest: &mut std::time::SystemTime) {
        let Ok(md) = fs::metadata(p) else { return };
        if md.is_dir() {
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    walk(&e.path(), newest);
                }
            }
        } else if let Ok(t) = md.modified() {
            if t > *newest {
                *newest = t;
            }
        }
    }
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    for p in paths {
        walk(p, &mut newest);
    }
    newest
}

/// `cargo test` does **not** rebuild the `cdylib` (`target/<profile>/libdriver.so`)
/// — it only builds the test binaries. Loading a stale `.so` would silently make
/// this entire differential suite vacuous (it would keep comparing the C library
/// against an old Rust build), so the harness rebuilds the `cdylib` itself and
/// then *verifies* it is newer than the sources.
pub fn ensure_rust_so_built() -> PathBuf {
    let profile_dir = target_profile_dir();
    let is_release = profile_dir.file_name().and_then(|s| s.to_str()) == Some("release");

    let mut args: Vec<String> = vec!["build".into(), "--offline".into()];
    if is_release {
        args.push("--release".into());
    }
    // Lets the feature-sweep script forward the exact feature flags in use, so
    // the `.so` under test is built with the same configuration as the tests.
    if let Ok(extra) = std::env::var("DIFFTEST_BUILD_ARGS") {
        args.extend(extra.split_whitespace().map(String::from));
    }

    let out = std::process::Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(manifest_dir())
        .output()
        .expect("spawn cargo build for the cdylib under test");
    assert!(
        out.status.success(),
        "`cargo {}` failed while rebuilding the cdylib under test:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );

    let so = profile_dir.join("libdriver.so");
    assert!(so.exists(), "cargo build did not produce {}", so.display());

    let src_time = newest_mtime(&[manifest_dir().join("src"), manifest_dir().join("Cargo.toml")]);
    let so_time = fs::metadata(&so).and_then(|m| m.modified()).expect("so mtime");
    assert!(
        so_time >= src_time,
        "{} is OLDER than src/ — the tests would run against a stale Rust build \
         and silently pass. Run `cargo build` (same profile/features) first.",
        so.display()
    );
    so
}

/// Same freshness guarantee for the C reference object: a stale C `.so` would
/// make the comparison meaningless in the other direction.
fn check_c_so_fresh(so: &PathBuf) {
    let src_time = newest_mtime(&[
        manifest_dir().join("c_src/src"),
        manifest_dir().join("c_src/include"),
        manifest_dir().join("c_src/CMakeLists.txt"),
    ]);
    let so_time = fs::metadata(so).and_then(|m| m.modified()).expect("c so mtime");
    assert!(
        so_time >= src_time,
        "{} is OLDER than c_src/ — rebuild the C reference library:\n  \
         cd c_src/build && cmake --build .",
        so.display()
    );
}

/// Both objects are named `libdriver.so`, and the C one carries
/// `SONAME libdriver.so`. Copy each to a distinct filename before `dlopen` so
/// the dynamic loader can never dedupe the two into a single mapping.
fn stage(src: &PathBuf, stem: &str) -> PathBuf {
    assert!(
        src.exists(),
        "missing shared object {}\n\
         build the C library with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         build the Rust library with:\n  cargo build",
        src.display()
    );
    let dst = scratch_dir().join(format!("lib{stem}.so"));
    fs::copy(src, &dst).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", src.display(), dst.display()));
    dst
}

/// The four exported entry points of one implementation.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    print_line: unsafe extern "C" fn(*const c_char),
    bad: unsafe extern "C" fn(),
    good: unsafe extern "C" fn(),
    driver: unsafe extern "C" fn(),
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            let print_line: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"printLine\0").expect("symbol printLine");
            let bad: Symbol<unsafe extern "C" fn()> = lib.get(b"bad\0").expect("symbol bad");
            let good: Symbol<unsafe extern "C" fn()> = lib.get(b"good\0").expect("symbol good");
            let driver: Symbol<unsafe extern "C" fn()> = lib.get(b"driver\0").expect("symbol driver");
            let (print_line, bad, good, driver) = (*print_line, *bad, *good, *driver);
            Impl { name, _lib: lib, print_line, bad, good, driver }
        }
    }

    /// Raw symbol addresses, used to prove the two libraries really are two
    /// distinct mappings (see `assert_distinct_mappings`).
    pub fn addrs(&self) -> [usize; 4] {
        [
            self.print_line as usize,
            self.bad as usize,
            self.good as usize,
            self.driver as usize,
        ]
    }

    /// `printLine(line)` with an arbitrary, possibly non-UTF-8, NUL-terminated
    /// byte string. `bytes` must already end in a NUL.
    pub unsafe fn print_line_raw(&self, bytes: *const c_char) {
        (self.print_line)(bytes)
    }
    pub unsafe fn bad(&self) {
        (self.bad)()
    }
    pub unsafe fn good(&self) {
        (self.good)()
    }
    pub unsafe fn driver(&self) {
        (self.driver)()
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_src = manifest_dir().join("c_src/build/libdriver.so");
        check_c_so_fresh(&c_src);
        let rust_src = ensure_rust_so_built();
        let c = Impl::load("C", stage(&c_src, "driver_c_ref"));
        let rust = Impl::load("Rust", stage(&rust_src, "driver_rs_ut"));
        assert_distinct_mappings(&c, &rust);
        Pair { c, rust }
    })
}

fn assert_distinct_mappings(c: &Impl, rust: &Impl) {
    for (i, (a, b)) in c.addrs().iter().zip(rust.addrs().iter()).enumerate() {
        assert_ne!(
            a, b,
            "symbol #{i} resolved to the same address in both libraries — the \
             loader deduped them, so the differential test would be vacuous"
        );
    }
}

/// Runs `f` with fd 1 redirected to a fresh temp file and returns everything
/// that was written to it.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = scratch_dir().join(format!(
        "cap-{}-{}.bin",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    unsafe {
        // Push out anything already buffered so it lands in the real stdout,
        // not in our capture file.
        fflush(std::ptr::null_mut());
    }

    let file = fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let bytes = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    bytes
}

/// Runs the same closure against the C impl and the Rust impl and asserts the
/// captured stdout bytes are identical.
pub fn assert_same<F: Fn(&Impl)>(what: &str, f: F) {
    let p = pair();
    let c_out = capture(|| f(&p.c));
    let rust_out = capture(|| f(&p.rust));
    if c_out != rust_out {
        panic!(
            "output mismatch for {what}\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
            c_out.len(),
            escape(&c_out),
            rust_out.len(),
            escape(&rust_out),
        );
    }
}

pub fn escape(bytes: &[u8]) -> String {
    let shown: &[u8] = if bytes.len() > 512 { &bytes[..512] } else { bytes };
    let mut s = String::new();
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > shown.len() {
        s.push_str(&format!("... (+{} more)", bytes.len() - shown.len()));
    }
    s
}

/// Minimal sequential test runner.
///
/// These suites use `harness = false` on purpose: the default libtest harness
/// runs test functions on several threads and writes its own `test ... ok`
/// progress lines to **fd 1**, which is the very file descriptor `capture()`
/// redirects. That leaks foreign bytes into a capture and produces bogus
/// mismatches. This runner executes cases strictly one at a time and reports
/// exclusively on **stderr**, so nothing but the library under test can ever
/// write to fd 1 during a capture.
pub struct Runner {
    filters: Vec<String>,
    ran: usize,
    skipped: usize,
    failures: Vec<String>,
    suite: &'static str,
}

impl Runner {
    pub fn new(suite: &'static str) -> Runner {
        let filters: Vec<String> = std::env::args()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .collect();
        eprintln!("\nrunning suite {suite}");
        Runner { filters, ran: 0, skipped: 0, failures: Vec::new(), suite }
    }

    pub fn case<F: FnOnce()>(&mut self, name: &str, f: F) {
        if !self.filters.is_empty() && !self.filters.iter().any(|p| name.contains(p.as_str())) {
            self.skipped += 1;
            return;
        }
        eprint!("test {name} ... ");
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match outcome {
            Ok(()) => {
                self.ran += 1;
                eprintln!("ok");
            }
            Err(_) => {
                self.ran += 1;
                self.failures.push(name.to_string());
                eprintln!("FAILED");
            }
        }
    }

    /// Prints the summary and terminates the process with the right exit code.
    pub fn finish(self) -> ! {
        eprintln!(
            "\nsuite {}: {} passed; {} failed; {} filtered out",
            self.suite,
            self.ran - self.failures.len(),
            self.failures.len(),
            self.skipped
        );
        if !self.failures.is_empty() {
            eprintln!("failures:");
            for f in &self.failures {
                eprintln!("    {f}");
            }
            std::process::exit(1);
        }
        std::process::exit(0);
    }
}

/// Builds a NUL-terminated C string from arbitrary non-NUL bytes.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    assert!(!bytes.contains(&0), "interior NUL is not expressible as a C string");
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// Deterministic xorshift64* PRNG — fixed seed keeps every randomized row
/// reproducible.
pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new() -> Rng {
        Rng(Self::SEED)
    }
    pub fn with_seed(seed: u64) -> Rng {
        Rng(if seed == 0 { Self::SEED } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform-ish in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    /// Random bytes drawn from `alphabet` (never contains NUL).
    pub fn bytes_from(&mut self, alphabet: &[u8], len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| alphabet[self.range(0, alphabet.len() - 1)])
            .collect()
    }
    /// Random bytes over the full expressible alphabet 0x01..=0xFF.
    pub fn nonzero_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let b = self.byte();
                if b == 0 {
                    1
                } else {
                    b
                }
            })
            .collect()
    }
}

pub const PRINTABLE: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 !#$&()*+,-./:;<=>?@[]^_{|}~";
pub const CONTROL: &[u8] = &[
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x7f,
];
pub const FORMAT_DIRECTIVES: &[&[u8]] = &[
    b"%s", b"%d", b"%n", b"%p", b"%%", b"%1$s", b"%.*s", b"%08x", b"%lu", b"%c", b"%hhn", b"%zu",
];
