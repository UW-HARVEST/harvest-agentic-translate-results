//! Differential-test harness.
//!
//! Both the C shared object and the Rust `cdylib` are loaded with `libloading`
//! and every call is made through the dynamic symbol table, so the tests
//! exercise the `#[no_mangle]` export wrappers exactly like an external C caller
//! would. No Rust function of the crate under test is ever called directly (the
//! crate is not even linked into the test binary).
//!
//! Configuration comes from the environment so one test binary can be pointed at
//! any `(OP, REPEAT)` pair:
//!
//! * `MD_C_SO`    — path to the C `.so`    (default `../cbuild/so/libdriver_add_5.so`)
//! * `MD_RUST_SO` — path to the Rust `.so` (default `target/<profile>/libmacrodepth_add_5.so`)
//! * `MD_OP`      — `add` | `sub` | `mul`  (default `add`) — used to sanity-check
//!   that the loaded C library really is the configuration under test
//! * `MD_REPEAT`  — `0..7`                 (default `5`) — likewise

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

use libloading::{Library, Symbol};

/* ------------------------------------------------------------------ */
/* libc bits the harness itself needs                                  */
/* ------------------------------------------------------------------ */

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/* ------------------------------------------------------------------ */
/* Function-pointer types of the public surface                        */
/* ------------------------------------------------------------------ */

pub type Bin = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Un = unsafe extern "C" fn(c_int) -> c_int;
pub type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

/// One loaded shared object plus its resolved symbols.
pub struct Driver {
    pub path: PathBuf,
    lib: Library,
}

impl Driver {
    pub fn open(path: PathBuf) -> Driver {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Driver { path, lib }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get::<T>(name) }.unwrap_or_else(|e| {
            panic!(
                "dlsym({}, {}) failed: {e}",
                self.path.display(),
                String::from_utf8_lossy(name)
            )
        })
    }

    pub fn op_add(&self) -> Symbol<'_, Bin> {
        self.sym(b"op_add\0")
    }
    pub fn op_sub(&self) -> Symbol<'_, Bin> {
        self.sym(b"op_sub\0")
    }
    pub fn op_mul(&self) -> Symbol<'_, Bin> {
        self.sym(b"op_mul\0")
    }
    pub fn helper_call(&self) -> Symbol<'_, Bin> {
        self.sym(b"helper_call\0")
    }
    pub fn helper_ptr(&self) -> Symbol<'_, Bin> {
        self.sym(b"helper_ptr\0")
    }
    pub fn use_generated(&self) -> Symbol<'_, Un> {
        self.sym(b"use_generated\0")
    }
    pub fn main_fn(&self) -> Symbol<'_, MainFn> {
        self.sym(b"main\0")
    }

    /// Address of the exported `G_OP` object (a writable `int (*)(int,int)`).
    pub fn g_op_slot(&self) -> *mut usize {
        *self.sym::<*mut usize>(b"G_OP\0")
    }
    /// Address of the exported `G_OP_NAME` object (a writable `const char *`).
    pub fn g_op_name_slot(&self) -> *mut *const c_char {
        *self.sym::<*mut *const c_char>(b"G_OP_NAME\0")
    }

    /// Current value stored in `G_OP`.
    pub fn g_op_value(&self) -> usize {
        unsafe { *self.g_op_slot() }
    }
    /// Store a value into `G_OP` (the C object lives in writable `.data`).
    pub fn set_g_op(&self, v: usize) {
        unsafe { *self.g_op_slot() = v }
    }
    /// Current value stored in `G_OP_NAME`.
    pub fn g_op_name_value(&self) -> *const c_char {
        unsafe { *self.g_op_name_slot() }
    }
    pub fn set_g_op_name(&self, v: *const c_char) {
        unsafe { *self.g_op_name_slot() = v }
    }

    /// Addresses of this library's three exported ops, in `add, sub, mul` order.
    pub fn op_addresses(&self) -> [usize; 3] {
        [
            *self.op_add() as *const () as usize,
            *self.op_sub() as *const () as usize,
            *self.op_mul() as *const () as usize,
        ]
    }

    /// Which of the three exported ops a raw pointer value refers to
    /// (`Some("add"|"sub"|"mul")`), so `G_OP` can be compared across libraries
    /// even though the absolute addresses necessarily differ.
    pub fn classify_op(&self, v: usize) -> Option<&'static str> {
        let [a, s, m] = self.op_addresses();
        match v {
            _ if v == a => Some("add"),
            _ if v == s => Some("sub"),
            _ if v == m => Some("mul"),
            _ => None,
        }
    }

    /// Restore the build-time default of `G_OP` / `G_OP_NAME`, so tests that
    /// mutate the globals do not leak state into later tests.
    pub fn reset_globals(&self, saved: (usize, *const c_char)) {
        self.set_g_op(saved.0);
        self.set_g_op_name(saved.1);
    }
    pub fn saved_globals(&self) -> (usize, *const c_char) {
        (self.g_op_value(), self.g_op_name_value())
    }
}

