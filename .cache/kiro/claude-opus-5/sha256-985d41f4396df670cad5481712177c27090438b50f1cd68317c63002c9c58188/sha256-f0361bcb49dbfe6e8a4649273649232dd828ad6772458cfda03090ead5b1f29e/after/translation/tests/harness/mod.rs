//! Differential-test harness.
//!
//! Both libraries are loaded as `.so` files through `libloading`; nothing is ever
//! called directly from the Rust crate under test. Every call is made in a
//! `fork()`ed child that operates on a `MAP_SHARED` anonymous mapping, so that
//!
//!   * a failing `assert()` in the C (`SIGABRT`) or a NULL dereference
//!     (`SIGSEGV`) is *observed* rather than killing the test runner, and
//!   * the C and the Rust see byte-identical *pointer values* (the mapping is
//!     inherited by both children), which matters because `cp_inflate` derives
//!     `first_bytes` from the low bits of the `in` pointer.

#![allow(dead_code)]

pub mod deflate;

use core::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Shared mapping layout
// ---------------------------------------------------------------------------

pub const REGION_SIZE: usize = 1 << 21; // 2 MiB

const OFF_DONE: usize = 0;
const OFF_RET: usize = 8;
const OFF_ERRLEN: usize = 16;
const OFF_ERR: usize = 64; // 960 bytes available

/// Start of the "input" scratch area (page aligned).
pub const OFF_IN: usize = 0x1000;
pub const IN_CAP: usize = 0x10000; // 64 KiB

/// Start of the "output" scratch area (page aligned).
pub const OFF_OUT: usize = 0x20000;
pub const OUT_CAP: usize = 0x40000; // 256 KiB

/// Default per-call budget for the forked child, in microseconds. Applied to both
/// libraries so "timed out" (SIGALRM) is itself a comparable outcome. Every test
/// case decodes at most a few hundred KiB, so a legitimate call finishes in
/// microseconds; only the C's self-corrupting `cp_dynamic` loop needs this.
pub const CHILD_TIMEOUT_USEC: i64 = 200_000;

static TIMEOUT_USEC: std::sync::atomic::AtomicI64 =
    std::sync::atomic::AtomicI64::new(CHILD_TIMEOUT_USEC);

pub fn timeout_get() -> i64 {
    TIMEOUT_USEC.load(std::sync::atomic::Ordering::Relaxed)
}
pub fn timeout_set(usec: i64) {
    TIMEOUT_USEC.store(usec, std::sync::atomic::Ordering::Relaxed);
}

pub struct Shared {
    base: *mut u8,
}
impl Shared {
    pub fn new() -> Shared {
        unsafe {
            let p = libc::mmap(
                ptr::null_mut(),
                REGION_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap failed");
            Shared { base: p as *mut u8 }
        }
    }

    #[inline]
    pub fn at(&self, off: usize) -> *mut u8 {
        assert!(off < REGION_SIZE);
        unsafe { self.base.add(off) }
    }

    #[inline]
    pub fn in_ptr(&self, extra: usize) -> *mut u8 {
        self.at(OFF_IN + extra)
    }

    #[inline]
    pub fn out_ptr(&self, extra: usize) -> *mut u8 {
        self.at(OFF_OUT + extra)
    }

    /// Zero the whole scratch area and the metadata block.
    pub fn clear(&self) {
        unsafe {
            ptr::write_bytes(self.base.add(OFF_IN), 0, IN_CAP);
            ptr::write_bytes(self.base.add(OFF_OUT), 0, OUT_CAP);
            ptr::write_bytes(self.base, 0, 1024);
        }
    }

    /// Fill the scratch area with a byte pattern (so that "untouched" bytes are
    /// distinguishable from written zeroes).
    pub fn fill(&self, byte: u8) {
        unsafe {
            ptr::write_bytes(self.base.add(OFF_IN), byte, IN_CAP);
            ptr::write_bytes(self.base.add(OFF_OUT), byte, OUT_CAP);
        }
    }

    /// Fill the scratch area with a position-dependent pattern, so that stray
    /// writes outside the intended range are visible in the snapshot.
    pub fn fill_pattern(&self) {
        unsafe {
            for off in [OFF_IN, OFF_OUT] {
                let cap = if off == OFF_IN { IN_CAP } else { OUT_CAP };
                let base = self.base.add(off);
                for i in 0..cap {
                    // cheap, deterministic, non-uniform
                    *base.add(i) = (i as u8) ^ ((i >> 8) as u8) ^ 0x5A;
                }
            }
        }
    }

    pub fn write(&self, off: usize, bytes: &[u8]) {
        assert!(off + bytes.len() <= REGION_SIZE);
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), self.base.add(off), bytes.len()) }
    }

    pub fn read(&self, off: usize, len: usize) -> Vec<u8> {
        assert!(off + len <= REGION_SIZE);
        let mut v = vec![0u8; len];
        unsafe { ptr::copy_nonoverlapping(self.base.add(off), v.as_mut_ptr(), len) }
        v
    }
}

