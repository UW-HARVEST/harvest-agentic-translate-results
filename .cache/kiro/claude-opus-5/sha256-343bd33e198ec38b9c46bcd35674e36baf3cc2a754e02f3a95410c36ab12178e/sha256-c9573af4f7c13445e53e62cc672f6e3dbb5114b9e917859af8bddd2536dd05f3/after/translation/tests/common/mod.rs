//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading` and every
//! call goes through `dlsym`, so the `#[no_mangle] extern "C"` export wrappers
//! are exercised exactly the way an external consumer would exercise them.
//! Nothing in the Rust crate is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::{Path, PathBuf};

pub type GotoFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type OpFn = unsafe extern "C" fn(i32, i32, *mut c_void) -> i32;

/// Repository root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// Locate the C shared object produced by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory name*, so the
/// file name is not fixed; we glob `c_src/build` for the single `lib*.so`.
pub fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && \
                 cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// Locate the Rust `cdylib`. Prefers the profile the test itself was built
/// with, so `cargo test --release` tests the release `.so`.
pub fn rust_so_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let preferred = if cfg!(debug_assertions) {
        ["target/debug", "target/release"]
    } else {
        ["target/release", "target/debug"]
    };
    for dir in preferred {
        let p = manifest.join(dir).join("libgotomach_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libgotomach_lib.so not found under {}/target/{{debug,release}}; run `cargo build --release`",
        manifest.display()
    );
}

/// The two libraries under test, held open for the lifetime of the test.
pub struct Pair {
    pub c: Library,
    pub r: Library,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c = Library::new(c_so_path()).expect("dlopen C .so");
            let r = Library::new(rust_so_path()).expect("dlopen Rust .so");
            Pair { c, r }
        }
    }

    pub fn gotomach(&self) -> (Symbol<'_, GotoFn>, Symbol<'_, GotoFn>) {
        unsafe {
            (
                self.c.get(b"gotomach\0").expect("C gotomach"),
                self.r.get(b"gotomach\0").expect("Rust gotomach"),
            )
        }
    }

    pub fn op(&self, name: &[u8]) -> (Symbol<'_, OpFn>, Symbol<'_, OpFn>) {
        let mut z = name.to_vec();
        z.push(0);
        unsafe {
            (
                self.c
                    .get(&z)
                    .unwrap_or_else(|e| panic!("C {}: {e}", String::from_utf8_lossy(name))),
                self.r
                    .get(&z)
                    .unwrap_or_else(|e| panic!("Rust {}: {e}", String::from_utf8_lossy(name))),
            )
        }
    }
}

pub const OP_NAMES: [&[u8]; 3] = [b"process_value", b"double_value", b"triple_value"];

/// Deterministic xorshift64* PRNG — fixed seeds keep every property-style test
/// reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform over the whole `i32` range, including `INT_MIN`/`INT_MAX`.
    pub fn i32_any(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    /// Uniform over `lo..=hi`.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// An `i32` that is deliberately *not* a valid `mode` (not 0, 1 or 2).
    pub fn invalid_mode(&mut self) -> i32 {
        loop {
            let v = self.i32_any();
            if !(0..=2).contains(&v) {
                return v;
            }
        }
    }
}

/// Call `gotomach` in both libraries and assert the returned `int` is identical.
#[track_caller]
pub fn assert_goto_eq(
    cf: &Symbol<'_, GotoFn>,
    rf: &Symbol<'_, GotoFn>,
    row: &str,
    iterations: i32,
    seed: i32,
    mode: i32,
    threshold: i32,
) -> i32 {
    let (a, b) = unsafe {
        (
            cf(iterations, seed, mode, threshold),
            rf(iterations, seed, mode, threshold),
        )
    };
    assert_eq!(
        a, b,
        "[{row}] gotomach(iterations={iterations}, seed={seed}, mode={mode}, \
         threshold={threshold}): C returned {a}, Rust returned {b}"
    );
    a
}

/// Call one of the operation functions in both libraries and compare.
#[track_caller]
pub fn assert_op_eq(
    cf: &Symbol<'_, OpFn>,
    rf: &Symbol<'_, OpFn>,
    row: &str,
    name: &str,
    value: i32,
    unused_param: i32,
    ctx: *mut c_void,
) -> i32 {
    let (a, b) = unsafe { (cf(value, unused_param, ctx), rf(value, unused_param, ctx)) };
    assert_eq!(
        a, b,
        "[{row}] {name}(value={value}, unused_param={unused_param}, ctx={ctx:p}): \
         C returned {a}, Rust returned {b}"
    );
    a
}

// ---------------------------------------------------------------------------
// stdout capture (row C34).
//
// Both shared objects log through the *same* glibc `puts`/`stdout` as this test
// binary, so redirecting fd 1 captures their output verbatim.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
    fn fdopen(fd: i32, mode: *const std::ffi::c_char) -> *mut c_void;
}

/// Run `f` with fd 1 redirected into a temporary file and return what was
/// written, as raw bytes.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let path = std::env::temp_dir().join(format!(
        "gotomach-capture-{}-{}-{tag}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let out = unsafe {
        fflush(std::ptr::null_mut()); // flush *all* streams before swapping fd 1
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("seek");
        file.read_to_end(&mut buf).expect("read capture");
        buf
    };
    let _ = std::fs::remove_file(&path);
    out
}

/// `puts` inside a freshly `dlopen`ed object allocates the stdout buffer on
/// first use; touching stdout once up front keeps allocation counts and capture
/// contents stable.
pub fn warm_up_stdout() {
    unsafe {
        let _ = fdopen(-1, b"r\0".as_ptr() as *const std::ffi::c_char);
        fflush(std::ptr::null_mut());
    }
}