/* ------------------------------------------------------------------ */
/* The pair under test                                                 */
/* ------------------------------------------------------------------ */

pub struct Pair {
    pub c: Driver,
    pub rs: Driver,
    pub op: String,
    pub repeat: c_int,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn default_c_so(op: &str, repeat: c_int) -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("cbuild/so")
        .join(format!("libdriver_{op}_{repeat}.so"))
}

fn default_rust_so() -> PathBuf {
    // target/<profile>/deps/<test-bin> -> target/<profile>/libmacrodepth_add_5.so
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("target dir")
        .join("libmacrodepth_add_5.so")
}

impl Pair {
    pub fn load() -> Pair {
        let op = std::env::var("MD_OP").unwrap_or_else(|_| "add".to_string());
        let repeat: c_int = std::env::var("MD_REPEAT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let c_path = std::env::var_os("MD_C_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_c_so(&op, repeat));
        let rs_path = std::env::var_os("MD_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(default_rust_so);
        let p = Pair {
            c: Driver::open(c_path),
            rs: Driver::open(rs_path),
            op,
            repeat,
        };
        p.self_check();
        p
    }

    /// Guard against a mis-configured harness: verify the *C* library really is
    /// the `(OP, REPEAT)` configuration the environment claims, using facts
    /// derived independently from the C macros. Without this, a mismatched pair
    /// of `.so`s would produce confusing "translation" failures.
    fn self_check(&self) {
        let name = unsafe { CStr::from_ptr(self.c.g_op_name_value()) }
            .to_str()
            .unwrap();
        assert_eq!(
            name, self.op,
            "loaded C .so reports OP={name} but MD_OP={} ({})",
            self.op,
            self.c.path.display()
        );
        assert_eq!(
            self.c.classify_op(self.c.g_op_value()),
            Some(self.op.as_str()),
            "C G_OP does not point at op_{}",
            self.op
        );
        // helper_call(0, 0) == INIT ∘ REP<REPEAT>, computed here from the C
        // macro definitions for the claimed configuration.
        let expect = expected_run_loop(&self.op, self.repeat);
        let (got, _) = capture(|| unsafe { (self.c.helper_call())(0, 0) });
        assert_eq!(
            got, expect,
            "loaded C .so does not behave like OP={} REPEAT={} (helper_call(0,0)={got}, expected {expect})",
            self.op, self.repeat
        );
    }
}

/// `INIT_FOR(op)` threaded through `REP<repeat>` — the value the C
/// `RUN_LOOP(OP, acc, REPEAT)` leaves in `acc`, recomputed independently of the
/// library under test.
pub fn expected_run_loop(op: &str, repeat: c_int) -> c_int {
    let mut acc: c_int = if op == "mul" { 1 } else { 0 };
    for i in 0..repeat {
        acc = match op {
            "add" => acc.wrapping_add(i),
            "sub" => acc.wrapping_sub(i),
            "mul" => acc.wrapping_mul(i.wrapping_add(1)),
            _ => unreachable!(),
        };
    }
    acc
}

/* ------------------------------------------------------------------ */
/* stdout/stderr capture (fd level, so C `printf` buffering is honored) */
/* ------------------------------------------------------------------ */

#[derive(PartialEq, Eq, Clone)]
pub struct Captured {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl std::fmt::Debug for Captured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: {:?}, stderr: {:?} }}",
            String::from_utf8_lossy(&self.out),
            String::from_utf8_lossy(&self.err)
        )
    }
}

