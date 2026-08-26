//! Shared support code for the C-vs-Rust differential test suite.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! driven exclusively through their exported C symbols — the Rust functions are
//! never called directly, so the `#[no_mangle]` export wrappers are covered
//! too.

#![allow(dead_code)]

use std::ffi::c_void;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use core::ffi::{c_char, c_int};

// ---------------------------------------------------------------- C types ---

/// `buffer_t` — must be layout compatible with the C struct.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BufferT {
    pub data: [u8; 256],
    pub length: usize,
    pub checksum: u32,
}

impl BufferT {
    pub fn zeroed() -> BufferT {
        BufferT {
            data: [0u8; 256],
            length: 0,
            checksum: 0,
        }
    }

    /// A buffer whose `data` is filled with a recognisable pattern so that
    /// "the callee must not touch the tail" is actually observable.
    pub fn patterned(fill: u8) -> BufferT {
        BufferT {
            data: [fill; 256],
            length: 0,
            checksum: 0xDEAD_BEEF,
        }
    }

    /// Every *named* byte of the struct: all 256 data bytes (so a modified tail
    /// is caught), the length and the checksum.  The 4 tail padding bytes are
    /// deliberately excluded — they are indeterminate in C *and* in Rust.
    pub fn full_repr(&self) -> ([u8; 256], usize, u32) {
        (self.data, self.length, self.checksum)
    }

    /// Only the portion the C code is guaranteed to define: `length`,
    /// `checksum` and `data[0..min(length,256)]`.
    pub fn defined_repr(&self) -> (usize, u32, Vec<u8>) {
        let n = self.length.min(256);
        (self.length, self.checksum, self.data[..n].to_vec())
    }
}

/// `buffer_array_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BufferArrayT {
    pub buffers: *mut BufferT,
    pub count: c_int,
    pub capacity: c_int,
}

pub const OP_COPY: c_int = 0;
pub const OP_REVERSE: c_int = 1;
pub const OP_MERGE: c_int = 2;
pub const OP_SPLIT: c_int = 3;
pub const OP_INTERLEAVE: c_int = 4;
pub const OP_ROTATE: c_int = 5;
pub const OP_CHECKSUM: c_int = 6;

// ------------------------------------------------------- symbol signatures ---
//
// `bool` is modelled as `u8`: a C `_Bool` is one byte and a foreign
// implementation may in principle hand back / receive a value other than 0 or 1,
// which would be an invalid Rust `bool`.

pub type FnCalculateChecksum = unsafe extern "C" fn(*const u8, usize) -> u32;
pub type FnValidateBuffer = unsafe extern "C" fn(*const BufferT) -> u8;
pub type FnInitBufferArray = unsafe extern "C" fn(c_int) -> *mut BufferArrayT;
pub type FnFreeBufferArray = unsafe extern "C" fn(*mut BufferArrayT);
pub type FnBufferCopy = unsafe extern "C" fn(*const BufferT, *mut BufferT) -> c_int;
pub type FnBufferReverse = unsafe extern "C" fn(*mut BufferT) -> c_int;
pub type FnBufferMerge =
    unsafe extern "C" fn(*const BufferT, *const BufferT, *mut BufferT) -> c_int;
pub type FnBufferSplit =
    unsafe extern "C" fn(*const BufferT, usize, *mut BufferT, *mut BufferT) -> c_int;
pub type FnBufferInterleave =
    unsafe extern "C" fn(*const BufferT, *const BufferT, *mut BufferT) -> c_int;
pub type FnBufferRotate = unsafe extern "C" fn(*mut BufferT, c_int) -> c_int;
pub type FnBufferConditionalCopy =
    unsafe extern "C" fn(*const BufferT, *mut BufferT, u8, u8) -> c_int;
pub type FnBufferCopyStrided = unsafe extern "C" fn(*const BufferT, *mut BufferT, c_int) -> c_int;
pub type FnProcessBufferArray = unsafe extern "C" fn(*mut BufferArrayT, c_int, c_int) -> c_int;
pub type FnReadBuffer = unsafe extern "C" fn(*mut BufferT) -> c_int;
pub type FnWriteBuffer = unsafe extern "C" fn(*const BufferT);
pub type FnMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

