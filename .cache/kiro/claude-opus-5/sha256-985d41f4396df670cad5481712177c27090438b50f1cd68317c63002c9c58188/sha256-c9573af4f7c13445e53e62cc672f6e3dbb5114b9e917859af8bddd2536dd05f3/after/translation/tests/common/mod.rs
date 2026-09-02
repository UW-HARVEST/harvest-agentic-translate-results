//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; the Rust
//! side is *never* called directly, so the `#[no_mangle]` export wrappers are
//! part of what is under test.

#![allow(dead_code)]

pub mod deflate;

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `cp_error_reason` is a process-global inside each `.so`, so concurrent test
/// threads would clobber each other's reading of it. Every differential call
/// takes this lock.
static CALL_LOCK: Mutex<()> = Mutex::new(());

pub fn call_lock() -> MutexGuard<'static, ()> {
    match CALL_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
pub type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to load {} ({}): {e}", name, path.display()));
        Lib { name, lib }
    }

    pub fn inflate(&self) -> Symbol<'_, InflateFn> {
        unsafe { self.lib.get(b"cp_inflate\0") }.expect("cp_inflate missing")
    }

    pub fn unfilter(&self) -> Symbol<'_, UnfilterFn> {
        unsafe { self.lib.get(b"unfilter\0") }.expect("unfilter missing")
    }

    /// Reads the `cp_error_reason` global out of this `.so` and copies it.
    pub fn error_reason(&self) -> Option<Vec<u8>> {
        unsafe {
            let s: Symbol<*mut *const c_char> =
                self.lib.get(b"cp_error_reason\0").expect("cp_error_reason missing");
            let p = **s;
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_bytes().to_vec())
            }
        }
    }

    pub fn set_error_reason_null(&self) {
        unsafe {
            let s: Symbol<*mut *const c_char> =
                self.lib.get(b"cp_error_reason\0").expect("cp_error_reason missing");
            **s = std::ptr::null();
        }
    }

    /// Raw bytes of an exported table.
    pub fn table(&self, sym: &[u8], len: usize) -> Vec<u8> {
        let mut name = sym.to_vec();
        name.push(0);
        unsafe {
            let s: Symbol<*const u8> = self.lib.get(&name).expect("table symbol missing");
            std::slice::from_raw_parts(*s, len).to_vec()
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so(dir: &str) -> PathBuf {
    let d = root().join("..").join("c_src").join(dir);
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&d)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", d.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name().unwrap().to_string_lossy().starts_with("lib")
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", d.display()))
}

pub struct Libs {
    /// C `.so` built exactly as documented (asserts LIVE).
    pub c: Lib,
    /// C `.so` built with `-DNDEBUG` (asserts elided) — matches Rust semantics.
    pub c_nd: Lib,
    /// The Rust `cdylib`.
    pub r: Lib,
    /// Instrumented copy of the C used only as an undefined-behaviour oracle
    /// (see `tools/build_lens_probe.sh`). `None` if it has not been built.
    pub probe: Option<Lib>,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| Libs {
        c: Lib::open("c", find_c_so("build")),
        c_nd: Lib::open("c_ndebug", find_c_so("build_ndebug")),
        r: Lib::open(
            "rust",
            root().join("target").join("release").join("libunfilter_lib.so"),
        ),
        probe: {
            let p = root().join("target").join("probe").join("liblens_probe.so");
            if p.exists() {
                Some(Lib::open("lens_probe", p))
            } else {
                None
            }
        },
    })
}

// ---------------------------------------------------------------------------
// PRNG (xorshift64*, fixed seed => reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        assert!(lo <= hi_inclusive, "Rng::range({lo}, {hi_inclusive}) is empty");
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

// ---------------------------------------------------------------------------
// Alignment-controlled buffers
//
// `cp_inflate` branches on `((size_t)in) & 3`, so a differential test is only
// meaningful when both libraries get an input pointer with the SAME alignment.
// ---------------------------------------------------------------------------

pub struct AlignedBuf {
    backing: Vec<u64>,
    offset: usize,
    len: usize,
}

