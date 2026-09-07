//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and called **only** through
//! their exported `driver` symbol, exactly as an external consumer would. The
//! Rust functions are never called directly, so the `#[no_mangle]` export
//! wrapper is under test too.
//!
//! Both `.so`s emit their output with libc `printf`, i.e. into the *same*
//! process-wide `stdout` `FILE`. Capturing them separately therefore has to
//! happen at the file-descriptor level: redirect fd 1 to a scratch file (or
//! pipe), run the calls, `fflush(NULL)`, restore fd 1, and read back the bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// libc bindings used by the harness itself (not by the code under test).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn setvbuf(
        stream: *mut std::ffi::c_void,
        buf: *mut std::ffi::c_char,
        mode: i32,
        size: usize,
    ) -> i32;
}

const IONBF: i32 = 2;
const IOFBF: i32 = 0;

/// The libc `stdout` `FILE *`, resolved at run time out of the C library that
/// is already mapped into this process. Taken from the *C* `.so`'s dependency
/// so that it is unambiguously the same stream both objects print to.
fn libc_stdout() -> *mut std::ffi::c_void {
    unsafe {
        // `libc.so.6` is already loaded; `Library::new` just bumps its refcount.
        static mut CACHED: *mut std::ffi::c_void = std::ptr::null_mut();
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let libc = Library::new("libc.so.6").expect("open libc.so.6");
            let sym: Symbol<*mut *mut std::ffi::c_void> =
                libc.get(b"stdout\0").expect("resolve libc `stdout`");
            CACHED = **sym;
            // Deliberately leak the handle: the resolved `FILE *` must stay
            // valid for the whole test process.
            std::mem::forget(libc);
        });
        CACHED
    }
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build && \
         cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // Explicit override, so a runner script can point the same test suite at the
    // debug and the release cdylib in turn.
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
        return p;
    }

    // `cargo test` places the cdylib in target/<profile>/ when it builds it.
    // Note it does not always build a cdylib-only crate's `.so` for `cargo
    // test`, hence the fallbacks.
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test-bin>  ->  .../target/<profile>/
        if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
    }
    candidates.push(crate_root().join("target/release/libdriver.so"));
    candidates.push(crate_root().join("target/debug/libdriver.so"));

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib libdriver.so not found. Tried: {candidates:?}. Run `cargo build` first.");
}

/// The signature of the single exported entry point: `void driver(float x)`.
pub type DriverFn = unsafe extern "C" fn(f32);

/// Both implementations, loaded through their `.so` exports.
pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
}

impl Impls {
    pub fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

            let c_sym: Symbol<DriverFn> = c_lib
                .get(b"driver\0")
                .expect("C `.so` must export `driver`");
            let rust_sym: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust `.so` must export `driver` (check #[no_mangle])");

            let c_driver = *c_sym;
            let rust_driver = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    }
}

/// Process-wide guard: fd-1 redirection is global state, so captures must never
/// overlap. Poison-tolerant, so one genuine divergence does not cascade into 25
/// misleading `PoisonError` failures that hide the real culprit.
pub fn lock_captures() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// How `stdout` should be buffered while the code under test runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buffering {
    /// Leave whatever mode the redirect target implies (file => fully buffered).
    Default,
    /// Force `_IONBF`, so every `printf` writes through immediately.
    Unbuffered,
}

/// Where the redirected fd 1 points while capturing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sink {
    /// A regular temporary file (libc picks fully-buffered).
    File,
    /// A pipe (libc also picks fully-buffered, but a different `st_mode` path,
    /// and the write side is a non-seekable fd).
    Pipe,
}

/// Run `f` with fd 1 redirected, then return everything it wrote.
///
/// `fflush(NULL)` is called before *and* after so that no unrelated buffered
/// output leaks into (or out of) the capture window.
pub fn capture_with(sink: Sink, buffering: Buffering, f: impl FnOnce()) -> Vec<u8> {
    let out = libc_stdout();

    // Flush anything already pending on the real stdout before we steal fd 1.
    unsafe {
        fflush(std::ptr::null_mut());
    }
    // Rust's own `std::io::stdout` buffer, too.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    match sink {
        Sink::File => {
            let dir = std::env::temp_dir();
            let path = dir.join(format!(
                "driver-difftest-{}-{:?}.out",
                std::process::id(),
                std::thread::current().id()
            ));
            let file = std::fs::File::create(&path).expect("create capture file");
            let bytes = unsafe { redirect_and_run(out, file.as_raw_fd(), buffering, f) };
            drop(file);
            let _: () = bytes;
            let data = std::fs::read(&path).expect("read capture file");
            let _ = std::fs::remove_file(&path);
            data
        }
        Sink::Pipe => {
            let mut fds = [0i32; 2];
            let rc = unsafe { pipe(fds.as_mut_ptr()) };
            assert_eq!(rc, 0, "pipe() failed");
            let (read_fd, write_fd) = (fds[0], fds[1]);

            // Drain the read end on a helper thread so a large volume of output
            // cannot deadlock against the 64 KiB pipe buffer.
            let reader = std::thread::spawn(move || {
                let mut f = unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(read_fd) };
                let mut buf = Vec::new();
                let _ = f.read_to_end(&mut buf);
                buf
            });

            unsafe { redirect_and_run(out, write_fd, buffering, f) };
            unsafe { close(write_fd) };

            reader.join().expect("capture reader thread")
        }
    }
}

