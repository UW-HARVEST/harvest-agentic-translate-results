//! Shared differential-testing harness.
//!
//! Loads BOTH shared libraries (the C one built by CMake and the Rust cdylib)
//! through `libloading` and calls every function purely through its exported
//! `extern "C"` symbol — never by calling Rust code directly — so the
//! `#[no_mangle]` wrappers and the C ABI are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// The `ProcessState` layout, replicated exactly as the C compiler lays it out:
//   PackedFlags flags;   // one 32-bit bit-field allocation unit  @ 0
//   TypeConfusion data;  // union { int; float; unsigned; char[4] } @ 4
//   char* buffer;        // @ 8
//   int capacity;        // @ 16   (struct size 24, align 8)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessState {
    pub flags: u32,
    pub data: u32,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

pub const FLAG1_SHIFT: u32 = 0;
pub const FLAG2_SHIFT: u32 = 1;
pub const FLAG3_SHIFT: u32 = 2;
pub const COUNTER_SHIFT: u32 = 3;
pub const MODE_SHIFT: u32 = 8;
pub const STATUS_SHIFT: u32 = 11;
pub const RESERVED_SHIFT: u32 = 16;

pub fn field(bits: u32, shift: u32, width: u32) -> u32 {
    (bits >> shift) & ((1u32 << width) - 1)
}

// ---------------------------------------------------------------------------
// libc bits we need for byte-exact stdout capture.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn free(p: *mut c_void);
    fn malloc(n: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Library handles.
// ---------------------------------------------------------------------------
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

pub type FnCreateState = unsafe extern "C" fn(c_int, c_int) -> *mut ProcessState;
pub type FnDestroyState = unsafe extern "C" fn(*mut ProcessState);
pub type FnProcessBuffer = unsafe extern "C" fn(*mut ProcessState, c_char) -> c_int;
pub type FnUpdateFlags = unsafe extern "C" fn(*mut ProcessState, c_int);
pub type FnConfuseTypes = unsafe extern "C" fn(*mut ProcessState, c_int) -> c_int;
pub type FnConfusion = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

impl Impl {
    fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, name, e))
        }
    }

    pub fn create_state(&self) -> Symbol<'_, FnCreateState> {
        self.sym("create_state")
    }
    pub fn destroy_state(&self) -> Symbol<'_, FnDestroyState> {
        self.sym("destroy_state")
    }
    pub fn process_buffer(&self) -> Symbol<'_, FnProcessBuffer> {
        self.sym("process_buffer")
    }
    pub fn update_flags(&self) -> Symbol<'_, FnUpdateFlags> {
        self.sym("update_flags")
    }
    pub fn confuse_types(&self) -> Symbol<'_, FnConfuseTypes> {
        self.sym("confuse_types")
    }
    pub fn confusion(&self) -> Symbol<'_, FnConfusion> {
        self.sym("confusion")
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

pub fn find_c_so() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

/// Locates the Rust cdylib and REFUSES to run against a stale one.
///
/// `cargo test` builds only the test binaries, not the `cdylib` artifact, so
/// without this freshness check the suite would happily keep validating an old
/// `.so` and report green for a broken `src/lib.rs`. Run `./run_tests.sh`
/// (or `cargo build --release`) to refresh it, or point `DIFFTEST_RUST_SO` at a
/// specific object.
pub fn find_rust_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let path = if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
        PathBuf::from(p)
    } else {
        let mut candidates = vec![];
        if let Ok(exe) = std::env::current_exe() {
            // .../target/<profile>/deps/<test bin>
            if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
                candidates.push(profile_dir.join("libconfusion_lib.so"));
            }
        }
        candidates.push(manifest.join("target/release/libconfusion_lib.so"));
        candidates.push(manifest.join("target/debug/libconfusion_lib.so"));
        candidates
            .iter()
            .find(|c| c.exists())
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "could not locate libconfusion_lib.so; tried {candidates:?}.\n\
                     Build it with `cargo build --release` (or run ./run_tests.sh)."
                )
            })
    };

    let src = manifest.join("src/lib.rs");
    let so_time = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", path.display()));
    let src_time = std::fs::metadata(&src)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", src.display()));
    assert!(
        so_time >= src_time,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild the cdylib — rebuild it with \
         `cargo build --release` (or run ./run_tests.sh) before testing.",
        path.display(),
        src.display()
    );
    path
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

/// Global lock: stdout redirection is process-wide, so only one differential
/// call may be in flight at a time.
static LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        unsafe {
            Pair {
                c: Impl {
                    name: "C",
                    lib: Library::new(&c_path)
                        .unwrap_or_else(|e| panic!("load {}: {e}", c_path.display())),
                },
                rust: Impl {
                    name: "Rust",
                    lib: Library::new(&r_path)
                        .unwrap_or_else(|e| panic!("load {}: {e}", r_path.display())),
                },
            }
        }
    })
}

