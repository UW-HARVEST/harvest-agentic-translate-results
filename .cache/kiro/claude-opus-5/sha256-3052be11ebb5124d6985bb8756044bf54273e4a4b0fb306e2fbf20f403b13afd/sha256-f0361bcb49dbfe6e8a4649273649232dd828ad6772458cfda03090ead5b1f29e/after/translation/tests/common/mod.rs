//! Shared differential-testing harness.
//!
//! Both libraries are loaded as shared objects with `libloading` and called
//! only through their exported symbols.  Every call happens in a `fork()`ed
//! child, because the ground-truth C library is compiled **without** `NDEBUG`
//! (see ERRORS.md) and therefore `abort()`s on a failed `assert()`.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

pub type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::os::unix::Library,
    pub pinflate: PinflateFn,
    pub error_reason: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let root = manifest().parent().unwrap().join("c_src").join("build");
    for e in std::fs::read_dir(&root).unwrap_or_else(|_| {
        panic!(
            "C build dir {} missing - build it first (cmake .. && cmake --build .)",
            root.display()
        )
    }) {
        let p = e.unwrap().path();
        if p.extension().map(|x| x == "so").unwrap_or(false) {
            return p;
        }
    }
    panic!("no .so found in {}", root.display());
}

pub fn rust_so_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let p = manifest()
        .join("target")
        .join(profile)
        .join("libpinflate_lib.so");
    assert!(p.exists(), "{} missing - run cargo build first", p.display());
    p
}

impl Lib {
    pub fn open(name: &'static str, path: &std::path::Path) -> Lib {
        unsafe {
            // RTLD_NOW so that every PLT entry is bound *before* we fork.
            // RTLD_LOCAL keeps the two libraries' identically named globals
            // (cp_error_reason, cp_len_base, ...) separate.
            let lib = libloading::os::unix::Library::open(
                Some(path),
                libc::RTLD_NOW | libc::RTLD_LOCAL,
            )
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));

            macro_rules! addr {
                ($sym:literal, $t:ty) => {{
                    let s: libloading::os::unix::Symbol<*mut u8> = lib
                        .get($sym)
                        .unwrap_or_else(|e| panic!("{} missing {}: {e}", name, stringify!($sym)));
                    s.into_raw() as $t
                }};
            }

            let f: libloading::os::unix::Symbol<PinflateFn> =
                lib.get(b"pinflate\0").expect("pinflate missing");
            let pinflate = *f;

            Lib {
                name,
                pinflate,
                error_reason: addr!(b"cp_error_reason\0", *mut *const c_char),
                fixed_table: addr!(b"cp_fixed_table\0", *mut u8),
                permutation_order: addr!(b"cp_permutation_order\0", *mut u8),
                len_extra_bits: addr!(b"cp_len_extra_bits\0", *mut u8),
                len_base: addr!(b"cp_len_base\0", *mut u32),
                dist_extra_bits: addr!(b"cp_dist_extra_bits\0", *mut u8),
                dist_base: addr!(b"cp_dist_base\0", *mut u32),
                _lib: lib,
            }
        }
    }
}

pub fn c_lib() -> Lib {
    Lib::open("C", &c_so_path())
}
pub fn rust_lib() -> Lib {
    Lib::open("Rust", &rust_so_path())
}

// ---------------------------------------------------------------------------
// A single differential case
// ---------------------------------------------------------------------------

pub const SLACK: usize = 64;

/// `cp_stored` copies `LEN` bytes (up to 65535) with **no** bound check against
/// either buffer, and `cp_ptr` can point up to 8 bytes *before* `in`.  Both
/// buffers therefore get a zeroed guard region so that the two libraries read
/// and write defined bytes; otherwise the over-read would return whatever
/// happens to follow the allocation, which depends on how each library
/// allocated its `cp_state_t` (C: `calloc`, Rust: the global allocator) and is
/// not a property of the translation.  The whole out region including the
/// guard is compared, so a stray write is still caught.
pub const GUARD: usize = 0x10000 + 128;

#[derive(Clone, Debug)]
pub struct Case {
    pub input: Vec<u8>,
    /// desired `in_ptr % 4`
    pub in_align: usize,
    /// value handed to the `in_bytes` parameter
    pub in_bytes: c_int,
    /// value handed to the `out_bytes` parameter
    pub out_bytes: c_int,
    /// bytes actually allocated for (and compared from) the out buffer
    pub out_alloc: usize,
    pub null_in: bool,
    pub null_out: bool,
    /// applied to both libraries' exported tables before the call
    pub patches: Vec<Patch>,
}

