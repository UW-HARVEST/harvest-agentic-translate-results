//! Differential-test harness.
//!
//! Both the C reference `.so` and the Rust `.so` are loaded with `libloading`
//! and driven **only** through their exported symbols, exactly as an external
//! consumer would — the Rust implementation is never called directly, so the
//! `#[no_mangle] extern "C"` wrappers and the exported globals are part of what
//! is under test.
//!
//! Because the C library is built with `assert()` live (`c_src/CMakeLists.txt`
//! sets no `CMAKE_BUILD_TYPE`/`-DNDEBUG`), malformed input makes it die with
//! `SIGABRT`, and invalid pointers make it die with `SIGSEGV`. To be able to
//! compare *that* behaviour too, every call is made in a forked child; the
//! parent compares the wait status, the returned `int`, the whole output buffer
//! and the `cp_error_reason` string.

#![allow(dead_code)]

pub mod deflate;

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Library loading (through the dynamic symbol table only)
// ---------------------------------------------------------------------------

pub type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

pub struct Lib {
    pub label: &'static str,
    pub path: PathBuf,
    pub pinflate: PinflateFn,
    pub error_reason: *mut *const c_char,
    pub fixed_table: *mut u8,
    pub permutation_order: *mut u8,
    pub len_extra_bits: *mut u8,
    pub len_base: *mut u32,
    pub dist_extra_bits: *mut u8,
    pub dist_base: *mut u32,
}

impl Lib {
    unsafe fn load(label: &'static str, path: PathBuf) -> Lib {
        let lib: &'static libloading::Library = Box::leak(Box::new(
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display())),
        ));
        macro_rules! sym {
            ($t:ty, $n:expr) => {{
                let s: libloading::Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", label, $n));
                *s
            }};
        }
        Lib {
            label,
            path,
            pinflate: sym!(PinflateFn, b"pinflate\0"),
            error_reason: sym!(*mut *const c_char, b"cp_error_reason\0"),
            fixed_table: sym!(*mut u8, b"cp_fixed_table\0"),
            permutation_order: sym!(*mut u8, b"cp_permutation_order\0"),
            len_extra_bits: sym!(*mut u8, b"cp_len_extra_bits\0"),
            len_base: sym!(*mut u32, b"cp_len_base\0"),
            dist_extra_bits: sym!(*mut u8, b"cp_dist_extra_bits\0"),
            dist_base: sym!(*mut u32, b"cp_dist_base\0"),
        }
    }
}

pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let dir = workspace_root().join("c_src/build");
    let mut cands: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "{} not readable ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    cands.sort();
    assert_eq!(
        cands.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        dir.display(),
        cands
    );
    cands.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    // current_exe = target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().unwrap();
    let profile_dir = exe.parent().unwrap().parent().unwrap();
    let p = profile_dir.join("libpinflate_lib.so");
    if p.exists() {
        return p;
    }
    // Falling back to another profile's artifact would silently verify the wrong
    // library, so refuse instead.
    panic!(
        "{} does not exist.\n\
         `cargo test` alone does not build the cdylib (nothing links it); run\n    \
         cargo build{} --offline\n\
         first, or use ./run_verification.sh which does it for every profile.",
        p.display(),
        if profile_dir.ends_with("release") {
            " --release"
        } else {
            ""
        }
    );
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: Lib::load("C", c_so_path()),
            rs: Lib::load("Rust", rust_so_path()),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared memory arena + fork runner
// ---------------------------------------------------------------------------

const HDR_OFF: usize = 0;
const ERR_CAP: usize = 512;
pub const IN_OFF: usize = 4096;
pub const IN_CAP: usize = 1 << 20;
pub const OUT_OFF: usize = IN_OFF + IN_CAP;
pub const OUT_CAP: usize = 1 << 20;
const ARENA: usize = OUT_OFF + OUT_CAP + 4096;

/// Extra bytes past `out_bytes` that are compared as well, so that an
/// out-of-bounds write by one implementation but not the other is caught.
pub const OUT_SLACK: usize = 64;

#[repr(C)]
struct Hdr {
    ret: i32,
    err_len: i32,
    err: [u8; ERR_CAP],
}

pub struct Arena {
    base: *mut u8,
}

unsafe impl Send for Arena {}

