//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every
//! call goes through the dynamic export, never through the Rust crate
//! directly. Each case is executed inside a `fork()`ed child so that the
//! undefined-behaviour cases the C accepts (null derefs, one-byte-before
//! writes) can be *compared* instead of killing the test process.

#![allow(dead_code, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI mirrors
// ---------------------------------------------------------------------------

/// Mirror of `os_data` from `c_src/include/lib.h` (9 `char *` in order:
/// name, version, major, minor, codename, platform, build, uname, arch).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OsData {
    pub f: [*mut c_char; 9],
}

pub const FIELD_NAMES: [&str; 9] = [
    "os_name",
    "os_version",
    "os_major",
    "os_minor",
    "os_codename",
    "os_platform",
    "os_build",
    "os_uname",
    "os_arch",
];

/// Mirror of glibc `regmatch_t` (`regoff_t` == `int` without
/// `_REGEX_LARGE_OFFSETS`; verified `sizeof(regmatch_t) == 8`).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegMatch {
    pub rm_so: c_int,
    pub rm_eo: c_int,
}

pub const SENTINEL: RegMatch = RegMatch {
    rm_so: -7777,
    rm_eo: -8888,
};

pub type ParseFn = unsafe extern "C" fn(*mut c_char, *mut OsData);
pub type ArchFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
pub type RegexFn =
    unsafe extern "C" fn(*const c_char, *const c_char, usize, *mut RegMatch) -> c_int;

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct pollfd {
    fd: c_int,
    events: i16,
    revents: i16,
}

const POLLIN: i16 = 0x001;

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn poll(fds: *mut pollfd, nfds: u64, timeout: c_int) -> c_int;
    fn malloc(n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: Library,
    pub arch: ArchFn,
    pub regexec: RegexFn,
    pub parse: ParseFn,
}

impl Lib {
    fn open(name: &'static str, path: &PathBuf) -> Lib {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
            let arch: Symbol<ArchFn> = lib
                .get(b"get_os_arch\0")
                .unwrap_or_else(|e| panic!("{} missing get_os_arch: {e}", path.display()));
            let regexec: Symbol<RegexFn> = lib
                .get(b"w_regexec\0")
                .unwrap_or_else(|e| panic!("{} missing w_regexec: {e}", path.display()));
            let parse: Symbol<ParseFn> = lib
                .get(b"parse_uname_string\0")
                .unwrap_or_else(|e| panic!("{} missing parse_uname_string: {e}", path.display()));
            let (a, r, p) = (*arch, *regexec, *parse);
            Lib {
                name,
                _lib: lib,
                arch: a,
                regexec: r,
                parse: p,
            }
        }
    }
}

pub fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

pub fn rust_lib_path() -> PathBuf {
    // current_exe() == <target>/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("deps dir has a parent")
        .to_path_buf();
    profile_dir.join("libdriver.so")
}

/// Opens both libraries. Panics with an actionable message if either is absent.
pub fn open_pair() -> (Lib, Lib) {
    let cp = c_lib_path();
    assert!(
        cp.exists(),
        "C shared library not built: {}\nrun: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        cp.display()
    );
    let rp = rust_lib_path();
    assert!(
        rp.exists(),
        "Rust cdylib not built: {}\nrun: cargo build (in translation/)",
        rp.display()
    );
    // `cargo test` does not necessarily rebuild a `cdylib`-only lib target, so
    // guard against silently testing a stale `.so`.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("lib.rs");
    if let (Ok(sm), Ok(lm)) = (
        std::fs::metadata(&src).and_then(|m| m.modified()),
        std::fs::metadata(&rp).and_then(|m| m.modified()),
    ) {
        assert!(
            lm >= sm,
            "STALE Rust cdylib: {} is older than {}\nrun `cargo build` before `cargo test`",
            rp.display(),
            src.display()
        );
    }
    (Lib::open("C", &cp), Lib::open("RUST", &rp))
}

// ---------------------------------------------------------------------------
// fork() based capture
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
pub struct Outcome {
    pub status: c_int,
    pub data: Vec<u8>,
    pub err: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            fmt,
            "Outcome {{ status: {:#x} ({}), data: {} bytes, stderr: {:?} }}",
            self.status,
            describe_status(self.status),
            self.data.len(),
            String::from_utf8_lossy(&self.err),
        )
    }
}

