//! Shared differential-testing harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven exclusively through their exported symbols, so the `#[no_mangle]`
//! wrappers of the Rust crate are part of what is under test.
//!
//! `pinflate` happily walks off the end of buffers and its `assert()`s are
//! live (the CMake project compiles without `NDEBUG`), so every call is made
//! inside a `fork()`ed child. That way an `abort()` or a Rust panic is
//! observed as "this side crashed" instead of taking the test runner down.

#![allow(dead_code)]

pub mod deflate;

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::raw::c_uchar;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::fs::read_dir(&build) {
        for entry in dir.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Which cargo profile the running test binary belongs to, derived from its own
/// path (`target/<profile>/deps/<test>`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

/// The Rust cdylib for the profile the test binary was built with.
///
/// The crate is `crate-type = ["cdylib"]` only, so integration tests have no
/// rlib to link against and cargo does **not** build the shared object as part
/// of `cargo test`. The harness therefore builds it itself, and refuses to fall
/// back to another profile's copy -- silently loading the wrong `.so` would let
/// a broken translation pass.
pub fn rust_so_path() -> PathBuf {
    static BUILD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILD
        .get_or_init(|| {
            let dir = profile_dir();
            let release = dir.file_name().and_then(|n| n.to_str()) == Some("release");
            let mut cmd = std::process::Command::new(
                std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()),
            );
            cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
                .arg("build")
                .arg("--offline");
            if release {
                cmd.arg("--release");
            }
            let out = cmd.output().expect("running cargo build for the cdylib");
            assert!(
                out.status.success(),
                "cargo build of the cdylib failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );

            let so = dir.join("libpinflate_lib.so");
            assert!(
                so.exists(),
                "{} was not produced by cargo build",
                so.display()
            );

            // Guard against a stale artifact: it must be at least as new as the
            // sources it is built from.
            let so_time = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
            for src in ["src/lib.rs", "Cargo.toml"] {
                let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(src);
                let t = std::fs::metadata(&p).and_then(|m| m.modified()).unwrap();
                assert!(
                    so_time >= t,
                    "{} is older than {}; the tests would run against a stale library",
                    so.display(),
                    p.display()
                );
            }
            so
        })
        .clone()
}

// ---------------------------------------------------------------------------
// Exported-symbol views
// ---------------------------------------------------------------------------

type PinflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// Descriptor for one of the exported global tables.
pub struct TableDesc {
    pub name: &'static str,
    /// Size in bytes.
    pub bytes: usize,
}

pub const TABLES: &[TableDesc] = &[
    TableDesc {
        name: "cp_fixed_table",
        bytes: 288 + 32,
    },
    TableDesc {
        name: "cp_permutation_order",
        bytes: 19,
    },
    TableDesc {
        name: "cp_len_extra_bits",
        bytes: 29 + 2,
    },
    TableDesc {
        name: "cp_len_base",
        bytes: (29 + 2) * 4,
    },
    TableDesc {
        name: "cp_dist_extra_bits",
        bytes: 30 + 2,
    },
    TableDesc {
        name: "cp_dist_base",
        bytes: (30 + 2) * 4,
    },
];

pub struct Impl {
    pub lib: Library,
    pub label: &'static str,
}

impl Impl {
    pub fn load(path: &Path, label: &'static str) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        Impl { lib, label }
    }

    pub fn pinflate(&self) -> Symbol<'_, PinflateFn> {
        unsafe { self.lib.get(b"pinflate\0") }.expect("pinflate symbol")
    }

    pub fn error_reason_slot(&self) -> *mut *const c_char {
        let sym: Symbol<'_, *mut *const c_char> =
            unsafe { self.lib.get(b"cp_error_reason\0") }.expect("cp_error_reason symbol");
        unsafe { *sym.into_raw() }
    }

    pub fn table_bytes(&self, name: &str, len: usize) -> Vec<u8> {
        let mut owned = name.as_bytes().to_vec();
        owned.push(0);
        let sym: Symbol<'_, *mut c_uchar> =
            unsafe { self.lib.get(&owned) }.unwrap_or_else(|e| panic!("symbol {name}: {e}"));
        let ptr = unsafe { *sym.into_raw() };
        unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
    }
}

