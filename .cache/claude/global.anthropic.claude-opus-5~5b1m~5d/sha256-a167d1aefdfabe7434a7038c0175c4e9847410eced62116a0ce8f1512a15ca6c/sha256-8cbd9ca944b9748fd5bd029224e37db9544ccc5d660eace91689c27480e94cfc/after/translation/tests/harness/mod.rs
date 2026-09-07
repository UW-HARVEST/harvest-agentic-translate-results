// Shared differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and calls every function
// across the FFI boundary, exactly as an external C consumer would. No Rust
// function of the crate under test is ever called directly, so the
// `#[no_mangle] extern "C"` export wrappers are themselves under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used to capture whatever the loaded libraries write to fd 1.
// These are plain platform libc calls (the same glibc both `.so`s use for
// `puts`), NOT functions of the crate under test.
// ---------------------------------------------------------------------------
extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Locating the two libraries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../translation  ->  .../
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}.\nBuild it first:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Prefer the exact artifact cargo built for this test run, when available.
    if let Some(p) = std::env::var_os("CARGO_CDYLIB_FILE_DRIVER_driver") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    if let Some(p) = std::env::var_os("CARGO_CDYLIB_FILE_DRIVER") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    // Otherwise derive it from the test executable's own profile directory:
    //     target/<profile>/deps/<test-exe>  ->  target/<profile>/libdriver.so
    //
    // There is deliberately NO fallback to a different profile. `cargo test`
    // does not build the `cdylib` artifact, so a permissive search would let a
    // `cargo test` (debug) run silently load the *release* `.so` and report a
    // pass for a profile that was never actually exercised.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test exe must live in target/<profile>/deps/");
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {}.\n\
         `cargo test` does not build the cdylib artifact; build it for THIS profile first:\n  \
         cd translation && cargo build{}\n\
         (No cross-profile fallback is allowed: loading another profile's .so would\n\
          report a pass for a profile that was never exercised.)",
        p.display(),
        if profile_dir.file_name().and_then(|s| s.to_str()) == Some("release") {
            " --release"
        } else {
            ""
        }
    );
    p
}

/// Absolute paths of the two libraries actually under test, for auditing.
pub fn loaded_paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

// ---------------------------------------------------------------------------
// The exported ABI, as seen from outside
// ---------------------------------------------------------------------------

type FnPrintLine = unsafe extern "C" fn(*const c_char);
type FnVoid = unsafe extern "C" fn();
type FnDriver = unsafe extern "C" fn(c_int);

pub struct Api {
    pub which: &'static str,
    _lib: Library,
    print_line: FnPrintLine,
    bad: FnVoid,
    good: FnVoid,
    driver: FnDriver,
}

impl Api {
    fn load(which: &'static str, path: &Path) -> Api {
        // RTLD_NOW semantics: libloading::Library::new uses RTLD_LAZY|RTLD_LOCAL,
        // so resolve every symbol immediately below to prove they all exist.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        unsafe {
            let print_line: Symbol<FnPrintLine> = lib
                .get(b"printLine\0")
                .unwrap_or_else(|e| panic!("{which}: missing symbol `printLine`: {e}"));
            let bad: Symbol<FnVoid> = lib
                .get(b"bad\0")
                .unwrap_or_else(|e| panic!("{which}: missing symbol `bad`: {e}"));
            let good: Symbol<FnVoid> = lib
                .get(b"good\0")
                .unwrap_or_else(|e| panic!("{which}: missing symbol `good`: {e}"));
            let driver: Symbol<FnDriver> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{which}: missing symbol `driver`: {e}"));
            let (print_line, bad, good, driver) = (*print_line, *bad, *good, *driver);
            Api { which, _lib: lib, print_line, bad, good, driver }
        }
    }

    /// `printLine` with an explicit, possibly-NULL pointer.
    pub unsafe fn print_line_raw(&self, p: *const c_char) {
        (self.print_line)(p)
    }

    /// `printLine` with a NUL-terminated payload. `bytes` must not be required
    /// to be UTF-8; a trailing NUL is appended here (interior NULs are kept, so
    /// `puts` truncation behaviour is exercised faithfully).
    pub fn print_line(&self, bytes: &[u8]) {
        let mut buf = Vec::with_capacity(bytes.len() + 1);
        buf.extend_from_slice(bytes);
        buf.push(0);
        unsafe { (self.print_line)(buf.as_ptr() as *const c_char) }
    }

    pub fn bad(&self) {
        unsafe { (self.bad)() }
    }

    pub fn good(&self) {
        unsafe { (self.good)() }
    }

    pub fn driver(&self, use_good: c_int) {
        unsafe { (self.driver)(use_good) }
    }

    /// Raw address of the exported `driver` symbol, so a test can re-type the
    /// call signature (e.g. to put garbage in the upper half of `rdi`).
    pub fn driver_addr(&self) -> *const () {
        self.driver as *const ()
    }

    /// Whether `dlsym` on this library resolves `name` (NUL-terminated).
    pub fn has_symbol(&self, name: &[u8]) -> bool {
        unsafe { self._lib.get::<*const c_void>(name).is_ok() }
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", &c_so_path()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Api::load("Rust", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both libraries write through the *same* process-wide glibc `stdout`, so fd 1
// is redirected to a temporary file around each invocation and `fflush(NULL)`
// is used to drain every stdio buffer before and after.
// ---------------------------------------------------------------------------

/// Serialises captures: fd 1 is process-global state.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp_dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let tmp_path = format!(
        "{}/driver-difftest-{}-{:?}.out",
        tmp_dir.trim_end_matches('/'),
        std::process::id(),
        std::thread::current().id()
    );
    let mut c_path = tmp_path.clone().into_bytes();
    c_path.push(0);

    unsafe {
        // Drain anything already pending so it does not land in our file.
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let fd = open(c_path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({tmp_path}) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // Drain whatever the library buffered into the redirected fd 1.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        lseek(fd, 0, 0 /* SEEK_SET */);
        close(fd);
    }

    let mut out = Vec::new();
    std::fs::File::open(&tmp_path)
        .expect("reopen capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&tmp_path);
    out
}

/// Run the same closure against the C and the Rust `.so` and assert the bytes
/// written to stdout are identical.
#[track_caller]
pub fn assert_same<F: Fn(&Api)>(label: &str, f: F) -> Vec<u8> {
    let c_out = capture(|| f(c_api()));
    let rust_out = capture(|| f(rust_api()));
    if c_out != rust_out {
        panic!(
            "stdout divergence [{label}]\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
            c_out.len(),
            show(&c_out),
            rust_out.len(),
            show(&rust_out),
        );
    }
    c_out
}

pub fn show(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(160).collect();
    let mut s = String::new();
    for &x in &head {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > head.len() {
        s.push_str(&format!("...(+{} bytes)", b.len() - head.len()));
    }
    format!("\"{s}\"")
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0DE;

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform-ish in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    pub fn range_incl(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// Random byte in `0x01..=0xFF` (never NUL, so it cannot truncate).
    pub fn nonnul_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    pub fn printable_byte(&mut self) -> u8 {
        0x20 + (self.next_u64() % (0x7f - 0x20)) as u8
    }
    pub fn nonnul_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonnul_byte()).collect()
    }
    pub fn printable_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.printable_byte()).collect()
    }
}