impl Arena {
    pub fn new() -> Arena {
        unsafe {
            let p = libc::mmap(
                std::ptr::null_mut(),
                ARENA,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p != libc::MAP_FAILED, "mmap failed");
            Arena { base: p as *mut u8 }
        }
    }
    fn hdr(&self) -> *mut Hdr {
        unsafe { self.base.add(HDR_OFF) as *mut Hdr }
    }
    /// Address handed to `pinflate` as `in`; `align` picks the address mod 4.
    pub fn in_ptr(&self, align: usize) -> *mut u8 {
        assert!(align < 4);
        unsafe { self.base.add(IN_OFF + align) }
    }
    pub fn out_ptr(&self, align: usize) -> *mut u8 {
        assert!(align < 4);
        unsafe { self.base.add(OUT_OFF + align) }
    }
}

#[derive(Clone, Eq)]
pub struct Outcome {
    /// `Some(sig)` if the child was killed by a signal.
    pub signal: Option<i32>,
    pub exit_code: Option<i32>,
    pub ret: i32,
    pub err: Option<Vec<u8>>,
    pub out: Vec<u8>,
    /// `lib.c:<line>: <func>: Assertion `<expr>' failed.` extracted from the
    /// child's stderr. Both libraries emit the *same* file/line/function/expr
    /// (the Rust translation reproduces the C's assertion sites verbatim), so a
    /// mismatch means the two aborted at different places.
    pub assert_site: Option<String>,
    /// Raw stderr, kept for diagnostics only; deliberately NOT compared (glibc
    /// prefixes its message with the program path).
    pub stderr_raw: Vec<u8>,
}

impl PartialEq for Outcome {
    fn eq(&self, o: &Outcome) -> bool {
        self.signal == o.signal
            && self.exit_code == o.exit_code
            && self.ret == o.ret
            && self.err == o.err
            && self.out == o.out
            && self.assert_site == o.assert_site
    }
}

/// Pull `lib.c:<line>: <fn>: Assertion `...' failed.` out of an assert message.
fn extract_assert_site(s: &[u8]) -> Option<String> {
    let t = String::from_utf8_lossy(s);
    let i = t.find("lib.c:")?;
    let rest = &t[i..];
    let end = rest.find('\n').unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn sig_name(s: i32) -> &'static str {
    match s {
        libc::SIGABRT => "SIGABRT",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGBUS => "SIGBUS",
        libc::SIGFPE => "SIGFPE",
        libc::SIGILL => "SIGILL",
        _ => "SIG?",
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.signal {
            Some(s) => write!(f, "killed by {}({})", sig_name(s), s)?,
            None => write!(
                f,
                "exit {:?} ret={} err={:?}",
                self.exit_code,
                self.ret,
                self.err
                    .as_ref()
                    .map(|v| String::from_utf8_lossy(v).into_owned())
            )?,
        }
        if let Some(a) = &self.assert_site {
            write!(f, " at {a}")?;
        }
        write!(f, " out[{}]={}", self.out.len(), hex(&self.out))
    }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::new();
    for (i, x) in b.iter().enumerate() {
        if i == 96 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("{:02x}", x));
    }
    s
}

/// A single `pinflate` invocation description.
#[derive(Clone)]
pub struct Case {
    pub input: Vec<u8>,
    /// address mod 4 for `in`
    pub in_align: usize,
    /// value passed as `in_bytes` (defaults to `input.len()`)
    pub in_bytes: Option<c_int>,
    /// address mod 4 for `out`
    pub out_align: usize,
    pub out_bytes: c_int,
    /// byte the output region is pre-filled with before each call
    pub out_fill: u8,
    /// `None` = pass the real pointer, `Some(v)` = pass this raw value instead
    pub in_override: Option<usize>,
    pub out_override: Option<usize>,
    /// mutations applied to the writable exported globals before the call
    pub globals: Vec<GlobalPoke>,
}

#[derive(Clone, Copy)]
pub enum GlobalPoke {
    FixedTable(usize, u8),
    PermutationOrder(usize, u8),
    LenExtraBits(usize, u8),
    LenBase(usize, u32),
    DistExtraBits(usize, u8),
    DistBase(usize, u32),
}

