//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as *shared libraries* and called through
//! their exported `process_strings` symbol, so the Rust `#[no_mangle]` wrapper
//! is exercised exactly like an external C caller would exercise it:
//!
//! * the C library is compiled from `c_src/src/lib.c` with `gcc -shared -fPIC`
//!   (nothing inside `c_src/` is modified),
//! * the Rust library is the `cdylib` of this crate, built into a separate
//!   target directory so that it can be produced from inside a test without
//!   fighting for cargo's build lock.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// `int process_strings(char *, size_t, const char *, size_t, int, uint32_t)`
pub type ProcessStrings = unsafe extern "C" fn(
    *mut c_char,
    usize,
    *const c_char,
    usize,
    c_int,
    u32,
) -> c_int;

pub struct Impls {
    pub c: ProcessStrings,
    pub rust: ProcessStrings,
    _libs: Vec<Library>,
}

// The libraries are only ever used through plain function pointers.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = std::fs::metadata(a).and_then(|m| m.modified());
    let mb = std::fs::metadata(b).and_then(|m| m.modified());
    match (ma, mb) {
        (Ok(ta), Ok(tb)) => ta > tb,
        _ => true,
    }
}

/// Compile `c_src/src/lib.c` into a shared library (once).
fn build_c_so() -> PathBuf {
    let root = manifest_dir();
    let out_dir = root.join("target").join("diff");
    std::fs::create_dir_all(&out_dir).expect("create target/diff");
    let out = out_dir.join("libcstrfun.so");
    let src = root.join("c_src").join("src").join("lib.c");
    if !out.exists() || newer(&src, &out) {
        let status = Command::new("gcc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&out)
            .arg(&src)
            .status()
            .expect("run gcc");
        assert!(status.success(), "compiling the C shared library failed");
    }
    out
}

/// Build this crate's `cdylib` into its own target directory (once).
fn build_rust_so() -> PathBuf {
    let root = manifest_dir();
    let target = root.join("target").join("sodiff");
    let out = target.join("debug").join("libdriver.so");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args(["build", "--offline", "--lib", "--target-dir"])
        .arg(&target)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .status()
        .expect("run cargo build for the cdylib");
    assert!(status.success(), "building the Rust cdylib failed");
    assert!(out.exists(), "{} was not produced", out.display());
    out
}

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = build_c_so();
        let rust_path = build_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path).expect("load C .so");
            let rust_lib = Library::new(&rust_path).expect("load Rust .so");
            let c_sym: Symbol<ProcessStrings> =
                c_lib.get(b"process_strings\0").expect("C process_strings");
            let rust_sym: Symbol<ProcessStrings> = rust_lib
                .get(b"process_strings\0")
                .expect("Rust process_strings");
            let c = *c_sym;
            let rust = *rust_sym;
            Impls {
                c,
                rust,
                _libs: vec![c_lib, rust_lib],
            }
        }
    })
}

// ---------------------------------------------------------------------------
// memory region handed to the library
// ---------------------------------------------------------------------------

/// A scratch region that holds the `input` and `reference` buffers.
///
/// The C code reads past the end of both buffers whenever they are not NUL
/// terminated, so the bytes that follow them must be identical for both
/// implementations: they live in one region that is refilled with a
/// deterministic pattern before every call pair.
pub struct Region {
    buf: Vec<u8>,
}

impl Region {
    pub const SIZE: usize = 16384;
    pub const INPUT: usize = 1024;
    pub const REF: usize = 8192;

    pub fn new() -> Region {
        Region {
            buf: vec![0u8; Region::SIZE],
        }
    }

