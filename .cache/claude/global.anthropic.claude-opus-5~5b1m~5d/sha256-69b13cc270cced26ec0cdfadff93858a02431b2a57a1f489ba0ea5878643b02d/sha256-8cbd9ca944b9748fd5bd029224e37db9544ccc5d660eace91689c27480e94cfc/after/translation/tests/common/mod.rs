//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and exposes a
//! uniform `Lib` wrapper so every test can drive the two implementations
//! through the exact same FFI calls.  Rust functions are NEVER called
//! directly — always through the dynamically loaded `#[no_mangle]` exports.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

// --------------------------------------------------------------------------
// libc bits we need for stdout capture / heap handling
// --------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

// --------------------------------------------------------------------------
// Library discovery
// --------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so_in(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for e in std::fs::read_dir(dir).ok()? {
        let p = e.ok()?.path();
        if p.extension().and_then(|s| s.to_str()) != Some("so") {
            continue;
        }
        let name = p.file_name()?.to_str()?.to_string();
        if let Some(pat) = must_contain {
            if !name.contains(pat) {
                continue;
            }
        }
        best = Some(p);
    }
    best
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("CHARINBUF_C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src/build");
    find_so_in(&build, None).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("CHARINBUF_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test executable lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    if let Some(p) = find_so_in(profile_dir, Some("charinbuf_lib")) {
        return p;
    }
    for cand in ["target/release", "target/debug"] {
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join(cand);
        if let Some(p) = find_so_in(&d, Some("charinbuf_lib")) {
            return p;
        }
    }
    panic!("no Rust cdylib (libcharinbuf_lib.so) found; run `cargo build`");
}

// --------------------------------------------------------------------------
// Typed symbol table
// --------------------------------------------------------------------------

pub type OpFn = unsafe extern "C" fn(c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub increment_counter: OpFn,
    pub decrement_counter: OpFn,
    pub multiply_counter: OpFn,
    pub reset_counter: OpFn,
    pub validate_uint16_range: OpFn,
    pub is_string_empty: unsafe extern "C" fn(*const c_char) -> c_int,
    pub find_char_in_buffer: unsafe extern "C" fn(*const c_char, usize, c_char) -> *mut c_char,
    pub create_buffer: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    pub apply_operation: unsafe extern "C" fn(Option<OpFn>, c_int) -> c_int,
    pub charinbuf: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

/// Every symbol the C `.so` exports (see SYMBOLS.md).
pub const EXPECTED_SYMBOLS: &[&str] = &[
    "apply_operation",
    "charinbuf",
    "create_buffer",
    "decrement_counter",
    "find_char_in_buffer",
    "increment_counter",
    "is_string_empty",
    "multiply_counter",
    "reset_counter",
    "validate_uint16_range",
];

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> T {
    let s: Symbol<T> = lib
        .get(format!("{name}\0").as_bytes())
        .unwrap_or_else(|e| panic!("symbol `{name}` missing: {e}"));
    *s
}

impl Lib {
    pub fn open(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()));
            Lib {
                name,
                increment_counter: sym(&lib, "increment_counter"),
                decrement_counter: sym(&lib, "decrement_counter"),
                multiply_counter: sym(&lib, "multiply_counter"),
                reset_counter: sym(&lib, "reset_counter"),
                validate_uint16_range: sym(&lib, "validate_uint16_range"),
                is_string_empty: sym(&lib, "is_string_empty"),
                find_char_in_buffer: sym(&lib, "find_char_in_buffer"),
                create_buffer: sym(&lib, "create_buffer"),
                apply_operation: sym(&lib, "apply_operation"),
                charinbuf: sym(&lib, "charinbuf"),
                path,
                _lib: lib,
            }
        }
    }

    /// Look a counter operation up by index, as a raw function pointer taken
    /// from THIS library (so `apply_operation` mutates this library's own
    /// `static counter`).
    pub fn op(&self, idx: usize) -> OpFn {
        match idx % 4 {
            0 => self.increment_counter,
            1 => self.decrement_counter,
            2 => self.multiply_counter,
            _ => self.reset_counter,
        }
    }

    /// `create_buffer` + copy the result out + `free` it. Returns
    /// `(was_null, bytes_including_nul)`.
    pub unsafe fn create_buffer_owned(&self, initial: *const c_char) -> (bool, Vec<u8>) {
        let p = (self.create_buffer)(initial);
        if p.is_null() {
            return (true, Vec::new());
        }
        let n = strlen(p);
        let out = std::slice::from_raw_parts(p as *const u8, n + 1).to_vec();
        free(p as *mut c_void);
        (false, out)
    }
}