pub fn describe_status(status: c_int) -> String {
    if status & 0x7f == 0 {
        format!("exited {}", (status >> 8) & 0xff)
    } else {
        format!("signal {}", status & 0x7f)
    }
}

/// Runs `f` in a forked child. Everything `f` appends to its buffer comes back
/// as `Outcome::data`; the child's `stderr` (fd 2) comes back as
/// `Outcome::err`; the child's wait status (including a fatal signal) comes
/// back as `Outcome::status`.
pub fn fork_run(f: impl FnOnce(&mut Vec<u8>)) -> Outcome {
    let mut dfd = [0 as c_int; 2];
    let mut efd = [0 as c_int; 2];
    unsafe {
        assert_eq!(pipe(dfd.as_mut_ptr()), 0, "pipe() failed");
        assert_eq!(pipe(efd.as_mut_ptr()), 0, "pipe() failed");
    }

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child ----
        unsafe {
            close(dfd[0]);
            close(efd[0]);
            dup2(efd[1], 2);
            close(efd[1]);

            let mut out: Vec<u8> = Vec::with_capacity(1 << 16);
            f(&mut out);

            let mut off = 0usize;
            while off < out.len() {
                let n = write(
                    dfd[1],
                    out.as_ptr().add(off) as *const c_void,
                    out.len() - off,
                );
                if n <= 0 {
                    break;
                }
                off += n as usize;
            }
            close(dfd[1]);
            _exit(0);
        }
    }

    // ---- parent ----
    unsafe {
        close(dfd[1]);
        close(efd[1]);
    }

    let mut data = Vec::new();
    let mut err = Vec::new();
    let mut open_d = true;
    let mut open_e = true;
    let mut chunk = [0u8; 8192];

    while open_d || open_e {
        let mut fds = [
            pollfd {
                fd: if open_d { dfd[0] } else { -1 },
                events: POLLIN,
                revents: 0,
            },
            pollfd {
                fd: if open_e { efd[0] } else { -1 },
                events: POLLIN,
                revents: 0,
            },
        ];
        let rc = unsafe { poll(fds.as_mut_ptr(), 2, 60_000) };
        if rc < 0 {
            continue; // EINTR
        }
        if rc == 0 {
            break; // 60s timeout guard
        }
        for (i, pf) in fds.iter().enumerate() {
            if pf.fd < 0 || pf.revents == 0 {
                continue;
            }
            let n = unsafe {
                read(
                    pf.fd,
                    chunk.as_mut_ptr() as *mut c_void,
                    chunk.len(),
                )
            };
            if n > 0 {
                let s = &chunk[..n as usize];
                if i == 0 {
                    data.extend_from_slice(s);
                } else {
                    err.extend_from_slice(s);
                }
            } else if i == 0 {
                open_d = false;
            } else {
                open_e = false;
            }
        }
    }

    unsafe {
        close(dfd[0]);
        close(efd[0]);
    }

    let mut status: c_int = 0;
    unsafe {
        waitpid(pid, &mut status, 0);
    }

    Outcome { status, data, err }
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

pub fn ser_cstr(out: &mut Vec<u8>, p: *const c_char) {
    if p.is_null() {
        out.push(b'N');
    } else {
        let n = unsafe { strlen(p) };
        out.push(b'S');
        out.extend_from_slice(&(n as u32).to_le_bytes());
        out.extend_from_slice(unsafe { std::slice::from_raw_parts(p as *const u8, n) });
    }
    out.push(b'|');
}

pub fn ser_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
    out.push(b'|');
}

// ---------------------------------------------------------------------------
// Input buffers with guard padding on both sides
// ---------------------------------------------------------------------------

pub const PAD: usize = 8;

pub struct Buf {
    pub base: *mut u8,
    pub total: usize,
}