#[derive(Clone, Debug)]
pub enum Patch {
    FixedTable(Vec<(usize, u8)>),
    PermutationOrder(Vec<(usize, u8)>),
    LenExtraBits(Vec<(usize, u8)>),
    LenBase(Vec<(usize, u32)>),
    DistExtraBits(Vec<(usize, u8)>),
    DistBase(Vec<(usize, u32)>),
}

impl Case {
    pub fn new(input: Vec<u8>, out_bytes: usize) -> Case {
        let n = input.len() as c_int;
        Case {
            input,
            in_align: 0,
            in_bytes: n,
            out_bytes: out_bytes as c_int,
            out_alloc: out_bytes + SLACK,
            null_in: false,
            null_out: false,
            patches: Vec::new(),
        }
    }
    pub fn align(mut self, a: usize) -> Case {
        self.in_align = a;
        self
    }
    pub fn in_bytes(mut self, n: c_int) -> Case {
        self.in_bytes = n;
        self
    }
    pub fn out_bytes(mut self, n: c_int) -> Case {
        self.out_bytes = n;
        self.out_alloc = if n > 0 { n as usize + SLACK } else { SLACK };
        self
    }
    pub fn out_alloc(mut self, n: usize) -> Case {
        self.out_alloc = n;
        self
    }
    pub fn null_in(mut self) -> Case {
        self.null_in = true;
        self
    }
    pub fn null_out(mut self) -> Case {
        self.null_out = true;
        self
    }
    pub fn patch(mut self, p: Patch) -> Case {
        self.patches.push(p);
        self
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum Outcome {
    /// normal return
    Ret {
        ret: c_int,
        reason: Option<Vec<u8>>,
        out: Vec<u8>,
        /// FNV-1a over the zeroed guard region that follows the compared part
        /// of the out buffer.  `cp_stored` can memcpy up to 65535 bytes with no
        /// bound check, so this catches divergent writes out there without
        /// shipping 64 KiB through the pipe for every case.
        guard_hash: u64,
    },
    /// killed by a signal (SIGABRT from a failed assert, SIGSEGV, ...)
    Signal(c_int),
    /// child exited non-zero / protocol failure
    Broken(String),
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Ret {
                ret,
                reason,
                out,
                guard_hash,
            } => {
                let r = reason
                    .as_ref()
                    .map(|v| String::from_utf8_lossy(v).to_string())
                    .unwrap_or_else(|| "<null>".into());
                write!(
                    f,
                    "Ret {{ ret: {ret}, reason: {r:?}, guard: {guard_hash:#x}, out({}): {} }}",
                    out.len(),
                    hex(out)
                )
            }
            Outcome::Signal(s) => write!(f, "Signal({s})"),
            Outcome::Broken(m) => write!(f, "Broken({m})"),
        }
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == 96 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{x:02x}"));
    }
    s
}

unsafe fn write_all(fd: c_int, buf: &[u8]) {
    let mut off = 0usize;
    while off < buf.len() {
        let n = libc::write(fd, buf.as_ptr().add(off) as *const c_void, buf.len() - off);
        if n <= 0 {
            libc::_exit(97);
        }
        off += n as usize;
    }
}

/// Fatal-signal handler installed in the child.
///
/// `/proc/sys/kernel/core_pattern` on this host pipes to `systemd-coredump`,
/// which costs ~380 ms for every `abort()` and is not suppressed by
/// `RLIMIT_CORE = 0`.  Turning the fatal signal into `_exit(128 + signo)`
/// keeps the outcome observable (the parent maps the code back to a signal)
/// while staying fast.  Both libraries are treated identically, so this does
/// not weaken the differential comparison.
extern "C" fn on_fatal(sig: c_int) {
    unsafe { libc::_exit(128 + sig) }
}

/// Run one case against one library, in a forked child.
pub fn run(lib: &Lib, case: &Case) -> Outcome {
    unsafe { run_inner(lib, case) }.0
}

/// As [`run`], but also returns whatever the child wrote to stderr.  For the C
/// library that is glibc's `assert()` message, which names the exact source
/// line - used by the Phase C tests to prove which `ERRORS.md` row was hit.
pub fn run2(lib: &Lib, case: &Case) -> (Outcome, String) {
    unsafe { run_inner(lib, case) }
}