/// The complete exported surface of one implementation.
pub struct Api {
    pub name: &'static str,
    pub calculate_checksum: FnCalculateChecksum,
    pub validate_buffer: FnValidateBuffer,
    pub init_buffer_array: FnInitBufferArray,
    pub free_buffer_array: FnFreeBufferArray,
    pub buffer_copy: FnBufferCopy,
    pub buffer_reverse: FnBufferReverse,
    pub buffer_merge: FnBufferMerge,
    pub buffer_split: FnBufferSplit,
    pub buffer_interleave: FnBufferInterleave,
    pub buffer_rotate: FnBufferRotate,
    pub buffer_conditional_copy: FnBufferConditionalCopy,
    pub buffer_copy_strided: FnBufferCopyStrided,
    pub process_buffer_array: FnProcessBufferArray,
    pub read_buffer: FnReadBuffer,
    pub write_buffer: FnWriteBuffer,
    pub main: FnMain,
    /// Only present in the Rust object (see SYMBOLS.md).
    pub reset_stdin: Option<unsafe extern "C" fn()>,
    _lib: &'static libloading::Library,
}

macro_rules! sym {
    ($lib:expr, $t:ty, $n:literal) => {
        *unsafe { $lib.get::<$t>(concat!($n, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $n, e))
    };
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {}", path.display(), e))
        }));
        Api {
            name,
            calculate_checksum: sym!(lib, FnCalculateChecksum, "calculate_checksum"),
            validate_buffer: sym!(lib, FnValidateBuffer, "validate_buffer"),
            init_buffer_array: sym!(lib, FnInitBufferArray, "init_buffer_array"),
            free_buffer_array: sym!(lib, FnFreeBufferArray, "free_buffer_array"),
            buffer_copy: sym!(lib, FnBufferCopy, "buffer_copy"),
            buffer_reverse: sym!(lib, FnBufferReverse, "buffer_reverse"),
            buffer_merge: sym!(lib, FnBufferMerge, "buffer_merge"),
            buffer_split: sym!(lib, FnBufferSplit, "buffer_split"),
            buffer_interleave: sym!(lib, FnBufferInterleave, "buffer_interleave"),
            buffer_rotate: sym!(lib, FnBufferRotate, "buffer_rotate"),
            buffer_conditional_copy: sym!(
                lib,
                FnBufferConditionalCopy,
                "buffer_conditional_copy"
            ),
            buffer_copy_strided: sym!(lib, FnBufferCopyStrided, "buffer_copy_strided"),
            process_buffer_array: sym!(lib, FnProcessBufferArray, "process_buffer_array"),
            read_buffer: sym!(lib, FnReadBuffer, "read_buffer"),
            write_buffer: sym!(lib, FnWriteBuffer, "write_buffer"),
            main: sym!(lib, FnMain, "main"),
            reset_stdin: unsafe {
                lib.get::<unsafe extern "C" fn()>(b"driver_reset_exported_stdin\0")
                    .ok()
                    .map(|s| *s)
            },
            _lib: lib,
        }
    }
}

// ------------------------------------------------------------ .so location ---

/// `target/<profile>` directory of the current test run.
pub fn target_profile_dir() -> PathBuf {
    // .../target/<profile>/deps/<testbin>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build (once) and return the path of the shared object compiled from the
/// untouched C sources.
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        assert!(src.exists(), "missing C source {}", src.display());
        let out_dir = target_profile_dir();
        std::fs::create_dir_all(&out_dir).ok();
        let out = out_dir.join("libdriver_c.so");
        let status = Command::new("gcc")
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&out)
            .arg(&src)
            .status()
            .expect("failed to run gcc");
        assert!(status.success(), "gcc failed building the C shared object");
        out
    })
    .as_path()
}

/// Path of the Rust `cdylib`.
///
/// `cargo test` only builds the `rlib` flavour of the library target, so if a
/// plain `cargo build` has not already produced `libdriver.so` we compile the
/// very same `src/lib.rs` into a `cdylib` with a direct `rustc` call (the crate
/// has no dependencies, so this is exact).
pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let dir = target_profile_dir();
        let src = manifest_dir().join("src/lib.rs");
        assert!(src.exists(), "missing {}", src.display());

        // Prefer the artifact a plain `cargo build` produced, but only if it is
        // not older than the sources — a stale object would silently make the
        // suite pass against code that no longer exists.
        let cargo_built = dir.join("libdriver.so");
        if cargo_built.exists() && !is_older_than_sources(&cargo_built) {
            return cargo_built;
        }

        let out = dir.join("libdriver_ffi.so");
        let status = Command::new("rustc")
            .args(["--edition", "2021", "--crate-type", "cdylib"])
            .args(["--crate-name", "driver"])
            .args(["-C", "opt-level=2"])
            .args(["-C", "debug-assertions=off"])
            .args(["-C", "overflow-checks=off"])
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .status()
            .expect("failed to run rustc");
        assert!(status.success(), "rustc failed building the Rust cdylib");
        out
    })
    .as_path()
}