impl Buf {
    pub fn new(input: &[u8]) -> Buf {
        assert!(
            !input.contains(&0),
            "test inputs must not contain embedded NUL"
        );
        let total = PAD + input.len() + 1 + PAD;
        unsafe {
            let base = malloc(total) as *mut u8;
            assert!(!base.is_null());
            std::ptr::write_bytes(base, 0xAA, total);
            std::ptr::copy_nonoverlapping(input.as_ptr(), base.add(PAD), input.len());
            *base.add(PAD + input.len()) = 0;
            Buf { base, total }
        }
    }

    pub fn ptr(&self) -> *mut c_char {
        unsafe { self.base.add(PAD) as *mut c_char }
    }

    pub fn region(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.base, self.total) }
    }
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Case {
    /// `get_os_arch(input)`; `None` == NULL pointer.
    Arch { input: Option<Vec<u8>> },
    /// `w_regexec(pat, subj, nmatch, pmatch)`.
    /// `arr = None` == NULL `pmatch`; `Some(n)` == n-entry sentinel-filled array.
    Regex {
        pat: Option<Vec<u8>>,
        subj: Option<Vec<u8>>,
        nmatch: usize,
        arr: Option<usize>,
    },
    /// `parse_uname_string(input, osd)`.
    Parse {
        input: Option<Vec<u8>>,
        osd_null: bool,
        prepopulate: bool,
        times: usize,
    },
}

impl Case {
    pub fn arch(s: &[u8]) -> Case {
        Case::Arch {
            input: Some(s.to_vec()),
        }
    }
    pub fn regex(pat: &[u8], subj: &[u8], nmatch: usize, arr: usize) -> Case {
        Case::Regex {
            pat: Some(pat.to_vec()),
            subj: Some(subj.to_vec()),
            nmatch,
            arr: Some(arr),
        }
    }
    pub fn parse(s: &[u8]) -> Case {
        Case::Parse {
            input: Some(s.to_vec()),
            osd_null: false,
            prepopulate: false,
            times: 1,
        }
    }
    pub fn parse_n(s: &[u8], times: usize) -> Case {
        Case::Parse {
            input: Some(s.to_vec()),
            osd_null: false,
            prepopulate: false,
            times,
        }
    }
    pub fn parse_prepop(s: &[u8]) -> Case {
        Case::Parse {
            input: Some(s.to_vec()),
            osd_null: false,
            prepopulate: true,
            times: 1,
        }
    }
}

/// Short, printable description used in failure messages.
pub fn describe(case: &Case) -> String {
    fn show(o: &Option<Vec<u8>>) -> String {
        match o {
            None => "NULL".to_string(),
            Some(v) if v.len() > 120 => format!(
                "{:?}..(len {})",
                String::from_utf8_lossy(&v[..120]),
                v.len()
            ),
            Some(v) => format!("{:?}", String::from_utf8_lossy(v)),
        }
    }
    match case {
        Case::Arch { input } => format!("get_os_arch({})", show(input)),
        Case::Regex {
            pat,
            subj,
            nmatch,
            arr,
        } => format!(
            "w_regexec(pat={}, subj={}, nmatch={}, pmatch={})",
            show(pat),
            show(subj),
            nmatch,
            match arr {
                None => "NULL".to_string(),
                Some(n) => format!("[{n}]"),
            }
        ),
        Case::Parse {
            input,
            osd_null,
            prepopulate,
            times,
        } => format!(
            "parse_uname_string({}, osd={}{}){}",
            show(input),
            if *osd_null { "NULL" } else { "&osd" },
            if *prepopulate { ", prepopulated" } else { "" },
            if *times == 1 {
                String::new()
            } else {
                format!(" x{times}")
            }
        ),
    }
}

static PREPOP: &[u8] = b"PREPOPULATED-SENTINEL\0";

