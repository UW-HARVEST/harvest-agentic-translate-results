//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared libraries are loaded with `libloading` (`RTLD_LOCAL`, so neither
//! interposes the other) and every call is made through `dlsym`-resolved
//! symbols — the Rust crate is never linked directly, so the `#[no_mangle]`
//! wrappers are part of what is under test.
//!
//! Every call runs in a **forked child**:
//!   * a failed `assert()` (SIGABRT) or a stray SIGSEGV becomes an observable,
//!     comparable outcome instead of killing the test runner, and
//!   * the two children are forked back-to-back from the same parent state, so
//!     glibc's arena is in an identical state for both — `malloc` returns the
//!     same addresses with the same contents, which makes the C's reads of
//!     uninitialised heap deterministic and therefore comparable.

#![allow(dead_code)]

pub mod deflate;
pub mod png;

use std::ffi::{c_char, c_int, c_void, OsStr};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Public C types
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut u8,
}

pub type LoadPngMem = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type CpInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub load_png_mem: LoadPngMem,
    pub cp_inflate: CpInflate,
    pub cp_error_reason: *mut *const c_char,
    pub cp_fixed_table: *mut u8,
    pub cp_permutation_order: *mut u8,
    pub cp_len_extra_bits: *mut u8,
    pub cp_len_base: *mut u32,
    pub cp_dist_extra_bits: *mut u8,
    pub cp_dist_base: *mut u32,
}

fn find_so(dir: &PathBuf, want: Option<&str>) -> PathBuf {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("so")))
        .filter(|p| match want {
            Some(w) => p.file_name().unwrap().to_string_lossy().contains(w),
            None => true,
        })
        .collect();
    hits.sort();
    assert!(
        !hits.is_empty(),
        "no .so found in {} (want={:?}) — build it first",
        dir.display(),
        want
    );
    hits.remove(0)
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    find_so(&manifest().parent().unwrap().join("c_src/build"), None)
}

pub fn rust_so_path() -> PathBuf {
    let p = rust_so_path_inner();
    // A stale .so would silently verify the previous translation, so refuse it.
    let src = manifest().join("src/lib.rs");
    if let (Ok(a), Ok(b)) = (std::fs::metadata(&p), std::fs::metadata(&src)) {
        if let (Ok(ta), Ok(tb)) = (a.modified(), b.modified()) {
            assert!(
                ta >= tb,
                "{} is older than src/lib.rs — rebuild it first \
                 (cargo build --release), otherwise these tests verify stale code",
                p.display()
            );
        }
    }
    p
}

