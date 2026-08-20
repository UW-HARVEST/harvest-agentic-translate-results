//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C ABI symbols.  The Rust implementation is
//! *never* called directly as a Rust function -- this is deliberate, so that the
//! `#[no_mangle] extern "C"` wrappers are part of what gets tested.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol, RTLD_LOCAL, RTLD_NOW};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
/// `char *w_utf8_filter(const char *string, bool replacement)`.
///
/// The `bool` is declared as `u32` on purpose: C's `_Bool` is passed in the low
/// byte of the argument register and gcc emits `cmpb $0x0,<slot>`, so a caller
/// may legally hand over any int-sized value.  Declaring it `u32` lets the tests
/// push *non-normalized* booleans (2, 0xFF, 0x100, ...) across the FFI boundary
/// exactly as a sloppy C caller would.
pub type FilterFn = unsafe extern "C" fn(*const c_char, u32) -> *mut c_char;

pub struct Api {
    pub name: &'static str,
    pub utf8_drop: DropFn,
    pub utf8_filter: FilterFn,
    _lib: Library,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Name of the profile directory the running test binary lives in
/// (`.../target/<profile>/deps/<test>-<hash>` ⇒ `<profile>`).
fn profile_name() -> String {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "debug".to_string())
}

/// The cargo features that are enabled for *this* test build.
///
/// `Cargo.toml` currently declares no features at all (see `CONFIGS.md`), so
/// this is empty; the list is kept explicit so that adding a feature later
/// automatically propagates into the `.so` we build below.
fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}