fn is_older_than_sources(artifact: &Path) -> bool {
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let a = m(artifact);
    for f in ["src/lib.rs", "src/lib_impl.rs", "src/main.rs"] {
        if m(&manifest_dir().join(f)) > a {
            return true;
        }
    }
    false
}

/// Build (once) and return the path of the *executable* produced from the
/// untouched C sources — the artifact `c_src/CMakeLists.txt` describes.
pub fn c_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let src = manifest_dir().join("c_src/src/main.c");
        let out = target_profile_dir().join("driver_c_exe");
        let status = Command::new("gcc")
            .args(["-O2", "-o"])
            .arg(&out)
            .arg(&src)
            .status()
            .expect("failed to run gcc");
        assert!(status.success(), "gcc failed building the C executable");
        out
    })
    .as_path()
}

/// The Rust executable cargo built for this test run.
pub fn rust_exe_path() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Everything a whole process run produces.
#[derive(PartialEq, Eq, Clone)]
pub struct Run {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit={} stdout({} bytes)={:?} stderr={:?}",
            self.code,
            self.stdout.len(),
            String::from_utf8_lossy(&self.stdout[..self.stdout.len().min(400)]),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Run an executable with `input` on stdin and collect everything.
pub fn run_exe(exe: &Path, input: &[u8]) -> Run {
    use std::io::Write as _;
    let mut child = std::process::Command::new(exe)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {}", exe.display(), e));
    {
        let mut si = child.stdin.take().unwrap();
        // The child may exit before consuming everything (e.g. a bad header),
        // which shows up as EPIPE — that is not a test failure.
        let _ = si.write_all(input);
        let _ = si.flush();
    }
    let out = child.wait_with_output().expect("wait_with_output");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

pub fn c_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| Api::load("C", c_so_path()))
}

pub fn rust_api() -> &'static Api {
    static A: OnceLock<Api> = OnceLock::new();
    A.get_or_init(|| Api::load("Rust", rust_so_path()))
}

/// Both implementations, C first.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// -------------------------------------------------------- fd 1 / 2 capture ---

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

fn scratch_dir() -> PathBuf {
    let d = std::env::var("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| target_profile_dir().join("difftmp"));
    std::fs::create_dir_all(&d).ok();
    d
}

static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn scratch_file(tag: &str) -> (std::fs::File, PathBuf) {
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let p = scratch_dir().join(format!("{}-{}-{}.bin", tag, std::process::id(), n));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&p)
        .expect("scratch file");
    (f, p)
}

/// Everything a call to one of the two libraries can observably produce.
#[derive(PartialEq, Eq, Clone)]
pub struct Obs<T> {
    pub ret: T,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Obs<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ret={:?} stdout={:?} stderr={:?}",
            self.ret,
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// Long-lived scratch files reused by every `observe()` call (creating and
/// unlinking two files per call would dominate the runtime of the fuzz loops).
struct Scratch {
    out: std::fs::File,
    err: std::fs::File,
    stdin: std::fs::File,
    stdin_path: PathBuf,
}

fn scratch() -> &'static Mutex<Scratch> {
    static S: OnceLock<Mutex<Scratch>> = OnceLock::new();
    S.get_or_init(|| {
        let (out, _) = scratch_file("cap-stdout");
        let (err, _) = scratch_file("cap-stderr");
        let (stdin, stdin_path) = scratch_file("cap-stdin");
        Mutex::new(Scratch {
            out,
            err,
            stdin,
            stdin_path,
        })
    })
}

fn rewind_truncate(f: &mut std::fs::File) {
    f.set_len(0).expect("truncate");
    f.seek(std::io::SeekFrom::Start(0)).expect("seek");
}

/// Restores the redirected descriptors even if the observed call panics — a
/// leaked redirection would send the test harness's own output into the capture
/// file and cascade into bogus failures everywhere else.
struct FdRestore {
    out: c_int,
    err: c_int,
    inp: Option<c_int>,
}

