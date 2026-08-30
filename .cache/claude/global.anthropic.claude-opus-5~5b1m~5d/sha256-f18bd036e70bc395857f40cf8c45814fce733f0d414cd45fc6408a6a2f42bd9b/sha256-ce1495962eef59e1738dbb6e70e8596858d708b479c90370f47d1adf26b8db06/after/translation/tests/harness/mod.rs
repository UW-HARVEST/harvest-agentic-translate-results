// Shared differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and calls every function
// across the FFI boundary, exactly as an external C consumer would. The Rust
// implementation is NEVER called directly -- only through the `.so`'s exported
// symbols, so the `#[no_mangle]`/`extern "C"` wrappers are under test too.
//
// All five public functions return `void`; their entire observable behaviour is
// the byte stream they write to stdout. So the harness redirects fd 1 to a
// temporary file around each call, `fflush`es every stdio stream, restores fd 1
// and compares the captured bytes byte-for-byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams.
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type FnVoid = unsafe extern "C" fn();
pub type FnInt = unsafe extern "C" fn(c_int);
pub type FnStr = unsafe extern "C" fn(*const c_char);

/// The five exported entry points, resolved by name out of a `.so`.
pub struct Api {
    pub which: &'static str,
    pub print_line: FnStr,
    pub print_int_line: FnInt,
    pub bad: FnVoid,
    pub good: FnVoid,
    pub driver: FnInt,
}

impl Api {
    pub fn print_line(&self, s: *const c_char) {
        unsafe { (self.print_line)(s) }
    }
    pub fn print_int_line(&self, v: i32) {
        unsafe { (self.print_int_line)(v as c_int) }
    }
    pub fn bad(&self) {
        unsafe { (self.bad)() }
    }
    pub fn good(&self) {
        unsafe { (self.good)() }
    }
    pub fn driver(&self, use_good: i32) {
        unsafe { (self.driver)(use_good as c_int) }
    }
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("symbol {:?} missing: {e}", String::from_utf8_lossy(name)));
    *s
}

