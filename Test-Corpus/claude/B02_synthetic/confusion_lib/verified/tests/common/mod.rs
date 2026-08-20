// Shared differential-test harness.
//
// Both implementations are loaded as shared objects through `libloading`
// (`dlopen` with RTLD_LOCAL, so the two libraries' identically-named exports do
// NOT interpose on each other) and are only ever reached through their exported
// C symbols — never by calling Rust functions directly. This exercises the
// `#[no_mangle] extern "C"` wrappers exactly as an external consumer would.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// The C aggregate types (layout verified against gcc x86-64 SysV):
//   PackedFlags   : size 4,  align 4  (one 32-bit storage unit)
//   TypeConfusion : size 4,  align 4
//   ProcessState  : size 24, align 8; flags@0 data@4 buffer@8 capacity@16
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct ProcessState {
    /// The single storage unit that holds every `PackedFlags` bit-field.
    pub flags: c_uint,
    /// The `TypeConfusion` union, viewed as its raw 32 bits.
    pub data: c_uint,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

pub const FLAG1_MASK: u32 = 0x0000_0001;
pub const FLAG2_MASK: u32 = 0x0000_0002;
pub const FLAG3_MASK: u32 = 0x0000_0004;
pub const COUNTER_MASK: u32 = 0x0000_00F8;
pub const MODE_MASK: u32 = 0x0000_0700;
pub const STATUS_MASK: u32 = 0x0000_F800;
pub const RESERVED_MASK: u32 = 0xFFFF_0000;

pub fn counter_of(bits: u32) -> u32 {
    (bits & COUNTER_MASK) >> 3
}
pub fn mode_of(bits: u32) -> u32 {
    (bits & MODE_MASK) >> 8
}

// ---------------------------------------------------------------------------
// libc bits used by the harness itself.
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    pub fn free(ptr: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
}

// ---------------------------------------------------------------------------
// One loaded implementation.
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    pub create_state: unsafe extern "C" fn(c_int, c_int) -> *mut ProcessState,
    pub destroy_state: unsafe extern "C" fn(*mut ProcessState),
    pub process_buffer: unsafe extern "C" fn(*mut ProcessState, c_char) -> c_int,
    pub update_flags: unsafe extern "C" fn(*mut ProcessState, c_int),
    pub confuse_types: unsafe extern "C" fn(*mut ProcessState, c_int) -> c_int,
    pub confusion: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

impl Impl {
    /// Loads a single implementation (used by the out-of-memory child process,
    /// which must keep its address-space budget minimal and symmetric).
    pub fn load_one(name: &'static str, path: &PathBuf) -> Impl {
        Impl::load(name, path)
    }

    fn load(name: &'static str, path: &PathBuf) -> Impl {
        // Leaked on purpose: the extracted function pointers must stay valid
        // for the whole test-binary lifetime.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        }));
        fn g<T: Copy>(lib: &'static Library, sym: &[u8]) -> T {
            let s: Symbol<T> = unsafe { lib.get(sym) }
                .unwrap_or_else(|e| panic!("dlsym({}) failed: {e}", String::from_utf8_lossy(sym)));
            *s
        }
        {
            Impl {
                name,
                create_state: g(lib, b"create_state\0"),
                destroy_state: g(lib, b"destroy_state\0"),
                process_buffer: g(lib, b"process_buffer\0"),
                update_flags: g(lib, b"update_flags\0"),
                confuse_types: g(lib, b"confuse_types\0"),
                confusion: g(lib, b"confusion\0"),
            }
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not built: {} (run: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.join("libconfusion_lib.so"),
        deps.parent().unwrap_or(deps).join("libconfusion_lib.so"),
    ];
    for c in &candidates {
        if c.exists() {
            assert_fresh(c);
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; looked in {:?} — run `cargo build` first",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library when
/// no test target links against it, so a stale `.so` would silently be tested.
/// Refuse to run in that case.
fn assert_fresh(so: &std::path::Path) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut sources = vec![manifest_dir().join("Cargo.toml")];
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    sources.push(p);
                }
            }
        }
    }
    for src in sources {
        if let Ok(t) = std::fs::metadata(&src).and_then(|m| m.modified()) {
            if t > so_time {
                let replace = newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true);
                if replace {
                    newest = Some((src, t));
                }
            }
        }
    }
    if let Some((src, _)) = newest {
        panic!(
            "STALE Rust cdylib: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib-only library — run \
             `cargo build` (or ./run_tests.sh) before testing.",
            so.display(),
            src.display()
        );
    }
}