/// Build (and return the path to) `libdriver.so` for the current profile and
/// feature set.
///
/// This is NOT optional bookkeeping: `cargo test` does **not** rebuild a
/// `crate-type = ["cdylib"]` library, because the integration tests do not
/// depend on it through the crate graph.  Loading
/// `target/<profile>/libdriver.so` directly therefore silently tests whatever
/// stale artefact happens to be lying around -- a differential suite that can
/// never fail.  Building into a *separate* target dir (so there is no lock
/// contention with the outer `cargo test`) guarantees the `.so` under test is
/// exactly the current `src/lib.rs` with exactly the current features.
fn rust_so_path() -> PathBuf {
    let root = crate_root();
    let profile = profile_name();
    let side = root.join("target").join("difftest-so");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = std::process::Command::new(&cargo);
    cmd.current_dir(&root)
        .env("CARGO_TARGET_DIR", &side)
        .env_remove("RUSTFLAGS")
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .arg("--no-default-features");
    let feats = enabled_features();
    if !feats.is_empty() {
        cmd.arg("--features").arg(feats.join(","));
    }
    if profile != "debug" {
        cmd.arg("--profile").arg(if profile == "release" {
            "release".to_string()
        } else {
            profile.clone()
        });
    }

    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} build --lib`: {e}"));
    assert!(
        out.status.success(),
        "`{cargo} build --lib` (into {side:?}) failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let p = side.join(&profile).join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {p:?} after a successful build"
    );
    p
}

fn c_so_path() -> PathBuf {
    let p = crate_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn load(name: &'static str, path: PathBuf) -> Api {
    // RTLD_LOCAL (the default, but made explicit) is important: it keeps the two
    // libraries from interposing each other's `w_utf8_drop`, so `w_utf8_filter`
    // in each library really calls its *own* scanner.
    let lib = unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL) }
        .unwrap_or_else(|e| panic!("dlopen({path:?}) failed: {e}"));
    let d: Symbol<DropFn> = unsafe { lib.get(b"w_utf8_drop\0") }
        .unwrap_or_else(|e| panic!("{name}: missing symbol w_utf8_drop: {e}"));
    let f: Symbol<FilterFn> = unsafe { lib.get(b"w_utf8_filter\0") }
        .unwrap_or_else(|e| panic!("{name}: missing symbol w_utf8_filter: {e}"));
    Api {
        name,
        utf8_drop: *d,
        utf8_filter: *f,
        _lib: lib,
    }
}

pub struct Pair {
    pub c: Api,
    pub rs: Api,
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: load("C", c_so_path()),
        rs: load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// small utilities
// ---------------------------------------------------------------------------

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for (i, x) in b.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{x:02X}"));
    }
    s
}

/// Turn a byte slice into a NUL-terminated buffer.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(bytes.len() + 1);
    v.extend_from_slice(bytes);
    v.push(0);
    v
}

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while unsafe { *q } != 0 {
        out.push(unsafe { *q });
        q = unsafe { q.add(1) };
    }
    out
}

/// Deterministic xorshift64* PRNG (fixed seed per test row ⇒ reproducible).
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
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }
    /// Random byte, never 0 (so it cannot accidentally terminate the string).
    pub fn nonzero_byte(&mut self) -> u8 {
        let b = (self.next_u64() >> 56) as u8;
        if b == 0 {
            1
        } else {
            b
        }
    }
    pub fn any_byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

// ---------------------------------------------------------------------------
// differential comparators
// ---------------------------------------------------------------------------

/// Compare `w_utf8_drop`.  Both libraries get the *same* buffer, so the returned
/// pointers are directly comparable; we report them as offsets.
///
/// `buf` must already be NUL-terminated.
#[track_caller]
pub fn cmp_drop(buf: &[u8]) -> usize {
    assert!(buf.last() == Some(&0), "cmp_drop: buffer must be NUL-terminated");
    let p = pair();
    let base = buf.as_ptr() as *const c_char;
    let rc = unsafe { (p.c.utf8_drop)(base) };
    let rr = unsafe { (p.rs.utf8_drop)(base) };
    let oc = rc as usize - base as usize;
    let or = rr as usize - base as usize;
    assert_eq!(
        oc,
        or,
        "w_utf8_drop offset mismatch: C={oc} Rust={or}\n  input = [{}]",
        hex(buf)
    );
    assert!(
        oc < buf.len(),
        "w_utf8_drop returned an out-of-bounds offset {oc} (len {})\n  input = [{}]",
        buf.len(),
        hex(buf)
    );
    oc
}

pub struct FilterOutcome {
    pub null: bool,
    pub bytes: Vec<u8>,
    /// `malloc_usable_size()` of the C buffer (informational; see
    /// [`cmp_filter_alloc_size`] for the differential allocation-size check).
    pub usable: usize,
}

/// Compare `w_utf8_filter`.  Frees both returned buffers with `libc::free`
/// (they come from `malloc`/`realloc`/`strdup` in both implementations).
///
/// `buf` must already be NUL-terminated.
#[track_caller]
pub fn cmp_filter(buf: &[u8], replacement: u32) -> FilterOutcome {
    assert!(
        buf.last() == Some(&0),
        "cmp_filter: buffer must be NUL-terminated"
    );
    let p = pair();
    let base = buf.as_ptr() as *const c_char;

    // The two calls are made one after the other with the previous buffer
    // already released, so both implementations see an equivalent heap state and
    // `malloc_usable_size` is directly comparable.
    let rc = unsafe { (p.c.utf8_filter)(base, replacement) };
    let c_null = rc.is_null();
    let (cv, cu) = if c_null {
        (Vec::new(), 0)
    } else {
        let v = unsafe { read_cstr(rc) };
        let u = unsafe { libc::malloc_usable_size(rc as *mut libc::c_void) };
        unsafe { libc::free(rc as *mut libc::c_void) };
        (v, u)
    };

    let rr = unsafe { (p.rs.utf8_filter)(base, replacement) };
    let r_null = rr.is_null();
    let (rv, ru) = if r_null {
        (Vec::new(), 0)
    } else {
        let v = unsafe { read_cstr(rr) };
        let u = unsafe { libc::malloc_usable_size(rr as *mut libc::c_void) };
        unsafe { libc::free(rr as *mut libc::c_void) };
        (v, u)
    };

    assert_eq!(
        c_null, r_null,
        "w_utf8_filter NULL-ness mismatch (replacement={replacement:#x}): C_null={c_null} Rust_null={r_null}\n  input = [{}]",
        hex(buf)
    );
    assert_eq!(
        cv.len(),
        rv.len(),
        "w_utf8_filter output length mismatch (replacement={replacement:#x}): C={} Rust={}\n  input   = [{}]\n  C out   = [{}]\n  Rust out= [{}]",
        cv.len(),
        rv.len(),
        hex(buf),
        hex(&cv),
        hex(&rv)
    );
    assert_eq!(
        cv,
        rv,
        "w_utf8_filter output mismatch (replacement={replacement:#x})\n  input   = [{}]\n  C out   = [{}]\n  Rust out= [{}]",
        hex(buf),
        hex(&cv),
        hex(&rv)
    );

    // One-sided sanity check that costs nothing and catches UNDER-allocation
    // (the dangerous half of a wrong `repl` / REPLACEMENT_INC schedule: the
    // output bytes still match, but the implementation wrote past its buffer).
    // `malloc_usable_size` is >= the requested size, so this holds for correct
    // code by construction while a too-small `size` shows up immediately.
    if !c_null {
        assert!(
            cu >= cv.len() + 1,
            "C w_utf8_filter overflowed its own buffer: usable={cu} but wrote {} bytes + NUL",
            cv.len()
        );
        assert!(
            ru >= rv.len() + 1,
            "Rust w_utf8_filter under-allocated / overflowed its buffer \
             (replacement={replacement:#x}): usable={ru} but wrote {} bytes + NUL \
             (C usable={cu})\n  input = [{}]",
            rv.len(),
            hex(buf)
        );
    }

    FilterOutcome {
        null: c_null,
        bytes: cv,
        usable: cu,
    }
}

/// Exact differential measurement of `w_utf8_filter`'s **internal allocation
/// arithmetic** (`size = strlen + 1`, then `+= REPLACEMENT_INC` once per
/// `repl < 3` hit).
///
/// Comparing `malloc_usable_size` in-process is useless: the two calls run
/// against different heap states, so glibc legitimately hands back chunks of
/// different sizes.  Instead each implementation is run in its **own forked
/// child**, both forked from the same parent at the same moment.  The children
/// therefore start from a bit-identical heap, which makes glibc's chunk choice a
/// pure function of the request sequence -- so any difference in the returned
/// chunk size is a real difference in the request sequence.
///
/// Returns `(c_usable, rust_usable)`.
#[track_caller]
pub fn cmp_filter_alloc_size(buf: &[u8], replacement: u32) -> (usize, usize) {
    assert!(buf.last() == Some(&0));
    let p = pair();
    let base = buf.as_ptr() as *const c_char;

    // Both pipes are created and both children are forked back-to-back, BEFORE
    // either result is collected.  This matters: glibc's choice of chunk (and
    // therefore `malloc_usable_size`) depends on the heap layout the child
    // inherits, so the two children must branch off a bit-identical parent
    // state.  Forking, waiting, then forking again lets the parent's heap move
    // in between and produces spurious 16-byte differences.
    let mut fds1 = [0i32; 2];
    let mut fds2 = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds1.as_mut_ptr()) }, 0, "pipe failed");
    assert_eq!(unsafe { libc::pipe(fds2.as_mut_ptr()) }, 0, "pipe failed");

    /// `u64::MAX` encodes "the call returned NULL".
    unsafe fn child_body(f: FilterFn, base: *const c_char, replacement: u32, wr: i32) -> ! {
        let r = unsafe { f(base, replacement) };
        let v: u64 = if r.is_null() {
            u64::MAX
        } else {
            unsafe { libc::malloc_usable_size(r as *mut libc::c_void) as u64 }
        };
        let bytes = v.to_le_bytes();
        unsafe { libc::write(wr, bytes.as_ptr() as *const libc::c_void, 8) };
        unsafe { libc::_exit(0) }
    }

    let pid1 = unsafe { libc::fork() };
    assert!(pid1 >= 0, "fork failed");
    if pid1 == 0 {
        unsafe { child_body(p.c.utf8_filter, base, replacement, fds1[1]) };
    }
    let pid2 = unsafe { libc::fork() };
    assert!(pid2 >= 0, "fork failed");
    if pid2 == 0 {
        unsafe { child_body(p.rs.utf8_filter, base, replacement, fds2[1]) };
    }

    unsafe {
        libc::close(fds1[1]);
        libc::close(fds2[1]);
    }

    let read_one = |rd: i32, pid: i32| -> u64 {
        let mut b = [0u8; 8];
        let n = unsafe { libc::read(rd, b.as_mut_ptr() as *mut libc::c_void, 8) };
        unsafe { libc::close(rd) };
        let mut status = 0i32;
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid, "waitpid");
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "measuring child died (status {status:#x})"
        );
        assert_eq!(n, 8, "short read from measuring child");
        u64::from_le_bytes(b)
    };

    let cu = read_one(fds1[0], pid1);
    let ru = read_one(fds2[0], pid2);

    assert_eq!(
        cu, ru,
        "w_utf8_filter internal allocation size differs (replacement={replacement:#x}): \
         C={cu} Rust={ru} (u64::MAX = returned NULL).  The `size` / `repl` / \
         REPLACEMENT_INC book-keeping diverges even though the output bytes match.\n  \
         input length = {}",
        buf.len() - 1
    );
    (cu as usize, ru as usize)
}

/// Run both entry points on the same input, for both `replacement` values.
#[track_caller]
pub fn cmp_all(buf: &[u8]) {
    cmp_drop(buf);
    cmp_filter(buf, 0);
    cmp_filter(buf, 1);
}

// ---------------------------------------------------------------------------
// UTF-8 generators (well-formed with respect to *this* library's rules)
// ---------------------------------------------------------------------------

/// Push a well-formed 1-byte sequence (0x01..0x7F -- 0x00 excluded, it would
/// terminate the string).
pub fn push_valid1(out: &mut Vec<u8>, r: &mut Rng) {
    out.push(1 + (r.below(0x7F) as u8));
}

/// Push a well-formed 2-byte sequence: lead 0xC2..0xDF, cont 0x80..0xBF.
pub fn push_valid2(out: &mut Vec<u8>, r: &mut Rng) {
    out.push(0xC2 + r.below(0xDF - 0xC2 + 1) as u8);
    out.push(0x80 + r.below(0x40) as u8);
}

/// Push a well-formed 3-byte sequence honouring the library's own guards
/// (0xE0 ⇒ b1 ≥ 0xA0, 0xED ⇒ b1 < 0xA0).
pub fn push_valid3(out: &mut Vec<u8>, r: &mut Rng) {
    let lead = 0xE0 + r.below(0x10) as u8;
    let b1 = match lead {
        0xE0 => 0xA0 + r.below(0x20) as u8,
        0xED => 0x80 + r.below(0x20) as u8,
        _ => 0x80 + r.below(0x40) as u8,
    };
    out.push(lead);
    out.push(b1);
    out.push(0x80 + r.below(0x40) as u8);
}

/// Push a well-formed 4-byte sequence honouring 0xF0 ⇒ b1 ≥ 0x90 and
/// 0xF4 ⇒ b1 ≤ 0x8F; leads limited to 0xF0..0xF4.
pub fn push_valid4(out: &mut Vec<u8>, r: &mut Rng) {
    let lead = 0xF0 + r.below(5) as u8;
    let b1 = match lead {
        0xF0 => 0x90 + r.below(0x30) as u8,
        0xF4 => 0x80 + r.below(0x10) as u8,
        _ => 0x80 + r.below(0x40) as u8,
    };
    out.push(lead);
    out.push(b1);
    out.push(0x80 + r.below(0x40) as u8);
    out.push(0x80 + r.below(0x40) as u8);
}

pub fn push_valid_any(out: &mut Vec<u8>, r: &mut Rng) {
    match r.below(4) {
        0 => push_valid1(out, r),
        1 => push_valid2(out, r),
        2 => push_valid3(out, r),
        _ => push_valid4(out, r),
    }
}

/// A byte that is *never* the start of a valid sequence on its own: continuation
/// bytes, the overlong 2-byte leads, and the out-of-range 4-byte leads.
pub const ALWAYS_INVALID_LEADS: &[u8] = &[
    0x80, 0x81, 0x8F, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFB, 0xFC, 0xFE,
    0xFF,
];

pub fn invalid_byte(r: &mut Rng) -> u8 {
    ALWAYS_INVALID_LEADS[r.below(ALWAYS_INVALID_LEADS.len() as u32) as usize]
}

// ---------------------------------------------------------------------------
// guard-page buffer (over-read detection)
// ---------------------------------------------------------------------------

/// Two mapped pages where the *second* one is `PROT_NONE`.
///
/// Inputs are placed so that their terminating NUL is the very last readable
/// byte before the guard page.  Both `valid_2/3/4` rely on C's `&&`
/// short-circuiting to avoid reading the continuation bytes of a sequence that
/// is truncated by the terminator; if either implementation reads even one byte
/// past the NUL, the process takes SIGSEGV instead of silently returning a
/// plausible answer.
pub struct GuardedBuf {
    base: *mut u8,
    page: usize,
}

impl GuardedBuf {
    pub fn new() -> Self {
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page * 2,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        let base = base as *mut u8;
        assert_eq!(
            unsafe { libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE) },
            0,
            "mprotect failed"
        );
        GuardedBuf { page, base }
    }

    /// Copy `bytes` followed by a NUL so that the NUL sits at the last readable
    /// address.  The returned slice includes the NUL.
    pub fn place(&self, bytes: &[u8]) -> &[u8] {
        let len = bytes.len() + 1;
        assert!(len <= self.page, "input too long for one page");
        let start = unsafe { self.base.add(self.page - len) };
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len());
            *start.add(bytes.len()) = 0;
            std::slice::from_raw_parts(start, len)
        }
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.base as *mut libc::c_void, self.page * 2) };
    }
}

// ---------------------------------------------------------------------------
// child-process helpers (for aborts and allocation failures)
// ---------------------------------------------------------------------------

/// How a forked child ended.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Exit {
    Code(i32),
    Signal(i32),
}

/// Fork and run `f` in the child; the child's exit status is returned.
/// `f` must call `libc::_exit` itself (or fall through, in which case the child
/// exits with code 0).  The child's stderr is redirected to /dev/null so that
/// `assert()` chatter does not pollute the test log.
pub fn in_child<F: FnOnce()>(silence_stderr: bool, f: F) -> Exit {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if silence_stderr {
                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
                if devnull >= 0 {
                    libc::dup2(devnull, 2);
                }
            }
            f();
            libc::_exit(0);
        }
        let mut status: i32 = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Exit::Signal(libc::WTERMSIG(status))
        } else {
            Exit::Code(libc::WEXITSTATUS(status))
        }
    }
}

/// Like [`in_child`], but also captures everything the child writes to stderr.
pub fn in_child_capture_stderr<F: FnOnce()>(f: F) -> (Exit, Vec<u8>) {
    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);
    let st = in_child(false, move || {
        unsafe {
            libc::close(rd);
            libc::dup2(wr, 2);
            libc::close(wr);
        }
        f();
    });
    unsafe { libc::close(wr) };
    let mut out = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(rd, chunk.as_mut_ptr() as *mut libc::c_void, chunk.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n as usize]);
    }
    unsafe { libc::close(rd) };
    (st, out)
}

/// Current virtual-memory size of this process, in bytes (from /proc/self/statm).
pub fn vm_size_bytes() -> usize {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: usize = s
        .split_whitespace()
        .next()
        .expect("statm field 0")
        .parse()
        .expect("statm parse");
    pages * 4096
}

/// Same as [`vm_size_bytes`] but **allocation-free**, so it is safe to call
/// inside a forked child right before clamping `RLIMIT_AS`.  Reading the value
/// in the child (rather than in the parent) is essential: the parent's VmSize
/// fluctuates as other tests run, which would make the head-room meaningless.
pub unsafe fn raw_vm_size_bytes() -> usize {
    let mut buf = [0u8; 128];
    let fd = unsafe { libc::open(b"/proc/self/statm\0".as_ptr() as *const c_char, libc::O_RDONLY) };
    if fd < 0 {
        return 0;
    }
    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    unsafe { libc::close(fd) };
    if n <= 0 {
        return 0;
    }
    let mut pages: usize = 0;
    for &b in buf[..n as usize].iter() {
        if b.is_ascii_digit() {
            pages = pages * 10 + (b - b'0') as usize;
        } else {
            break;
        }
    }
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    pages * if page == 0 { 4096 } else { page }
}

/// Exit code used by the child when `setrlimit` itself fails.
pub const EXIT_SETRLIMIT_FAILED: i32 = 66;

/// Make **every** subsequent `malloc`/`realloc` in this (child) process fail.
///
/// Clamping `RLIMIT_AS` alone is not enough: a forked child inherits the
/// parent's heap, which already contains plenty of free space, so a fresh
/// `malloc` can be served without touching the address-space limit at all.
/// So we clamp the limit *and then* drain every last usable byte out of the
/// allocator with a descending ladder of request sizes (deliberately leaking
/// everything).  After this returns, `malloc(n)` fails for any `n >= 16`.
pub unsafe fn exhaust_allocator(headroom: usize) {
    let limit = unsafe { raw_vm_size_bytes() } + headroom;
    let rl = libc::rlimit {
        rlim_cur: limit as libc::rlim_t,
        rlim_max: limit as libc::rlim_t,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &rl) } != 0 {
        unsafe { libc::_exit(EXIT_SETRLIMIT_FAILED) };
    }
    const LADDER: [usize; 10] = [
        4 << 20,
        1 << 20,
        256 << 10,
        64 << 10,
        16 << 10,
        4 << 10,
        1 << 10,
        256,
        64,
        16,
    ];
    for &sz in LADDER.iter() {
        loop {
            let p = unsafe { libc::malloc(sz) };
            if p.is_null() {
                break;
            }
            // leaked on purpose: the point is to hold on to the address space
            std::hint::black_box(p);
        }
    }
}