fn rust_so_path_inner() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest().join("target/release");
    if rel.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&rel) {
            if rd
                .filter_map(|e| e.ok())
                .any(|e| e.path().extension() == Some(OsStr::new("so")))
            {
                return find_so(&rel, Some("load_png_mem_lib"));
            }
        }
    }
    find_so(&manifest().join("target/debug"), Some("load_png_mem_lib"))
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Lib {
    let lib = libloading::Library::new(path)
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    macro_rules! fun {
        ($t:ty, $s:literal) => {{
            let sym: libloading::Symbol<$t> = lib
                .get($s)
                .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", name, $s));
            *sym
        }};
    }
    macro_rules! data {
        ($t:ty, $s:literal) => {{
            let sym: libloading::Symbol<*mut $t> = lib
                .get($s)
                .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", name, $s));
            sym.into_raw().into_raw() as *mut $t
        }};
    }
    let load_png_mem = fun!(LoadPngMem, b"load_png_mem\0");
    let cp_inflate = fun!(CpInflate, b"cp_inflate\0");
    let cp_error_reason = data!(*const c_char, b"cp_error_reason\0");
    let cp_fixed_table = data!(u8, b"cp_fixed_table\0");
    let cp_permutation_order = data!(u8, b"cp_permutation_order\0");
    let cp_len_extra_bits = data!(u8, b"cp_len_extra_bits\0");
    let cp_len_base = data!(u32, b"cp_len_base\0");
    let cp_dist_extra_bits = data!(u8, b"cp_dist_extra_bits\0");
    let cp_dist_base = data!(u32, b"cp_dist_base\0");
    Lib {
        name,
        _lib: lib,
        load_png_mem,
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

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

static mut PAIR: Option<Pair> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Both libraries, loaded once per test process.
pub fn libs() -> &'static Pair {
    unsafe {
        INIT.call_once(|| {
            let c = load("C", &c_so_path());
            let r = load("RUST", &rust_so_path());
            PAIR = Some(Pair { c, r });
        });
        #[allow(static_mut_refs)]
        PAIR.as_ref().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Fork harness + shared-memory result block
// ---------------------------------------------------------------------------

pub const SHM_CAP: usize = 1 << 22; // 4 MiB of payload room

/// Backstop against a malformed input sending either library into an endless
/// loop (`cp_decode` can consume 0 bits and spin forever). A *one-sided* timeout
/// is re-run with `SLOW_ALARM_SECS` before being believed, because the C is
/// built at `-O0` and the Rust `.so` at `-O3`, so "one side timed out" on its own
/// says nothing about behaviour.
pub const FAST_ALARM_SECS: u32 = 2;
pub const SLOW_ALARM_SECS: u32 = 300;

pub const SIGALRM: i32 = 14;

static ALARM_SECS: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(FAST_ALARM_SECS);
/// Extra address space (bytes) the child may use beyond the parent's current
/// footprint, or 0 for unlimited. Capping this makes absurd geometries fail
/// deterministically at `malloc` in *both* libraries instead of spending minutes
/// walking gigabytes.
static EXTRA_AS_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PARENT_VM_BYTES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_child_as_limit(extra_bytes: u64) {
    EXTRA_AS_BYTES.store(extra_bytes, std::sync::atomic::Ordering::SeqCst);
}

fn parent_vm_bytes() -> u64 {
    let v = PARENT_VM_BYTES.load(std::sync::atomic::Ordering::SeqCst);
    if v != 0 {
        return v;
    }
    let page = 4096u64;
    let v = std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().next().and_then(|w| w.parse::<u64>().ok()))
        .map(|pages| pages * page)
        .unwrap_or(1 << 30);
    PARENT_VM_BYTES.store(v, std::sync::atomic::Ordering::SeqCst);
    v
}

#[repr(C)]
pub struct Shm {
    pub returned: i32,
    pub ret: i64,
    pub w: i32,
    pub h: i32,
    pub pix_null: i32,
    pub err_present: i32,
    pub err_len: i32,
    pub err: [u8; 512],
    pub payload_len: i64,
    pub payload: [u8; SHM_CAP],
}

#[derive(Debug, Clone, Eq)]
pub struct Outcome {
    /// `Some(signal)` if the child was killed, else `None`.
    pub signal: Option<i32>,
    /// true if the library call returned instead of aborting.
    pub returned: bool,
    pub ret: i64,
    pub w: i32,
    pub h: i32,
    pub pix_null: bool,
    pub err: Option<Vec<u8>>,
    pub payload: Vec<u8>,
    /// Whatever the child wrote to fd 2 (glibc's `__assert_fail` message for the
    /// C library; the Rust library's `abort()` is silent). Deliberately EXCLUDED
    /// from equality — it is diagnostic only, used to prove which `assert()` a
    /// given input reaches.
    pub stderr: Vec<u8>,
}

impl PartialEq for Outcome {
    fn eq(&self, o: &Self) -> bool {
        self.signal == o.signal
            && self.returned == o.returned
            && self.ret == o.ret
            && self.w == o.w
            && self.h == o.h
            && self.pix_null == o.pix_null
            && self.err == o.err
            && self.payload == o.payload
    }
}

impl Outcome {
    pub fn err_str(&self) -> String {
        match &self.err {
            None => "<null>".to_string(),
            Some(v) => String::from_utf8_lossy(v).into_owned(),
        }
    }
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
    /// Everything except the payload, for compact assertion messages.
    pub fn head(&self) -> String {
        format!(
            "signal={:?} returned={} ret={} w={} h={} pix_null={} err={:?} payload_len={} stderr={:?}",
            self.signal,
            self.returned,
            self.ret,
            self.w,
            self.h,
            self.pix_null,
            self.err_str(),
            self.payload.len(),
            self.stderr_str().trim_end()
        )
    }
}

unsafe fn capture_fd() -> c_int {
    let name = b"diff-stderr\0";
    let fd = libc::memfd_create(name.as_ptr() as *const c_char, 0);
    assert!(fd >= 0, "memfd_create failed");
    fd
}

unsafe fn read_capture(fd: c_int) -> Vec<u8> {
    let n = libc::lseek(fd, 0, libc::SEEK_END);
    if n <= 0 {
        libc::close(fd);
        return Vec::new();
    }
    let n = (n as usize).min(4096);
    libc::lseek(fd, 0, libc::SEEK_SET);
    let mut v = vec![0u8; n];
    let got = libc::read(fd, v.as_mut_ptr() as *mut c_void, n);
    libc::close(fd);
    v.truncate(if got > 0 { got as usize } else { 0 });
    v
}

unsafe fn shm_new() -> *mut Shm {
    let p = libc::mmap(
        std::ptr::null_mut(),
        std::mem::size_of::<Shm>(),
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_SHARED | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    assert!(p != libc::MAP_FAILED, "mmap failed");
    p as *mut Shm
}

unsafe fn shm_free(p: *mut Shm) {
    libc::munmap(p as *mut c_void, std::mem::size_of::<Shm>());
}

/// Copy the NUL-terminated `cp_error_reason` of `lib` into `shm`.
pub unsafe fn capture_error(lib: &Lib, shm: *mut Shm) {
    let p = *lib.cp_error_reason;
    if p.is_null() {
        (*shm).err_present = 0;
        return;
    }
    (*shm).err_present = 1;
    let mut n = 0usize;
    while n < 511 && *p.add(n) != 0 {
        n += 1;
    }
    std::ptr::copy_nonoverlapping(p as *const u8, (*shm).err.as_mut_ptr(), n);
    (*shm).err_len = n as i32;
}

pub unsafe fn set_payload(shm: *mut Shm, src: *const u8, len: usize) {
    let len = len.min(SHM_CAP);
    if len > 0 {
        std::ptr::copy_nonoverlapping(src, (*shm).payload.as_mut_ptr(), len);
    }
    (*shm).payload_len = len as i64;
}

fn read_outcome(shm: *mut Shm, status: i32, cap_fd: c_int) -> Outcome {
    unsafe {
        let signal = if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        };
        let plen = (*shm).payload_len.max(0) as usize;
        let plen = plen.min(SHM_CAP);
        Outcome {
            signal,
            returned: (*shm).returned != 0,
            ret: (*shm).ret,
            w: (*shm).w,
            h: (*shm).h,
            pix_null: (*shm).pix_null != 0,
            err: if (*shm).err_present != 0 {
                Some((&(*shm).err)[..(*shm).err_len.max(0) as usize].to_vec())
            } else {
                None
            },
            payload: (&(*shm).payload)[..plen].to_vec(),
            stderr: read_capture(cap_fd),
        }
    }
}

/// Child-side setup: no core dumps (this host pipes them to systemd-coredump,
/// which costs seconds per SIGABRT and these tests trip `assert()` thousands of
/// times), stderr to the capture fd, and an alarm as a livelock backstop.
unsafe fn child_setup(cap_fd: c_int) {
    let rl = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    libc::setrlimit(libc::RLIMIT_CORE, &rl);
    // RLIMIT_CORE is ignored when core_pattern pipes to a helper (as it does on
    // this host), so also mark the process non-dumpable — otherwise every
    // SIGABRT costs ~0.6 s in systemd-coredump.
    libc::prctl(libc::PR_SET_DUMPABLE, 0);
    let extra = EXTRA_AS_BYTES.load(std::sync::atomic::Ordering::SeqCst);
    if extra != 0 {
        let cap = PARENT_VM_BYTES.load(std::sync::atomic::Ordering::SeqCst) + extra;
        let rl = libc::rlimit {
            rlim_cur: cap,
            rlim_max: cap,
        };
        libc::setrlimit(libc::RLIMIT_AS, &rl);
    }
    libc::dup2(cap_fd, 2);
    libc::alarm(ALARM_SECS.load(std::sync::atomic::Ordering::SeqCst));
}

/// Run `f(&C, shm)` and `f(&RUST, shm)` in two children forked back-to-back and
/// return both outcomes.
///
/// If exactly ONE side hits the watchdog, the pair is re-run with a much longer
/// alarm: a one-sided timeout at 2 s is far more likely to be the `-O0` C simply
/// being slower than the `-O3` Rust than a behavioural difference, and reporting
/// it either way without checking would be wrong.
pub fn run_pair<F>(f: F) -> (Outcome, Outcome)
where
    F: Fn(&Lib, *mut Shm),
{
    let (c, r) = run_pair_once(&f);
    if (c.signal == Some(SIGALRM)) != (r.signal == Some(SIGALRM)) {
        ALARM_SECS.store(SLOW_ALARM_SECS, std::sync::atomic::Ordering::SeqCst);
        let out = run_pair_once(&f);
        ALARM_SECS.store(FAST_ALARM_SECS, std::sync::atomic::Ordering::SeqCst);
        return out;
    }
    (c, r)
}

fn run_pair_once<F>(f: &F) -> (Outcome, Outcome)
where
    F: Fn(&Lib, *mut Shm),
{
    let libs = libs();
    parent_vm_bytes();
    unsafe {
        let sc = shm_new();
        let sr = shm_new();
        let fc = capture_fd();
        let fr = capture_fd();

        // No parent-side allocation between the two forks: both children
        // inherit a byte-identical heap.
        let pid_c = libc::fork();
        assert!(pid_c >= 0, "fork failed");
        if pid_c == 0 {
            child_setup(fc);
            f(&libs.c, sc);
            (*sc).returned = 1;
            capture_error(&libs.c, sc);
            libc::_exit(0);
        }
        let mut st_c: c_int = 0;
        libc::waitpid(pid_c, &mut st_c, 0);

        let pid_r = libc::fork();
        assert!(pid_r >= 0, "fork failed");
        if pid_r == 0 {
            child_setup(fr);
            f(&libs.r, sr);
            (*sr).returned = 1;
            capture_error(&libs.r, sr);
            libc::_exit(0);
        }
        let mut st_r: c_int = 0;
        libc::waitpid(pid_r, &mut st_r, 0);

        let oc = read_outcome(sc, st_c, fc);
        let or = read_outcome(sr, st_r, fr);
        shm_free(sc);
        shm_free(sr);
        (oc, or)
    }
}

/// Assert the two outcomes are identical, with a diff-friendly message.
#[track_caller]
pub fn assert_same(label: &str, c: &Outcome, r: &Outcome) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE [{label}]\n  C   : {}\n  RUST: {}\n", c.head(), r.head());
    if c.payload != r.payload {
        if c.payload.len() != r.payload.len() {
            msg += &format!(
                "  payload length differs: C={} RUST={}\n",
                c.payload.len(),
                r.payload.len()
            );
        }
        let n = c.payload.len().min(r.payload.len());
        let mut shown = 0;
        for i in 0..n {
            if c.payload[i] != r.payload[i] {
                msg += &format!(
                    "  payload[{i}]: C=0x{:02x} RUST=0x{:02x}\n",
                    c.payload[i], r.payload[i]
                );
                shown += 1;
                if shown == 12 {
                    msg += "  ...\n";
                    break;
                }
            }
        }
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// Canned callers
// ---------------------------------------------------------------------------

/// `load_png_mem(png, png.len())`; payload = the `w*h*4` pixel bytes that
/// `cp_convert`/`cp_depalette` actually write (the tail of the allocation is
/// uninitialised by the C's own design and is not part of the contract).
pub fn call_load_png(png: &[u8]) -> (Outcome, Outcome) {
    call_load_png_len(png, png.len() as c_int)
}

pub fn call_load_png_len(png: &[u8], len: c_int) -> (Outcome, Outcome) {
    let png = png.to_vec();
    run_pair(move |lib, shm| unsafe {
        let img = (lib.load_png_mem)(png.as_ptr(), len);
        (*shm).w = img.w;
        (*shm).h = img.h;
        (*shm).pix_null = img.pix.is_null() as i32;
        (*shm).ret = if img.pix.is_null() { 0 } else { 1 };
        if !img.pix.is_null() {
            let n = (img.w as i64) * (img.h as i64) * 4;
            let n = if n < 0 { 0 } else { n as usize };
            set_payload(shm, img.pix, n);
        }
    })
}

/// `cp_inflate(in, in_bytes, out, out_bytes)` with `out` pre-filled with 0xAA so
/// untouched output bytes are deterministic; payload = the whole out buffer.
pub fn call_inflate(input: &[u8], in_bytes: c_int, out_bytes: c_int) -> (Outcome, Outcome) {
    call_inflate_cfg(input, in_bytes, out_bytes, 0, |_| {})
}

/// As `call_inflate`, plus `in_shift` extra leading pad bytes so the `in`
/// pointer takes a chosen alignment, and a hook to mutate the exported tables
/// inside the child before the call.
pub fn call_inflate_cfg<P>(
    input: &[u8],
    in_bytes: c_int,
    out_bytes: c_int,
    in_shift: usize,
    prep: P,
) -> (Outcome, Outcome)
where
    P: Fn(&Lib) + Copy + Send + 'static,
{
    let input = input.to_vec();
    run_pair(move |lib, shm| unsafe {
        prep(lib);
        // 16-aligned base (glibc malloc guarantees 16) + in_shift.
        let cap = input.len() + in_shift + 64;
        let base = libc::malloc(cap) as *mut u8;
        assert!(!base.is_null());
        libc::memset(base as *mut c_void, 0, cap);
        let inp = base.add(in_shift);
        std::ptr::copy_nonoverlapping(input.as_ptr(), inp, input.len());

        let ob = if out_bytes > 0 { out_bytes as usize } else { 0 };
        let out = libc::malloc(ob.max(1)) as *mut u8;
        assert!(!out.is_null());
        libc::memset(out as *mut c_void, 0xAA, ob.max(1));

        let ret = (lib.cp_inflate)(inp as *mut c_void, in_bytes, out as *mut c_void, out_bytes);
        (*shm).ret = ret as i64;
        set_payload(shm, out, ob);
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.u32() % n
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

pub const SEED: u64 = 0x5EED_C0DE_5EED_C0DE;
