//! Shared differential-testing harness.
//!
//! Both the C `.so` (built by CMake from `c_src/`) and the Rust `.so` (the
//! crate's `cdylib`) are loaded with `dlopen`/`dlsym` and driven purely through
//! their exported symbols — no Rust function is ever called directly, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.
//!
//! Because `lib.c` is compiled without `NDEBUG`, invalid input makes it
//! `abort()`, and several inputs make it read/write outside the caller's
//! buffers. Every call is therefore executed in a forked child process using
//! page-guarded shared mappings:
//!
//! * output buffers are `MAP_SHARED` anonymous mappings followed by a
//!   `PROT_NONE` guard page, so the child's writes are visible to the parent
//!   and an overrun turns into a deterministic `SIGSEGV` at the very same
//!   offset in both libraries;
//! * the *same* input mapping is handed to both libraries, so even the C code's
//!   deliberate out-of-bounds head/tail reads observe identical bytes;
//! * the child's `stderr` is captured so that glibc's `__assert_fail` message
//!   can be compared against `src/cassert.rs`'s emulation.

#![allow(dead_code)]

pub mod cmodel;
pub mod deflate;

use libloading::os::unix::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;
pub type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// All 9 exported symbols of one shared library.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
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

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

fn sym_addr(lib: &Library, name: &[u8]) -> *mut u8 {
    unsafe {
        let s: Symbol<*mut u8> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {:?}: {e}", String::from_utf8_lossy(name)));
        s.into_raw() as *mut u8
    }
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        unsafe {
            let unfilter: UnfilterFn = std::mem::transmute(sym_addr(&lib, b"unfilter\0"));
            let cp_inflate: InflateFn = std::mem::transmute(sym_addr(&lib, b"cp_inflate\0"));
            Lib {
                name,
                unfilter,
                cp_inflate,
                cp_error_reason: sym_addr(&lib, b"cp_error_reason\0") as *mut *const c_char,
                cp_fixed_table: sym_addr(&lib, b"cp_fixed_table\0"),
                cp_permutation_order: sym_addr(&lib, b"cp_permutation_order\0"),
                cp_len_extra_bits: sym_addr(&lib, b"cp_len_extra_bits\0"),
                cp_len_base: sym_addr(&lib, b"cp_len_base\0") as *mut u32,
                cp_dist_extra_bits: sym_addr(&lib, b"cp_dist_extra_bits\0"),
                cp_dist_base: sym_addr(&lib, b"cp_dist_base\0") as *mut u32,
                path,
                _lib: lib,
            }
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cmake -S c_src -B c_src/build \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build",
        p.display()
    );
    p
}

fn newest_mtime(dir: &std::path::Path) -> std::time::SystemTime {
    let mut newest = std::time::SystemTime::UNIX_EPOCH;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                        newest = newest.max(m);
                    }
                }
            }
        }
    }
    newest
}

fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libunfilter_lib.so
    //
    // NB: `cargo test` alone does *not* relink the cdylib (only an `.rmeta` is
    // produced for the lib target), so guard against testing a stale artifact.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    let mut found: Option<PathBuf> = None;
    for _ in 0..3 {
        let cand = dir.join("libunfilter_lib.so");
        if cand.exists() {
            found = Some(cand);
            break;
        }
        dir = match dir.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }
    let so = found.unwrap_or_else(|| {
        panic!(
            "Rust cdylib libunfilter_lib.so not found near {}. Run `cargo build` first.",
            exe.display()
        )
    });
    let so_m = std::fs::metadata(&so)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let src_m = newest_mtime(&manifest_dir().join("src"));
    assert!(
        so_m >= src_m,
        "{} is OLDER than src/*.rs — `cargo test` does not relink the cdylib.\n\
         Run `cargo build` (or scripts/run_tests.sh) before testing.",
        so.display()
    );
    so
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