pub fn c_impl() -> Impl {
    Impl::load(&c_so_path(), "C")
}

pub fn rust_impl() -> Impl {
    Impl::load(&rust_so_path(), "Rust")
}

// ---------------------------------------------------------------------------
// Result of one `pinflate` invocation
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Outcome {
    /// `None` when the callee aborted / panicked / segfaulted.
    pub ret: Option<c_int>,
    /// The first `out_bytes` of the output buffer (pre-filled with [`OUT_FILL`]).
    pub out: Vec<u8>,
    /// FNV-1a hash over the *whole* output allocation, slack included, so that
    /// writes past `out_end` are compared too.
    pub full_hash: u64,
    /// `cp_error_reason` after the call: `None` for NULL.
    pub error: Option<Vec<u8>>,
    /// How the child died, when it did.
    pub crash: Option<Death>,
}

/// Why a child stopped producing records.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Death {
    /// A fatal signal: an `assert()` in C, a panic in Rust, or a real fault.
    /// The signal number is kept for diagnostics only -- the C code's
    /// out-of-bounds accesses make the exact signal unstable, so deaths of this
    /// kind compare equal to each other.
    Fatal(c_int),
    /// The case did not finish inside its CPU budget.
    Timeout,
    /// The child vanished without an interpretable status.
    Unknown(c_int),
}

impl Death {
    fn same_kind(self, other: Death) -> bool {
        matches!(
            (self, other),
            (Death::Fatal(_), Death::Fatal(_))
                | (Death::Timeout, Death::Timeout)
                | (Death::Unknown(_), Death::Unknown(_))
        )
    }
}

/// Classifies a `waitpid` status coming from a child that stopped early.
fn classify(status: c_int) -> Death {
    if libc::WIFEXITED(status) {
        let code = libc::WEXITSTATUS(status);
        if code == CHILD_TIMEOUT_EXIT {
            return Death::Timeout;
        }
        if code >= CHILD_FATAL_BASE {
            return Death::Fatal(code - CHILD_FATAL_BASE);
        }
        return Death::Unknown(code);
    }
    if libc::WIFSIGNALED(status) {
        return Death::Fatal(libc::WTERMSIG(status));
    }
    Death::Unknown(status)
}

impl Outcome {
    pub fn crashed(&self) -> bool {
        self.ret.is_none()
    }

    pub fn summary(&self) -> String {
        match self.ret {
            None => format!("DIED({:?})", self.crash),
            Some(r) => format!(
                "ret={} out={} hash={:#x} err={:?}",
                r,
                hexdump(&self.out),
                self.full_hash,
                self.error
                    .as_ref()
                    .map(|e| String::from_utf8_lossy(e).into_owned())
            ),
        }
    }
}