impl Drop for FdRestore {
    fn drop(&mut self) {
        unsafe {
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            fflush(std::ptr::null_mut());
            dup2(self.out, 1);
            dup2(self.err, 2);
            close(self.out);
            close(self.err);
            if let Some(si) = self.inp {
                dup2(si, 0);
                close(si);
            }
        }
    }
}

/// These tests redirect the *process's* stdout/stderr, so they can only be run
/// one at a time — otherwise the harness's own progress output (or another
/// test's output) lands in the capture files.
fn assert_serial() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let env_ok = std::env::var("RUST_TEST_THREADS")
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        let args: Vec<String> = std::env::args().collect();
        let arg_ok = args.windows(2).any(|w| w[0] == "--test-threads" && w[1] == "1")
            || args.iter().any(|a| a == "--test-threads=1");
        assert!(
            env_ok || arg_ok,
            "the differential tests redirect the process's stdout/stderr and must run \
             serially: set RUST_TEST_THREADS=1 (the crate's .cargo/config.toml does \
             this automatically) or pass `-- --test-threads=1`"
        );
    });
}

/// Run `f` with fd 1 and fd 2 redirected into scratch files, and return what it
/// wrote. `stdin_data`, if given, is installed as fd 0 (at offset 0) first.
///
/// The whole process's file descriptors are involved, so calls are serialised
/// through a global mutex.
pub fn observe<T>(stdin_data: Option<&[u8]>, f: impl FnOnce() -> T) -> Obs<T> {
    assert_serial();
    let _g = capture_lock();
    let mut s = match scratch().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    // Make sure nothing that was produced *before* the redirection lands in the
    // capture files (glibc's stdout is block buffered; Rust's is line buffered).
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    rewind_truncate(&mut s.out);
    rewind_truncate(&mut s.err);
    if let Some(d) = stdin_data {
        rewind_truncate(&mut s.stdin);
        s.stdin.write_all(d).unwrap();
        s.stdin.flush().unwrap();
        s.stdin.seek(std::io::SeekFrom::Start(0)).unwrap();
    }

    let guard = unsafe {
        let so = dup(1);
        let se = dup(2);
        let si = if stdin_data.is_some() {
            Some(dup(0))
        } else {
            None
        };
        assert!(so >= 0 && se >= 0, "dup failed");
        let guard = FdRestore {
            out: so,
            err: se,
            inp: si,
        };
        dup2(s.out.as_raw_fd(), 1);
        dup2(s.err.as_raw_fd(), 2);
        if stdin_data.is_some() {
            dup2(s.stdin.as_raw_fd(), 0);
        }
        guard
    };

    let ret = f();
    drop(guard); // flushes and restores 1/2/0

    let (stdout, stderr) = (read_from_start(&mut s.out), read_from_start(&mut s.err));

    Obs { ret, stdout, stderr }
}

/// Path of the scratch file that `observe()` installs as fd 0 — needed by the
/// C side's `freopen(..., stdin)`.
pub fn observe_stdin_path() -> PathBuf {
    let s = match scratch().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    s.stdin_path.clone()
}

fn read_from_start(f: &mut std::fs::File) -> Vec<u8> {
    let mut v = Vec::new();
    f.seek(std::io::SeekFrom::Start(0)).ok();
    f.read_to_end(&mut v).ok();
    v
}

fn read_all(p: &Path) -> Vec<u8> {
    let mut v = Vec::new();
    if let Ok(mut f) = std::fs::File::open(p) {
        f.read_to_end(&mut v).ok();
    }
    v
}

/// Re-point the C library's `FILE *stdin` at `path`, giving it a fresh buffer
/// and offset 0 — the C counterpart of `driver_reset_exported_stdin`.
pub fn c_freopen_stdin(path: &Path) {
    extern "C" {
        fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    }
    // `stdin` is a `FILE *` *variable* in glibc.
    extern "C" {
        static mut stdin: *mut c_void;
    }
    let p = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let m = std::ffi::CString::new("r").unwrap();
    unsafe {
        let s = core::ptr::addr_of_mut!(stdin);
        let r = freopen(p.as_ptr(), m.as_ptr(), *s);
        assert!(!r.is_null(), "freopen(stdin) failed");
    }
}

// ------------------------------------------------------------------- utils ---