impl AlignedBuf {
    /// Buffer whose `as_mut_ptr()` satisfies `ptr % 4 == align`, holding `data`
    /// followed by `pad` bytes of slack (`cp_stored`'s `memcpy` and the `words`
    /// reads can touch a little past `in_bytes`).
    pub fn new(data: &[u8], align: usize, pad: usize) -> AlignedBuf {
        assert!(align < 4);
        let total = align + data.len() + pad + 16;
        let mut backing = vec![0u64; total / 8 + 2];
        let base = backing.as_mut_ptr() as usize;
        assert_eq!(base % 8, 0, "Vec<u64> must be 8-aligned");
        let offset = align;
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                (backing.as_mut_ptr() as *mut u8).add(offset),
                data.len(),
            );
        }
        AlignedBuf { backing, offset, len: data.len() }
    }

    pub fn zeroed(len: usize, align: usize, pad: usize) -> AlignedBuf {
        AlignedBuf::new(&vec![0u8; len], align, pad)
    }

    pub fn ptr(&mut self) -> *mut u8 {
        unsafe { (self.backing.as_mut_ptr() as *mut u8).add(self.offset) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// The logical contents plus the slack, so buffer overruns are observable.
    pub fn full(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self.backing.as_ptr() as *const u8).add(self.offset),
                self.backing.len() * 8 - self.offset,
            )
        }
    }

    pub fn contents(&self) -> &[u8] {
        &self.full()[..self.len]
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct InflateOutcome {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub err: Option<Vec<u8>>,
}

/// Which C build to compare against.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CBuild {
    /// as documented (asserts live) — only safe on inputs that cannot assert
    AsBuilt,
    /// `-DNDEBUG` — matches the Rust translation's documented semantics
    NDebug,
}

/// `cp_stored` copies `LEN` bytes with `memcpy` and `LEN` is a `uint16_t`, so a
/// malformed stream can read up to 65535 bytes past `in_bytes` and write up to
/// 65535 bytes past `out_bytes` -- neither is bounds-checked in the C. Both
/// buffers are padded by more than that so the over-read lands on deterministic
/// zeros in *both* libraries instead of on unrelated heap, and the over-write
/// stays inside memory the harness owns.
const OVERRUN_PAD: usize = 65536 + 256;

fn run_inflate(
    lib: &Lib,
    input: &[u8],
    in_align: usize,
    in_bytes: c_int,
    out_len: usize,
    out_align: usize,
    out_bytes: c_int,
) -> InflateOutcome {
    lib.set_error_reason_null();
    let mut inbuf = AlignedBuf::new(input, in_align, OVERRUN_PAD);
    let mut outbuf = AlignedBuf::zeroed(out_len, out_align, OVERRUN_PAD);
    let f = lib.inflate();
    let ret = unsafe {
        f(
            inbuf.ptr() as *mut c_void,
            in_bytes,
            outbuf.ptr() as *mut c_void,
            out_bytes,
        )
    };
    // Compare the requested output *plus* the overrun window, so a write past
    // out_bytes is caught rather than silently ignored.
    let full = outbuf.full();
    let n = (out_len + OVERRUN_PAD).min(full.len());
    InflateOutcome {
        ret,
        out: full[..n].to_vec(),
        err: lib.error_reason(),
    }
}

pub struct InflateCase<'a> {
    pub input: &'a [u8],
    pub in_align: usize,
    pub in_bytes: Option<c_int>,
    pub out_len: usize,
    pub out_align: usize,
    pub out_bytes: Option<c_int>,
}

impl<'a> InflateCase<'a> {
    pub fn new(input: &'a [u8], out_len: usize) -> Self {
        InflateCase {
            input,
            in_align: 0,
            in_bytes: None,
            out_len,
            out_align: 0,
            out_bytes: None,
        }
    }
    pub fn in_align(mut self, a: usize) -> Self {
        self.in_align = a;
        self
    }
    pub fn out_align(mut self, a: usize) -> Self {
        self.out_align = a;
        self
    }
    pub fn in_bytes(mut self, n: c_int) -> Self {
        self.in_bytes = Some(n);
        self
    }
    pub fn out_bytes(mut self, n: c_int) -> Self {
        self.out_bytes = Some(n);
        self
    }
}