/// The two implementations under test: `(c, rust)`.
pub fn impls() -> &'static (Impl, Impl) {
    static IMPLS: OnceLock<(Impl, Impl)> = OnceLock::new();
    IMPLS.get_or_init(|| {
        (
            Impl::load("C", &c_so_path()),
            Impl::load("Rust", &rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level (both libraries `printf` through
// the very same glibc `stdout` stream, so this captures either one).
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<u64> {
    static L: OnceLock<Mutex<u64>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(0))
}

/// Capturing the libraries' stdout works by redirecting file descriptor 1,
/// which is process-wide: libtest must therefore run the tests serially,
/// otherwise its own progress output would land in the capture file.
fn assert_serial() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        let mut serial = std::env::var("RUST_TEST_THREADS").as_deref() == Ok("1");
        for (i, a) in args.iter().enumerate() {
            if a == "--test-threads=1" || a == "-j1" {
                serial = true;
            }
            if a == "--test-threads" && args.get(i + 1).map(|s| s.as_str()) == Some("1") {
                serial = true;
            }
        }
        assert!(
            serial,
            "these differential tests capture stdout by redirecting file \
             descriptor 1, which requires libtest to run serially.\n\
             Run them as:  cargo test -- --test-threads=1   (or use ./run_tests.sh)"
        );
    });
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// `(f's return value, exact bytes written to stdout)`.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    assert_serial();
    let mut guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    *guard += 1;
    let seq = *guard;

    // Push any pending Rust-side (test harness) output out to the real stdout
    // first, then flush every C stdio stream, so nothing already buffered can
    // leak into the capture file.
    std::io::stdout().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };

    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "confusion_capture_{}_{}.txt",
        std::process::id(),
        seq
    ));
    let mut file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let r = f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut out = Vec::new();
    file.read_to_end(&mut out).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&path);

    (r, out)
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Asserts that the two `(value, stdout)` observations are identical.
pub fn assert_same<R: PartialEq + std::fmt::Debug>(
    ctx: &str,
    c: (R, Vec<u8>),
    rust: (R, Vec<u8>),
) {
    assert_eq!(
        c.0, rust.0,
        "[{ctx}] return value differs: C={:?} Rust={:?}\n  C stdout   : {}\n  Rust stdout: {}",
        c.0,
        rust.0,
        show(&c.1),
        show(&rust.1)
    );
    assert_eq!(
        c.1,
        rust.1,
        "[{ctx}] stdout differs:\n  C   : {}\n  Rust: {}",
        show(&c.1),
        show(&rust.1)
    );
}

// ---------------------------------------------------------------------------
// A snapshot of everything observable about a `ProcessState`.
//
// Deliberately excludes the 4 bytes of tail padding and the bytes of `buffer`
// past its NUL terminator: those are never written by the C code (they keep
// whatever `malloc` returned), so they are not part of the defined behaviour.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub null: bool,
    pub flags: u32,
    pub data: u32,
    pub capacity: c_int,
    pub buffer_null: bool,
    /// Bytes of `buffer` up to and including the NUL terminator (empty when
    /// `capacity == 0`, since `snprintf(buf, 0, …)` writes nothing at all).
    pub buffer: Vec<u8>,
}

pub unsafe fn snapshot(state: *const ProcessState) -> Snapshot {
    if state.is_null() {
        return Snapshot {
            null: true,
            flags: 0,
            data: 0,
            capacity: 0,
            buffer_null: true,
            buffer: Vec::new(),
        };
    }
    let s = &*state;
    let buffer = if s.buffer.is_null() || s.capacity <= 0 {
        Vec::new()
    } else {
        let n = strlen(s.buffer);
        std::slice::from_raw_parts(s.buffer as *const u8, n + 1).to_vec()
    };
    Snapshot {
        null: false,
        flags: s.flags,
        data: s.data,
        capacity: s.capacity,
        buffer_null: s.buffer.is_null(),
        buffer,
    }
}

/// Writes `bytes` (plus a NUL terminator) into `state->buffer`.
pub unsafe fn set_buffer(state: *mut ProcessState, bytes: &[u8]) {
    let s = &mut *state;
    assert!(!s.buffer.is_null());
    assert!(
        bytes.len() + 1 <= s.capacity as usize,
        "buffer content does not fit in capacity"
    );
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), s.buffer as *mut u8, bytes.len());
    *s.buffer.add(bytes.len()) = 0;
}

/// Writes `bytes` verbatim (no terminator added) into `state->buffer`.
pub unsafe fn set_buffer_raw(state: *mut ProcessState, bytes: &[u8]) {
    let s = &mut *state;
    assert!(!s.buffer.is_null());
    assert!(bytes.len() <= s.capacity as usize);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), s.buffer as *mut u8, bytes.len());
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_C0FF_EE00_1234;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(s)
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
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}