pub struct Libs {
    pub c: Lib,
    pub r: Lib,
}

static LIBS: std::sync::OnceLock<std::sync::Mutex<Libs>> = std::sync::OnceLock::new();

/// Acquire the two libraries.
///
/// This is a process-wide lock, held for the duration of a test, because
///   * both libraries carry a `static int counter` that is global per `.so`, and
///   * stdout capture rebinds file descriptor 1, which is process-global.
///
/// Serialising here keeps the differential comparisons deterministic even
/// though `cargo test` runs test functions on many threads.
pub fn libs() -> std::sync::MutexGuard<'static, Libs> {
    LIBS.get_or_init(|| {
        std::sync::Mutex::new(Libs {
            c: Lib::open("C", c_so_path()),
            r: Lib::open("RUST", rust_so_path()),
        })
    })
    .lock()
    .unwrap_or_else(|e| e.into_inner())
}

// --------------------------------------------------------------------------
// stdout capture (fd-level, so it catches printf from inside the .so)
// --------------------------------------------------------------------------

/// Redirects file descriptor 1 into a temp file for the duration of `f`,
/// returning `(f's value, captured bytes)`.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};

    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = Path::new(&dir).join(format!(
        "charinbuf-cap-{}-{}.txt",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("temp capture file");

    #[cfg(unix)]
    let target_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    let (result, bytes) = unsafe {
        // Flush whatever is pending on the real stdout first — both libc's
        // FILE* buffers and Rust std's own LineWriter (libtest writes the
        // "test <name> ... " prefix through the latter without a newline, so
        // it would otherwise be flushed *into* our capture file).
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
        }
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target_fd, 1) >= 0, "dup2 failed");

        let r = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore dup2 failed");
        close(saved);

        let mut buf = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("seek");
        file.read_to_end(&mut buf).expect("read capture");
        (r, buf)
    };
    drop(file);
    let _ = std::fs::remove_file(&path);
    (result, bytes)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

/// Assert that both libraries return the same value AND print the same bytes.
pub fn assert_same_call<T: PartialEq + std::fmt::Debug>(
    what: &str,
    c: (T, Vec<u8>),
    r: (T, Vec<u8>),
) {
    assert_eq!(
        c.0, r.0,
        "{what}: return value diverged (C={:?}, Rust={:?})\n  C stdout:    {}\n  Rust stdout: {}",
        c.0,
        r.0,
        show(&c.1),
        show(&r.1)
    );
    assert_eq!(
        c.1,
        r.1,
        "{what}: stdout diverged\n  C stdout:    {}\n  Rust stdout: {}",
        show(&c.1),
        show(&r.1)
    );
}

// --------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property tests)
// --------------------------------------------------------------------------

pub const SEED: u64 = 0x2024_C0FF_EE00_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A "spicy" i32: biased towards boundary values.
    pub fn spicy_i32(&mut self) -> i32 {
        const SPECIALS: [i32; 14] = [
            i32::MIN,
            i32::MIN + 1,
            -65537,
            -65536,
            -65535,
            -1,
            0,
            1,
            2,
            65534,
            65535,
            65536,
            65537,
            i32::MAX,
        ];
        match self.next_u64() % 3 {
            0 => SPECIALS[self.below(SPECIALS.len())],
            1 => self.range_i32(-70000, 70000),
            _ => self.next_i32(),
        }
    }
    /// A random NUL-free byte string, NUL-terminated.
    pub fn cstring(&mut self, max_len: usize) -> Vec<u8> {
        let n = self.below(max_len + 1);
        let mut v = Vec::with_capacity(n + 1);
        for _ in 0..n {
            let mut b = self.next_u8();
            if b == 0 {
                b = 1;
            }
            v.push(b);
        }
        v.push(0);
        v
    }
}