    /// Junk that mimics a stack frame: six non-zero bytes then two zero bytes
    /// per machine word, so an overread stops within at most eight bytes.
    fn sparse_junk(i: usize) -> u8 {
        if i % 8 >= 6 {
            0
        } else {
            let mixed = (i as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .rotate_left(17)
                ^ 0x5851_F42D_4C95_7F2D;
            ((mixed >> 24) as u8) | 0x80
        }
    }

    fn dense_junk(i: usize) -> u8 {
        let mixed = (i as u64)
            .wrapping_mul(0xD6E8_FEB8_6659_FD93)
            .rotate_left(29)
            ^ 0x2545_F491_4F6C_DD1D;
        ((mixed >> 32) as u8) | 0x01
    }

    /// Place both buffers, surrounded by "stack like" junk.
    pub fn place(&mut self, input: &[u8], reference: &[u8]) {
        for i in 0..Region::SIZE {
            self.buf[i] = Region::sparse_junk(i);
        }
        self.buf[Region::INPUT..Region::INPUT + input.len()].copy_from_slice(input);
        self.buf[Region::REF..Region::REF + reference.len()].copy_from_slice(reference);
    }

    /// Place both buffers with `tail` non-zero junk bytes after them, so an
    /// overread runs for a long distance before it hits a NUL.
    pub fn place_dense(&mut self, input: &[u8], reference: &[u8], tail: usize) {
        for i in 0..Region::SIZE {
            self.buf[i] = Region::dense_junk(i);
        }
        self.buf[Region::INPUT..Region::INPUT + input.len()].copy_from_slice(input);
        self.buf[Region::REF..Region::REF + reference.len()].copy_from_slice(reference);
        let stop_in = Region::INPUT + input.len() + tail;
        let stop_ref = Region::REF + reference.len() + tail;
        if stop_in < Region::SIZE {
            self.buf[stop_in] = 0;
        }
        if stop_ref < Region::SIZE {
            self.buf[stop_ref] = 0;
        }
    }

    pub fn input_ptr(&mut self) -> *mut c_char {
        unsafe { self.buf.as_mut_ptr().add(Region::INPUT) as *mut c_char }
    }

    pub fn ref_ptr(&mut self) -> *const c_char {
        unsafe { self.buf.as_mut_ptr().add(Region::REF) as *const c_char }
    }

    /// Overwrite bytes at an arbitrary offset inside the region.
    pub fn write(&mut self, off: usize, data: &[u8]) {
        self.buf[off..off + data.len()].copy_from_slice(data);
    }

    pub fn at(&mut self, off: usize) -> *mut c_char {
        unsafe { self.buf.as_mut_ptr().add(off) as *mut c_char }
    }
}

// ---------------------------------------------------------------------------
// calling both implementations
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Args {
    pub input: *mut c_char,
    pub input_len: usize,
    pub reference: *const c_char,
    pub ref_len: usize,
    pub operation: c_int,
    pub flags: u32,
}

/// Call both implementations directly and assert they agree.
#[track_caller]
pub fn diff(args: Args, what: &str) -> i32 {
    let im = impls();
    let c = unsafe {
        (im.c)(
            args.input,
            args.input_len,
            args.reference,
            args.ref_len,
            args.operation,
            args.flags,
        )
    };
    let r = unsafe {
        (im.rust)(
            args.input,
            args.input_len,
            args.reference,
            args.ref_len,
            args.operation,
            args.flags,
        )
    };
    assert_eq!(
        c, r,
        "C/Rust divergence for {what}: op={} flags={:#x} input_len={} ref_len={}",
        args.operation, args.flags, args.input_len, args.ref_len
    );
    c
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Value(i32),
    Signal(i32),
    Other(i32),
}

/// Call one implementation in a forked child, so that a deliberate
/// out-of-bounds walk (which the C code performs for some inputs) is observable
/// instead of killing the test process.
pub fn call_forked(f: ProcessStrings, args: Args) -> Outcome {
    unsafe {
        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe");
        let pid = libc::fork();
        assert!(pid >= 0, "fork");
        if pid == 0 {
            libc::close(fds[0]);
            // The C code walks off the end of its buffers for some inputs; do
            // not litter the tree with core files when that kills the child.
            let no_core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            let v = f(
                args.input,
                args.input_len,
                args.reference,
                args.ref_len,
                args.operation,
                args.flags,
            );
            let bytes = v.to_ne_bytes();
            libc::write(fds[1], bytes.as_ptr() as *const libc::c_void, 4);
            libc::_exit(0);
        }
        libc::close(fds[1]);
        let mut buf = [0u8; 4];
        let n = libc::read(fds[0], buf.as_mut_ptr() as *mut libc::c_void, 4);
        libc::close(fds[0]);
        let mut status: c_int = 0;
        libc::waitpid(pid, &mut status, 0);
        if libc::WIFSIGNALED(status) {
            Outcome::Signal(libc::WTERMSIG(status))
        } else if libc::WIFEXITED(status) && n == 4 {
            Outcome::Value(i32::from_ne_bytes(buf))
        } else if libc::WIFEXITED(status) {
            Outcome::Other(libc::WEXITSTATUS(status))
        } else {
            Outcome::Other(status)
        }
    }
}

/// Call both implementations in forked children and assert the outcomes agree
/// (same value, or killed by the same signal).
#[track_caller]
pub fn diff_forked(args: Args, what: &str) -> Outcome {
    let im = impls();
    let c = call_forked(im.c, args);
    let r = call_forked(im.rust, args);
    assert_eq!(
        c, r,
        "C/Rust divergence for {what}: op={} flags={:#x} input_len={} ref_len={}",
        args.operation, args.flags, args.input_len, args.ref_len
    );
    c
}

/// `diff`, but run in forked children whenever the configuration can make the C
/// code walk off the end of its buffers (`operation == 4` with
/// `case_sensitive != 0` underflows `text_len - pattern_len`, and any
/// caller-supplied length may be nonsense).
#[track_caller]
pub fn diff_auto(args: Args, what: &str) -> Outcome {
    let risky = (args.operation == 4 && args.flags & 0x02 != 0)
        || args.input_len > Region::SIZE
        || args.ref_len > Region::SIZE;
    if risky {
        diff_forked(args, what)
    } else {
        Outcome::Value(diff(args, what))
    }
}

// ---------------------------------------------------------------------------
// executable level harness
// ---------------------------------------------------------------------------

/// Bytes of the modelled `main` frame that `probe/inject_frame` may overwrite
/// (everything below the saved `rbp`).
pub const MODEL_END: usize = 2096;

fn out_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("diff");
    std::fs::create_dir_all(&d).expect("create target/diff");
    d
}