/// Deterministic SplitMix64 — no external RNG dependency, fully reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `[0, n)`; `n == 0` yields 0.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A buffer with a random length in `[lo, hi]`, random bytes and a
    /// consistent checksum.
    pub fn buffer(&mut self, lo: usize, hi: usize) -> BufferT {
        let len = lo + self.below(hi - lo + 1);
        self.buffer_len(len)
    }
    pub fn buffer_len(&mut self, len: usize) -> BufferT {
        let mut b = BufferT::zeroed();
        // Fill the *whole* data array so the tail is meaningful for
        // "must not be touched" assertions.
        for i in 0..256 {
            b.data[i] = self.u8();
        }
        b.length = len;
        b.checksum = checksum(&b.data[..len.min(256)]);
        b
    }
}

/// Reference implementation of the C checksum, used only to build inputs whose
/// `checksum` field is *consistent* (so the warning path is not triggered).
pub fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    for &b in data {
        sum = (sum << 3) ^ (b as u32);
    }
    sum
}

/// Assert that both implementations observed the exact same thing.
#[track_caller]
pub fn same<T: PartialEq + std::fmt::Debug>(what: &str, c: &Obs<T>, r: &Obs<T>) {
    if c.ret != r.ret || c.stdout != r.stdout || c.stderr != r.stderr {
        panic!(
            "divergence in {}\n  C   : {:?}\n  Rust: {:?}",
            what, c, r
        );
    }
}

#[track_caller]
pub fn same_buf(what: &str, c: &BufferT, r: &BufferT) {
    if c.full_repr() != r.full_repr() {
        let first_diff = (0..256).find(|&i| c.data[i] != r.data[i]);
        panic!(
            "buffer divergence in {}\n  C   : len={} cks={} data[..{}]={:?}\n  Rust: len={} cks={} data[..{}]={:?}\n  first differing data index: {:?}",
            what,
            c.length,
            c.checksum,
            c.length.min(256),
            &c.data[..c.length.min(256)],
            r.length,
            r.checksum,
            r.length.min(256),
            &r.data[..r.length.min(256)],
            first_diff
        );
    }
}

/// Run one scenario against **both** shared objects and require byte-identical
/// results.
///
/// * `setup` produces the working set of `buffer_t`s (called twice, once per
///   implementation — it must be deterministic; this is asserted).
/// * `body` receives the implementation and a raw pointer to the working set,
///   so scenarios are free to alias arguments (`src == dst`, …) exactly like a
///   real C caller could.
///
/// Compared: return value, stdout, stderr and the **full 272-byte image** of
/// every buffer in the working set (so "the callee left the tail alone" is
/// verified too).  `full_image = false` restricts the buffer comparison to the
/// bytes the C code is guaranteed to define, which is needed only where the C
/// original copies the tail of an uninitialized stack object.
#[track_caller]
pub fn diff_bufs<S, F>(what: &str, setup: S, body: F, full_image: bool)
where
    S: Fn() -> Vec<BufferT>,
    F: Fn(&Api, *mut BufferT) -> i64,
{
    let (c, r) = both();
    let mut cv = setup();
    let mut rv = setup();
    assert_eq!(cv.len(), rv.len(), "{}: setup is not deterministic", what);
    for i in 0..cv.len() {
        assert!(
            cv[i].full_repr() == rv[i].full_repr(),
            "{}: setup is not deterministic at buf[{}]",
            what,
            i
        );
    }

    let cp = cv.as_mut_ptr();
    let co = observe(None, || body(c, cp));
    let rp = rv.as_mut_ptr();
    let ro = observe(None, || body(r, rp));

    same(what, &co, &ro);
    for i in 0..cv.len() {
        let tag = format!("{} buf[{}]", what, i);
        if full_image {
            same_buf(&tag, &cv[i], &rv[i]);
        } else {
            same_buf_defined(&tag, &cv[i], &rv[i]);
        }
    }
}

/// Like `same_buf` but only over the bytes the C code actually defines
/// (used where the C original copies an uninitialized stack object's tail).
#[track_caller]
pub fn same_buf_defined(what: &str, c: &BufferT, r: &BufferT) {
    if c.defined_repr() != r.defined_repr() {
        panic!(
            "buffer divergence in {}\n  C   : {:?}\n  Rust: {:?}",
            what,
            c.defined_repr(),
            r.defined_repr()
        );
    }
}