/// The [`Outcome::full_hash`] a run must have if it writes exactly `want` into
/// an `out_bytes`-sized buffer and touches nothing else.
pub fn expected_full_hash(want: &[u8], out_bytes: usize) -> u64 {
    let total = out_bytes + SLACK;
    assert!(want.len() <= total);
    let mut buf = want.to_vec();
    buf.resize(total, OUT_FILL);
    fnv1a(&buf)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Full hex, for pasting into a reproducer.
pub fn hexdump_full(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hexdump(bytes: &[u8]) -> String {
    let shown: Vec<String> = bytes.iter().take(64).map(|b| format!("{b:02x}")).collect();
    let mut s = shown.join("");
    if bytes.len() > 64 {
        s.push_str("...");
    }
    format!("[{}]{}", bytes.len(), s)
}

// ---------------------------------------------------------------------------
// fork()-isolated invocation
// ---------------------------------------------------------------------------

/// Byte pattern the output buffer is pre-filled with, so that "untouched"
/// bytes are visible in the comparison.
pub const OUT_FILL: u8 = 0xCD;

/// `cp_stored` copies `LEN` bytes with no regard for `out_end`, and `LEN` is a
/// 16-bit field, so both buffers get 64 KiB of slack. That keeps the C code's
/// out-of-bounds accesses inside memory we own, which is what makes the two
/// runs comparable at all instead of randomly corrupting the child's heap.
pub const SLACK: usize = 0x1_0000 + 64;

/// How many bytes of the output buffer travel back for diagnostics / equality.
/// Anything beyond that is still covered by [`Outcome::full_hash`].
pub const VISIBLE_MAX: usize = 4096;

/// Calls `pinflate` in a forked child and ships the observable state back
/// through a pipe.
///
/// `input_offset` shifts the input pointer inside its allocation, which is how
/// the `first_bytes` / alignment paths of `pinflate` get exercised.
pub fn run(imp: &Impl, input: &[u8], input_offset: usize, out_bytes: usize) -> Outcome {
    run_raw(imp, input, input_offset, out_bytes, input.len() as c_int)
}

/// Same as [`run`] but with an explicit `in_bytes` argument, for tests that
/// want to lie about the input length.
pub fn run_raw(
    imp: &Impl,
    input: &[u8],
    input_offset: usize,
    out_bytes: usize,
    in_bytes: c_int,
) -> Outcome {
    // Over-allocate both buffers: an offset input pointer, the C code's habit
    // of rounding the input pointer up to a word boundary, and `cp_stored`'s
    // unchecked copy all reach outside the nominal ranges.
    let mut in_buf = vec![0u8; input_offset + input.len() + SLACK];
    in_buf[input_offset..input_offset + input.len()].copy_from_slice(input);
    let out_total = out_bytes + SLACK;
    let mut out_buf = vec![OUT_FILL; out_total];

    let in_ptr = unsafe { in_buf.as_mut_ptr().add(input_offset) } as *mut c_void;
    let out_ptr = out_buf.as_mut_ptr() as *mut c_void;

    let err_slot = imp.error_reason_slot();
    let f = imp.pinflate();
    let func: PinflateFn = *f;

    let mut fds = [0 as c_int; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        panic!("pipe() failed");
    }
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");

    let limit_ms = case_time_limit_ms();
    if pid == 0 {
        // ---- child ----
        unsafe {
            libc::close(rd);
            // Assertions firing on purpose are expected; keep their noise out
            // of the test log unless explicitly asked for.
            child_setup();
            *err_slot = std::ptr::null();
            arm_case_timer(limit_ms);
            let ret = func(in_ptr, in_bytes, out_ptr, out_bytes as c_int);
            disarm_case_timer();

            let mut payload: Vec<u8> = Vec::new();
            payload.extend_from_slice(&ret.to_ne_bytes());
            payload.extend_from_slice(
                &fnv1a(std::slice::from_raw_parts(out_buf.as_ptr(), out_total)).to_ne_bytes(),
            );
            let visible = out_bytes.min(VISIBLE_MAX);
            payload.extend_from_slice(&(visible as u32).to_ne_bytes());
            payload.extend_from_slice(std::slice::from_raw_parts(out_buf.as_ptr(), visible));
            let err = *err_slot;
            if err.is_null() {
                payload.extend_from_slice(&u32::MAX.to_ne_bytes());
            } else {
                let bytes = std::ffi::CStr::from_ptr(err).to_bytes();
                payload.extend_from_slice(&(bytes.len() as u32).to_ne_bytes());
                payload.extend_from_slice(bytes);
            }

            write_all(wr, &payload);
            libc::close(wr);
            libc::_exit(0);
        }
    }

    // ---- parent ----
    unsafe { libc::close(wr) };
    let mut buf = Vec::new();
    {
        let mut file = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(rd) };
        let _ = file.read_to_end(&mut buf);
    }
    let mut status: c_int = 0;
    unsafe { libc::waitpid(pid, &mut status, 0) };

    let ok = libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
    if !ok || buf.len() < 16 {
        return Outcome {
            ret: None,
            out: Vec::new(),
            full_hash: 0,
            error: None,
            crash: Some(classify(status)),
        };
    }

    let ret = c_int::from_ne_bytes(buf[0..4].try_into().unwrap());
    let full_hash = u64::from_ne_bytes(buf[4..12].try_into().unwrap());
    let olen = u32::from_ne_bytes(buf[12..16].try_into().unwrap()) as usize;
    let out = buf[16..16 + olen].to_vec();
    let elen = u32::from_ne_bytes(buf[16 + olen..20 + olen].try_into().unwrap());
    let error = if elen == u32::MAX {
        None
    } else {
        Some(buf[20 + olen..20 + olen + elen as usize].to_vec())
    };
    Outcome {
        ret: Some(ret),
        out,
        full_hash,
        error,
        crash: None,
    }
}

/// Exit code the forked child uses when a case exceeds its time budget.
pub const CHILD_TIMEOUT_EXIT: c_int = 71;
/// Fatal signals leave via `100 + signo` so the parent can tell them apart.
pub const CHILD_FATAL_BASE: c_int = 100;

/// Per-case *CPU* budget. Legitimate cases finish in well under a millisecond,
/// while a malformed stream can put `cp_dynamic` or `cp_block` into a loop that
/// only a timer breaks. Measuring CPU rather than wall-clock time keeps the
/// bound tight without turning machine load into flakiness.
/// Override with `PINFLATE_CASE_LIMIT_MS`.
pub fn case_time_limit_ms() -> i64 {
    std::env::var("PINFLATE_CASE_LIMIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
}

extern "C" fn fatal_handler(sig: c_int) {
    unsafe { libc::_exit(CHILD_FATAL_BASE + sig) };
}

extern "C" fn alarm_handler(_sig: c_int) {
    unsafe { libc::_exit(CHILD_TIMEOUT_EXIT) };
}

unsafe fn arm_case_timer(limit_ms: i64) {
    let it = libc::itimerval {
        it_interval: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        it_value: libc::timeval {
            tv_sec: limit_ms / 1000,
            tv_usec: ((limit_ms % 1000) * 1000) as libc::suseconds_t,
        },
    };
    libc::setitimer(libc::ITIMER_VIRTUAL, &it, std::ptr::null_mut());
}

unsafe fn disarm_case_timer() {
    let it = libc::itimerval {
        it_interval: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
        it_value: libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        },
    };
    libc::setitimer(libc::ITIMER_REAL, &it, std::ptr::null_mut());
}

/// Assertions firing on purpose are expected, so keep their noise out of the
/// test log and -- much more importantly -- stop `abort()` from writing a core
/// dump, which otherwise costs hundreds of milliseconds per crashing case.
unsafe fn child_setup() {
    let lim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    libc::setrlimit(libc::RLIMIT_CORE, &lim);
    // `core_pattern` here pipes to systemd-coredump, which is invoked even with
    // RLIMIT_CORE at 0 for some signals and costs ~150 ms a pop. Catch the
    // fatal signals and leave via _exit() instead.
    for sig in [
        libc::SIGABRT,
        libc::SIGSEGV,
        libc::SIGBUS,
        libc::SIGILL,
        libc::SIGFPE,
        libc::SIGTRAP,
    ] {
        libc::signal(sig, fatal_handler as *const () as libc::sighandler_t);
    }
    libc::signal(libc::SIGVTALRM, alarm_handler as *const () as libc::sighandler_t);
    if std::env::var_os("PINFLATE_TEST_VERBOSE").is_none() {
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
        if devnull >= 0 {
            libc::dup2(devnull, 2);
            libc::close(devnull);
        }
    }
}

unsafe fn write_all(fd: c_int, mut data: &[u8]) {
    while !data.is_empty() {
        let n = libc::write(fd, data.as_ptr() as *const c_void, data.len());
        if n <= 0 {
            return;
        }
        data = &data[n as usize..];
    }
}

// ---------------------------------------------------------------------------
// Batched invocation
// ---------------------------------------------------------------------------

/// One `pinflate` invocation.
#[derive(Clone)]
pub struct Case {
    pub input: Vec<u8>,
    pub offset: usize,
    pub out_bytes: usize,
    pub in_bytes: c_int,
}

impl Case {
    pub fn new(input: &[u8], offset: usize, out_bytes: usize) -> Case {
        Case {
            input: input.to_vec(),
            offset,
            out_bytes,
            in_bytes: input.len() as c_int,
        }
    }
}

/// Runs many cases with as few `fork()`s as possible: one child processes
/// cases until it dies, the parent re-forks after the offending case. A single
/// fork per case would otherwise dominate the runtime of the fuzzing tests.
pub fn run_batch(imp: &Impl, cases: &[Case]) -> Vec<Outcome> {
    let mut out: Vec<Outcome> = Vec::with_capacity(cases.len());
    let mut start = 0usize;
    while start < cases.len() {
        let (records, status) = run_child(imp, &cases[start..]);
        let n = records.len();
        out.extend(records);
        if start + n >= cases.len() {
            break;
        }
        // cases[start + n] is the one that killed the child.
        out.push(Outcome {
            ret: None,
            out: Vec::new(),
            full_hash: 0,
            error: None,
            crash: Some(classify(status)),
        });
        start += n + 1;
    }
    assert_eq!(out.len(), cases.len());
    out
}

fn run_child(imp: &Impl, cases: &[Case]) -> (Vec<Outcome>, c_int) {
    let err_slot = imp.error_reason_slot();
    let f = imp.pinflate();
    let func: PinflateFn = *f;

    let mut fds = [0 as c_int; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        panic!("pipe() failed");
    }
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");

    let limit_ms = case_time_limit_ms();
    if pid == 0 {
        unsafe {
            libc::close(rd);
            child_setup();
            for case in cases {
                let mut in_buf = vec![0u8; case.offset + case.input.len() + SLACK];
                in_buf[case.offset..case.offset + case.input.len()].copy_from_slice(&case.input);
                let out_total = case.out_bytes + SLACK;
                let mut out_buf = vec![OUT_FILL; out_total];
                let in_ptr = in_buf.as_mut_ptr().add(case.offset) as *mut c_void;
                let out_ptr = out_buf.as_mut_ptr() as *mut c_void;

                *err_slot = std::ptr::null();
                arm_case_timer(limit_ms);
                let ret = func(in_ptr, case.in_bytes, out_ptr, case.out_bytes as c_int);
                disarm_case_timer();

                let mut rec: Vec<u8> = Vec::new();
                rec.extend_from_slice(&ret.to_ne_bytes());
                rec.extend_from_slice(
                    &fnv1a(std::slice::from_raw_parts(out_buf.as_ptr(), out_total)).to_ne_bytes(),
                );
                let visible = case.out_bytes.min(VISIBLE_MAX) as u32;
                rec.extend_from_slice(&visible.to_ne_bytes());
                rec.extend_from_slice(std::slice::from_raw_parts(
                    out_buf.as_ptr(),
                    visible as usize,
                ));
                let err = *err_slot;
                if err.is_null() {
                    rec.extend_from_slice(&u32::MAX.to_ne_bytes());
                } else {
                    let bytes = std::ffi::CStr::from_ptr(err).to_bytes();
                    rec.extend_from_slice(&(bytes.len() as u32).to_ne_bytes());
                    rec.extend_from_slice(bytes);
                }

                let mut framed = (rec.len() as u32).to_ne_bytes().to_vec();
                framed.extend_from_slice(&rec);
                write_all(wr, &framed);
            }
            libc::close(wr);
            libc::_exit(0);
        }
    }

    unsafe { libc::close(wr) };
    let mut buf = Vec::new();
    {
        let mut file = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(rd) };
        let _ = file.read_to_end(&mut buf);
    }
    let mut status: c_int = 0;
    unsafe { libc::waitpid(pid, &mut status, 0) };

    // Decode as many complete records as arrived.
    let mut records = Vec::new();
    let mut p = 0usize;
    while p + 4 <= buf.len() {
        let len = u32::from_ne_bytes(buf[p..p + 4].try_into().unwrap()) as usize;
        if p + 4 + len > buf.len() {
            break;
        }
        let rec = &buf[p + 4..p + 4 + len];
        p += 4 + len;
        let ret = c_int::from_ne_bytes(rec[0..4].try_into().unwrap());
        let full_hash = u64::from_ne_bytes(rec[4..12].try_into().unwrap());
        let olen = u32::from_ne_bytes(rec[12..16].try_into().unwrap()) as usize;
        let o = rec[16..16 + olen].to_vec();
        let elen = u32::from_ne_bytes(rec[16 + olen..20 + olen].try_into().unwrap());
        let error = if elen == u32::MAX {
            None
        } else {
            Some(rec[20 + olen..20 + olen + elen as usize].to_vec())
        };
        records.push(Outcome {
            ret: Some(ret),
            out: o,
            full_hash,
            error,
            crash: None,
        });
    }
    (records, status)
}

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

