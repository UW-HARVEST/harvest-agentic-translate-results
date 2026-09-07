// Differential test harness: loads BOTH the C .so and the Rust .so via
// libloading and compares (return value + captured stdout bytes) for every row
// of CONFIGS.md and ERRORS.md.
//
// Nothing here calls the Rust crate directly -- every Rust call goes through
// dlopen/dlsym on the cdylib, exactly as an external C consumer would, so the
// #[no_mangle] export wrappers are under test too.

#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture and for freeing malloc'ed returns.
// Both .so's share this process's libc, so `free` here matches their `malloc`.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Locating the two shared objects.
// ---------------------------------------------------------------------------
fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return p.into();
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut cands: Vec<_> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} -- build the C library first", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().unwrap().to_str().unwrap().starts_with("lib")
        })
        .collect();
    cands.sort();
    assert!(!cands.is_empty(), "no .so found in {}", build.display());
    cands.remove(0)
}

fn find_rust_so() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return p.into();
    }
    let name = "libcomplexmode_lib.so";
    for profile in ["release", "debug"] {
        let p = manifest_dir().join("target").join(profile).join(name);
        if p.exists() {
            return p;
        }
    }
    panic!("{name} not found under target/{{release,debug}} -- run `cargo build --release`");
}

// ---------------------------------------------------------------------------
// The two loaded libraries. RTLD_LOCAL (libloading's default) keeps the
// identically-named symbols of the two .so's from colliding.
// ---------------------------------------------------------------------------
pub struct Libs {
    pub c: Library,
    pub r: Library,
}

pub fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| unsafe {
        let cp = find_c_so();
        let rp = find_rust_so();
        eprintln!("C   .so: {}", cp.display());
        eprintln!("Rust.so: {}", rp.display());
        Libs {
            c: Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {}: {e}", cp.display())),
            r: Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {}: {e}", rp.display())),
        }
    })
}

// Typed symbol accessors -----------------------------------------------------
pub type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
pub type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
pub type FnCopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnComplexmode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub fn sym<T: 'static>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe {
        lib.get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("dlsym {name}: {e}"))
    }
}

/// Run `f` once against the C library and once against the Rust library.
pub fn each<T: 'static, R>(name: &'static str, mut f: impl FnMut(&Symbol<'static, T>) -> R) -> (R, R) {
    let l = libs();
    let cs: Symbol<'static, T> = sym(&l.c, name);
    let rs: Symbol<'static, T> = sym(&l.r, name);
    let a = f(&cs);
    let b = f(&rs);
    (a, b)
}

// ---------------------------------------------------------------------------
// stdout capture.
//
// The C library reports errors on stdout via printf, so stdout text is part of
// the observable result and must be compared byte-for-byte. Redirection is
// process-global, so it is serialized behind a mutex (tests otherwise run in
// parallel threads).
// ---------------------------------------------------------------------------
fn capture_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek};
    use std::os::unix::io::AsRawFd;

    let _g = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = tempfile();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp.as_raw_fd(), 1) >= 0, "dup2 failed");
        let out = f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        let mut buf = Vec::new();
        tmp.rewind().unwrap();
        tmp.read_to_end(&mut buf).unwrap();
        (out, buf)
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!(
        "difftest-{}-{}-{n}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .unwrap();
    let _ = std::fs::remove_file(&p); // unlink; fd stays valid
    f
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*), fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Mix of full-width, small, and boundary-adjacent values, so rows hit both
    /// ordinary arithmetic and wraparound.
    pub fn i32_mixed(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 8) as u32 as i32,
            1 => ((r >> 8) as i32) % 256,
            2 => -(((r >> 8) as i32).rem_euclid(256)),
            3 => i32::MAX - ((r >> 8) as i32).rem_euclid(4),
            4 => i32::MIN + ((r >> 8) as i32).rem_euclid(4),
            5 => ((r >> 8) as i32) % 65536,
            6 => 0,
            _ => (r >> 8) as u32 as i32,
        }
    }
    pub fn range(&mut self, lo: u64, hi_incl: u64) -> u64 {
        lo + self.next_u64() % (hi_incl - lo + 1)
    }
    pub fn byte_nonzero(&mut self) -> u8 {
        let b = (self.next_u64() & 0xFF) as u8;
        if b == 0 { 1 } else { b }
    }
}

/// The hard boundary values every row is also driven with.
pub const BOUNDS: &[i32] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    127,
    -128,
    128,
    255,
    256,
    65535,
    65536,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