/// `(c_lib, rust_lib)`
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| {
        (
            Lib::open("C", c_so_path()),
            Lib::open("Rust", rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// Page-guarded shared buffers
// ---------------------------------------------------------------------------

pub const PAGE: usize = 4096;

pub struct GuardedBuf {
    map: *mut u8,
    base: *mut u8,
    usable: usize,
    total: usize,
}

unsafe impl Send for GuardedBuf {}
unsafe impl Sync for GuardedBuf {}

impl GuardedBuf {
    /// `usable` bytes of `MAP_SHARED` memory with a `PROT_NONE` guard page
    /// *before* and *after*, so that both over- and under-runs turn into a
    /// `SIGSEGV` at exactly the same offset in both libraries.
    pub fn new(min_usable: usize) -> GuardedBuf {
        let pages = (min_usable + PAGE - 1) / PAGE;
        let usable = pages.max(1) * PAGE;
        let total = usable + 2 * PAGE;
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap failed");
            let map = p as *mut u8;
            assert_eq!(
                libc::mprotect(map as *mut c_void, PAGE, libc::PROT_NONE),
                0,
                "mprotect leading guard page failed"
            );
            assert_eq!(
                libc::mprotect(map.add(PAGE + usable) as *mut c_void, PAGE, libc::PROT_NONE),
                0,
                "mprotect trailing guard page failed"
            );
            GuardedBuf {
                map,
                base: map.add(PAGE),
                usable,
                total,
            }
        }
    }

    pub fn ptr(&self) -> *mut u8 {
        self.base
    }
    pub fn usable(&self) -> usize {
        self.usable
    }
    pub fn fill(&self, v: u8) {
        unsafe { std::ptr::write_bytes(self.base, v, self.usable) }
    }
    pub fn write_at(&self, off: usize, data: &[u8]) {
        assert!(off + data.len() <= self.usable);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.base.add(off), data.len()) }
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.usable) }
    }
    pub fn snapshot(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.map as *mut c_void, self.total);
        }
    }
}

// ---------------------------------------------------------------------------
// Fork-based call runner
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    /// `Some(signal)` if the child was killed by a signal.
    pub signal: Option<i32>,
    pub exit: i32,
    /// return value of the library function (meaningless when `signal.is_some()`)
    pub ret: i32,
    /// `cp_error_reason` after the call (`None` == still NULL)
    pub err: Option<Vec<u8>>,
    /// full usable output mapping after the call
    pub out: Vec<u8>,
    /// child's stderr, path-normalised
    pub stderr: String,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outcome")
            .field("signal", &self.signal)
            .field("exit", &self.exit)
            .field("ret", &self.ret)
            .field(
                "err",
                &self
                    .err
                    .as_ref()
                    .map(|e| String::from_utf8_lossy(e).into_owned()),
            )
            .field("out.len", &self.out.len())
            .field("out[..64]", &&self.out[..self.out.len().min(64)])
            .field("stderr", &self.stderr)
            .finish()
    }
}

/// glibc prints the *absolute* `__FILE__` that CMake passed to the compiler,
/// the Rust emulation prints the in-repo relative path. Keep everything from
/// the last `lib.c` onwards: `lib.c:<line>: <func>: Assertion `<expr>' failed.`
pub fn normalize_stderr(s: &str) -> String {
    match s.rfind("lib.c:") {
        Some(i) => s[i..].to_string(),
        None => s.to_string(),
    }
}

/// Real-time budget for a single library call in the forked child.
pub const WATCHDOG_USEC: i64 = 150_000;

pub struct Runner {
    errfd: c_int,
    errpath: PathBuf,
    res: GuardedBuf,
}

impl Runner {
    pub fn new(tag: &str) -> Runner {
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = PathBuf::from(dir).join(format!(
            "cdiff-stderr-{}-{}-{:?}",
            std::process::id(),
            tag,
            std::thread::current().id()
        ));
        let f = std::fs::File::create(&path).expect("create stderr capture file");
        let fd = {
            use std::os::unix::io::IntoRawFd;
            f.into_raw_fd()
        };
        Runner {
            errfd: fd,
            errpath: path,
            res: GuardedBuf::new(PAGE),
        }
    }