// ---------------------------------------------------------------------------
// Loaded library
// ---------------------------------------------------------------------------

pub type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;
pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub unfilter: UnfilterFn,
    pub cp_inflate: InflateFn,
    pub cp_error_reason: *mut *const c_char,
    pub cp_fixed_table: *mut u8,
    pub cp_permutation_order: *mut u8,
    pub cp_len_extra_bits: *mut u8,
    pub cp_len_base: *mut u32,
    pub cp_dist_extra_bits: *mut u8,
    pub cp_dist_base: *mut u32,
}

impl Lib {
    unsafe fn open(name: &'static str, path: &str) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {path}: {e}"));
        macro_rules! sym {
            ($t:ty, $n:expr) => {{
                let s: libloading::Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {}", path, $n, e));
                *s
            }};
        }
        macro_rules! var {
            ($t:ty, $n:expr) => {{
                let s: libloading::Symbol<*mut $t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} missing symbol {:?}: {}", path, $n, e));
                s.into_raw().into_raw() as *mut $t
            }};
        }
        let unfilter = sym!(UnfilterFn, b"unfilter\0");
        let cp_inflate = sym!(InflateFn, b"cp_inflate\0");
        let cp_error_reason = var!(*const c_char, b"cp_error_reason\0");
        let cp_fixed_table = var!(u8, b"cp_fixed_table\0");
        let cp_permutation_order = var!(u8, b"cp_permutation_order\0");
        let cp_len_extra_bits = var!(u8, b"cp_len_extra_bits\0");
        let cp_len_base = var!(u32, b"cp_len_base\0");
        let cp_dist_extra_bits = var!(u8, b"cp_dist_extra_bits\0");
        let cp_dist_base = var!(u32, b"cp_dist_base\0");
        Lib {
            name,
            _lib: lib,
            unfilter,
            cp_inflate,
            cp_error_reason,
            cp_fixed_table,
            cp_permutation_order,
            cp_len_extra_bits,
            cp_len_base,
            cp_dist_extra_bits,
            cp_dist_base,
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
    pub shared: Shared,
}

fn repo_root() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

pub fn c_so_path() -> std::path::PathBuf {
    let root = repo_root();
    let dir = root.join("c_src/build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(e.path());
            }
        }
    }
    found.unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            dir.display()
        )
    })
}

/// Path to the Rust `.so`. Both `target/release` and `target/debug` may exist, so
/// pick the most recently built one — otherwise a stale artifact from the other
/// profile could silently be the thing under test. Override with
/// `RUST_SO=/path/to/lib.so`.
pub fn rust_so_path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("RUST_SO") {
        return std::path::PathBuf::from(p);
    }
    let root = repo_root();
    let mut best: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for prof in ["release", "debug"] {
        let p = root.join(format!("translation/target/{prof}/libunfilter_lib.so"));
        if let Ok(md) = std::fs::metadata(&p) {
            if let Ok(m) = md.modified() {
                if best.as_ref().map(|(bm, _)| m > *bm).unwrap_or(true) {
                    best = Some((m, p));
                }
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!("no Rust .so found; run `cargo build --release` in translation/")
    })
}

static ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