/// Runs `cp_inflate` in both libraries and asserts byte-identical results.
/// Returns the (shared) outcome.
pub fn diff_inflate(case: InflateCase<'_>, build: CBuild, label: &str) -> InflateOutcome {
    let _guard = call_lock();
    let l = libs();
    let cl = match build {
        CBuild::AsBuilt => &l.c,
        CBuild::NDebug => &l.c_nd,
    };
    let in_bytes = case.in_bytes.unwrap_or(case.input.len() as c_int);
    let out_bytes = case.out_bytes.unwrap_or(case.out_len as c_int);

    let cres = run_inflate(
        cl,
        case.input,
        case.in_align,
        in_bytes,
        case.out_len,
        case.out_align,
        out_bytes,
    );
    let rres = run_inflate(
        &l.r,
        case.input,
        case.in_align,
        in_bytes,
        case.out_len,
        case.out_align,
        out_bytes,
    );

    if cres.ret != rres.ret {
        panic!(
            "[{label}] cp_inflate return mismatch: C={} Rust={}\n  in_align={} in_bytes={} out_bytes={}\n  input={:02x?}",
            cres.ret, rres.ret, case.in_align, in_bytes, out_bytes,
            &case.input[..case.input.len().min(96)]
        );
    }
    if cres.err != rres.err {
        panic!(
            "[{label}] cp_error_reason mismatch:\n  C   ={:?}\n  Rust={:?}",
            cres.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            rres.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        );
    }
    if cres.out != rres.out {
        let at = cres
            .out
            .iter()
            .zip(rres.out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        let lo = at.saturating_sub(8);
        let hi = (at + 24).min(cres.out.len());
        panic!(
            "[{label}] output mismatch at byte {at} (len {}):\n  C   ={:02x?}\n  Rust={:02x?}",
            cres.out.len(),
            &cres.out[lo..hi],
            &rres.out[lo..hi]
        );
    }
    cres
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnfilterOutcome {
    pub ret: c_int,
    pub raw: Vec<u8>,
}

/// Runs `unfilter` in both libraries on identical copies of `raw` and asserts
/// the return value *and* the in-place mutation match byte-for-byte.
pub fn diff_unfilter(
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: &[u8],
    build: CBuild,
    label: &str,
) -> UnfilterOutcome {
    let _guard = call_lock();
    let l = libs();
    let cl = match build {
        CBuild::AsBuilt => &l.c,
        CBuild::NDebug => &l.c_nd,
    };

    let mut cbuf = raw.to_vec();
    let mut rbuf = raw.to_vec();
    let cret = unsafe { (cl.unfilter())(w, h, bpp, cbuf.as_mut_ptr()) };
    let rret = unsafe { (l.r.unfilter())(w, h, bpp, rbuf.as_mut_ptr()) };

    if cret != rret {
        panic!("[{label}] unfilter return mismatch: C={cret} Rust={rret} (w={w} h={h} bpp={bpp})");
    }
    if cbuf != rbuf {
        let at = cbuf
            .iter()
            .zip(rbuf.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        let lo = at.saturating_sub(8);
        let hi = (at + 24).min(cbuf.len());
        panic!(
            "[{label}] unfilter buffer mismatch at byte {at} (w={w} h={h} bpp={bpp} len={}):\n  C   ={:02x?}\n  Rust={:02x?}",
            cbuf.len(),
            &cbuf[lo..hi],
            &rbuf[lo..hi]
        );
    }
    UnfilterOutcome { ret: cret, raw: cbuf }
}

// ---------------------------------------------------------------------------
// Crash-tolerant runner
//
// The C `.so` produced by the documented CMake build has live `assert()`s (no
// CMAKE_BUILD_TYPE => no -DNDEBUG), and neither function does any null
// checking. Malformed input therefore makes the C `abort()` or `SIGSEGV`, which
// would take the whole test binary down. Every such case is run in a forked
// child so the outcome (exit code or signal) becomes an observable value that
// can be compared between the two libraries.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn setitimer(which: c_int, new: *const ITimerVal, old: *mut ITimerVal) -> c_int;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
struct ITimerVal {
    it_interval: TimeVal,
    it_value: TimeVal,
}

const ITIMER_REAL: c_int = 0;

/// Wall-clock budget for a child. Every call under test operates on buffers of
/// at most a few KiB and completes in microseconds, so anything still running
/// after this is stuck in the C's `while (!bfinal)` loop reading stale bits --
/// which is itself a behaviour the Rust must reproduce, so `Signaled(SIGALRM)`
/// is a comparable outcome rather than a test failure.
const CHILD_TIMEOUT_USEC_DEFAULT: i64 = 300_000;

fn child_timeout_usec() -> i64 {
    static V: OnceLock<i64> = OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("CHILD_TIMEOUT_USEC")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(CHILD_TIMEOUT_USEC_DEFAULT)
    })
}

fn arm_child_timeout() {
    let usec = child_timeout_usec();
    let t = ITimerVal {
        it_interval: TimeVal { tv_sec: 0, tv_usec: 0 },
        it_value: TimeVal {
            tv_sec: usec / 1_000_000,
            tv_usec: usec % 1_000_000,
        },
    };
    unsafe { setitimer(ITIMER_REAL, &t, std::ptr::null_mut()) };
}

const PROT_READ_WRITE: c_int = 0x1 | 0x2;
const MAP_SHARED_ANON: c_int = 0x01 | 0x20;
pub const SIGABRT: c_int = 6;
pub const SIGSEGV: c_int = 11;
pub const SIGALRM: c_int = 14;
/// Wall-clock budget for a child; a hung child becomes `Signaled(SIGALRM)`
/// rather than hanging the suite.

/// Page-aligned shared memory that survives `fork` in both directions.
struct Shared {
    p: *mut u8,
    len: usize,
}

impl Shared {
    fn new(len: usize) -> Shared {
        let len = (len + 4095) & !4095;
        let p = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ_WRITE,
                MAP_SHARED_ANON,
                -1,
                0,
            )
        };
        assert!(p as isize != -1, "mmap failed");
        unsafe { std::ptr::write_bytes(p as *mut u8, 0, len) };
        Shared { p: p as *mut u8, len }
    }
    fn at(&self, off: usize) -> *mut u8 {
        unsafe { self.p.add(off) }
    }
    fn slice(&self, off: usize, n: usize) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p.add(off), n) }
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        unsafe { munmap(self.p as *mut c_void, self.len) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildStatus {
    Exited(c_int),
    Signaled(c_int),
}

impl ChildStatus {
    pub fn is_abort(self) -> bool {
        self == ChildStatus::Signaled(SIGABRT)
    }
    pub fn is_segv(self) -> bool {
        self == ChildStatus::Signaled(SIGSEGV)
    }
    pub fn is_timeout(self) -> bool {
        self == ChildStatus::Signaled(SIGALRM)
    }
    pub fn crashed(self) -> bool {
        matches!(self, ChildStatus::Signaled(_))
    }
}

fn wait_for(pid: c_int) -> ChildStatus {
    let mut status: c_int = 0;
    unsafe { waitpid(pid, &mut status, 0) };
    if status & 0x7f == 0x7f {
        // stopped; treat as a crash we cannot classify
        ChildStatus::Signaled((status >> 8) & 0xff)
    } else if status & 0x7f != 0 {
        ChildStatus::Signaled(status & 0x7f)
    } else {
        ChildStatus::Exited((status >> 8) & 0xff)
    }
}

/// Header laid out at the start of the shared region.
const HDR_RET: usize = 0;
const HDR_ERRLEN: usize = 4;
const HDR_ERR: usize = 8;
const HDR_ERR_CAP: usize = 504;
const HDR_SIZE: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildOutcome {
    pub status: ChildStatus,
    pub ret: c_int,
    pub buf: Vec<u8>,
    pub err: Option<Vec<u8>>,
}

unsafe fn publish(sh: &Shared, ret: c_int, err: Option<Vec<u8>>) {
    std::ptr::write(sh.at(HDR_RET) as *mut c_int, ret);
    match err {
        None => std::ptr::write(sh.at(HDR_ERRLEN) as *mut c_int, -1),
        Some(v) => {
            let n = v.len().min(HDR_ERR_CAP);
            std::ptr::write(sh.at(HDR_ERRLEN) as *mut c_int, n as c_int);
            std::ptr::copy_nonoverlapping(v.as_ptr(), sh.at(HDR_ERR), n);
        }
    }
}

fn read_published(
    sh: &Shared,
    buf_off: usize,
    buf_len: usize,
) -> (c_int, Option<Vec<u8>>, Vec<u8>) {
    unsafe {
        let ret = std::ptr::read(sh.at(HDR_RET) as *const c_int);
        let errlen = std::ptr::read(sh.at(HDR_ERRLEN) as *const c_int);
        let err = if errlen < 0 {
            None
        } else {
            Some(sh.slice(HDR_ERR, errlen as usize).to_vec())
        };
        (ret, err, sh.slice(buf_off, buf_len).to_vec())
    }
}

/// `cp_inflate` in a forked child. `out` lives in shared memory so the parent
/// can inspect whatever the child wrote before crashing.
#[allow(clippy::too_many_arguments)]
pub fn inflate_in_child(
    lib: &Lib,
    input: &[u8],
    in_align: usize,
    in_bytes: c_int,
    out_len: usize,
    out_align: usize,
    out_bytes: c_int,
    null_in: bool,
    null_out: bool,
) -> ChildOutcome {
    let sh = Shared::new(HDR_SIZE + 4096 + out_len + OVERRUN_PAD + 4096);
    let buf_off = HDR_SIZE + 4096 + out_align;
    let buf_cmp = out_len + OVERRUN_PAD;
    // The input buffer is written before the fork; the child inherits it.
    let mut inbuf = AlignedBuf::new(input, in_align, OVERRUN_PAD);
    let in_ptr = if null_in {
        std::ptr::null_mut()
    } else {
        inbuf.ptr() as *mut c_void
    };
    let out_ptr = if null_out {
        std::ptr::null_mut()
    } else {
        sh.at(buf_off) as *mut c_void
    };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            arm_child_timeout();
            lib.set_error_reason_null();
            let f = lib.inflate();
            let ret = f(in_ptr, in_bytes, out_ptr, out_bytes);
            let err = lib.error_reason();
            publish(&sh, ret, err);
            _exit(0);
        }
    }
    let status = wait_for(pid);
    let (ret, err, buf) = read_published(&sh, buf_off, buf_cmp);
    ChildOutcome { status, ret, buf, err }
}