static CAPTURE_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Run `f` with fds 1 and 2 redirected into temporary files and return its
/// result together with everything that was written to them.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Captured) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let n = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir();
    let mk = |tag: &str| {
        let p = dir.join(format!("md_cap_{}_{}_{}.txt", std::process::id(), n, tag));
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&p)
            .expect("temp capture file");
        (p, f)
    };
    let (p_out, mut f_out) = mk("out");
    let (p_err, mut f_err) = mk("err");

    let r = unsafe {
        fflush(std::ptr::null_mut()); // flush anything already pending
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        dup2(f_out.as_raw_fd(), 1);
        dup2(f_err.as_raw_fd(), 2);

        let r = f();

        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
        r
    };

    let mut out = Vec::new();
    let mut err = Vec::new();
    f_out.seek(SeekFrom::Start(0)).unwrap();
    f_out.read_to_end(&mut out).unwrap();
    f_err.seek(SeekFrom::Start(0)).unwrap();
    f_err.read_to_end(&mut err).unwrap();
    drop(f_out);
    drop(f_err);
    let _ = std::fs::remove_file(p_out);
    let _ = std::fs::remove_file(p_err);

    (r, Captured { out, err })
}

/// Run `f` in a forked child with its output discarded and return the raw
/// `waitpid` status, so a deliberate crash can be compared between the two
/// libraries instead of taking the whole test process down.
pub fn status_in_child<F: FnOnce()>(f: F) -> c_int {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let devnull = std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .expect("open /dev/null");
            {
                use std::os::unix::io::AsRawFd;
                dup2(devnull.as_raw_fd(), 1);
                dup2(devnull.as_raw_fd(), 2);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = -1;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    }
}

/// Like [`status_in_child`], but the child keeps whatever fds the closure sets
/// up itself and its `int` result is reported through the exit code, so a
/// function's return value *and* its survival can be compared at once.
pub fn exit_code_in_child<F: FnOnce() -> c_int>(f: F) -> c_int {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let rc = f();
            _exit(rc & 0x7f);
        }
        let mut status: c_int = -1;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    }
}

/// Close a file descriptor (used to make `stdout`/`stderr` unwritable).
pub fn close_fd(fd: c_int) {
    unsafe {
        fflush(std::ptr::null_mut());
        close(fd);
    }
}

/// Decode a `waitpid` status into a comparable, printable form.
pub fn decode_status(status: c_int) -> String {
    let low = status & 0x7f;
    if low == 0x7f {
        format!("stopped({})", (status >> 8) & 0xff)
    } else if low == 0 {
        format!("exit({})", (status >> 8) & 0xff)
    } else {
        format!("signal({low})")
    }
}

/* ------------------------------------------------------------------ */
/* argv helpers                                                        */
/* ------------------------------------------------------------------ */

/// Owns the `CString`s backing a C `argv` array.
pub struct Argv {
    _owned: Vec<Option<CString>>,
    ptrs: Vec<*mut c_char>,
}

impl Argv {
    /// Build an `argv` from strings; `None` becomes a NULL entry.
    pub fn new(items: &[Option<&str>]) -> Argv {
        let owned: Vec<Option<CString>> = items
            .iter()
            .map(|s| s.map(|s| CString::new(s).unwrap()))
            .collect();
        let ptrs: Vec<*mut c_char> = owned
            .iter()
            .map(|o| match o {
                Some(c) => c.as_ptr() as *mut c_char,
                None => std::ptr::null_mut(),
            })
            .collect();
        Argv { _owned: owned, ptrs }
    }
    pub fn strs(items: &[&str]) -> Argv {
        let v: Vec<Option<&str>> = items.iter().map(|s| Some(*s)).collect();
        Argv::new(&v)
    }
    pub fn as_ptr(&mut self) -> *mut *mut c_char {
        self.ptrs.as_mut_ptr()
    }
}