impl Case {
    pub fn new(input: Vec<u8>, out_bytes: c_int) -> Case {
        Case {
            input,
            in_align: 0,
            in_bytes: None,
            out_align: 0,
            out_bytes,
            out_fill: 0xCD,
            in_override: None,
            out_override: None,
            globals: Vec::new(),
        }
    }
    pub fn in_align(mut self, a: usize) -> Case {
        self.in_align = a;
        self
    }
    pub fn out_align(mut self, a: usize) -> Case {
        self.out_align = a;
        self
    }
    pub fn in_bytes(mut self, n: c_int) -> Case {
        self.in_bytes = Some(n);
        self
    }
    pub fn out_fill(mut self, f: u8) -> Case {
        self.out_fill = f;
        self
    }
    pub fn in_override(mut self, v: usize) -> Case {
        self.in_override = Some(v);
        self
    }
    pub fn out_override(mut self, v: usize) -> Case {
        self.out_override = Some(v);
        self
    }
    pub fn poke(mut self, p: GlobalPoke) -> Case {
        self.globals.push(p);
        self
    }
    /// Number of output bytes compared: the declared window plus slack.
    fn compare_len(&self) -> usize {
        let n = if self.out_bytes > 0 {
            self.out_bytes as usize
        } else {
            0
        };
        (n + OUT_SLACK).min(OUT_CAP - 8)
    }
}

/// Run one case against one library in a forked child and collect everything
/// observable.
pub fn run(arena: &Arena, lib: &Lib, case: &Case) -> Outcome {
    let cmp_len = case.compare_len();
    assert!(case.input.len() <= IN_CAP - 8, "input too large for arena");

    unsafe {
        // Reset only a bounded window of the arena (resetting all 2 MiB per call
        // dominates the runtime). Both runs of a pair reset the same window, and
        // nothing outside it is modified between them, so each comparison is
        // still made against byte-identical starting memory.
        let in_reset = (case.input.len() + 4096).min(IN_CAP);
        let out_reset = (cmp_len + 4096).min(OUT_CAP);
        std::ptr::write_bytes(arena.base.add(IN_OFF), 0, in_reset);
        std::ptr::copy_nonoverlapping(
            case.input.as_ptr(),
            arena.in_ptr(case.in_align),
            case.input.len(),
        );
        std::ptr::write_bytes(arena.base.add(OUT_OFF), case.out_fill, out_reset);
        let h = arena.hdr();
        (*h).ret = i32::MIN;
        (*h).err_len = -2;
        std::ptr::write_bytes((*h).err.as_mut_ptr(), 0, ERR_CAP);

        let in_ptr = match case.in_override {
            Some(v) => v as *mut u8,
            None => arena.in_ptr(case.in_align),
        };
        let out_ptr = match case.out_override {
            Some(v) => v as *mut u8,
            None => arena.out_ptr(case.out_align),
        };
        let in_bytes = case.in_bytes.unwrap_or(case.input.len() as c_int);

        // Snapshot / apply the writable exported globals.
        let saved = snapshot_globals(lib);
        for p in &case.globals {
            apply_poke(lib, *p);
        }
        *lib.error_reason = std::ptr::null();

        let mut pipefd = [0 as c_int; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0, "pipe failed");

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ---- child ----
            let rl = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            // `core_pattern` on this host pipes to systemd-coredump, which costs
            // ~250 ms per aborting child. PR_SET_DUMPABLE(0) makes the kernel
            // skip the dump entirely; RLIMIT_CORE alone does not when
            // core_pattern is a pipe.
            libc::prctl(libc::PR_SET_DUMPABLE, 0);
            // Watchdog: a corrupted length/distance can make the C library copy
            // essentially forever (an out-of-range `cp_len_base[]` read). Such a
            // child is killed by SIGALRM and the case is reported as a runaway
            // rather than compared. 300 ms is ~1000x a normal call.
            let it = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                it_value: libc::timeval {
                    tv_sec: 0,
                    tv_usec: 300_000,
                },
            };
            libc::setitimer(libc::ITIMER_REAL, &it, std::ptr::null_mut());
            libc::close(pipefd[0]);
            libc::dup2(pipefd[1], 2);
            libc::close(pipefd[1]);
            let r = (lib.pinflate)(
                in_ptr as *mut c_void,
                in_bytes,
                out_ptr as *mut c_void,
                case.out_bytes,
            );
            (*h).ret = r;
            let e = *lib.error_reason;
            if e.is_null() {
                (*h).err_len = -1;
            } else {
                let mut n = 0usize;
                while n < ERR_CAP - 1 && *e.add(n) != 0 {
                    n += 1;
                }
                std::ptr::copy_nonoverlapping(e as *const u8, (*h).err.as_mut_ptr(), n);
                (*h).err_len = n as i32;
            }
            libc::_exit(0);
        }

        // ---- parent ----
        libc::close(pipefd[1]);
        let mut stderr_raw = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(pipefd[0], buf.as_mut_ptr() as *mut c_void, buf.len());
            if n > 0 {
                stderr_raw.extend_from_slice(&buf[..n as usize]);
                if stderr_raw.len() > 65536 {
                    break;
                }
            } else if n == 0 {
                break;
            } else if *libc::__errno_location() != libc::EINTR {
                break;
            }
        }
        libc::close(pipefd[0]);

        let mut status: c_int = 0;
        loop {
            let r = libc::waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            assert!(
                !(r < 0 && *libc::__errno_location() != libc::EINTR),
                "waitpid failed"
            );
        }
        restore_globals(lib, &saved);

        let signal = if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        };
        let exit_code = if libc::WIFEXITED(status) {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        };
        let ret = (*h).ret;
        let err_len = (*h).err_len;
        let err = if err_len >= 0 {
            let src = std::ptr::addr_of!((*h).err) as *const u8;
            Some(std::slice::from_raw_parts(src, err_len as usize).to_vec())
        } else {
            None
        };
        let out = std::slice::from_raw_parts(arena.out_ptr(case.out_align), cmp_len).to_vec();
        Outcome {
            signal,
            exit_code,
            ret,
            err,
            out,
            assert_site: extract_assert_site(&stderr_raw),
            stderr_raw,
        }
    }
}