    /// Fork, run `f` in the child, and collect the outcome.
    ///
    /// `err_global` is the library's `cp_error_reason` slot; it is reset to NULL
    /// in the child before the call so the observation is per-call.
    pub fn run<F: FnOnce() -> c_int>(
        &self,
        err_global: *mut *const c_char,
        out: &GuardedBuf,
        f: F,
    ) -> Outcome {
        unsafe {
            std::ptr::write_bytes(self.res.ptr(), 0, PAGE);
            (self.res.ptr().add(4) as *mut i32).write(-2);
            libc::ftruncate(self.errfd, 0);
            libc::lseek(self.errfd, 0, libc::SEEK_SET);

            let pid = libc::fork();
            assert!(pid >= 0, "fork failed");
            if pid == 0 {
                libc::dup2(self.errfd, 2);
                // A large part of this suite makes the library `abort()`. Without
                // suppressing core dumps the kernel spends ~0.9 s per call piping
                // the image to systemd-coredump. `RLIMIT_CORE = 0` is *not*
                // enough for a `|pipe` core_pattern (the kernel raises the limit
                // to infinity for pipes unless it is exactly 1), so make the
                // child non-dumpable as well.
                libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
                let no_core = libc::rlimit {
                    rlim_cur: 1,
                    rlim_max: 1,
                };
                libc::setrlimit(libc::RLIMIT_CORE, &no_core);
                // Malformed streams can make `cp_block` spin forever (a garbage
                // Huffman key can consume 0 bits and emit nothing). Arm a real
                // timer so such a call terminates with SIGALRM instead of
                // hanging; "spins forever" is then just another outcome class
                // that both libraries must agree on.
                let it = libc::itimerval {
                    it_interval: libc::timeval {
                        tv_sec: 0,
                        tv_usec: 0,
                    },
                    it_value: libc::timeval {
                        tv_sec: 0,
                        tv_usec: WATCHDOG_USEC,
                    },
                };
                libc::setitimer(libc::ITIMER_REAL, &it, std::ptr::null_mut());
                *err_global = std::ptr::null();
                let ret = f();
                let slot = self.res.ptr();
                (slot as *mut i32).write(ret);
                let e = *err_global;
                if e.is_null() {
                    (slot.add(4) as *mut i32).write(-1);
                } else {
                    let mut n = 0usize;
                    while n < 1024 && *e.add(n) != 0 {
                        n += 1;
                    }
                    std::ptr::copy_nonoverlapping(e as *const u8, slot.add(8), n);
                    (slot.add(4) as *mut i32).write(n as i32);
                }
                libc::_exit(0);
            }

            let mut status: c_int = 0;
            while libc::waitpid(pid, &mut status, 0) < 0 {
                if *libc::__errno_location() != libc::EINTR {
                    panic!("waitpid failed");
                }
            }
            let signal = if libc::WIFSIGNALED(status) {
                Some(libc::WTERMSIG(status))
            } else {
                None
            };
            let exit = if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else {
                -1
            };
            let slot = self.res.ptr();
            let ret = (slot as *const i32).read();
            let errlen = (slot.add(4) as *const i32).read();
            let err = if errlen >= 0 {
                Some(std::slice::from_raw_parts(slot.add(8), errlen as usize).to_vec())
            } else {
                None
            };
            let raw = std::fs::read(&self.errpath).unwrap_or_default();
            Outcome {
                signal,
                exit,
                ret,
                err,
                out: out.snapshot(),
                stderr: normalize_stderr(&String::from_utf8_lossy(&raw)),
            }
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.errfd);
        }
        let _ = std::fs::remove_file(&self.errpath);
    }
}