/// Compile one of the ptrace helpers from `probe/`.
pub fn build_helper(name: &str, src: &str) -> PathBuf {
    let exe = out_dir().join(name);
    let src = manifest_dir().join("probe").join(src);
    let status = Command::new("gcc")
        .arg("-O2")
        .arg("-o")
        .arg(&exe)
        .arg(&src)
        .status()
        .expect("run gcc");
    assert!(status.success(), "compiling {} failed", src.display());
    exe
}

/// Build the C driver the way `c_src/CMakeLists.txt` does (without touching
/// `c_src/`).
pub fn c_driver() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = manifest_dir();
        let build_dir = out_dir().join("cbuild");
        let exe = build_dir.join("driver");
        let ok = Command::new("cmake")
            .arg("-S")
            .arg(root.join("c_src"))
            .arg("-B")
            .arg(&build_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            && Command::new("cmake")
                .arg("--build")
                .arg(&build_dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        if !ok || !exe.exists() {
            std::fs::create_dir_all(&build_dir).expect("create build dir");
            let status = Command::new("gcc")
                .arg("-o")
                .arg(&exe)
                .arg(root.join("c_src").join("src").join("main.c"))
                .arg(root.join("c_src").join("src").join("lib.c"))
                .status()
                .expect("run gcc");
            assert!(status.success(), "building the C driver failed");
        }
        exe
    })
    .as_path()
}

pub fn rust_driver() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Address of `process_strings` in the (non-PIE) C driver.
pub fn bp_addr(driver: &Path) -> String {
    let out = Command::new("nm").arg(driver).output().expect("run nm");
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let addr = it.next().unwrap_or("");
        let kind = it.next().unwrap_or("");
        let name = it.next().unwrap_or("");
        if name == "process_strings" && (kind == "T" || kind == "t") {
            return addr.to_string();
        }
    }
    panic!("process_strings not found in {}", driver.display());
}

/// A stdin file that is unique per call - the tests inside one binary run in
/// parallel threads and must not share it.
pub fn unique_stdin(input: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = out_dir().join(format!("stdin_{}_{}.txt", std::process::id(), n));
    std::fs::write(&path, input).expect("write stdin file");
    path
}