pub fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Runs `f` with fd 1 redirected into a temporary file and returns
/// `(result, captured stdout bytes)`.
///
/// Both libraries print through the *same* process libc `stdout`, so a
/// `dup2` + `fflush(NULL)` sandwich captures their output identically.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/difftmp");
    std::fs::create_dir_all(&path).expect("create capture dir");
    path.push(format!(
        "difftest-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));

    let file = std::fs::File::options()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .expect("create capture file");

    let out = unsafe {
        fflush(std::ptr::null_mut()); // flush everything pending
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        let r = f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        r
    };

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (out, bytes)
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}

// ---------------------------------------------------------------------------
// Helpers for inspecting states across the FFI boundary
// ---------------------------------------------------------------------------

/// A snapshot of everything observable about a `ProcessState`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StateSnapshot {
    pub is_null: bool,
    pub flags: u32,
    pub data: u32,
    pub capacity: c_int,
    pub buffer_is_null: bool,
    pub buffer: Vec<u8>,
}

pub unsafe fn snapshot(state: *mut ProcessState, read_buffer: bool) -> StateSnapshot {
    if state.is_null() {
        return StateSnapshot {
            is_null: true,
            flags: 0,
            data: 0,
            capacity: 0,
            buffer_is_null: true,
            buffer: vec![],
        };
    }
    let s = unsafe { std::ptr::read(state) };
    let buffer = if read_buffer && !s.buffer.is_null() {
        let len = unsafe { strlen(s.buffer) };
        unsafe { std::slice::from_raw_parts(s.buffer as *const u8, len) }.to_vec()
    } else {
        vec![]
    };
    StateSnapshot {
        is_null: false,
        flags: s.flags,
        data: s.data,
        capacity: s.capacity,
        buffer_is_null: s.buffer.is_null(),
        buffer,
    }
}

/// A `ProcessState` allocated with libc `malloc` (so either library's
/// `destroy_state` may free it) whose `buffer` holds `content` + NUL.
pub struct SyntheticState {
    pub ptr: *mut ProcessState,
}

impl SyntheticState {
    pub fn new(flags: u32, data: u32, capacity: c_int, content: &[u8]) -> Self {
        unsafe {
            let buf = malloc(content.len() + 1) as *mut c_char;
            assert!(!buf.is_null());
            std::ptr::copy_nonoverlapping(content.as_ptr(), buf as *mut u8, content.len());
            *buf.add(content.len()) = 0;
            let st = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;
            assert!(!st.is_null());
            std::ptr::write(
                st,
                ProcessState {
                    flags,
                    data,
                    buffer: buf,
                    capacity,
                },
            );
            SyntheticState { ptr: st }
        }
    }

    /// A state with a NULL buffer (reachable only synthetically).
    pub fn with_null_buffer(flags: u32, data: u32, capacity: c_int) -> Self {
        unsafe {
            let st = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;
            assert!(!st.is_null());
            std::ptr::write(
                st,
                ProcessState {
                    flags,
                    data,
                    buffer: std::ptr::null_mut(),
                    capacity,
                },
            );
            SyntheticState { ptr: st }
        }
    }
}

impl Drop for SyntheticState {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                let b = (*self.ptr).buffer;
                if !b.is_null() {
                    free(b as *mut c_void);
                }
                free(self.ptr as *mut c_void);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u32() >> 7) as u8
    }
}

pub const INTERESTING_I32: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    -4,
    7,
    8,
    -8,
    9,
    10,
    -10,
    31,
    32,
    63,
    64,
    100,
    -100,
    255,
    256,
    -256,
    1078530011,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x7F800000u32 as i32, // +inf as float bits
    0xFF800000u32 as i32, // -inf
    0x7FC00000u32 as i32, // quiet NaN
    0x7F800001u32 as i32, // signalling NaN
    0xFFC00000u32 as i32, // negative NaN
    0x00000001,           // smallest denormal
    0x80000001u32 as i32, // negative denormal
    0x3F800000,           // 1.0f
    0xBF800000u32 as i32, // -1.0f
    0x4EFFFFFF,           // ~2.147483e9 -> *100 overflows int
    0x4F000000,
    0xCF000000u32 as i32,
    0x7F7FFFFF, // FLT_MAX
    0xFF7FFFFFu32 as i32,
    0x41200000, // 10.0f
    0x42C80000, // 100.0f
    0x00800000, // smallest normal
];