/// Assert that the two outcomes are identical, dumping both on failure.
#[track_caller]
pub fn same(ctx: &str, c: &Outcome, r: &Outcome) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE [{ctx}]\n");
    if c.signal != r.signal {
        msg += &format!("  signal: C={:?} Rust={:?}\n", c.signal, r.signal);
    }
    if c.exit != r.exit {
        msg += &format!("  exit:   C={} Rust={}\n", c.exit, r.exit);
    }
    if c.ret != r.ret {
        msg += &format!("  ret:    C={} Rust={}\n", c.ret, r.ret);
    }
    if c.err != r.err {
        msg += &format!(
            "  cp_error_reason: C={:?} Rust={:?}\n",
            c.err.as_ref().map(|e| String::from_utf8_lossy(e)),
            r.err.as_ref().map(|e| String::from_utf8_lossy(e))
        );
    }
    if c.stderr != r.stderr {
        msg += &format!("  stderr: C={:?}\n          Rust={:?}\n", c.stderr, r.stderr);
    }
    if c.out != r.out {
        let n = c.out.len().min(r.out.len());
        let first = (0..n).find(|&i| c.out[i] != r.out[i]);
        msg += &format!(
            "  out differs (len C={} Rust={}), first mismatch at {:?}\n",
            c.out.len(),
            r.out.len(),
            first
        );
        if let Some(i) = first {
            let lo = i.saturating_sub(8);
            let hi = (i + 24).min(n);
            msg += &format!("    C   [{lo}..{hi}] = {:02x?}\n", &c.out[lo..hi]);
            msg += &format!("    Rust[{lo}..{hi}] = {:02x?}\n", &r.out[lo..hi]);
        }
    }
    panic!("{msg}");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        (self.next_u64() % n as u64) as u32
    }
    /// uniform in `lo..=hi`
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u32) as i32
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.u8()).collect()
    }
}

// ---------------------------------------------------------------------------
// unfilter helpers
// ---------------------------------------------------------------------------

/// Bytes the C code touches for `w`,`h`,`bpp >= 0`: `h * (w*bpp + 1)`.
pub fn unfilter_span(w: i32, h: i32, bpp: i32) -> usize {
    if h <= 0 {
        return 1;
    }
    let len = (w as i64) * (bpp as i64);
    ((h as i64) * (len + 1)).max(1) as usize
}

/// In-process differential `unfilter` call on two identical copies of `buf`.
/// Returns the (identical) return value and resulting buffer.
#[track_caller]
pub fn check_unfilter(ctx: &str, w: c_int, h: c_int, bpp: c_int, buf: &[u8]) -> (c_int, Vec<u8>) {
    let (c, r) = libs();
    let mut a = buf.to_vec();
    let mut b = buf.to_vec();
    let ra = unsafe { (c.unfilter)(w, h, bpp, a.as_mut_ptr()) };
    let rb = unsafe { (r.unfilter)(w, h, bpp, b.as_mut_ptr()) };
    if ra != rb {
        panic!("DIVERGENCE [{ctx}] unfilter(w={w},h={h},bpp={bpp}) ret: C={ra} Rust={rb}");
    }
    if a != b {
        let n = a.len();
        let i = (0..n).find(|&i| a[i] != b[i]).unwrap();
        let lo = i.saturating_sub(8);
        let hi = (i + 24).min(n);
        panic!(
            "DIVERGENCE [{ctx}] unfilter(w={w},h={h},bpp={bpp}) buffer differs at {i}\n\
             \x20 in  [{lo}..{hi}] = {:02x?}\n  C   [{lo}..{hi}] = {:02x?}\n  Rust[{lo}..{hi}] = {:02x?}",
            &buf[lo..hi],
            &a[lo..hi],
            &b[lo..hi]
        );
    }
    (ra, a)
}