/// Convenience: the default configuration (regular file, default buffering).
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    capture_with(Sink::File, Buffering::Default, f)
}

/// The unsafe core of the redirection dance.
unsafe fn redirect_and_run(
    out: *mut std::ffi::c_void,
    target_fd: i32,
    buffering: Buffering,
    f: impl FnOnce(),
) {
    unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target_fd, 1) >= 0, "dup2 onto fd 1 failed");

        // Re-point the FILE's buffer so the mode matches `buffering`. Passing a
        // NULL buffer lets libc allocate its own.
        match buffering {
            Buffering::Unbuffered => {
                setvbuf(out, std::ptr::null_mut(), IONBF, 0);
            }
            Buffering::Default => {
                setvbuf(out, std::ptr::null_mut(), IOFBF, 0);
            }
        }

        f();

        // Push everything the code under test buffered into `target_fd` while
        // fd 1 still points there.
        fflush(std::ptr::null_mut());

        // Restore, and put stdout back into a sane fully-buffered state.
        assert!(dup2(saved, 1) >= 0, "dup2 restore of fd 1 failed");
        close(saved);
        setvbuf(out, std::ptr::null_mut(), IOFBF, 0);
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers.
// ---------------------------------------------------------------------------

/// Call the C `driver` and the Rust `driver` on the same `f32` and assert their
/// stdout bytes are identical.
///
/// The two calls happen in *separate* capture windows so a divergence in one
/// cannot mask the other.
pub fn assert_same(impls: &Impls, x: f32) {
    let _g = lock_captures();
    assert_same_locked(impls, x, Sink::File, Buffering::Default);
}

pub fn assert_same_locked(impls: &Impls, x: f32, sink: Sink, buffering: Buffering) {
    let bits = x.to_bits();
    let c_out = capture_with(sink, buffering, || unsafe { (impls.c_driver)(x) });
    let rust_out = capture_with(sink, buffering, || unsafe { (impls.rust_driver)(x) });
    compare(bits, &c_out, &rust_out, sink, buffering);
}

/// Compare a whole batch of inputs, one capture window for the entire batch on
/// each side. This additionally checks that repeated calls accumulate output the
/// same way (no stray flush, no extra or missing newline).
pub fn assert_same_batch(impls: &Impls, xs: &[f32], sink: Sink, buffering: Buffering) {
    let _g = lock_captures();

    let c_out = capture_with(sink, buffering, || {
        for &x in xs {
            unsafe { (impls.c_driver)(x) };
        }
    });
    let rust_out = capture_with(sink, buffering, || {
        for &x in xs {
            unsafe { (impls.rust_driver)(x) };
        }
    });

    if c_out != rust_out {
        // Narrow the failure down to the first differing line for a useful
        // message, then re-check that input on its own.
        let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
        let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
        let idx = c_lines
            .iter()
            .zip(r_lines.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c_lines.len().min(r_lines.len()));
        let culprit = xs.get(idx).copied();
        panic!(
            "batch divergence at line {idx} (input {culprit:?}, bits {:?}), sink={sink:?} \
             buffering={buffering:?}\n  C   : {:?}\n  Rust: {:?}",
            culprit.map(f32::to_bits),
            String::from_utf8_lossy(c_lines.get(idx).copied().unwrap_or(b"<none>")),
            String::from_utf8_lossy(r_lines.get(idx).copied().unwrap_or(b"<none>")),
        );
    }

    // Every call must contribute exactly one `\n`-terminated line.
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        xs.len(),
        "C produced the wrong number of lines for {} inputs",
        xs.len()
    );
    assert_eq!(
        c_out.len(),
        xs.len() * 9,
        "each line must be 8 hex digits + newline"
    );
}

fn compare(bits: u32, c_out: &[u8], rust_out: &[u8], sink: Sink, buffering: Buffering) {
    assert_eq!(
        c_out,
        rust_out,
        "divergence for f32 bits {bits:#010x} (value {:?}), sink={sink:?} buffering={buffering:?}\n\
         \x20 C   : {:?}\n  Rust: {:?}",
        f32::from_bits(bits),
        String::from_utf8_lossy(c_out),
        String::from_utf8_lossy(rust_out),
    );

    // Independently cross-check against the reference formatting so a
    // both-sides-identically-wrong result cannot slip through.
    let expected: String = bits
        .to_ne_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        + "\n";
    assert_eq!(
        String::from_utf8_lossy(c_out),
        expected,
        "C output did not match the reference hex dump for bits {bits:#010x}"
    );
}

// ---------------------------------------------------------------------------
// Fixed-seed PRNG so every randomized row is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new() -> Self {
        Rng(Self::SEED)
    }

    pub fn with_seed(seed: u64) -> Self {
        Rng(if seed == 0 { Self::SEED } else { seed })
    }

    /// xorshift64*
    pub fn next_u64(&mut self) -> u64 {
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

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