pub fn pair() -> Pair {
    ONCE.get_or_init(|| {});
    unsafe {
        let c = Lib::open("C", c_so_path().to_str().unwrap());
        let rust = Lib::open("Rust", rust_so_path().to_str().unwrap());
        Pair {
            c,
            rust,
            shared: Shared::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Isolated execution
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Outcome {
    /// false when the child died before reporting (abort / segv)
    pub completed: bool,
    /// terminating signal, or 0 when the child exited normally
    pub signal: i32,
    pub exit_code: i32,
    pub ret: i32,
    /// contents of `cp_error_reason` (None == NULL pointer)
    pub err: Option<Vec<u8>>,
    /// snapshot of the region range requested by the caller
    pub snap: Vec<u8>,
    /// Whatever the child wrote to stderr. Used *only* to identify which C
    /// `assert()` fired (glibc prints the expression); deliberately EXCLUDED
    /// from equality, because the Rust translation aborts without printing.
    pub stderr: Vec<u8>,
}

impl PartialEq for Outcome {
    fn eq(&self, other: &Self) -> bool {
        self.completed == other.completed
            && self.signal == other.signal
            && self.exit_code == other.exit_code
            && self.ret == other.ret
            && self.err == other.err
            && self.snap == other.snap
    }
}
impl Eq for Outcome {}

impl Outcome {
    /// The assertion expression glibc reported, if the child died on an
    /// `assert()`. Only ever non-empty for the C library.
    pub fn assert_expr(&self) -> Option<String> {
        let s = String::from_utf8_lossy(&self.stderr);
        let start = s.find("Assertion `")? + "Assertion `".len();
        let rest = &s[start..];
        let end = rest.find('\'')?;
        Some(rest[..end].to_string())
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.completed {
            write!(
                f,
                "Outcome{{ died: signal={} ({}), exit_code={}",
                self.signal,
                signame(self.signal),
                self.exit_code
            )?;
            if let Some(a) = self.assert_expr() {
                write!(f, ", assert=`{a}`")?;
            }
            return write!(f, " }}");
        }
        write!(
            f,
            "Outcome{{ ret={}, err={:?}, snap_len={}, snap_sha={:016x} }}",
            self.ret,
            self.err.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
            self.snap.len(),
            fnv(&self.snap)
        )
    }
}

pub fn signame(s: i32) -> &'static str {
    match s {
        0 => "none",
        libc::SIGABRT => "SIGABRT",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGBUS => "SIGBUS",
        libc::SIGILL => "SIGILL",
        libc::SIGFPE => "SIGFPE",
        libc::SIGTRAP => "SIGTRAP",
        libc::SIGKILL => "SIGKILL",
        _ => "other",
    }
}

pub fn fnv(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Run `f` against `lib` in a forked child, then snapshot `snap` = (offset, len)
/// out of the shared region.
pub fn run<F>(sh: &Shared, lib: &Lib, snap: (usize, usize), f: F) -> Outcome
where
    F: FnOnce(&Lib) -> c_int,
{
    unsafe {
        // metadata reset (the caller owns scratch-area setup)
        ptr::write_bytes(sh.base, 0, 1024);
        *(sh.at(OFF_ERRLEN) as *mut i32) = -1;

        // Capture the child's stderr so a glibc assert message can be read back.
        // The messages are ~130 bytes, far below the pipe buffer, so the parent
        // can safely read after waitpid.
        let mut fds = [0 as c_int; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe failed");
        let (rd, wr) = (fds[0], fds[1]);

        let budget = timeout_get();
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            libc::close(rd);
            libc::dup2(wr, 2);
            libc::close(wr);
            // No core dumps: the C aborts/segfaults thousands of times across the
            // suite and a core-dump handler would dominate the runtime.
            let no_core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            // `RLIMIT_CORE` alone is not enough: this host's
            // /proc/sys/kernel/core_pattern pipes to systemd-coredump, which the
            // kernel invokes regardless, costing ~0.5 s per crashing child.
            // Marking the process non-dumpable suppresses the dump while still
            // reporting the terminating signal through waitpid().
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
            // Watchdog: some malformed inputs make both libraries loop for a very
            // long time (the C's `cp_dynamic` can overwrite its own loop bounds).
            // Both get the same sub-second budget, so "timed out" is itself a
            // comparable outcome (SIGALRM).
            let it = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                it_value: libc::timeval {
                    tv_sec: 0,
                    tv_usec: budget,
                },
            };
            libc::setitimer(libc::ITIMER_REAL, &it, ptr::null_mut());
            let ret = f(lib);
            let off = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                it_value: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
            };
            libc::setitimer(libc::ITIMER_REAL, &off, ptr::null_mut());
            *(sh.at(OFF_RET) as *mut i32) = ret;
            let er = *lib.cp_error_reason;
            if er.is_null() {
                *(sh.at(OFF_ERRLEN) as *mut i32) = -1;
            } else {
                let mut n = 0usize;
                while n < 900 && *er.add(n) != 0 {
                    n += 1;
                }
                ptr::copy_nonoverlapping(er as *const u8, sh.at(OFF_ERR), n);
                *(sh.at(OFF_ERRLEN) as *mut i32) = n as i32;
            }
            *(sh.at(OFF_DONE) as *mut i32) = 1;
            libc::_exit(0);
        }

        // ---- parent ----
        libc::close(wr);
        let mut stderr_buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = libc::read(rd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n > 0 {
                stderr_buf.extend_from_slice(&chunk[..n as usize]);
                if stderr_buf.len() > 64 * 1024 {
                    break;
                }
            } else if n == 0 {
                break;
            } else if *libc::__errno_location() != libc::EINTR {
                break;
            }
        }
        libc::close(rd);

        let mut status: c_int = 0;
        let mut r = libc::waitpid(pid, &mut status, 0);
        while r < 0 && *libc::__errno_location() == libc::EINTR {
            r = libc::waitpid(pid, &mut status, 0);
        }
        let signal = if libc::WIFSIGNALED(status) {
            libc::WTERMSIG(status)
        } else {
            0
        };
        let exit_code = if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else {
            -1
        };
        let completed = *(sh.at(OFF_DONE) as *const i32) == 1;
        let ret = *(sh.at(OFF_RET) as *const i32);
        let errlen = *(sh.at(OFF_ERRLEN) as *const i32);
        let err = if errlen < 0 {
            None
        } else {
            Some(sh.read(OFF_ERR, errlen as usize))
        };
        Outcome {
            completed,
            signal,
            exit_code,
            ret,
            err,
            snap: sh.read(snap.0, snap.1),
            stderr: stderr_buf,
        }
    }
}

/// Deterministic xorshift PRNG so every property test is reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    #[inline]
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    #[inline]
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// uniform in `0..n`
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    #[inline]
    pub fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        lo + (self.next_u64() % ((hi_inclusive - lo + 1) as u64)) as i64
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Panic with a full description when the two outcomes differ.
#[track_caller]
pub fn diff(what: &str, c: &Outcome, r: &Outcome) {
    if c == r {
        return;
    }
    let mut extra = String::new();
    if c.completed && r.completed && c.snap != r.snap {
        let n = c.snap.len().min(r.snap.len());
        let mut shown = 0;
        for i in 0..n {
            if c.snap[i] != r.snap[i] {
                extra.push_str(&format!(
                    "\n  first byte diff at snap[{i}]: C=0x{:02x} Rust=0x{:02x}",
                    c.snap[i], r.snap[i]
                ));
                shown += 1;
                if shown >= 8 {
                    break;
                }
            }
        }
    }
    panic!("DIVERGENCE [{what}]\n  C   : {c:?}\n  Rust: {r:?}{extra}");
}

/// Run the same `unfilter` call against both libraries and compare.
///
/// `data` is placed at `OFF_OUT + pad`; the whole `[OFF_OUT, OFF_OUT + pad +
/// data.len() + tail)` window is snapshotted so that writes before/after the
/// nominal buffer are caught too.
#[track_caller]
pub fn diff_unfilter(
    p: &Pair,
    what: &str,
    w: c_int,
    h: c_int,
    bpp: c_int,
    data: &[u8],
    pad: usize,
    tail: usize,
) -> Vec<Outcome> {
    let sh = &p.shared;
    let raw_off = OFF_OUT + pad;
    let snap = (OFF_OUT, pad + data.len() + tail);
    assert!(snap.1 <= OUT_CAP);
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(raw_off, data);
        outs.push(run(sh, lib, snap, |l| unsafe {
            (l.unfilter)(w, h, bpp, sh.at(raw_off))
        }));
    }
    diff(
        &format!("{what} unfilter(w={w},h={h},bpp={bpp},len={})", data.len()),
        &outs[0],
        &outs[1],
    );
    outs
}