/// Forked `unfilter` call (for parameters that may crash or walk out of bounds).
pub fn run_unfilter_forked(
    lib: &Lib,
    runner: &Runner,
    w: c_int,
    h: c_int,
    bpp: c_int,
    buf: &GuardedBuf,
    off: usize,
    init: &[u8],
    null_ptr: bool,
) -> Outcome {
    buf.fill(0);
    buf.write_at(0, init);
    let f = lib.unfilter;
    let p = if null_ptr {
        std::ptr::null_mut()
    } else {
        unsafe { buf.ptr().add(off) }
    };
    runner.run(lib.cp_error_reason, buf, move || unsafe { f(w, h, bpp, p) })
}

/// Forked `cp_inflate` call. `in_buf` is shared by both libraries.
pub fn run_inflate_forked(
    lib: &Lib,
    runner: &Runner,
    in_ptr: *mut u8,
    in_bytes: c_int,
    out: &GuardedBuf,
    out_bytes: c_int,
    out_fill: u8,
) -> Outcome {
    out.fill(out_fill);
    let f = lib.cp_inflate;
    let ip = in_ptr as *mut c_void;
    let op = out.ptr() as *mut c_void;
    runner.run(lib.cp_error_reason, out, move || unsafe {
        f(ip, in_bytes, op, out_bytes)
    })
}

/// Run the same `cp_inflate` call against both libraries and assert identity.
/// Returns the shared outcome.
#[track_caller]
pub fn check_inflate(
    ctx: &str,
    runner: &Runner,
    in_ptr: *mut u8,
    in_bytes: c_int,
    out_c: &GuardedBuf,
    out_r: &GuardedBuf,
    out_bytes: c_int,
) -> Outcome {
    let (c, r) = libs();
    let oc = run_inflate_forked(c, runner, in_ptr, in_bytes, out_c, out_bytes, 0xA5);
    let or = run_inflate_forked(r, runner, in_ptr, in_bytes, out_r, out_bytes, 0xA5);
    same(ctx, &oc, &or);
    oc
}

// ---------------------------------------------------------------------------
// cp_inflate harness
// ---------------------------------------------------------------------------

impl Outcome {
    /// After the process died from a signal, the contents of the output mapping
    /// and the return value are not observable by a caller, so they are not
    /// part of the compared contract (the assertion message + signal are).
    pub fn normalize(mut self) -> Outcome {
        if self.signal.is_some() {
            self.out.clear();
            self.ret = 0;
            self.err = None;
        }
        self
    }
}

pub struct InflateHarness {
    pub runner: Runner,
    pub inbuf: GuardedBuf,
    pub out_c: GuardedBuf,
    pub out_r: GuardedBuf,
}

impl InflateHarness {
    pub fn new(tag: &str, in_cap: usize, out_cap: usize) -> InflateHarness {
        InflateHarness {
            runner: Runner::new(tag),
            inbuf: GuardedBuf::new(in_cap),
            out_c: GuardedBuf::new(out_cap),
            out_r: GuardedBuf::new(out_cap),
        }
    }

    /// Place `stream` at byte offset `align` of the (page-aligned) input mapping
    /// so that `((size_t)in) & 3 == align & 3`, then call both libraries.
    /// The rest of the input mapping is zero-filled, so the C code's deliberate
    /// out-of-bounds head/tail reads see the same bytes in both runs (the *same*
    /// mapping is handed to both).
    #[track_caller]
    pub fn call(&self, ctx: &str, stream: &[u8], align: usize, out_bytes: i32) -> Outcome {
        assert!(align + stream.len() <= self.inbuf.usable());
        self.inbuf.fill(0);
        self.inbuf.write_at(align, stream);
        let in_ptr = unsafe { self.inbuf.ptr().add(align) };
        let (c, r) = libs();
        let oc = run_inflate_forked(
            c,
            &self.runner,
            in_ptr,
            stream.len() as c_int,
            &self.out_c,
            out_bytes,
            0xA5,
        )
        .normalize();
        let or = run_inflate_forked(
            r,
            &self.runner,
            in_ptr,
            stream.len() as c_int,
            &self.out_r,
            out_bytes,
            0xA5,
        )
        .normalize();
        same(ctx, &oc, &or);
        // Third opinion: the independent transcription of lib.c in `cmodel`.
        // Skipped when the C code would perform undefined behaviour, since then
        // there is no defined result for anybody to agree on.
        let m = self.model(stream, align, stream.len() as c_int, out_bytes);
        if m.defined() {
            if let Err(e) = model_matches(&oc, &m) {
                panic!("[{ctx}] library disagrees with the independent C model: {e}\n  model = {:?}\n  lib   = {oc:?}", m.end);
            }
        }
        oc
    }