unsafe fn run_inner(lib: &Lib, case: &Case) -> (Outcome, String) {
    // --- everything is allocated & initialised in the parent, so the child
    // --- performs no allocation before calling into the library.
    let in_len = case.input.len();
    let in_total = GUARD + in_len + GUARD;
    let in_alloc = libc::malloc(in_total) as *mut u8;
    assert!(!in_alloc.is_null());
    std::ptr::write_bytes(in_alloc, 0, in_total);
    let base = in_alloc.add(GUARD);
    let off = (case.in_align + 4 - (base as usize % 4)) % 4;
    let in_ptr = base.add(off);
    if in_len > 0 {
        std::ptr::copy_nonoverlapping(case.input.as_ptr(), in_ptr, in_len);
    }

    let out_cmp = case.out_alloc.max(1);
    let out_total = out_cmp + GUARD;
    let out_alloc = libc::malloc(out_total) as *mut u8;
    assert!(!out_alloc.is_null());
    std::ptr::write_bytes(out_alloc, 0, out_total);
    let out_alloc_len = out_cmp;

    // apply table patches identically to this library
    for p in &case.patches {
        match p {
            Patch::FixedTable(v) => {
                for (i, x) in v {
                    *lib.fixed_table.add(*i) = *x;
                }
            }
            Patch::PermutationOrder(v) => {
                for (i, x) in v {
                    *lib.permutation_order.add(*i) = *x;
                }
            }
            Patch::LenExtraBits(v) => {
                for (i, x) in v {
                    *lib.len_extra_bits.add(*i) = *x;
                }
            }
            Patch::LenBase(v) => {
                for (i, x) in v {
                    *lib.len_base.add(*i) = *x;
                }
            }
            Patch::DistExtraBits(v) => {
                for (i, x) in v {
                    *lib.dist_extra_bits.add(*i) = *x;
                }
            }
            Patch::DistBase(v) => {
                for (i, x) in v {
                    *lib.dist_base.add(*i) = *x;
                }
            }
        }
    }

    let in_arg = if case.null_in {
        std::ptr::null_mut()
    } else {
        in_ptr as *mut c_void
    };
    let out_arg = if case.null_out {
        std::ptr::null_mut()
    } else {
        out_alloc as *mut c_void
    };

    let mut fds = [0 as c_int; 2];
    assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);

    let errpath = format!(
        "/tmp/pinflate_diff_{}_{}.err\0",
        std::process::id(),
        lib.name
    );
    let errfd = libc::open(
        errpath.as_ptr() as *const c_char,
        libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
        0o600 as libc::c_uint,
    );
    // unlink immediately: the fd (and the child's dup of it) keeps the inode
    // alive, so nothing is left behind in /tmp
    if errfd >= 0 {
        libc::unlink(errpath.as_ptr() as *const c_char);
    }

    let pid = libc::fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---------------- child ----------------
        libc::close(fds[0]);
        if errfd >= 0 {
            libc::dup2(errfd, 2);
        } else {
            let dn = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
            if dn >= 0 {
                libc::dup2(dn, 2);
            }
        }
        let f = lib.pinflate;
        for s in [
            libc::SIGABRT,
            libc::SIGSEGV,
            libc::SIGBUS,
            libc::SIGFPE,
            libc::SIGILL,
            libc::SIGALRM,
            libc::SIGTRAP,
        ] {
            libc::signal(s, on_fatal as usize);
        }
        // Watchdog: some malformed streams make `pinflate` loop forever (a
        // zero-length match that also consumes zero bits neither advances
        // `out` nor `bits_left`).  A 250 ms ITIMER_REAL turns that into an
        // observable SIGALRM, identical for both libraries.
        let it = libc::itimerval {
            it_interval: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            it_value: libc::timeval {
                tv_sec: 0,
                tv_usec: 40_000,
            },
        };
        libc::setitimer(libc::ITIMER_REAL, &it, std::ptr::null_mut());
        let ret = f(in_arg, case.in_bytes, out_arg, case.out_bytes);
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
        libc::setitimer(libc::ITIMER_REAL, &off, std::ptr::null_mut());
        let reason = *lib.error_reason;
        write_all(fds[1], &ret.to_ne_bytes());
        let rlen: i32 = if reason.is_null() {
            -1
        } else {
            libc::strlen(reason) as i32
        };
        write_all(fds[1], &rlen.to_ne_bytes());
        if rlen > 0 {
            write_all(
                fds[1],
                std::slice::from_raw_parts(reason as *const u8, rlen as usize),
            );
        }
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut k = out_alloc_len;
        while k < out_total {
            h ^= *out_alloc.add(k) as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
            k += 1;
        }
        write_all(fds[1], &h.to_ne_bytes());
        write_all(fds[1], &(out_alloc_len as i32).to_ne_bytes());
        write_all(fds[1], std::slice::from_raw_parts(out_alloc, out_alloc_len));
        libc::_exit(0);
    }

    // ---------------- parent ----------------
    libc::close(fds[1]);
    let mut data: Vec<u8> = Vec::with_capacity(out_alloc_len + 64);
    let mut tmp = [0u8; 8192];
    loop {
        let n = libc::read(fds[0], tmp.as_mut_ptr() as *mut c_void, tmp.len());
        if n <= 0 {
            break;
        }
        data.extend_from_slice(&tmp[..n as usize]);
    }
    libc::close(fds[0]);
    let mut status: c_int = 0;
    libc::waitpid(pid, &mut status, 0);

    // slurp whatever the child wrote to stderr
    let mut errtxt = String::new();
    if errfd >= 0 {
        libc::lseek(errfd, 0, libc::SEEK_SET);
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(errfd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            errtxt.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
        }
        libc::close(errfd);
    }

    // undo the patches so the parent's tables stay pristine for the next case
    reset_tables(lib);

    libc::free(in_alloc as *mut c_void);
    libc::free(out_alloc as *mut c_void);

    if libc::WIFSIGNALED(status) {
        return (Outcome::Signal(libc::WTERMSIG(status)), errtxt);
    }
    if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) >= 128 {
        // the child's fatal-signal handler translated the signal into an exit
        // code (see `on_fatal`)
        return (
            Outcome::Signal(libc::WEXITSTATUS(status) - 128),
            errtxt,
        );
    }
    if !libc::WIFEXITED(status) || libc::WEXITSTATUS(status) != 0 {
        return (
            Outcome::Broken(format!("child status {status}")),
            errtxt,
        );
    }

    let mut p = 0usize;
    macro_rules! take {
        ($n:expr) => {{
            if p + $n > data.len() {
                return (
                    Outcome::Broken(format!("short read at {p}, have {}", data.len())),
                    errtxt,
                );
            }
            let s = &data[p..p + $n];
            #[allow(unused_assignments)]
            {
                p += $n;
            }
            s
        }};
    }
    let ret = c_int::from_ne_bytes(take!(4).try_into().unwrap());
    let rlen = i32::from_ne_bytes(take!(4).try_into().unwrap());
    let reason = if rlen < 0 {
        None
    } else if rlen == 0 {
        Some(Vec::new())
    } else {
        Some(take!(rlen as usize).to_vec())
    };
    let guard_hash = u64::from_ne_bytes(take!(8).try_into().unwrap());
    let olen = i32::from_ne_bytes(take!(4).try_into().unwrap()) as usize;
    let out = take!(olen).to_vec();
    (
        Outcome::Ret {
            ret,
            reason,
            out,
            guard_hash,
        },
        errtxt,
    )
}

