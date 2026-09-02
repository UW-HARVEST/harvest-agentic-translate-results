//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function
//! purely through its exported C symbol — never through the Rust crate
//! directly — so the `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// stdout capture (both .so's print through the test process's libc stdout)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// Builds a `ProcessState` by hand with libc `malloc`, so tests can reach
/// states `create_state` can never produce: a NULL buffer, arbitrary
/// bit-field contents (including non-zero `status`/`reserved`), and buffers
/// holding arbitrary bytes.
///
/// The allocation comes from the same libc allocator both `.so`s use, so
/// `destroy_state` may free it. Buffers are `malloc`ed separately and
/// NUL-terminated.
pub unsafe fn make_state(
    flags_raw: u32,
    data_raw: u32,
    buffer: Option<&[u8]>,
    capacity: c_int,
) -> *mut c_void {
    unsafe {
        let p = malloc(24);
        assert!(!p.is_null());
        let s = p as *mut RawState;
        let buf = match buffer {
            None => std::ptr::null_mut(),
            Some(bytes) => {
                let b = malloc(bytes.len() + 1) as *mut u8;
                assert!(!b.is_null());
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), b, bytes.len());
                *b.add(bytes.len()) = 0;
                b as *mut c_char
            }
        };
        (*s).flags = flags_raw;
        (*s).data = data_raw;
        (*s).buffer = buf;
        (*s).capacity = capacity;
        p
    }
}

/// Frees a state built by `make_state` without going through `destroy_state`.
pub unsafe fn drop_state(p: *mut c_void) {
    unsafe {
        if p.is_null() {
            return;
        }
        let s = p as *mut RawState;
        if !(*s).buffer.is_null() {
            free((*s).buffer as *mut c_void);
        }
        free(p);
    }
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Runs `f` with fd 1 redirected to a temp file; returns `(result, stdout_bytes)`.
///
/// fd 1 is process-global, so captures must be serialized across the test
/// harness's threads (see also `.cargo/config.toml`, which pins
/// `RUST_TEST_THREADS=1` so libtest's own progress output cannot land inside a
/// capture window).
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    // Push out anything libtest (Rust-side, buffered independently of libc)
    // has queued, so it is written to the real fd 1 and not into our capture.
    let _ = std::io::stdout().flush();

    let path = format!(
        "/tmp/.difftest_cap_{}_{:p}\0",
        std::process::id(),
        &f as *const _
    );
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let tmp = open(path.as_ptr() as *const c_char, O_RDWR | O_CREAT | O_TRUNC, 0o600);
        assert!(tmp >= 0, "open temp failed");
        dup2(tmp, 1);

        let r = f();

        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);

        lseek(tmp, 0, 0);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = read(tmp, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(tmp);
        unlink(path.as_ptr() as *const c_char);
        (r, out)
    }
}

// ---------------------------------------------------------------------------
// Mirror of the C `ProcessState` layout (verified: size 24, offsets 0/4/8/16).
// Only used to *inspect* opaque state returned by the libraries.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RawState {
    pub flags: u32,
    pub data: u32,
    pub buffer: *mut c_char,
    pub capacity: c_int,
}

impl RawState {
    pub fn flag1(&self) -> u32 {
        self.flags & 1
    }
    pub fn flag2(&self) -> u32 {
        (self.flags >> 1) & 1
    }
    pub fn flag3(&self) -> u32 {
        (self.flags >> 2) & 1
    }
    pub fn counter(&self) -> u32 {
        (self.flags >> 3) & 0x1F
    }
    pub fn mode(&self) -> u32 {
        (self.flags >> 8) & 0x7
    }
    pub fn status(&self) -> u32 {
        (self.flags >> 11) & 0x1F
    }
    pub fn reserved(&self) -> u32 {
        (self.flags >> 16) & 0xFFFF
    }
}

/// Everything observable about a state, independent of heap addresses.
#[derive(Debug, PartialEq, Eq)]
pub struct StateSnapshot {
    pub flags: u32,
    pub data: u32,
    pub capacity: c_int,
    pub buffer_null: bool,
    /// NUL-terminated string content of `buffer` (excludes the terminator).
    pub buffer_cstr: Vec<u8>,
}

