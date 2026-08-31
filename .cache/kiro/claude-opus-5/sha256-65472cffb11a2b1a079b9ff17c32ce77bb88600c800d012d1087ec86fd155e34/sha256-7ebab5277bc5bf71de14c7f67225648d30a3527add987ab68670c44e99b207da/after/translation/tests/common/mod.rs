//! Shared harness: locates and dlopen()s both the C reference `.so` and the
//! Rust `.so`, and provides stdout capture so `printf` output can be compared
//! byte-for-byte.
//!
//! Both libraries are only ever reached through `libloading`, i.e. through
//! their exported symbols, exactly like an external C caller would.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C stream. The C `.so`, the Rust `.so`
    /// and this test binary all share the one `libc.so.6` (and hence the one
    /// `stdout` `FILE`), so flushing here drains output produced inside either
    /// library.
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    let root = workspace_root();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\nLooked in: {candidates:?}"
    );
}

fn rust_lib_path() -> PathBuf {
    let root = workspace_root();
    // Prefer the artifact built with the same profile as this test binary, so
    // `cargo test` and `cargo test --release` each exercise their own cdylib.
    let profiles: [&str; 2] = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };

    let mut candidates = Vec::new();
    for p in profiles {
        if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
            candidates.push(PathBuf::from(&dir).join(p).join("libdriver.so"));
        }
        candidates.push(root.join("translation/target").join(p).join("libdriver.so"));
    }

    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found; run `cargo build --release` in translation/. Looked in: {candidates:?}"
            )
        })
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Libs {
    pub fn load() -> Self {
        let c_path = c_lib_path();
        let rust_path = rust_lib_path();
        // Default flags are RTLD_LAZY | RTLD_LOCAL, so the two libraries'
        // identically named exports do not collide with one another.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    }

    pub fn foo(&self) -> (Symbol<'_, FooFn>, Symbol<'_, FooFn>) {
        (
            unsafe { self.c.get(b"foo\0") }.expect("C .so does not export `foo`"),
            unsafe { self.rust.get(b"foo\0") }.expect("Rust .so does not export `foo`"),
        )
    }

    pub fn driver(&self) -> (Symbol<'_, DriverFn>, Symbol<'_, DriverFn>) {
        (
            unsafe { self.c.get(b"driver\0") }.expect("C .so does not export `driver`"),
            unsafe { self.rust.get(b"driver\0") }.expect("Rust .so does not export `driver`"),
        )
    }
}

/// Serialises stdout redirection; `cargo test` runs test fns on many threads.
pub static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 pointed at a temporary file and returns everything that
/// was written, including output buffered inside libc.
///
/// Only safe if nothing else in this process writes to stdout concurrently, so
/// the stdout-comparing tests live in a test binary with a single `#[test]`.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_stdout_{}_{:p}.bin",
        std::process::id(),
        &tmp as *const _
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("create stdout capture file");

    // Rust's own stdout is buffered separately from libc's; drain it so the
    // test harness' bookkeeping cannot land in the capture file.
    let _ = std::io::Write::flush(&mut std::io::stdout());

    unsafe {
        // Drain anything already pending so it is not attributed to `f`.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        f();

        // stdout is a regular file here, so it is fully buffered: flush before
        // restoring the original descriptor.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore stdout failed");
        close(saved);
    }

    let mut out = Vec::new();
    let mut file = file;
    use std::io::Seek;
    file.rewind().expect("rewind capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&tmp);
    out
}

/// `nm -D --defined-only` symbol names for a shared object.
pub fn exported_symbols(path: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// NUL-terminates `bytes` for passing as `const char *`.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    assert!(
        !bytes.contains(&0),
        "input must not contain an interior NUL"
    );
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// Deterministic byte-source so both libraries always see identical inputs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Any byte except 0 (an interior NUL would truncate the C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
}