/// Restore the six exported input tables to their documented initial contents.
pub fn reset_tables(lib: &Lib) {
    unsafe {
        std::ptr::copy_nonoverlapping(FIXED_TABLE.as_ptr(), lib.fixed_table, 320);
        std::ptr::copy_nonoverlapping(PERMUTATION_ORDER.as_ptr(), lib.permutation_order, 19);
        std::ptr::copy_nonoverlapping(LEN_EXTRA_BITS.as_ptr(), lib.len_extra_bits, 31);
        std::ptr::copy_nonoverlapping(LEN_BASE.as_ptr(), lib.len_base, 31);
        std::ptr::copy_nonoverlapping(DIST_EXTRA_BITS.as_ptr(), lib.dist_extra_bits, 32);
        std::ptr::copy_nonoverlapping(DIST_BASE.as_ptr(), lib.dist_base, 32);
    }
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    pub fn new() -> Pair {
        Pair {
            c: c_lib(),
            r: rust_lib(),
        }
    }
    /// Runs `case` on both libraries and returns the (identical) outcome,
    /// panicking with a detailed report on any divergence.
    pub fn check(&self, label: &str, case: &Case) -> Outcome {
        self.check2(label, case).0
    }

    /// As [`Pair::check`], but also hands back the C library's stderr, which
    /// carries glibc's `assert()` message (file, line and expression) and thus
    /// identifies exactly which `ERRORS.md` row was hit.
    pub fn check2(&self, label: &str, case: &Case) -> (Outcome, String) {
        let (a, aerr) = run2(&self.c, case);
        let (b, _berr) = run2(&self.r, case);
        if a != b {
            panic!(
                "DIVERGENCE [{label}]\n  in_bytes={} out_bytes={} align={} in({})={}\n  C   : {:?}\n  C stderr: {}\n  Rust: {:?}",
                case.in_bytes,
                case.out_bytes,
                case.in_align,
                case.input.len(),
                hex(&case.input),
                a,
                aerr.trim(),
                b
            );
        }
        (a, aerr)
    }
}