pub type CreateFn = unsafe extern "C" fn(c_int, c_int) -> *mut c_void;
pub type DestroyFn = unsafe extern "C" fn(*mut c_void);
pub type ProcessFn = unsafe extern "C" fn(*mut c_void, c_char) -> c_int;
pub type UpdateFn = unsafe extern "C" fn(*mut c_void, c_int);
pub type ConfuseFn = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
pub type ConfusionFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation, with all six exports resolved by symbol name.
pub struct Impl {
    _lib: Library,
    pub name: &'static str,
    pub create_state: CreateFn,
    pub destroy_state: DestroyFn,
    pub process_buffer: ProcessFn,
    pub update_flags: UpdateFn,
    pub confuse_types: ConfuseFn,
    pub confusion: ConfusionFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| {
                            panic!(
                                "{} missing symbol {}: {e}",
                                name,
                                String::from_utf8_lossy(&$s[..$s.len() - 1])
                            )
                        });
                    *s.into_raw()
                }};
            }
            let create_state = sym!(CreateFn, b"create_state\0");
            let destroy_state = sym!(DestroyFn, b"destroy_state\0");
            let process_buffer = sym!(ProcessFn, b"process_buffer\0");
            let update_flags = sym!(UpdateFn, b"update_flags\0");
            let confuse_types = sym!(ConfuseFn, b"confuse_types\0");
            let confusion = sym!(ConfusionFn, b"confusion\0");
            Impl {
                _lib: lib,
                name,
                create_state,
                destroy_state,
                process_buffer,
                update_flags,
                confuse_types,
                confusion,
            }
        }
    }

    /// Reads back everything observable from an opaque state pointer.
    ///
    /// `read_buffer`: when the C `snprintf` was given size 0 the buffer is left
    /// uninitialized, so its contents are indeterminate and must not be
    /// compared. Callers pass `false` in that case.
    pub unsafe fn snapshot(&self, state: *mut c_void, read_buffer: bool) -> StateSnapshot {
        let raw = unsafe { *(state as *const RawState) };
        let buffer_null = raw.buffer.is_null();
        let mut buffer_cstr = Vec::new();
        if read_buffer && !buffer_null {
            let mut i = 0usize;
            loop {
                let b = unsafe { *raw.buffer.add(i) } as u8;
                if b == 0 {
                    break;
                }
                buffer_cstr.push(b);
                i += 1;
                assert!(i < 1 << 20, "unterminated buffer");
            }
        }
        StateSnapshot {
            flags: raw.flags,
            data: raw.data,
            capacity: raw.capacity,
            buffer_null,
            buffer_cstr,
        }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} not built ({e}); run cmake first", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    assert!(!candidates.is_empty(), "no .so found in {}", dir.display());
    candidates.remove(0)
}

fn find_rust_so() -> PathBuf {
    // Allow pointing the suite at a specific build (e.g. the debug profile,
    // where overflow checks are enabled, or a specific feature combination).
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO_PATH does not exist: {}", p.display());
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libconfusion_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libconfusion_lib.so not found under {}; run `cargo build --release`",
        base.display()
    );
}

/// The pair of loaded implementations. Loaded once per test process.
pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible test inputs)
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// An i32 biased toward interesting values (boundaries, small magnitudes,
    /// float-special bit patterns) mixed with uniform full-range draws.
    pub fn interesting_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 24] = [
            0,
            1,
            -1,
            2,
            -2,
            9,
            10,
            -9,
            -10,
            i32::MIN,
            i32::MAX,
            i32::MIN + 1,
            i32::MAX - 1,
            1078530011,
            0x0000_0001,
            0x0080_0000,
            0x7F7F_FFFF,
            0x7F80_0000u32 as i32,
            0x7FC0_0000u32 as i32,
            0xFF80_0000u32 as i32,
            0xFFC0_0000u32 as i32,
            0x4248_3fu32 as i32,
            100,
            -100,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len() as u64) as usize],
            1 => (self.below(2001) as i64 - 1000) as i32,
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

pub fn assert_out_eq(ctx: &str, c_out: &[u8], rs_out: &[u8]) {
    if c_out == rs_out {
        return;
    }
    // Report a small window around the first divergence rather than dumping
    // megabytes of matching output.
    let at = c_out
        .iter()
        .zip(rs_out.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c_out.len().min(rs_out.len()));
    let start = c_out[..at].iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let start = start.saturating_sub(200);
    let end_c = (at + 300).min(c_out.len());
    let end_r = (at + 300).min(rs_out.len());
    panic!(
        "stdout divergence [{ctx}]\n  first differing byte offset: {at}\n  \
         lengths: C={} Rust={}\n  C   ...{:?}...\n  Rust...{:?}...",
        c_out.len(),
        rs_out.len(),
        String::from_utf8_lossy(&c_out[start..end_c]),
        String::from_utf8_lossy(&rs_out[start..end_r]),
    );
}

pub fn assert_ret_eq<T: PartialEq + std::fmt::Debug>(ctx: &str, c: T, rs: T) {
    assert_eq!(c, rs, "return-value divergence [{ctx}]");
}

pub fn assert_snap_eq(ctx: &str, c: &StateSnapshot, rs: &StateSnapshot) {
    if c != rs {
        panic!(
            "state divergence [{ctx}]\n  C   flags={:#010x} data={:#010x} cap={} buf_null={} buf={:?}\n  Rust flags={:#010x} data={:#010x} cap={} buf_null={} buf={:?}",
            c.flags,
            c.data,
            c.capacity,
            c.buffer_null,
            String::from_utf8_lossy(&c.buffer_cstr),
            rs.flags,
            rs.data,
            rs.capacity,
            rs.buffer_null,
            String::from_utf8_lossy(&rs.buffer_cstr),
        );
    }
}