/// Byte length of the buffer `unfilter(w, h, bpp, .)` reads/writes for
/// non-degenerate arguments: `h` rows of `1 + w*bpp` bytes.
pub fn unfilter_buf_len(w: c_int, h: c_int, bpp: c_int) -> usize {
    if h <= 0 {
        return 1;
    }
    let len = (w as i64) * (bpp as i64);
    let per = 1 + len.max(0);
    (per * h as i64) as usize
}

/// Run the same `cp_inflate` call against both libraries and compare.
///
/// The input is copied to `OFF_IN + in_align` (so the low two bits of the `in`
/// pointer — and therefore the C's `first_bytes` path — are under test control),
/// the output buffer starts at `OFF_OUT`.
#[track_caller]
pub fn diff_inflate(
    p: &Pair,
    what: &str,
    input: &[u8],
    in_align: usize,
    in_bytes: c_int,
    out_bytes: c_int,
    snap_len: usize,
) -> Vec<Outcome> {
    let sh = &p.shared;
    let snap = (OFF_OUT, snap_len);
    assert!(snap_len <= OUT_CAP);
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(OFF_IN + in_align, input);
        outs.push(run(sh, lib, snap, |l| unsafe {
            (l.cp_inflate)(
                sh.in_ptr(in_align) as *mut c_void,
                in_bytes,
                sh.out_ptr(0) as *mut c_void,
                out_bytes,
            )
        }));
    }
    diff(
        &format!(
            "{what} cp_inflate(in_align={in_align},in_bytes={in_bytes},out_bytes={out_bytes})"
        ),
        &outs[0],
        &outs[1],
    );
    outs
}