/// Executes one case against `lib`, appending a canonical byte encoding of
/// every observable output to `out`.
pub fn run_case(lib: &Lib, case: &Case, out: &mut Vec<u8>) {
    unsafe {
        match case {
            Case::Arch { input } => {
                out.extend_from_slice(b"ARCH:");
                let ret = match input {
                    None => (lib.arch)(std::ptr::null_mut()),
                    Some(v) => {
                        let buf = Buf::new(v);
                        let r = (lib.arch)(buf.ptr());
                        // input must not be modified by get_os_arch
                        ser_bytes(out, buf.region());
                        r
                    }
                };
                ser_cstr(out, ret);
            }
            Case::Regex {
                pat,
                subj,
                nmatch,
                arr,
            } => {
                out.extend_from_slice(b"RGX:");
                let pbuf = pat.as_ref().map(|v| Buf::new(v));
                let sbuf = subj.as_ref().map(|v| Buf::new(v));
                let pp = pbuf
                    .as_ref()
                    .map(|b| b.ptr() as *const c_char)
                    .unwrap_or(std::ptr::null());
                let sp = sbuf
                    .as_ref()
                    .map(|b| b.ptr() as *const c_char)
                    .unwrap_or(std::ptr::null());
                let mut m: Vec<RegMatch> = match arr {
                    None => Vec::new(),
                    Some(n) => vec![SENTINEL; *n],
                };
                let mp = match arr {
                    None => std::ptr::null_mut(),
                    Some(_) => m.as_mut_ptr(),
                };
                let ret = (lib.regexec)(pp, sp, *nmatch, mp);
                out.extend_from_slice(&ret.to_le_bytes());
                out.push(b'|');
                out.extend_from_slice(&(m.len() as u32).to_le_bytes());
                for e in &m {
                    out.extend_from_slice(&e.rm_so.to_le_bytes());
                    out.extend_from_slice(&e.rm_eo.to_le_bytes());
                }
                out.push(b'|');
            }
            Case::Parse {
                input,
                osd_null,
                prepopulate,
                times,
            } => {
                out.extend_from_slice(b"PRS:");
                let mut osd = OsData {
                    f: [std::ptr::null_mut(); 9],
                };
                if *prepopulate {
                    for i in 0..9 {
                        osd.f[i] = PREPOP.as_ptr() as *mut c_char;
                    }
                }
                let osdp = if *osd_null {
                    std::ptr::null_mut()
                } else {
                    &mut osd as *mut OsData
                };
                match input {
                    None => {
                        for _ in 0..*times {
                            (lib.parse)(std::ptr::null_mut(), osdp);
                        }
                    }
                    Some(v) => {
                        let buf = Buf::new(v);
                        for _ in 0..*times {
                            (lib.parse)(buf.ptr(), osdp);
                        }
                        ser_bytes(out, buf.region());
                    }
                }
                for i in 0..9 {
                    out.extend_from_slice(FIELD_NAMES[i].as_bytes());
                    out.push(b'=');
                    ser_cstr(out, osd.f[i]);
                }
            }
        }
        out.push(b'\n');
    }
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

fn run_batch(lib: &Lib, cases: &[Case]) -> Outcome {
    fork_run(|out| {
        for (i, c) in cases.iter().enumerate() {
            out.extend_from_slice(format!("#{i} ").as_bytes());
            run_case(lib, c, out);
        }
        out.extend_from_slice(format!("[DONE {}]", cases.len()).as_bytes());
    })
}

fn run_single(lib: &Lib, case: &Case) -> Outcome {
    fork_run(|out| run_case(lib, case, out))
}

/// Runs one case in one child and returns the raw outcome, so a test can
/// assert on the *specific* termination (e.g. "this really did SIGSEGV")
/// instead of only "C and Rust agree".
pub fn probe(lib: &Lib, case: &Case) -> Outcome {
    run_single(lib, case)
}

/// `Some(signum)` if the child died from a signal, `None` if it exited.
pub fn signal_of(status: c_int) -> Option<c_int> {
    if status & 0x7f == 0 {
        None
    } else {
        Some(status & 0x7f)
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn render(o: &Outcome) -> String {
    format!(
        "status={} ({})\n  data(len {})={}\n  data(lossy)={:?}\n  stderr={:?}",
        o.status,
        describe_status(o.status),
        o.data.len(),
        hex(&o.data),
        String::from_utf8_lossy(&o.data),
        String::from_utf8_lossy(&o.err),
    )
}

/// Runs every case in both libraries and asserts identical observable output.
///
/// Cases are first run as a single batched child per library (fast); on any
/// divergence the offending case is localised by re-running each case in its
/// own child, so the failure message names the exact input.
pub fn assert_same(c: &Lib, r: &Lib, cases: &[Case], label: &str) {
    assert!(!cases.is_empty(), "{label}: no cases generated");
    let oc = run_batch(c, cases);
    let or = run_batch(r, cases);
    if oc == or {
        // Guard against a "both crashed on case 0" false pass: a batched run
        // must complete cleanly and emit one line per case.
        assert_eq!(
            oc.status & 0x7f,
            0,
            "{label}: batched child died ({}) — use assert_same_isolated for UB rows\n{}",
            describe_status(oc.status),
            render(&oc)
        );
        assert!(
            oc.data.ends_with(format!("[DONE {}]", cases.len()).as_bytes()),
            "{label}: batched child did not run all {} cases to completion\n{}",
            cases.len(),
            render(&oc)
        );
        eprintln!("[ok] {label}: {} cases matched", cases.len());
        return;
    }

    for case in cases.iter() {
        let a = run_single(c, case);
        let b = run_single(r, case);
        if a != b {
            panic!(
                "\n=== DIVERGENCE in {label} ===\ncase: {}\n--- C ---\n  {}\n--- RUST ---\n  {}\n",
                describe(case),
                render(&a),
                render(&b),
            );
        }
    }

    panic!(
        "\n=== DIVERGENCE in {label} (batch only; no single case reproduces) ===\n\
         --- C ---\n  {}\n--- RUST ---\n  {}\n",
        render(&oc),
        render(&or)
    );
}

/// Same as [`assert_same`] but always one child per case (used for the
/// undefined-behaviour / crashing rows so one case cannot mask another).
pub fn assert_same_isolated(c: &Lib, r: &Lib, cases: &[Case], label: &str) {
    assert!(!cases.is_empty(), "{label}: no cases generated");
    for case in cases.iter() {
        let a = run_single(c, case);
        let b = run_single(r, case);
        assert!(
            a == b,
            "\n=== DIVERGENCE in {label} ===\ncase: {}\n--- C ---\n  {}\n--- RUST ---\n  {}\n",
            describe(case),
            render(&a),
            render(&b),
        );
    }
    eprintln!("[ok] {label}: {} isolated cases matched", cases.len());
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) + generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Random NUL-free bytes over a mixed alphabet (ASCII printable, digits,
    /// separators the parser cares about, whitespace, and high bytes).
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        const ALPHA: &[u8] =
            b"abcXYZ0123456789 .:|[]()-_/\t\n+*?\\^$e{}\x80\xff\xc3\xa9\x7f\x01";
        (0..len).map(|_| *self.pick(ALPHA)).collect()
    }

    /// Random NUL-free digits-and-dots version string.
    pub fn version(&mut self, comps: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..comps {
            if i > 0 {
                v.push(b'.');
            }
            let digits = self.range(1, 4);
            for _ in 0..digits {
                v.push(b'0' + self.below(10) as u8);
            }
        }
        v
    }

    /// Random word made of letters (never contains a marker substring).
    pub fn word(&mut self, len: usize) -> Vec<u8> {
        const A: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        (0..len).map(|_| *self.pick(A)).collect()
    }

    // Length-randomising wrappers (avoid `rng.f(rng.range(..))` borrow issues).
    pub fn word_between(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.word(n)
    }
    pub fn bytes_between(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.bytes(n)
    }
    pub fn version_between(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.version(n)
    }
    /// Mixed bytes drawn from high bytes, tabs, newlines and printable ASCII.
    pub fn noise_between(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        (0..n)
            .map(|_| match self.below(4) {
                0 => 0x80 + self.below(0x80) as u8,
                1 => b'\t',
                2 => b'\n',
                _ => 0x21 + self.below(0x5e) as u8,
            })
            .collect()
    }
}

pub const ARCHS: [&[u8]; 12] = [
    b"x86_64", b"i386", b"i686", b"sparc", b"amd64", b"i86pc", b"ia64", b"AIX", b"armv6",
    b"armv7", b"aarch64", b"arm64",
];

pub const SEED: u64 = 0x5EED_1234;
