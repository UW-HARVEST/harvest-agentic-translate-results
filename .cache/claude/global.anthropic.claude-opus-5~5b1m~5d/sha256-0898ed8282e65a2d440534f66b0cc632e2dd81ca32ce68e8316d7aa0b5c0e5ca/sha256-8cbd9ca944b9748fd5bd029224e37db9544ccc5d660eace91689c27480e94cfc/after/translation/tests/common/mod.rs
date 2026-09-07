//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! purely through their exported `extern "C"` symbols — the Rust crate is never
//! called directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// The C `foo_t` layout, as produced by gcc/clang on the SysV/ELF ABI:
//
//   typedef struct {
//       unsigned int x : 2;   // byte 0, bits 0..1
//       unsigned int y : 3;   // byte 0, bits 2..4
//       bool         b : 1;   // byte 0, bit  5
//       int          z;       // offset 4
//   } foo_t;                  // sizeof == 8, alignof == 4
//
// The raw 8-byte image is what `print_foo` receives, so tests build it by hand
// (including the padding bits/bytes the C code must ignore).
// ---------------------------------------------------------------------------

pub const FOO_SIZE: usize = 8;

/// Build the raw 8-byte `foo_t` image from an explicit storage byte, the three
/// bytes of inter-field padding, and `z`.
pub fn foo_image(storage: u8, pad: [u8; 3], z: i32) -> [u8; FOO_SIZE] {
    let mut img = [0u8; FOO_SIZE];
    img[0] = storage;
    img[1] = pad[0];
    img[2] = pad[1];
    img[3] = pad[2];
    img[4..8].copy_from_slice(&z.to_ne_bytes());
    img
}

/// The storage byte the C `driver` builds for the given arguments:
/// `x & 0x3` into bits 0..1, `y & 0x7` into bits 2..4, `b & 0x1` into bit 5.
/// (Bits 6..7 are left uninitialised by the C code, hence 0 here.)
pub fn packed_storage(x: u32, y: u32, b: u8) -> u8 {
    ((x as u8) & 0x3) | (((y as u8) & 0x7) << 2) | ((b & 0x1) << 5)
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is two levels up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to the release build.
    manifest_dir().join("target/release/libdriver.so")
}

/// `void driver(unsigned int x, unsigned int y, bool b, int z)`
///
/// `bool` is taken as `u8` so tests can pass non-canonical `_Bool` bytes.
pub type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
/// `void print_foo(const foo_t *foo)`
pub type PrintFooFn = unsafe extern "C" fn(*const u8);

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub driver: DriverFn,
    pub print_foo: PrintFooFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{name} shared library not found at {}",
            path.display()
        );
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({name}): {e}", path.display()));
            let driver: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"));
            let print_foo: Symbol<PrintFooFn> = lib
                .get(b"print_foo\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `print_foo`: {e}"));
            let driver = *driver;
            let print_foo = *print_foo;
            Impl {
                name,
                _lib: lib,
                driver,
                print_foo,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

/// Both libraries, loaded once per test process.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &c_so_path()),
        rust: Impl::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both `.so`s call glibc's `printf`, which writes to the process-wide `stdout`
// FILE*.  Redirecting fd 1 to a temp file around the call therefore captures
// the bytes from either implementation identically.
// ---------------------------------------------------------------------------

extern "C" {
    fn fflush(stream: *mut libc::FILE) -> c_int;
}

/// fd 1 is process-global, so captures must not overlap. `cargo test` runs
/// tests on parallel threads, hence this lock.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f`, returning everything it wrote to `stdout` as raw bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush anything the harness itself buffered so it does not leak in:
        // Rust's own `stdout` buffer first, then every C stream.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut()); // fflush(NULL) => flush all streams
        let mut file = tempfile();
        let tmp_fd = as_raw_fd(&file);

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2 onto stdout failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restoring stdout failed");
        libc::close(saved);

        file.seek(SeekFrom::Start(0)).expect("seek temp file");
        let mut out = Vec::new();
        file.read_to_end(&mut out).expect("read temp file");
        out
    }
}

fn as_raw_fd(f: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    f.as_raw_fd()
}

/// An unlinked temporary file (no external crate needed).
fn tempfile() -> std::fs::File {
    use std::os::unix::ffi::OsStrExt;
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let mut name = dir;
    name.push(format!(
        "driver-difftest-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&name)
        .unwrap_or_else(|e| panic!("cannot create temp file {}: {e}", name.display()));
    // Unlink immediately; the fd keeps it alive.
    unsafe {
        let mut c_name: Vec<u8> = name.as_os_str().as_bytes().to_vec();
        c_name.push(0);
        libc::unlink(c_name.as_ptr() as *const c_char);
    }
    file
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `driver(x, y, b, z)` on one implementation, capturing its stdout.
pub fn run_driver(imp: &Impl, x: u32, y: u32, b: u8, z: i32) -> Vec<u8> {
    capture_stdout(|| unsafe { (imp.driver)(x, y, b, z) })
}

/// Call `print_foo(&image)` on one implementation, capturing its stdout.
pub fn run_print_foo(imp: &Impl, image: &[u8; FOO_SIZE]) -> Vec<u8> {
    capture_stdout(|| unsafe { (imp.print_foo)(image.as_ptr()) })
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Assert C and Rust `driver` produce byte-identical stdout.
#[track_caller]
pub fn assert_driver_eq(x: u32, y: u32, b: u8, z: i32) {
    let p = pair();
    let c = run_driver(&p.c, x, y, b, z);
    let r = run_driver(&p.rust, x, y, b, z);
    assert_eq!(
        c,
        r,
        "driver(x={x:#x}, y={y:#x}, b={b:#x}, z={z}) diverged:\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c),
        show(&r)
    );
    // Sanity: the C implementation always emits exactly one line.
    assert!(
        c.ends_with(b"\n") && c.iter().filter(|&&ch| ch == b'\n').count() == 1,
        "unexpected C output shape: \"{}\"",
        show(&c)
    );
}

/// Assert C and Rust `print_foo` produce byte-identical stdout for a raw image.
#[track_caller]
pub fn assert_print_foo_eq(storage: u8, pad: [u8; 3], z: i32) {
    let p = pair();
    let img = foo_image(storage, pad, z);
    let c = run_print_foo(&p.c, &img);
    let r = run_print_foo(&p.rust, &img);
    assert_eq!(
        c,
        r,
        "print_foo(storage={storage:#04x}, pad={pad:?}, z={z}) diverged:\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c),
        show(&r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// `z` values chosen to exercise every interesting shape of `%d` output.
pub const Z_BOUNDARIES: [i32; 11] = [
    i32::MIN,
    i32::MIN + 1,
    -1_000_000_000,
    -100,
    -10,
    -1,
    0,
    1,
    10,
    i32::MAX - 1,
    i32::MAX,
];