/// Like [`diff_inflate`] but additionally asserts the decoded prefix equals
/// `expect` in **both** libraries (catches the case where C and Rust agree on a
/// wrong answer, and validates the test's own encoder).
#[track_caller]
pub fn diff_inflate_expect(
    p: &Pair,
    what: &str,
    input: &[u8],
    in_align: usize,
    out_bytes: c_int,
    expect: &[u8],
) {
    let sh = &p.shared;
    let snap_len = (out_bytes.max(0) as usize + 64).min(OUT_CAP);
    let snap = (OFF_OUT, snap_len);
    let mut outs = Vec::new();
    for lib in [&p.c, &p.rust] {
        sh.fill_pattern();
        sh.write(OFF_IN + in_align, input);
        outs.push(run(sh, lib, snap, |l| unsafe {
            (l.cp_inflate)(
                sh.in_ptr(in_align) as *mut c_void,
                input.len() as c_int,
                sh.out_ptr(0) as *mut c_void,
                out_bytes,
            )
        }));
    }
    let tag = format!("{what} cp_inflate(in_align={in_align},in_bytes={},out_bytes={out_bytes})", input.len());
    diff(&tag, &outs[0], &outs[1]);
    let o = &outs[0];
    assert!(o.completed, "[{tag}] expected a normal return, got {o:?}");
    assert_eq!(o.ret, 1, "[{tag}] expected success, got {o:?}");
    assert_eq!(
        &o.snap[..expect.len()],
        expect,
        "[{tag}] both libraries agreed but the decoded bytes are wrong"
    );
}

/// True when the two outcomes differ *only* because exactly one child was killed
/// by the watchdog. That can be a timing artifact rather than a real divergence.
pub fn one_sided_timeout(a: &Outcome, b: &Outcome) -> bool {
    let alarmed = |o: &Outcome| !o.completed && o.signal == libc::SIGALRM;
    alarmed(a) != alarmed(b)
}

/// Compare a pair of outcomes; if they differ only by a one-sided watchdog kill,
/// re-run the whole case with a 10-second budget before declaring a divergence.
#[track_caller]
pub fn diff_or_retry<F>(tag: &str, mut go: F)
where
    F: FnMut() -> (Outcome, Outcome),
{
    let (c, r) = go();
    if c == r {
        return;
    }
    if one_sided_timeout(&c, &r) {
        let saved = timeout_get();
        timeout_set(10_000_000);
        let (c2, r2) = go();
        timeout_set(saved);
        diff(&format!("{tag} (re-checked with a 10s budget)"), &c2, &r2);
        return;
    }
    diff(tag, &c, &r);
}