/// `unfilter` in a forked child; `raw` lives in shared memory.
pub fn unfilter_in_child(
    lib: &Lib,
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: &[u8],
    null_raw: bool,
) -> ChildOutcome {
    let sh = Shared::new(HDR_SIZE + 4096 + raw.len() + 4096);
    let buf_off = HDR_SIZE + 4096;
    unsafe { std::ptr::copy_nonoverlapping(raw.as_ptr(), sh.at(buf_off), raw.len()) };
    let raw_ptr = if null_raw {
        std::ptr::null_mut()
    } else {
        sh.at(buf_off)
    };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            arm_child_timeout();
            lib.set_error_reason_null();
            let f = lib.unfilter();
            let ret = f(w, h, bpp, raw_ptr);
            let err = lib.error_reason();
            publish(&sh, ret, err);
            _exit(0);
        }
    }
    let status = wait_for(pid);
    let (ret, err, buf) = read_published(&sh, buf_off, raw.len());
    ChildOutcome { status, ret, buf, err }
}

/// Compares two `ChildOutcome`s, ignoring the payload when either side crashed
/// before publishing (a crashed child never wrote the header).
pub fn assert_child_match(a: &ChildOutcome, b: &ChildOutcome, label: &str) {
    assert_eq!(
        a.status, b.status,
        "[{label}] child status mismatch: C={:?} Rust={:?}",
        a.status, b.status
    );
    if a.status.crashed() {
        // Both crashed the same way; the in-flight buffer contents at the
        // moment of a SIGSEGV/SIGABRT are not a defined part of the contract.
        return;
    }
    assert_eq!(
        a.ret, b.ret,
        "[{label}] return mismatch: C={} Rust={}",
        a.ret, b.ret
    );
    assert_eq!(
        a.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        b.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        "[{label}] cp_error_reason mismatch"
    );
    if a.buf != b.buf {
        let at = a
            .buf
            .iter()
            .zip(b.buf.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(0);
        let lo = at.saturating_sub(8);
        let hi = (at + 24).min(a.buf.len());
        panic!(
            "[{label}] buffer mismatch at byte {at}:\n  C   ={:02x?}\n  Rust={:02x?}",
            &a.buf[lo..hi],
            &b.buf[lo..hi]
        );
    }
}

/// Writes a byte into an exported table inside a `.so` (used to reach
/// `ERRORS.md` A9, which is only reachable by corrupting `cp_fixed_table`).
pub fn poke_table(lib: &Lib, sym: &[u8], index: usize, value: u8) -> u8 {
    let mut name = sym.to_vec();
    name.push(0);
    unsafe {
        let s: Symbol<*mut u8> = lib.lib.get(&name).expect("table symbol missing");
        let p = (*s).add(index);
        let old = *p;
        *p = value;
        old
    }
}

/// Runs the input through the instrumented probe copy of the C and reports
/// whether `cp_dynamic`'s `lens[]` fill loop wrote at or past index 320 — i.e.
/// whether the *real* C would have overrun the array and corrupted its own
/// stack frame. Exact, because the probe is the same code with a bigger array.
///
/// Returns `None` when the probe library has not been built, or when the probe
/// itself crashed/hung before it could report.
pub fn lens_overrun(input: &[u8], out_len: usize, out_bytes: c_int) -> Option<bool> {
    let lib = libs().probe.as_ref()?;
    let sh = Shared::new(HDR_SIZE + 4096 + out_len + OVERRUN_PAD + 4096);
    let buf_off = HDR_SIZE + 4096;
    let mut inbuf = AlignedBuf::new(input, 0, OVERRUN_PAD);
    let in_ptr = inbuf.ptr() as *mut c_void;
    let out_ptr = sh.at(buf_off) as *mut c_void;
    let in_bytes = input.len() as c_int;

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            arm_child_timeout();
            let flag: Symbol<*mut c_int> =
                lib.lib.get(b"cp_lens_overrun\0").expect("cp_lens_overrun missing");
            **flag = 0;
            let f = lib.inflate();
            let _ = f(in_ptr, in_bytes, out_ptr, out_bytes);
            std::ptr::write(sh.at(HDR_RET) as *mut c_int, **flag);
            std::ptr::write(sh.at(HDR_ERRLEN) as *mut c_int, 1);
            _exit(0);
        }
    }
    let status = wait_for(pid);
    if status.crashed() {
        // The probe could not finish; treat the verdict as unknown so the
        // caller falls back to the conservative path.
        return None;
    }
    unsafe {
        if std::ptr::read(sh.at(HDR_ERRLEN) as *const c_int) != 1 {
            return None;
        }
        Some(std::ptr::read(sh.at(HDR_RET) as *const c_int) != 0)
    }
}