#[derive(PartialEq, Eq, Debug)]
pub struct Run {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Run a program with `input` on stdin. A child killed by a signal is reported
/// as `128 + signal` (the shell's convention, which the injector reproduces).
fn run_prog(path: &Path, input: &str) -> Run {
    let stdin_file = unique_stdin(input);
    let out = Command::new("sh")
        .arg("-c")
        .arg("ulimit -c 0; \"$0\" < \"$1\"")
        .arg(path)
        .arg(&stdin_file)
        .output()
        .expect("run program");
    let _ = std::fs::remove_file(&stdin_file);
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

pub fn run_rust_exe(input: &str) -> Run {
    run_prog(rust_driver(), input)
}

pub fn run_c_exe(input: &str) -> Run {
    run_prog(c_driver(), input)
}

struct Injector {
    injector: PathBuf,
    driver: PathBuf,
    bp: String,
    junk: PathBuf,
}

fn injector() -> &'static Injector {
    static I: OnceLock<Injector> = OnceLock::new();
    I.get_or_init(|| {
        let injector = build_helper("inject_frame", "inject_frame.c");
        let driver = c_driver().to_path_buf();
        let bp = bp_addr(&driver);
        let junk = out_dir().join("frame_junk.bin");
        std::fs::write(&junk, &driver::frame_junk::FRAME_JUNK[..MODEL_END])
            .expect("write junk snapshot");
        Injector {
            injector,
            driver,
            bp,
            junk,
        }
    })
}

/// Run the C driver with the uninitialised part of its `main` frame overwritten
/// by the snapshot the Rust translation uses, so that the comparison does not
/// depend on the loader left-overs of the current environment.
pub fn run_c_exe_injected(input: &str) -> Run {
    let h = injector();
    let stdin_file = unique_stdin(input);
    let out = Command::new("sh")
        .arg("-c")
        .arg("ulimit -c 0; exec timeout 25 \"$0\" \"$1\" \"$2\" \"$3\" \"$4\"")
        .arg(&h.injector)
        .arg(&h.driver)
        .arg(&h.bp)
        .arg(&h.junk)
        .arg(&stdin_file)
        .output()
        .expect("run injector");
    let _ = std::fs::remove_file(&stdin_file);
    let code = out.status.code().unwrap_or(-1);
    assert_ne!(
        code, 90,
        "the injector never reached process_strings - the frame was not controlled"
    );
    assert_ne!(code, 91, "the child died before process_strings was reached");
    assert!(
        code < 80 || code > 128,
        "injector failed with code {code}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        code,
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

#[track_caller]
fn assert_same(c: Run, r: Run, what: &str, input: &str) -> Run {
    assert_eq!(
        c,
        r,
        "C/Rust divergence for {what}\ninput: {:?}\nC:    code={} out={:?} err={:?}\nRust: code={} out={:?} err={:?}",
        &input[..input.len().min(400)],
        c.code,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&c.stderr),
        r.code,
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr),
    );
    c
}

/// Differential run of the two real executables (only valid for inputs whose
/// result does not depend on the uninitialised frame bytes).
#[track_caller]
pub fn diff_exe(input: &str, what: &str) -> Run {
    let c = run_c_exe(input);
    let r = run_rust_exe(input);
    assert_same(c, r, what, input)
}

/// Differential run with a controlled `main` frame.
#[track_caller]
pub fn diff_exe_injected(input: &str, what: &str) -> Run {
    let c = run_c_exe_injected(input);
    let r = run_rust_exe(input);
    assert_same(c, r, what, input)
}

/// Build the stdin token stream `main` expects.
pub fn exe_case(op: i64, flags: u32, input: &[u8], reference: &[u8]) -> String {
    let mut s = format!("{op} {flags} {}", input.len());
    for b in input {
        s.push(' ');
        s.push_str(&b.to_string());
    }
    s.push(' ');
    s.push_str(&reference.len().to_string());
    for b in reference {
        s.push(' ');
        s.push_str(&b.to_string());
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// deterministic randomness
// ---------------------------------------------------------------------------

/// Deterministic splitmix64. Interior mutability keeps call sites readable
/// (`rand_bytes(&rng, rng.below(8), rng.bool())`).
pub struct Rng(std::cell::Cell<u64>);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(std::cell::Cell::new(seed ^ 0x9E37_79B9_7F4A_7C15))
    }

    pub fn next_u64(&self) -> u64 {
        let mut z = self.0.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        self.0.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn byte(&self) -> u8 {
        (self.next_u64() >> 13) as u8
    }

    pub fn pick<T: Copy>(&self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    pub fn bool(&self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// The literals the C code compares against.
pub const COMMANDS: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
pub const VARIATIONS: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
pub const LITERALS: [&[u8]; 10] = [
    b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET", b"ADMIN", b"VALID", b"OK", b"NONE", b"EMPTY",
];

/// Random byte string; `terminated` appends a NUL.
pub fn rand_bytes(rng: &Rng, len: usize, terminated: bool) -> Vec<u8> {
    let mut v: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
    if terminated {
        v.push(0);
    }
    v
}