/* ------------------------------------------------------------------ */
/* deterministic RNG (xorshift64*)                                     */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

pub const SEED: u64 = 0x0020_2409_17C0_FFEE;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
    /// Small values (including negatives) — hits the non-wrapping code paths.
    pub fn small_i32(&mut self) -> c_int {
        (self.next_i32() % 2001) - 1000
    }
    pub fn in_range(&mut self, lo: c_int, hi: c_int) -> c_int {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
}

/// Boundary operand pairs every binary entry point is probed with.
pub const BOUNDARY_PAIRS: &[(c_int, c_int)] = &[
    (0, 0),
    (0, 1),
    (1, 0),
    (0, -1),
    (-1, 0),
    (1, 1),
    (1, -1),
    (-1, -1),
    (2, 3),
    (-2, 3),
    (2, -3),
    (i32::MAX, 0),
    (i32::MAX, 1),
    (i32::MAX, -1),
    (i32::MAX, i32::MAX),
    (i32::MIN, 0),
    (i32::MIN, 1),
    (i32::MIN, -1),
    (i32::MIN, i32::MIN),
    (i32::MIN, i32::MAX),
    (i32::MAX, i32::MIN),
    (65536, 65536),
    (-65536, 65537),
    (i32::MAX / 2, 2),
    (i32::MIN / 2, 2),
];

/* ------------------------------------------------------------------ */
/* differential assertions                                             */
/* ------------------------------------------------------------------ */

/// Call a `(int,int) -> int` export on both libraries and require identical
/// return value *and* identical captured stdout/stderr bytes.
pub fn diff_bin(pair: &Pair, name: &str, pick: impl Fn(&Driver) -> Symbol<'_, Bin>, a: c_int, b: c_int) {
    let (rc, cap) = capture(|| unsafe { (pick(&pair.c))(a, b) });
    let (rr, rap) = capture(|| unsafe { (pick(&pair.rs))(a, b) });
    assert_eq!(
        rc, rr,
        "{name}({a}, {b}) return mismatch [OP={} REPEAT={}]: C={rc} Rust={rr}",
        pair.op, pair.repeat
    );
    assert_eq!(
        cap, rap,
        "{name}({a}, {b}) output mismatch [OP={} REPEAT={}]",
        pair.op, pair.repeat
    );
}

/// Same, for the `(int) -> int` export.
pub fn diff_un(pair: &Pair, name: &str, pick: impl Fn(&Driver) -> Symbol<'_, Un>, n: c_int) {
    let (rc, cap) = capture(|| unsafe { (pick(&pair.c))(n) });
    let (rr, rap) = capture(|| unsafe { (pick(&pair.rs))(n) });
    assert_eq!(
        rc, rr,
        "{name}({n}) return mismatch [OP={} REPEAT={}]: C={rc} Rust={rr}",
        pair.op, pair.repeat
    );
    assert_eq!(
        cap, rap,
        "{name}({n}) output mismatch [OP={} REPEAT={}]",
        pair.op, pair.repeat
    );
}

/// Call `main` on both libraries with the same `argv` contents.
pub fn diff_main(pair: &Pair, argc: c_int, items: &[Option<&str>]) {
    let mut av_c = Argv::new(items);
    let mut av_r = Argv::new(items);
    let (rc, cap) = capture(|| unsafe { (pair.c.main_fn())(argc, av_c.as_ptr()) });
    let (rr, rap) = capture(|| unsafe { (pair.rs.main_fn())(argc, av_r.as_ptr()) });
    assert_eq!(
        rc, rr,
        "main({argc}, {items:?}) return mismatch [OP={} REPEAT={}]: C={rc} Rust={rr}",
        pair.op, pair.repeat
    );
    assert_eq!(
        cap, rap,
        "main({argc}, {items:?}) output mismatch [OP={} REPEAT={}]",
        pair.op, pair.repeat
    );
}

pub fn diff_main_strs(pair: &Pair, args: &[&str]) {
    let items: Vec<Option<&str>> = args.iter().map(|s| Some(*s)).collect();
    diff_main(pair, args.len() as c_int, &items);
}
