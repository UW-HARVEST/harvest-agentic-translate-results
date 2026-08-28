// Shared differential-test harness.
//
// Loads BOTH shared objects with `libloading` and calls every function through
// its exported C symbol. Nothing in the Rust crate is ever called directly, so
// the `#[no_mangle] extern "C"` wrappers are part of what is under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// The private `StringBuffer` typedef from c_src/src/lib.c. Not in the public
// header, but part of the observable ABI: `create_buffer` returns one.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

pub type FnCreate = unsafe extern "C" fn(c_int) -> *mut StringBuffer;
pub type FnAppend = unsafe extern "C" fn(*mut StringBuffer, *const c_char) -> c_int;
pub type FnDestroy = unsafe extern "C" fn(*mut StringBuffer);
pub type FnOpName = unsafe extern "C" fn(c_int) -> *const c_char;
pub type FnPerform = unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int;
pub type FnBuffapp = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation, reached exclusively through `dlsym`.
pub struct Api {
    pub tag: &'static str,
    pub path: PathBuf,
    pub create_buffer: FnCreate,
    pub append_to_buffer: FnAppend,
    pub destroy_buffer: FnDestroy,
    pub get_operation_name: FnOpName,
    pub perform_operation: FnPerform,
    pub buffapp: FnBuffapp,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// The C `.so` name is derived from the parent directory name by CMake, so it is
/// environment dependent -- glob for it rather than hard-coding it.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}\nBuild the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so") && p.is_file()
        })
        .collect();
    found.sort();
    found
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no lib*.so in {}", build.display()))
}

/// Prefer the release cdylib (what an external consumer would ship); fall back
/// to the debug one that `cargo test` itself produces.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = repo_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libbuffapp_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!("libbuffapp_lib.so not found; run `cargo build --release` first");
}

unsafe fn load(tag: &'static str, path: PathBuf) -> Api {
    // RTLD_LOCAL (libloading's default) is essential: both objects export the
    // same six names, and we must resolve each within its own object.
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
    ));
    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: Symbol<$t> = lib
                .get($n)
                .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", tag, stringify!($n)));
            *s
        }};
    }
    Api {
        tag,
        path,
        create_buffer: sym!(FnCreate, b"create_buffer\0"),
        append_to_buffer: sym!(FnAppend, b"append_to_buffer\0"),
        destroy_buffer: sym!(FnDestroy, b"destroy_buffer\0"),
        get_operation_name: sym!(FnOpName, b"get_operation_name\0"),
        perform_operation: sym!(FnPerform, b"perform_operation\0"),
        buffapp: sym!(FnBuffapp, b"buffapp\0"),
    }
}

struct Both {
    c: Api,
    r: Api,
}
unsafe impl Sync for Both {}
unsafe impl Send for Both {}

static BOTH: OnceLock<Both> = OnceLock::new();

fn both() -> &'static Both {
    BOTH.get_or_init(|| unsafe {
        Both {
            c: load("C", c_so_path()),
            r: load("RUST", rust_so_path()),
        }
    })
}

pub fn c() -> &'static Api {
    &both().c
}
pub fn rs() -> &'static Api {
    &both().r
}