// ---------------------------------------------------------------------------
// Reference copies of the exported tables (from c_src/src/lib.c)
// ---------------------------------------------------------------------------

pub static FIXED_TABLE: [u8; 320] = {
    let mut t = [0u8; 320];
    let mut i = 0;
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    while i < 320 {
        t[i] = 5;
        i += 1;
    }
    t
};

pub static PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
pub static LEN_EXTRA_BITS: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
pub static LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
pub static DIST_EXTRA_BITS: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];
pub static DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// ---------------------------------------------------------------------------
// DEFLATE stream construction
// ---------------------------------------------------------------------------

pub struct BitWriter {
    pub buf: Vec<u8>,
    pub nbits: usize,
}

impl BitWriter {
    pub fn new() -> BitWriter {
        BitWriter {
            buf: Vec::new(),
            nbits: 0,
        }
    }
    /// LSB-first, the way `cp_read_bits` consumes them.
    pub fn bits(&mut self, val: u32, n: u32) {
        for i in 0..n {
            let bit = ((val >> i) & 1) as u8;
            if self.nbits % 8 == 0 {
                self.buf.push(0);
            }
            let idx = self.nbits / 8;
            self.buf[idx] |= bit << (self.nbits % 8);
            self.nbits += 1;
        }
    }
    /// Huffman code: MSB of the code first.
    pub fn code(&mut self, code: u32, n: u32) {
        for i in (0..n).rev() {
            self.bits((code >> i) & 1, 1);
        }
    }
    pub fn align(&mut self) {
        while self.nbits % 8 != 0 {
            self.bits(0, 1);
        }
    }
    pub fn raw(&mut self, b: &[u8]) {
        self.align();
        self.buf.extend_from_slice(b);
        self.nbits += b.len() * 8;
    }
    /// Trailing zero bytes so that `cp_peak_bits(s,16)` on the final symbol
    /// always has bits available (the C asserts `bits_left > 0`).
    pub fn pad(&mut self, n: usize) {
        self.align();
        for _ in 0..n {
            self.buf.push(0);
            self.nbits += 8;
        }
    }
    pub fn finish(mut self, pad: usize) -> Vec<u8> {
        self.pad(pad);
        self.buf
    }
}