    /// Like [`InflateHarness::call`] but runs `setup` inside the forked child
    /// first, so that mutations of the library's exported globals are per-call
    /// (the parent's copies stay pristine thanks to fork's copy-on-write).
    #[track_caller]
    pub fn call_with_setup(
        &self,
        ctx: &str,
        stream: &[u8],
        align: usize,
        out_bytes: i32,
        setup: &dyn Fn(&Lib),
    ) -> Outcome {
        assert!(align + stream.len() <= self.inbuf.usable());
        self.inbuf.fill(0);
        self.inbuf.write_at(align, stream);
        let in_ptr = unsafe { self.inbuf.ptr().add(align) };
        let (c, r) = libs();
        let n = stream.len() as c_int;
        let mut outs = Vec::new();
        for (lib, buf) in [(c, &self.out_c), (r, &self.out_r)] {
            buf.fill(0xA5);
            let f = lib.cp_inflate;
            let ip = in_ptr as *mut c_void;
            let op = buf.ptr() as *mut c_void;
            outs.push(
                self.runner
                    .run(lib.cp_error_reason, buf, || unsafe {
                        setup(lib);
                        f(ip, n, op, out_bytes)
                    })
                    .normalize(),
            );
        }
        same(ctx, &outs[0], &outs[1]);
        outs.remove(0)
    }

    /// Non-asserting variant of [`InflateHarness::call`]: returns both outcomes.
    pub fn call_pair(&self, stream: &[u8], align: usize, out_bytes: i32) -> (Outcome, Outcome) {
        assert!(align + stream.len() <= self.inbuf.usable());
        self.inbuf.fill(0);
        self.inbuf.write_at(align, stream);
        let in_ptr = unsafe { self.inbuf.ptr().add(align) };
        let (c, r) = libs();
        let oc = run_inflate_forked(
            c,
            &self.runner,
            in_ptr,
            stream.len() as c_int,
            &self.out_c,
            out_bytes,
            0xA5,
        )
        .normalize();
        let or = run_inflate_forked(
            r,
            &self.runner,
            in_ptr,
            stream.len() as c_int,
            &self.out_r,
            out_bytes,
            0xA5,
        )
        .normalize();
        (oc, or)
    }

    /// Like [`call`] but with an explicit `in_bytes` (may disagree with
    /// `stream.len()` to test truncation / oversizing) and a NULL-`in` option.
    #[track_caller]
    pub fn call_raw(
        &self,
        ctx: &str,
        stream: &[u8],
        align: usize,
        in_bytes: i32,
        out_bytes: i32,
        null_in: bool,
        null_out: bool,
    ) -> Outcome {
        assert!(align + stream.len() <= self.inbuf.usable());
        self.inbuf.fill(0);
        self.inbuf.write_at(align, stream);
        let in_ptr = if null_in {
            std::ptr::null_mut()
        } else {
            unsafe { self.inbuf.ptr().add(align) }
        };
        let (c, r) = libs();
        let mut outs = Vec::new();
        for (lib, buf) in [(c, &self.out_c), (r, &self.out_r)] {
            buf.fill(0xA5);
            let f = lib.cp_inflate;
            let ip = in_ptr as *mut c_void;
            let op = if null_out {
                std::ptr::null_mut()
            } else {
                buf.ptr() as *mut c_void
            };
            outs.push(
                self.runner
                    .run(lib.cp_error_reason, buf, move || unsafe {
                        f(ip, in_bytes, op, out_bytes)
                    })
                    .normalize(),
            );
        }
        same(ctx, &outs[0], &outs[1]);
        outs.remove(0)
    }
}