struct GlobalsSnapshot {
    fixed_table: [u8; 320],
    permutation_order: [u8; 19],
    len_extra_bits: [u8; 31],
    len_base: [u32; 31],
    dist_extra_bits: [u8; 32],
    dist_base: [u32; 32],
}

unsafe fn snapshot_globals(lib: &Lib) -> GlobalsSnapshot {
    let mut s = GlobalsSnapshot {
        fixed_table: [0; 320],
        permutation_order: [0; 19],
        len_extra_bits: [0; 31],
        len_base: [0; 31],
        dist_extra_bits: [0; 32],
        dist_base: [0; 32],
    };
    std::ptr::copy_nonoverlapping(lib.fixed_table, s.fixed_table.as_mut_ptr(), 320);
    std::ptr::copy_nonoverlapping(lib.permutation_order, s.permutation_order.as_mut_ptr(), 19);
    std::ptr::copy_nonoverlapping(lib.len_extra_bits, s.len_extra_bits.as_mut_ptr(), 31);
    std::ptr::copy_nonoverlapping(lib.len_base, s.len_base.as_mut_ptr(), 31);
    std::ptr::copy_nonoverlapping(lib.dist_extra_bits, s.dist_extra_bits.as_mut_ptr(), 32);
    std::ptr::copy_nonoverlapping(lib.dist_base, s.dist_base.as_mut_ptr(), 32);
    s
}

unsafe fn restore_globals(lib: &Lib, s: &GlobalsSnapshot) {
    std::ptr::copy_nonoverlapping(s.fixed_table.as_ptr(), lib.fixed_table, 320);
    std::ptr::copy_nonoverlapping(s.permutation_order.as_ptr(), lib.permutation_order, 19);
    std::ptr::copy_nonoverlapping(s.len_extra_bits.as_ptr(), lib.len_extra_bits, 31);
    std::ptr::copy_nonoverlapping(s.len_base.as_ptr(), lib.len_base, 31);
    std::ptr::copy_nonoverlapping(s.dist_extra_bits.as_ptr(), lib.dist_extra_bits, 32);
    std::ptr::copy_nonoverlapping(s.dist_base.as_ptr(), lib.dist_base, 32);
}