/// Canonical DEFLATE code assignment - identical to `cp_build`.
pub fn canonical(lens: &[u8]) -> Vec<u32> {
    let mut counts = [0u32; 17];
    for &l in lens {
        counts[l as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 17];
    for n in 1..17 {
        next[n] = (next[n - 1] + counts[n - 1]) << 1;
    }
    let mut out = vec![0u32; lens.len()];
    for (i, &l) in lens.iter().enumerate() {
        if l != 0 {
            out[i] = next[l as usize];
            next[l as usize] += 1;
        }
    }
    out
}

#[derive(Clone)]
pub struct Huff {
    pub lens: Vec<u8>,
    pub codes: Vec<u32>,
}

impl Huff {
    pub fn new(lens: Vec<u8>) -> Huff {
        let codes = canonical(&lens);
        Huff { lens, codes }
    }
    pub fn put(&self, w: &mut BitWriter, sym: usize) {
        let l = self.lens[sym];
        assert!(l != 0, "symbol {sym} has no code");
        w.code(self.codes[sym], l as u32);
    }
    pub fn has(&self, sym: usize) -> bool {
        sym < self.lens.len() && self.lens[sym] != 0
    }
}

pub fn fixed_lit() -> Huff {
    Huff::new(FIXED_TABLE[..288].to_vec())
}
pub fn fixed_dst() -> Huff {
    Huff::new(FIXED_TABLE[288..].to_vec())
}

/// One emitted DEFLATE token.
#[derive(Clone, Debug)]
pub enum Tok {
    Lit(u8),
    /// raw length/distance symbol pair with explicit extra-bit payloads
    Raw {
        lsym: usize,
        lextra: u32,
        dsym: usize,
        dextra: u32,
    },
    End,
}

pub fn len_to_sym(len: u32) -> (usize, u32) {
    for s in (0..29).rev() {
        let base = LEN_BASE[s];
        if len >= base && len < base + (1u32 << LEN_EXTRA_BITS[s]) {
            return (257 + s, len - base);
        }
    }
    panic!("bad length {len}");
}

pub fn dist_to_sym(d: u32) -> (usize, u32) {
    for s in (0..30).rev() {
        let base = DIST_BASE[s];
        if d >= base && d < base + (1u32 << DIST_EXTRA_BITS[s]) {
            return (s, d - base);
        }
    }
    panic!("bad distance {d}");
}

pub fn match_tok(len: u32, dist: u32) -> Tok {
    let (lsym, lextra) = len_to_sym(len);
    let (dsym, dextra) = dist_to_sym(dist);
    Tok::Raw {
        lsym,
        lextra,
        dsym,
        dextra,
    }
}

/// Emit tokens with the supplied literal/distance trees.
pub fn emit_tokens(w: &mut BitWriter, lit: &Huff, dst: &Huff, toks: &[Tok]) {
    for t in toks {
        match *t {
            Tok::Lit(b) => lit.put(w, b as usize),
            Tok::End => lit.put(w, 256),
            Tok::Raw {
                lsym,
                lextra,
                dsym,
                dextra,
            } => {
                lit.put(w, lsym);
                let ls = lsym - 257;
                w.bits(lextra, LEN_EXTRA_BITS[ls] as u32);
                dst.put(w, dsym);
                w.bits(dextra, DIST_EXTRA_BITS[dsym] as u32);
            }
        }
    }
}

/// Reference simulation of `cp_block`, used only to size the output buffer.
pub fn simulate(prefix: &[u8], toks: &[Tok]) -> Vec<u8> {
    let mut out = prefix.to_vec();
    for t in toks {
        match *t {
            Tok::Lit(b) => out.push(b),
            Tok::End => {}
            Tok::Raw {
                lsym,
                lextra,
                dsym,
                dextra,
            } => {
                let ls = lsym - 257;
                let length = (lextra + LEN_BASE[ls]) as usize;
                let dist = (dextra + DIST_BASE[dsym]) as usize;
                for _ in 0..length {
                    let v = out[out.len() - dist];
                    out.push(v);
                }
            }
        }
    }
    out
}

pub fn fixed_block(w: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
    w.bits(bfinal as u32, 1);
    w.bits(1, 2);
    let lit = fixed_lit();
    let dst = fixed_dst();
    emit_tokens(w, &lit, &dst, toks);
    lit.put(w, 256);
}

pub fn stored_block(w: &mut BitWriter, bfinal: bool, payload: &[u8]) {
    w.bits(bfinal as u32, 1);
    w.bits(0, 2);
    w.align();
    let len = payload.len() as u16;
    w.bits(len as u32, 16);
    w.bits(!len as u32 & 0xFFFF, 16);
    w.raw(payload);
}

// ---------------------------------------------------------------------------
// Dynamic block
// ---------------------------------------------------------------------------

/// A code-length instruction for the HLIT+HDIST length list.
#[derive(Clone, Copy, Debug)]
pub enum CL {
    /// literal code length 0..=15
    Lit(u8),
    /// opcode 16: copy previous, `n` in 3..=6
    Rep(u32),
    /// opcode 17: zeros, `n` in 3..=10
    Z3(u32),
    /// opcode 18: zeros, `n` in 11..=138
    Z11(u32),
}

pub struct Dynamic {
    pub nlit: usize,
    pub ndst: usize,
    pub nlen: usize,
    pub lenlens: [u8; 19],
    pub prog: Vec<CL>,
}

/// Expand a `CL` program to the flat code-length array the C ends up with.
pub fn expand(prog: &[CL]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    for c in prog {
        match *c {
            CL::Lit(l) => v.push(l),
            CL::Rep(n) => {
                for _ in 0..n {
                    let prev = *v.last().unwrap();
                    v.push(prev);
                }
            }
            CL::Z3(n) | CL::Z11(n) => {
                for _ in 0..n {
                    v.push(0);
                }
            }
        }
    }
    v
}

impl Dynamic {
    pub fn write(&self, w: &mut BitWriter, bfinal: bool, toks: &[Tok]) {
        self.write_ord(w, bfinal, toks, &PERMUTATION_ORDER)
    }

    pub fn write_ord(
        &self,
        w: &mut BitWriter,
        bfinal: bool,
        toks: &[Tok],
        order: &[u8; 19],
    ) {
        w.bits(bfinal as u32, 1);
        w.bits(2, 2);
        w.bits((self.nlit - 257) as u32, 5);
        w.bits((self.ndst - 1) as u32, 5);
        w.bits((self.nlen - 4) as u32, 4);
        for i in 0..self.nlen {
            w.bits(self.lenlens[order[i] as usize] as u32, 3);
        }
        let clh = Huff::new(self.lenlens.to_vec());
        for c in &self.prog {
            match *c {
                CL::Lit(l) => clh.put(w, l as usize),
                CL::Rep(n) => {
                    clh.put(w, 16);
                    w.bits(n - 3, 2);
                }
                CL::Z3(n) => {
                    clh.put(w, 17);
                    w.bits(n - 3, 3);
                }
                CL::Z11(n) => {
                    clh.put(w, 18);
                    w.bits(n - 11, 7);
                }
            }
        }
        let flat = expand(&self.prog);
        assert!(
            flat.len() >= self.nlit + self.ndst,
            "program yields {} lengths, need {}",
            flat.len(),
            self.nlit + self.ndst
        );
        let lit = Huff::new(flat[..self.nlit].to_vec());
        let dst = Huff::new(flat[self.nlit..self.nlit + self.ndst].to_vec());
        emit_tokens(w, &lit, &dst, toks);
        lit.put(w, 256);
    }
}

/// Build a `CL` program that produces exactly `lens`, using only `CL::Lit`.
pub fn prog_literal(lens: &[u8]) -> Vec<CL> {
    lens.iter().map(|&l| CL::Lit(l)).collect()
}

/// Code lengths for the 19 code-length symbols, given which of them are used.
pub fn lenlens_for(used: &[usize]) -> [u8; 19] {
    // Give every used symbol the same length ceil(log2(n)) -- a complete code
    // requires the count to be a power of two, so pad with extra symbols.
    let mut set: Vec<usize> = used.to_vec();
    set.sort_unstable();
    set.dedup();
    let mut bits = 1u32;
    while (1usize << bits) < set.len() {
        bits += 1;
    }
    let target = 1usize << bits;
    // pad with unused symbols so the code is complete
    let mut i = 0usize;
    while set.len() < target && i < 19 {
        if !set.contains(&i) {
            set.push(i);
        }
        i += 1;
    }
    set.sort_unstable();
    let mut out = [0u8; 19];
    for s in set {
        out[s] = bits as u8;
    }
    out
}

/// Minimal complete Huffman lengths over `n` symbols (all `log2(n)` when `n`
/// is a power of two).  Returns lengths for indices `0..n`.
pub fn flat_lens(n: usize) -> Vec<u8> {
    assert!(n >= 1);
    if n == 1 {
        return vec![1];
    }
    let mut bits = 1u32;
    while (1usize << bits) < n {
        bits += 1;
    }
    if (1usize << bits) == n {
        return vec![bits as u8; n];
    }
    // n is not a power of two: build a complete code by splitting.
    // Standard approach: k = n - 2^(bits-1) symbols get `bits`, the rest
    // get `bits-1`.  (Kraft: k*2^-bits + (n-k)*2^-(bits-1) == 1)
    let half = 1usize << (bits - 1);
    let k = 2 * (n - half);
    let mut v = vec![bits as u8; k];
    v.extend(std::iter::repeat(bits as u8 - 1).take(n - k));
    v
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64)
// ---------------------------------------------------------------------------

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 17) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}