impl InflateHarness {
    /// Run the independent C model (`cmodel`) on exactly the same input.
    pub fn model(
        &self,
        stream: &[u8],
        align: usize,
        in_bytes: i32,
        out_bytes: i32,
    ) -> cmodel::ModelResult {
        let mut map = vec![0u8; self.inbuf.usable()];
        map[align..align + stream.len()].copy_from_slice(stream);
        cmodel::run(&map, align, in_bytes, out_bytes, self.out_c.usable())
    }
}

/// Compare a library outcome against the independent model.
pub fn model_matches(o: &Outcome, m: &cmodel::ModelResult) -> Result<(), String> {
    use cmodel::End;
    match &m.end {
        End::Ret(r, msg) => {
            if o.signal.is_some() {
                return Err(format!(
                    "model says ret={r} but the library died with {:?}",
                    o.signal
                ));
            }
            if o.ret != *r {
                return Err(format!("model ret={r}, library ret={}", o.ret));
            }
            let lib_msg = o.err.as_ref().map(|e| String::from_utf8_lossy(e).into_owned());
            let mdl_msg = msg.map(|m| m.to_string());
            if lib_msg != mdl_msg {
                return Err(format!("model err={mdl_msg:?}, library err={lib_msg:?}"));
            }
            if o.out != m.out {
                let n = o.out.len().min(m.out.len());
                let i = (0..n).find(|&i| o.out[i] != m.out[i]);
                return Err(format!(
                    "output differs from the model at {i:?} (lens {} vs {})",
                    o.out.len(),
                    m.out.len()
                ));
            }
            Ok(())
        }
        End::Abort(line) => {
            if o.signal != Some(libc::SIGABRT) {
                return Err(format!(
                    "model says abort at lib.c:{line}, library gave signal={:?} ret={} err={:?}",
                    o.signal,
                    o.ret,
                    o.err.as_ref().map(|e| String::from_utf8_lossy(e).into_owned())
                ));
            }
            let want = format!("lib.c:{line}:");
            if !o.stderr.starts_with(&want) {
                return Err(format!(
                    "model says abort at {want}, library said {:?}",
                    o.stderr
                ));
            }
            Ok(())
        }
        End::Loop => {
            if o.signal != Some(libc::SIGALRM) {
                return Err(format!(
                    "model says the C code spins forever, library gave signal={:?} stderr={:?}",
                    o.signal, o.stderr
                ));
            }
            Ok(())
        }
        End::Fault => {
            if o.signal != Some(libc::SIGSEGV) {
                return Err(format!(
                    "model says the C code reads outside the mapping, library gave signal={:?}",
                    o.signal
                ));
            }
            Ok(())
        }
    }
}

/// Assert that a successful decode produced exactly `expected` followed by
/// untouched fill bytes.
#[track_caller]
pub fn expect_output(ctx: &str, o: &Outcome, expected: &[u8], out_bytes: i32) {
    assert_eq!(o.signal, None, "[{ctx}] unexpected signal: {o:?}");
    assert_eq!(o.ret, 1, "[{ctx}] cp_inflate failed: {o:?}");
    assert_eq!(
        &o.out[..expected.len()],
        expected,
        "[{ctx}] decoded payload differs from the reference model"
    );
    assert!(
        o.out[expected.len()..].iter().all(|&b| b == 0xA5),
        "[{ctx}] bytes past the decoded payload were modified (out_bytes={out_bytes})"
    );
}