pub const ITERS: usize = 256;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a malloc'ed C string produced by `create_result_string` /
/// `multiply_with_log`. The buffer is exactly 64 bytes; bytes past the NUL are
/// uninitialised heap, so only up to and including the NUL is compared.
pub unsafe fn read_c_buf(p: *const c_char, cap: usize) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    for i in 0..cap {
        let b = unsafe { *p.add(i) } as u8;
        v.push(b);
        if b == 0 {
            break;
        }
    }
    Some(v)
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Assert two (return value, stdout) observations are identical.
pub fn same<T: PartialEq + std::fmt::Debug>(
    what: &str,
    c: (T, Vec<u8>),
    r: (T, Vec<u8>),
) {
    assert_eq!(c.0, r.0, "{what}: return value differs (C={:?} Rust={:?})", c.0, r.0);
    assert_eq!(
        c.1,
        r.1,
        "{what}: stdout differs\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.1),
        show(&r.1)
    );
}

// ---------------------------------------------------------------------------
// fork-based differential call.
//
// Some rows of ERRORS.md are reached only by inputs on which the C itself
// crashes (an unchecked `*log_msg` store, a `memcpy` out of a buffer the C
// never sized). "Both crash" is still a comparable result -- the *signal* and
// the output-so-far must match -- but a crash in-process would take the test
// runner with it. Running the call in a forked child makes those rows testable.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn _exit(code: c_int) -> !;
}

/// What a forked differential call observed.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ForkResult {
    /// `Some(v)` if the callee returned normally, `None` if it never got there.
    pub value: Option<i64>,
    /// Terminating signal number, or 0 for a normal exit.
    pub signal: c_int,
    /// Normal exit status.
    pub exit: c_int,
    /// Bytes the callee wrote to stdout before finishing/dying.
    pub stdout: Vec<u8>,
}

/// Run `f` in a forked child with stdout redirected, and report its return
/// value, termination reason and stdout.
pub fn fork_call(f: impl FnOnce() -> i64) -> ForkResult {
    use std::io::{Read, Seek};
    use std::os::unix::io::AsRawFd;

    let _g = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = tempfile();
    let mut fds = [0 as c_int; 2];
    unsafe {
        assert!(pipe(fds.as_mut_ptr()) == 0, "pipe failed");
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            close(fds[0]);
            dup2(tmp.as_raw_fd(), 1);
            let v = f();
            fflush(std::ptr::null_mut());
            let bytes = v.to_le_bytes();
            write(fds[1], bytes.as_ptr() as *const c_void, 8);
            close(fds[1]);
            _exit(0);
        }
        // ---- parent ----
        close(fds[1]);
        let mut buf = [0u8; 8];
        let mut got = 0usize;
        while got < 8 {
            let n = read(fds[0], buf.as_mut_ptr().add(got) as *mut c_void, 8 - got);
            if n <= 0 {
                break;
            }
            got += n as usize;
        }
        close(fds[0]);
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);

        let mut out = Vec::new();
        tmp.rewind().unwrap();
        tmp.read_to_end(&mut out).unwrap();

        ForkResult {
            value: if got == 8 { Some(i64::from_le_bytes(buf)) } else { None },
            signal: status & 0x7f,
            exit: (status >> 8) & 0xff,
            stdout: out,
        }
    }
}

/// Run `f` against both libraries in forked children and require identical
/// observations (return value, termination signal, exit code, stdout).
pub fn fork_same<T: 'static>(
    what: &str,
    name: &'static str,
    mut f: impl FnMut(&Symbol<'static, T>) -> i64,
) -> ForkResult {
    let l = libs();
    let cs: Symbol<'static, T> = sym(&l.c, name);
    let rs: Symbol<'static, T> = sym(&l.r, name);
    let c = fork_call(|| f(&cs));
    let r = fork_call(|| f(&rs));
    assert_eq!(
        c.value, r.value,
        "{what}: return value differs (C={:?} Rust={:?})",
        c.value, r.value
    );
    assert_eq!(
        (c.signal, c.exit),
        (r.signal, r.exit),
        "{what}: termination differs (C sig={} exit={}, Rust sig={} exit={})",
        c.signal,
        c.exit,
        r.signal,
        r.exit
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "{what}: stdout differs\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&c.stdout),
        show(&r.stdout)
    );
    c
}

/// Read a source file, normalising CRLF, for the structural checks that back
/// the ERRORS.md rows which are unreachable at runtime (heap exhaustion).
pub fn read_src(rel_from_workspace: &str) -> String {
    let p = manifest_dir().parent().unwrap().join(rel_from_workspace);
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
        .replace("\r\n", "\n")
}