/// What the C side is additionally required to do for a case.
enum Expect {
    /// Nothing; only C/Rust agreement matters.
    Agree,
    /// C must return 1 (the stream is well-formed enough to run to completion).
    Ret1,
    /// C must return 1 and its output must start with these bytes.
    Exact(Vec<u8>),
}

/// Collects cases, then evaluates them in as few forked children as possible.
pub struct Differ {
    pub c: Impl,
    pub rs: Impl,
    pending: Vec<(String, Case, Expect)>,
}

impl Differ {
    pub fn new() -> Differ {
        Differ {
            c: c_impl(),
            rs: rust_impl(),
            pending: Vec::new(),
        }
    }

    pub fn check(&mut self, what: &str, input: &[u8], input_offset: usize, out_bytes: usize) {
        self.pending.push((
            what.to_string(),
            Case::new(input, input_offset, out_bytes),
            Expect::Agree,
        ));
    }

    /// Same as [`Differ::check`] but with an explicit `in_bytes`, for tests
    /// that want to lie about the input length.
    pub fn check_raw(
        &mut self,
        what: &str,
        input: &[u8],
        input_offset: usize,
        out_bytes: usize,
        in_bytes: c_int,
    ) {
        let mut case = Case::new(input, input_offset, out_bytes);
        case.in_bytes = in_bytes;
        self.pending.push((what.to_string(), case, Expect::Agree));
    }