/// Verdict for a single differential comparison of a `cp_inflate` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffVerdict {
    /// Identical return value, `cp_error_reason` and output bytes.
    Identical,
    /// Both libraries died the same way (same signal).
    BothCrashed,
    /// They differ, and the instrumented probe confirms the input drives
    /// `cp_dynamic`'s `lens[]` past its 320-byte bound, corrupting the C's own
    /// stack frame. Undefined behaviour with no defined result to match.
    CLensOverrun,
    /// They differ and the probe could not reach a verdict.
    ProbeInconclusive,
}

/// Compares two `ChildOutcome`s from the same `cp_inflate` input, tolerating
/// *only* divergences the `lens_overrun` oracle attributes to the C's
/// out-of-bounds stack write. Anything else panics.
pub fn compare_or_ub(
    c: &ChildOutcome,
    r: &ChildOutcome,
    stream: &[u8],
    out_len: usize,
    out_bytes: c_int,
    label: &str,
) -> DiffVerdict {
    let diverged = if c.status != r.status {
        true
    } else if c.status.crashed() {
        false
    } else {
        c.ret != r.ret || c.err != r.err || c.buf != r.buf
    };

    if !diverged {
        return if c.status.crashed() {
            DiffVerdict::BothCrashed
        } else {
            DiffVerdict::Identical
        };
    }

    if !c.status.crashed() && r.status.crashed() {
        panic!(
            "[{label}] Rust crashed ({:?}) where the C returned normally\n  input({}) = {:02x?}",
            r.status,
            stream.len(),
            stream
        );
    }

    match lens_overrun(stream, out_len, out_bytes) {
        Some(true) => DiffVerdict::CLensOverrun,
        Some(false) => panic!(
            "[{label}] divergence WITHOUT a lens[] overrun -- this is a translation bug\n  input({}) = {:02x?}\n  C   = status {:?} ret {} err {:?}\n  Rust= status {:?} ret {} err {:?}",
            stream.len(),
            stream,
            c.status,
            c.ret,
            c.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            r.status,
            r.ret,
            r.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        ),
        None => DiffVerdict::ProbeInconclusive,
    }
}