/// Iterate `(c_api, rust_api)` so every test body runs identically on both.
pub fn pair() -> (&'static Api, &'static Api) {
    (c(), rs())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { SEED } else { s })
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
        self.next_u64() as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// An `i32` biased toward interesting values (boundaries, small magnitudes,
    /// values around multiples of 4) mixed with uniform draws.
    pub fn spicy_i32(&mut self) -> i32 {
        const SPECIAL: [i32; 20] = [
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 2,
            -2_000_000_000,
            -100_000,
            -8,
            -7,
            -5,
            -4,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            i32::MAX - 1,
            i32::MAX,
        ];
        match self.below(4) {
            0 => SPECIAL[self.below(SPECIAL.len() as u64) as usize],
            1 => self.range_i32(-20, 20),
            2 => self.range_i32(-1000, 1000),
            _ => self.next_i32(),
        }
    }
    /// `ascii_bytes` with a randomly chosen length in `0..n`.
    pub fn ascii_below(&mut self, n: u64) -> Vec<u8> {
        let len = self.below(n) as usize;
        self.ascii_bytes(len)
    }
    pub fn ascii_bytes(&mut self, len: usize) -> Vec<u8> {
        // Printable, never contains NUL, so it round-trips through strcpy.
        (0..len)
            .map(|_| b'!' + (self.below(93) as u8))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// C-string helpers
// ---------------------------------------------------------------------------
pub fn cstring(s: &[u8]) -> Vec<c_char> {
    assert!(!s.contains(&0), "test strings must not embed NUL");
    let mut v: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

/// Read a NUL-terminated string returned across the FFI boundary.
pub unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    assert!(!p.is_null(), "unexpected NULL string");
    CStr::from_ptr(p).to_bytes().to_vec()
}

/// Read exactly `n` bytes of a buffer's `data`.
pub unsafe fn read_n(p: *const c_char, n: usize) -> Vec<u8> {
    assert!(!p.is_null());
    std::slice::from_raw_parts(p as *const u8, n).to_vec()
}

/// The full comparable state of a `StringBuffer`: the scalar fields plus the
/// `data` bytes up to and including the terminating NUL at `length`.
#[derive(Debug, PartialEq, Eq)]
pub struct BufState {
    pub is_null: bool,
    pub capacity: c_int,
    pub length: c_int,
    pub bytes: Vec<u8>,
}

pub unsafe fn snapshot(b: *const StringBuffer) -> BufState {
    if b.is_null() {
        return BufState { is_null: true, capacity: 0, length: 0, bytes: Vec::new() };
    }
    let length = (*b).length;
    let data = (*b).data;
    // Only bytes actually written by the implementation are comparable;
    // everything past the NUL at `length` is uninitialised malloc memory.
    let bytes = if data.is_null() || length < 0 {
        Vec::new()
    } else {
        read_n(data, length as usize + 1)
    };
    BufState { is_null: false, capacity: (*b).capacity, length, bytes }
}

/// Like `snapshot`, but reads `extra` bytes past the NUL. Only valid when the
/// caller knows those bytes were written by an earlier append (high-water mark).
pub unsafe fn snapshot_hwm(b: *const StringBuffer, hwm: usize) -> BufState {
    let mut s = snapshot(b);
    if !s.is_null && !(*b).data.is_null() {
        let n = hwm.max(s.bytes.len());
        s.bytes = read_n((*b).data, n);
    }
    s
}

// ---------------------------------------------------------------------------
// stdout capture (buffapp calls printf)
// ---------------------------------------------------------------------------
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with fd 1 redirected into a temp file and return `(result, bytes)`.
/// Serialised process-wide because fd 1 is global state.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush anything already pending so it lands on the real stdout and
        // cannot spill into our capture file: first Rust's own `Stdout`
        // buffer (libtest's progress lines live there), then all C streams.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        libc::fflush(std::ptr::null_mut());

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!("{dir}/buffapp_cap_{}_{:p}.tmp", std::process::id(), &saved);
        let cpath = cstring(path.as_bytes());
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open {path} failed");

        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        let r = f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        libc::lseek(fd, 0, libc::SEEK_SET);
        let mut out = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = libc::read(fd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(fd);
        libc::unlink(cpath.as_ptr());
        (r, out)
    }
}

// ---------------------------------------------------------------------------
// Crash / signal comparison for the UB rows of ERRORS.md.
//
// Runs `f` in a forked child and reports how the child terminated. Both
// implementations must terminate with the *same* signal number -- "both failed
// somehow" is not accepted.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signal(i32),
}

pub fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    let _g = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: silence any output, run the call, exit 0 if it survives.
            let devnull = cstring(b"/dev/null");
            let fd = libc::open(devnull.as_ptr(), libc::O_WRONLY, 0);
            if fd >= 0 {
                libc::dup2(fd, 1);
                libc::dup2(fd, 2);
            }
            f();
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        loop {
            let r = libc::waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            if r < 0 && *libc::__errno_location() != libc::EINTR {
                panic!("waitpid failed");
            }
        }
        if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    }
}

/// Assert that both implementations terminate identically for a UB input.
pub fn assert_same_outcome(label: &str, cf: impl FnOnce(), rf: impl FnOnce()) -> Outcome {
    let oc = outcome_of(cf);
    let or = outcome_of(rf);
    assert_eq!(oc, or, "{label}: C outcome {oc:?} != Rust outcome {or:?}");
    oc
}

/// Reserve a big read/write anonymous mapping (virtual only, MAP_NORESERVE) so
/// that a wildly out-of-bounds `strcpy` offset lands on valid memory and can be
/// observed instead of faulting. Returns NULL if the reservation is refused.
pub struct BigMap {
    pub base: *mut u8,
    pub len: usize,
}

impl BigMap {
    pub fn new(len: usize) -> Option<BigMap> {
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            );
            if p == libc::MAP_FAILED {
                None
            } else {
                Some(BigMap { base: p as *mut u8, len })
            }
        }
    }
}

impl Drop for BigMap {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base as *mut c_void, self.len);
        }
    }
}

// ---------------------------------------------------------------------------
// The five operation-name literals, as they appear in the C source.
// ---------------------------------------------------------------------------
pub const OP_NAMES: [&[u8]; 5] = [b"add", b"subtract", b"multiply", b"divide", b"unknown"];