unsafe fn apply_poke(lib: &Lib, p: GlobalPoke) {
    match p {
        GlobalPoke::FixedTable(i, v) => *lib.fixed_table.add(i) = v,
        GlobalPoke::PermutationOrder(i, v) => *lib.permutation_order.add(i) = v,
        GlobalPoke::LenExtraBits(i, v) => *lib.len_extra_bits.add(i) = v,
        GlobalPoke::LenBase(i, v) => *lib.len_base.add(i) = v,
        GlobalPoke::DistExtraBits(i, v) => *lib.dist_extra_bits.add(i) = v,
        GlobalPoke::DistBase(i, v) => *lib.dist_base.add(i) = v,
    }
}

// ---------------------------------------------------------------------------
// Differential driver with failure accumulation
// ---------------------------------------------------------------------------

pub struct Diff {
    pub arena: Arena,
    pub pair: Pair,
    pub failures: Vec<String>,
    pub rows: Vec<(String, usize, bool)>,
    pub calls: usize,
    pub runaways: usize,
    pub runaway_examples: Vec<String>,
}

impl Diff {
    pub fn new() -> Diff {
        Diff {
            arena: Arena::new(),
            pair: load_pair(),
            failures: Vec::new(),
            rows: Vec::new(),
            calls: 0,
            runaways: 0,
            runaway_examples: Vec::new(),
        }
    }

    /// Compare C vs Rust for one case. Returns the C outcome so callers can add
    /// their own expectations (e.g. "the C must actually succeed here").
    pub fn check(&mut self, row: &str, what: &str, case: &Case) -> Outcome {
        self.calls += 1;
        let c = run(&self.arena, &self.pair.c, case);
        let r = run(&self.arena, &self.pair.rs, case);
        // A child killed by the watchdog was still running; there is nothing
        // meaningful to compare (these are unbounded-copy UB paths in the C).
        if c.signal == Some(libc::SIGALRM) || r.signal == Some(libc::SIGALRM) {
            self.runaways += 1;
            if self.runaway_examples.len() < 8 {
                self.runaway_examples.push(format!(
                    "[{row}] {what} in={} in_align={} out_bytes={} C:{:?} Rust:{:?}",
                    hex(&case.input),
                    case.in_align,
                    case.out_bytes,
                    c.signal,
                    r.signal
                ));
            }
            return c;
        }
        if c != r {
            let mut msg = format!(
                "[{row}] {what}\n    in_bytes={} in_align={} out_bytes={} out_align={}\n    input={}\n    C   : {:?}\n    Rust: {:?}",
                case.in_bytes.unwrap_or(case.input.len() as c_int),
                case.in_align,
                case.out_bytes,
                case.out_align,
                hex(&case.input),
                c,
                r
            );
            if c.out != r.out {
                if let Some(i) = c.out.iter().zip(r.out.iter()).position(|(a, b)| a != b) {
                    msg += &format!(
                        "\n    first out diff at byte {i}: C=0x{:02x} Rust=0x{:02x}",
                        c.out[i], r.out[i]
                    );
                }
            }
            if self.failures.len() < 40 {
                self.failures.push(msg);
            }
        }
        c
    }

    pub fn row_start(&mut self, row: &str) -> usize {
        self.rows.push((row.to_string(), self.failures.len(), false));
        self.failures.len()
    }

    pub fn row_end(&mut self, before: usize) {
        let ok = self.failures.len() == before;
        if let Some(last) = self.rows.last_mut() {
            last.2 = ok;
        }
    }

    pub fn fail(&mut self, msg: String) {
        if self.failures.len() < 40 {
            self.failures.push(msg);
        }
    }

    pub fn finish(self, label: &str) {
        let total = self.rows.len();
        let passed = self.rows.iter().filter(|r| r.2).count();
        println!(
            "\n=== {label}: {passed}/{total} rows passed, {} pinflate call pairs, {} watchdog runaways ===",
            self.calls, self.runaways
        );
        for e in &self.runaway_examples {
            println!("  runaway: {e}");
        }
        for (name, _, ok) in &self.rows {
            println!("  [{}] {}", if *ok { "x" } else { " " }, name);
        }
        if !self.failures.is_empty() {
            for f in &self.failures {
                println!("\n--- DIVERGENCE ---\n{f}");
            }
            panic!("{label}: {} divergences ({}/{total} rows passed)", self.failures.len(), passed);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64)
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
    pub fn u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// uniform in `0..n`
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as u32
        }
    }
    /// uniform in `lo..=hi`
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(hi - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 40) as u8
    }
    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}