fn load(path: &PathBuf, which: &'static str) -> &'static Api {
    // RTLD_LOCAL (libloading's default) keeps the two libraries' identically
    // named symbols from colliding with each other.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    }));
    Box::leak(Box::new(unsafe {
        Api {
            which,
            print_line: sym(lib, b"printLine\0"),
            print_int_line: sym(lib, b"printIntLine\0"),
            bad: sym(lib, b"bad\0"),
            good: sym(lib, b"good\0"),
            driver: sym(lib, b"driver\0"),
        }
    }))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C ground-truth library built by `c_src/CMakeLists.txt`.
pub fn c_api() -> &'static Api {
    static C: OnceLock<&'static Api> = OnceLock::new();
    C.get_or_init(|| {
        let p = std::env::var("C_DYLIB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir().join("../c_src/build/libdriver.so"));
        assert!(
            p.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        load(&p, "C")
    })
}

/// The Rust cdylib under test, loaded from the profile dir of the running test
/// binary (`target/<profile>/libdriver.so`).
///
/// `cargo test` builds the test binaries but NOT the `cdylib` artifact, so if
/// the `.so` is absent we build it here with the same profile. That keeps a
/// bare `cargo test` working while still going through `dlopen`+`dlsym` only.
pub fn rust_api() -> &'static Api {
    static R: OnceLock<&'static Api> = OnceLock::new();
    R.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_DYLIB") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "RUST_DYLIB={} does not exist", p.display());
            return load(&p, "Rust");
        }

        // current_exe = target/<profile>/deps/<test>-<hash>
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|deps| deps.parent())
            .expect("profile dir")
            .to_path_buf();
        let p = profile_dir.join("libdriver.so");

        // ALWAYS rebuild, never just "build if missing": a stale `libdriver.so`
        // left over from an earlier `cargo build` would silently be tested
        // instead of the current `src/lib.rs`, so every source change -- and
        // every real bug -- would appear to pass. This is load-bearing.
        let profile = profile_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("debug")
            .to_string();
        let mut cmd =
            std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        cmd.arg("build")
            .arg("--offline")
            .current_dir(manifest_dir())
            // Never let cargo's own chatter reach fd 1: if this build were ever
            // triggered inside a capture window it would pollute the capture.
            .stdout(std::process::Stdio::null());
        if profile == "release" {
            cmd.arg("--release");
        } else if profile != "debug" {
            cmd.arg("--profile").arg(&profile);
        }
        // Mirror the feature selection the test binary was compiled with, so the
        // cdylib we load is built for the same feature combo.
        if let Ok(flags) = std::env::var("DRIVER_TEST_CARGO_FLAGS") {
            for f in flags.split_whitespace() {
                cmd.arg(f);
            }
        }
        let st = cmd.status().expect("failed to spawn cargo build for the cdylib");
        assert!(st.success(), "cargo build of the cdylib failed");

        assert!(
            p.exists(),
            "Rust cdylib not found at {}; build it with `cargo build`",
            p.display()
        );

        // Freshness guard: the artifact must be at least as new as the source.
        // If this ever trips, the tests would have been verifying stale code.
        if let (Ok(so), Ok(src)) = (
            std::fs::metadata(&p).and_then(|m| m.modified()),
            std::fs::metadata(manifest_dir().join("src/lib.rs")).and_then(|m| m.modified()),
        ) {
            assert!(
                so >= src,
                "STALE ARTIFACT: {} is older than src/lib.rs -- the tests would be \
                 verifying out-of-date code. Run `cargo build` (or delete target/).",
                p.display()
            );
        }

        load(&p, "Rust")
    })
}

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run `f` with fd 1 redirected to a temp file; return everything written.
/// Serialised, because fd 1 is process-global. Must not be nested.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Force both libraries to be loaded (and the cdylib built) BEFORE fd 1 is
    // retargeted -- otherwise the nested `cargo build` and `dlopen` diagnostics
    // would be written into the capture file. Neither of these calls re-enters
    // `capture`, so this cannot recurse.
    let _ = c_api();
    let _ = rust_api();

    // libtest writes its own progress text ("test foo ... ok") to fd 1. If test
    // threads ran in parallel, another thread's progress line would land inside
    // our capture window and corrupt the comparison. `.cargo/config.toml` pins
    // RUST_TEST_THREADS=1; enforce it loudly rather than silently mis-compare.
    let threads = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        threads, "1",
        "these tests capture the process-global fd 1 and therefore require \
         RUST_TEST_THREADS=1 (set in translation/.cargo/config.toml); got {threads:?}. \
         Re-run as `cargo test` from the crate root, or `cargo test -- --test-threads=1` \
         with RUST_TEST_THREADS=1 exported."
    );

    // Push libtest's partial, newline-less "test foo ... " out to the real fd 1
    // before we retarget it: Rust's stdout is a LineWriter and would otherwise
    // flush that text into our capture file later.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("driver_cap_{}_{n}.bin", std::process::id()));
    let file = File::create(&path).expect("create temp capture file");
    let fd = file.as_raw_fd();

    unsafe {
        // Don't let previously buffered bytes leak into this capture.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // Force stdio to hand everything to the (now file-backed) fd 1.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    drop(file);
    let out = std::fs::read(&path).expect("read temp capture file");
    let _ = std::fs::remove_file(&path);
    out
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 400 {
        s.push_str(&format!("... ({} bytes total)", b.len()));
    }
    s
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a.iter().zip(b.iter()).position(|(x, y)| x != y);
    match at {
        Some(i) => format!(
            "first differing byte at offset {i}: C=0x{:02x} Rust=0x{:02x}",
            a[i], b[i]
        ),
        None => format!("common prefix equal; lengths differ: C={} Rust={}", a.len(), b.len()),
    }
}

/// Apply the same sequence of calls to the C `.so` and to the Rust `.so` and
/// assert the captured stdout bytes are identical.
pub fn assert_same<F: Fn(&Api)>(label: &str, f: F) {
    let c = capture(|| f(c_api()));
    let r = capture(|| f(rust_api()));
    assert!(
        c == r,
        "DIVERGENCE in [{label}]\n  {}\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
        first_diff(&c, &r),
        c.len(),
        show(&c),
        r.len(),
        show(&r),
    );
}

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub const SEED: u64 = 0x5EED_1234_5EED_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A byte in `0x01..=0xff` (never NUL: a NUL would terminate the C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
}

/// NUL-terminate `payload` and hand the raw pointer to `f`.
pub fn with_cstr<R, F: FnOnce(*const c_char) -> R>(payload: &[u8], f: F) -> R {
    let mut buf = Vec::with_capacity(payload.len() + 1);
    buf.extend_from_slice(payload);
    buf.push(0);
    f(buf.as_ptr() as *const c_char)
}

/// `printLine(payload)` on both libraries, compared byte-for-byte.
pub fn assert_same_line(label: &str, payload: &[u8]) {
    assert_same(label, |api| with_cstr(payload, |p| api.print_line(p)));
}

/// Split `items` into chunks and compare each chunk in one capture, so a
/// divergence stays easy to localise while still exercising cross-call
/// ordering and stdio buffering.
pub fn assert_same_chunked<T, F>(label: &str, items: &[T], chunk: usize, apply: F)
where
    T: Clone,
    F: Fn(&Api, &T) + Copy,
{
    for (i, group) in items.chunks(chunk).enumerate() {
        assert_same(&format!("{label} chunk {i}"), |api| {
            for it in group {
                apply(api, it);
            }
        });
    }
}