    /// Requires that C decodes the stream to exactly `expected`, so a test
    /// cannot pass by having both implementations bail out immediately.
    pub fn check_ok(
        &mut self,
        what: &str,
        input: &[u8],
        input_offset: usize,
        out_bytes: usize,
        expected: &[u8],
    ) {
        self.pending.push((
            what.to_string(),
            Case::new(input, input_offset, out_bytes),
            Expect::Exact(expected.to_vec()),
        ));
    }

    /// Requires only that C ran to completion (`ret == 1`). Used where the C
    /// code succeeds but its output is not the textbook inflate result --
    /// `cp_stored`'s source pointer, for instance, is derived from bit-buffer
    /// bookkeeping and is off whenever the `final_word` path inflated `count`.
    /// C is the ground truth there, so only C/Rust agreement is asserted.
    pub fn check_ret1(&mut self, what: &str, input: &[u8], input_offset: usize, out_bytes: usize) {
        self.pending.push((
            what.to_string(),
            Case::new(input, input_offset, out_bytes),
            Expect::Ret1,
        ));
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn finish(self, label: &str) {
        let cases: Vec<Case> = self.pending.iter().map(|(_, c, _)| c.clone()).collect();
        let a = run_batch(&self.c, &cases);
        let b = run_batch(&self.rs, &cases);

        let mut failures: Vec<String> = Vec::new();
        let mut total_diffs = 0usize;
        let mut classes: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        let mut expectation_failures: Vec<String> = Vec::new();

        for (i, (what, case, expect)) in self.pending.iter().enumerate() {
            let (ca, ra) = (&a[i], &b[i]);
            let both_died = match (ca.crash, ra.crash) {
                (Some(x), Some(y)) => x.same_kind(y),
                _ => false,
            };
            if ca != ra && !both_died {
                let cls = |o: &Outcome| match (&o.crash, o.ret) {
                    (Some(d), _) => format!("{d:?}"),
                    (None, Some(r)) => format!("ret={r}"),
                    _ => "?".to_string(),
                };
                total_diffs += 1;
                let class = format!("C={} Rust={}", cls(ca), cls(ra));
                *classes.entry(class.clone()).or_default() += 1;
                // Debug aid: `PINFLATE_ONLY_CLASS='C=Fatal(6) Rust=ret=0'`
                // narrows the reported failures to one outcome class while the
                // counts above still cover all of them. It only filters what is
                // *printed*; the test still fails.
                if let Ok(only) = std::env::var("PINFLATE_ONLY_CLASS") {
                    if class != only {
                        continue;
                    }
                }
                failures.push(format!(
                    "case `{what}` (offset={}, out_bytes={}, in_bytes={})\n  \
                     input = {}\n  C    : {}\n  Rust : {}",
                    case.offset,
                    case.out_bytes,
                    case.in_bytes,
                    hexdump_full(&case.input),
                    ca.summary(),
                    ra.summary()
                ));
            }
            match expect {
                Expect::Agree => {}
                Expect::Ret1 => {
                    if ca.ret != Some(1) {
                        expectation_failures.push(format!(
                            "case `{what}` (offset={}, out_bytes={}): C did not run to \
                             completion: {}\n  input = {}",
                            case.offset,
                            case.out_bytes,
                            ca.summary(),
                            hexdump_full(&case.input)
                        ));
                    }
                }
                Expect::Exact(want) => {
                    if ca.ret != Some(1) {
                        expectation_failures.push(format!(
                            "case `{what}` (offset={}, out_bytes={}): C failed: {}\n  input = {}",
                            case.offset,
                            case.out_bytes,
                            ca.summary(),
                            hexdump_full(&case.input)
                        ));
                    } else if ca.full_hash != expected_full_hash(want, case.out_bytes) {
                        expectation_failures.push(format!(
                            "case `{what}`: C did not decode to the expected bytes \
                             (test-vector bug)\n  got  {}\n  want {}",
                            hexdump(&ca.out),
                            hexdump(want)
                        ));
                    }
                }
            }
        }

        if !expectation_failures.is_empty() {
            let shown: Vec<String> = expectation_failures.iter().take(5).cloned().collect();
            panic!(
                "{label}: {} of {} test vectors are not what the C code decodes:\n{}",
                expectation_failures.len(),
                self.pending.len(),
                shown.join("\n")
            );
        }
        if total_diffs != 0 {
            let shown: Vec<String> = failures.iter().take(4).cloned().collect();
            panic!(
                "{label}: {} of {} cases differ between C and Rust\nclasses: {:?}\n{}",
                total_diffs,
                self.pending.len(),
                classes,
                shown.join("\n")
            );
        }
        let died = a.iter().filter(|o| o.crashed()).count();
        let timeouts = a.iter().filter(|o| o.crash == Some(Death::Timeout)).count();
        eprintln!(
            "{label}: {} cases matched ({died} where the C code aborts or hangs, \
             {timeouts} of them non-terminating)",
            self.pending.len()
        );
    }
}

/// Scales the fuzzers' iteration counts, as a percentage.
/// `PINFLATE_FUZZ_SCALE=25` runs a quarter of the cases; the default is 100.
pub fn fuzz_iters(base: usize) -> usize {
    let pct: usize = std::env::var("PINFLATE_FUZZ_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    (base * pct / 100).max(1)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (no external crates)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
