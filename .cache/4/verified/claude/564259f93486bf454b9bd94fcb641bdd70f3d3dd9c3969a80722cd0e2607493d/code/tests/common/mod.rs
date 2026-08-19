// Shared differential-test harness.
//
// Both implementations under test are loaded as SHARED OBJECTS via `libloading`
// and invoked only through their exported `driver` symbol. The Rust
// implementation is never called directly as a Rust function, so the
// `#[no_mangle] extern "C"` export wrapper is exercised too.

#![allow(dead_code)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;

/// Signature of the single public entry point: `void driver(const char*, const char*)`.
pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

extern "C" {
    /// glibc's `FILE *stdout`. Both `.so`s share this exact stream object, so a
    /// single `fflush` drains whichever library just printed.
    static stdout: *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;

    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn ftruncate(fd: c_int, length: i64) -> c_int;

    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
    fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int;

    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, length: usize) -> c_int;
    fn mprotect(addr: *mut c_void, length: usize, prot: c_int) -> c_int;
    fn sysconf(name: c_int) -> i64;
}

#[repr(C)]
struct Rlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_CORE: c_int = 4; // Linux

/// Called in every forked child: the invalid-pointer rows deliberately crash the
/// child, and writing a core dump for each one is both slow (tens of seconds
/// across the matrix) and messy. Disabling core dumps does not change the
/// reported termination signal.
unsafe fn disable_core_dumps() {
    let rl = Rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    setrlimit(RLIMIT_CORE, &rl);
}

const SEEK_SET: c_int = 0;
const PROT_NONE: c_int = 0x0;
const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const MAP_FAILED: isize = -1;
const SC_PAGESIZE: c_int = 30; // Linux _SC_PAGESIZE

/// Serialises every manipulation of file descriptor 1, which is process-global.
/// `cargo test` runs test functions on parallel threads, so without this the
/// stdout redirections would race.
static IO_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed keeps every row reproducible.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C5BD_1234_9ABC;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }

    /// Inclusive range `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    /// Any byte that can legally appear inside a NUL-terminated string: 1..=255.
    pub fn byte_nonzero(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }

    /// Random NUL-terminated buffer of `len` payload bytes drawn from `alphabet`.
    pub fn string_from(&mut self, len: usize, alphabet: &[u8]) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(alphabet[self.below(alphabet.len())]);
        }
        v.push(0);
        v
    }

    /// Random NUL-terminated buffer of `len` payload bytes over the full
    /// non-NUL byte domain 0x01..=0xFF.
    pub fn string_full_domain(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len + 1);
        for _ in 0..len {
            v.push(self.byte_nonzero());
        }
        v.push(0);
        v
    }
}

/// Full non-NUL byte domain, 0x01..=0xFF (255 values).
pub fn all_nonzero_bytes() -> Vec<u8> {
    (1u16..=255).map(|b| b as u8).collect()
}

/// Naive oracle implementing the documented `strcspn` contract. Used only to
/// sanity-check that the harness itself is wired up correctly; the real
/// assertions always compare C output against Rust output.
pub fn oracle_strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let reject_end = s2.iter().position(|&b| b == 0).unwrap_or(s2.len());
    let reject = &s2[..reject_end];
    let mut n = 0usize;
    for &b in s1 {
        if b == 0 || reject.contains(&b) {
            break;
        }
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Locates the Rust `cdylib` to compare against the C `.so`.
///
/// Preference order:
///  1. `$RUST_DRIVER_SO`, so a specific artifact can be pinned.
///  2. `target/release/libdriver.so` — the **deliverable**. This is the artifact
///     that corresponds to the C `.so`: an optimised shared library built with
///     the crate's declared `[profile.release] panic = "abort"`. It is the
///     default so the suite measures shipping behaviour, not dev diagnostics.
///  3. The cdylib next to the running test binary, then `target/debug`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut candidates: Vec<PathBuf> = vec![root.join("release").join("libdriver.so")];
    // The test executable lives in target/<profile>/deps/, so the cdylib built
    // from this same crate sits one directory up.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libdriver.so"));
            if let Some(profile) = deps.parent() {
                candidates.push(profile.join("libdriver.so"));
            }
        }
    }
    candidates.push(root.join("debug").join("libdriver.so"));

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found. Looked in: {candidates:#?}\n\
         Build it with: cargo build --offline --no-default-features --release"
    );
}

// ---------------------------------------------------------------------------
// Harness: loads both .so files and captures their stdout
// ---------------------------------------------------------------------------

pub struct Harness {
    _c_lib: libloading::Library,
    _rs_lib: libloading::Library,
    pub c_driver: DriverFn,
    pub rs_driver: DriverFn,
    cap_path: PathBuf,
    cap_fd: c_int,
}

impl Harness {
    pub fn new() -> Harness {
        unsafe {
            let c_lib = libloading::Library::new(c_so_path()).expect("dlopen C libdriver.so");
            let rs_lib = libloading::Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");

            let c_sym: libloading::Symbol<DriverFn> =
                c_lib.get(b"driver\0").expect("C .so must export `driver`");
            let rs_sym: libloading::Symbol<DriverFn> = rs_lib
                .get(b"driver\0")
                .expect("Rust .so must export `driver`");

            let c_driver = *c_sym;
            let rs_driver = *rs_sym;

            // Guard against dlsym collapsing both handles onto one definition,
            // which would make every comparison vacuously pass.
            assert_ne!(
                c_driver as usize, rs_driver as usize,
                "C and Rust `driver` resolved to the SAME address; the two \
                 libraries are not being tested independently"
            );

            // One reusable capture file, truncated per call.
            let mut cap_path = std::env::temp_dir();
            cap_path.push(format!(
                "driver_cap_{}_{:?}.out",
                std::process::id(),
                std::thread::current().id()
            ));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&cap_path)
                .expect("create capture file");
            // Leak the File so the fd stays valid for the harness's lifetime.
            let cap_fd = {
                use std::os::fd::IntoRawFd;
                file.into_raw_fd()
            };

            Harness {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c_driver,
                rs_driver,
                cap_path,
                cap_fd,
            }
        }
    }

    /// Runs `f` once per case **in a forked child** whose fd 1 is the capture
    /// file, returning the exact bytes the library wrote plus how the child
    /// terminated.
    ///
    /// Forking is what makes the capture trustworthy: only the child's fd 1 is
    /// redirected, so `libtest`'s own progress output (`test foo ... ok`) stays
    /// on the real stdout and can never contaminate the captured bytes. It also
    /// means a panic/abort inside either library is observed as a child
    /// termination status instead of killing the test runner.
    ///
    /// Batching every case into a single child keeps the large randomized sweeps
    /// fast while still comparing every byte.
    fn capture(&self, f: DriverFn, cases: &[(*const c_char, *const c_char)]) -> Capture {
        unsafe {
            let _guard = IO_LOCK.lock().unwrap();

            lseek(self.cap_fd, 0, SEEK_SET);
            ftruncate(self.cap_fd, 0);

            // Drain both runtimes' buffers so the child inherits no pending
            // bytes that it could flush into the capture file.
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            fflush(std::ptr::null_mut()); // fflush(NULL): flush every glibc stream

            let pid = fork();
            assert!(pid >= 0, "fork() failed");
            if pid == 0 {
                // Child: only our fd 1 points at the capture file.
                disable_core_dumps();
                dup2(self.cap_fd, 1);
                for &(s1, s2) in cases {
                    f(s1, s2);
                }
                fflush(stdout);
                _exit(0);
            }

            let mut status: c_int = 0;
            let w = waitpid(pid, &mut status, 0);
            assert_eq!(w, pid, "waitpid() failed");

            lseek(self.cap_fd, 0, SEEK_SET);
            let mut out = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = read(self.cap_fd, buf.as_mut_ptr() as *mut c_void, buf.len());
                if n <= 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n as usize]);
            }
            Capture {
                bytes: out,
                exited: wif_exited(status),
                exit_status: wexit_status(status),
                signaled: wif_signaled(status),
                termsig: wterm_sig(status),
            }
        }
    }

    fn capture_full_c(&self, cases: &[(*const c_char, *const c_char)]) -> Capture {
        self.capture(self.c_driver, cases)
    }

    fn capture_full_rs(&self, cases: &[(*const c_char, *const c_char)]) -> Capture {
        self.capture(self.rs_driver, cases)
    }

    /// Bytes written by the C library; panics if it did not run to completion.
    pub fn capture_c(&self, cases: &[(*const c_char, *const c_char)]) -> Vec<u8> {
        let r = self.capture_full_c(cases);
        assert!(r.clean(), "C library did not complete normally: {r:?}");
        r.bytes
    }

    /// Bytes written by the Rust library; panics if it did not run to completion.
    pub fn capture_rs(&self, cases: &[(*const c_char, *const c_char)]) -> Vec<u8> {
        let r = self.capture_full_rs(cases);
        assert!(r.clean(), "Rust library did not complete normally: {r:?}");
        r.bytes
    }

    /// Runs every case through BOTH `.so`s and asserts the captured stdout is
    /// byte-identical. On mismatch, re-runs case-by-case to pinpoint and print
    /// the first divergence.
    pub fn assert_same(&self, label: &str, cases: &[Case]) {
        assert!(!cases.is_empty(), "{label}: no cases generated");
        let ptrs: Vec<(*const c_char, *const c_char)> = cases.iter().map(|c| c.ptrs()).collect();

        let c_res = self.capture_full_c(&ptrs);
        let rs_res = self.capture_full_rs(&ptrs);
        let (c_out, rs_out) = (c_res.bytes.clone(), rs_res.bytes.clone());

        assert!(
            c_res.clean(),
            "{label}: the C library itself did not complete normally: {c_res:?}"
        );
        assert_eq!(
            (rs_res.exited, rs_res.exit_status, rs_res.signaled, rs_res.termsig),
            (c_res.exited, c_res.exit_status, c_res.signaled, c_res.termsig),
            "{label}: termination mismatch (did Rust panic/abort where C did not?)\n  \
             C    = {c_res:?}\n  Rust = {rs_res:?}"
        );

        if c_out == rs_out {
            // Every case must also have produced exactly one line.
            let lines = c_out.iter().filter(|&&b| b == b'\n').count();
            assert_eq!(
                lines,
                cases.len(),
                "{label}: expected one output line per case ({}), got {lines}. \
                 Captured: {:?}",
                cases.len(),
                String::from_utf8_lossy(&c_out)
            );
            return;
        }

        // Bisect to the first differing case for a useful failure message.
        for (i, case) in cases.iter().enumerate() {
            let one = [case.ptrs()];
            let c1 = self.capture_c(&one);
            let r1 = self.capture_rs(&one);
            if c1 != r1 {
                panic!(
                    "{label}: DIVERGENCE at case #{i}\n  \
                     s1 = {}\n  s2 = {}\n  \
                     C   output = {:?}\n  Rust output = {:?}\n  \
                     naive oracle = {}",
                    case.describe_s1(),
                    case.describe_s2(),
                    String::from_utf8_lossy(&c1),
                    String::from_utf8_lossy(&r1),
                    oracle_strcspn(&case.s1, &case.s2),
                );
            }
        }
        panic!(
            "{label}: batch outputs differ but no single case does (aggregate \
             mismatch)\n  C   ({} bytes) = {:?}\n  Rust ({} bytes) = {:?}",
            c_out.len(),
            String::from_utf8_lossy(&c_out),
            rs_out.len(),
            String::from_utf8_lossy(&rs_out),
        );
    }

    /// Confirms the C library agrees with the naive `strcspn` contract, proving
    /// the harness (redirection, batching, pointer setup) is sound.
    pub fn assert_c_matches_oracle(&self, label: &str, cases: &[Case]) {
        let ptrs: Vec<(*const c_char, *const c_char)> = cases.iter().map(|c| c.ptrs()).collect();
        let c_out = self.capture_c(&ptrs);
        let mut expected = Vec::new();
        for case in cases {
            expected.extend_from_slice(oracle_strcspn(&case.s1, &case.s2).to_string().as_bytes());
            expected.push(b'\n');
        }
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&expected),
            "{label}: harness sanity check failed — C output does not match the \
             naive strcspn oracle"
        );
    }

    // -----------------------------------------------------------------------
    // Crash-parity testing for the undefined-behaviour / invalid-pointer rows
    // -----------------------------------------------------------------------

    /// Calls `f(s1, s2)` in a forked child and reports how the child ended plus
    /// anything it managed to print.
    fn run_forked(&self, f: DriverFn, s1: *const c_char, s2: *const c_char) -> ChildOutcome {
        unsafe {
            let _guard = IO_LOCK.lock().unwrap();

            let mut path = std::env::temp_dir();
            path.push(format!(
                "driver_fork_{}_{}.out",
                std::process::id(),
                CHILD_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            ));
            let file = std::fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .expect("create fork capture file");
            let fd = {
                use std::os::fd::AsRawFd;
                file.as_raw_fd()
            };

            fflush(stdout);
            let pid = fork();
            assert!(pid >= 0, "fork() failed");
            if pid == 0 {
                // Child: redirect stdout to the file, run, flush, exit cleanly.
                disable_core_dumps();
                dup2(fd, 1);
                f(s1, s2);
                fflush(stdout);
                _exit(0);
            }

            let mut status: c_int = 0;
            let w = waitpid(pid, &mut status, 0);
            assert_eq!(w, pid, "waitpid() failed");

            drop(file);
            let printed = std::fs::read(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);

            ChildOutcome {
                signaled: wif_signaled(status),
                termsig: wterm_sig(status),
                exited: wif_exited(status),
                exit_status: wexit_status(status),
                printed,
            }
        }
    }

    pub fn fork_c(&self, s1: *const c_char, s2: *const c_char) -> ChildOutcome {
        self.run_forked(self.c_driver, s1, s2)
    }

    pub fn fork_rs(&self, s1: *const c_char, s2: *const c_char) -> ChildOutcome {
        self.run_forked(self.rs_driver, s1, s2)
    }

    /// Asserts that C and Rust *fault identically* on an invalid pointer.
    ///
    /// The C library has no in-band error channel (`driver` returns `void` and
    /// validates nothing), so the observable rejection for an invalid pointer is
    /// the terminating signal. This asserts the specific signal — not merely
    /// "both failed somehow" — and that neither library printed anything.
    ///
    /// The one sanctioned difference: when the Rust library under test was built
    /// with `debug_assertions` (cargo's `dev` profile), rustc emits optional
    /// "UB checks" that turn a null-pointer dereference into a controlled Rust
    /// panic (`SIGABRT`) instead of letting it fault (`SIGSEGV`). That is a
    /// development diagnostic, not library behaviour: the shipping `release`
    /// artifact faults with exactly C's signal, which is verified by running this
    /// same suite against `target/release/libdriver.so` (the default).
    pub fn assert_fault_parity(&self, label: &str, s1: *const c_char, s2: *const c_char) {
        let c = self.fork_c(s1, s2);
        let r = self.fork_rs(s1, s2);

        assert!(
            c.signaled && (c.termsig == SIGSEGV || c.termsig == SIGBUS),
            "{label}: expected the C library to die of a memory fault, got {c:?}"
        );
        assert!(
            c.printed.is_empty(),
            "{label}: C should not have printed anything, got {:?}",
            String::from_utf8_lossy(&c.printed)
        );
        assert!(
            r.printed.is_empty(),
            "{label}: Rust should not have printed anything, got {:?}",
            String::from_utf8_lossy(&r.printed)
        );
        assert!(
            r.signaled,
            "{label}: Rust survived an input that killed C\n  C    = {c:?}\n  Rust = {r:?}"
        );

        if rust_ub_checks_enabled() && r.termsig == SIGABRT {
            // Documented dev-profile behaviour; release parity is asserted by the
            // default run of this suite.
            return;
        }
        assert_eq!(
            r.termsig, c.termsig,
            "{label}: signal mismatch — C and Rust must be rejected identically\n  \
             C    = {c:?}\n  Rust = {r:?}"
        );
    }

    /// Asserts that C and Rust react to an invalid/boundary input in exactly the
    /// same way: same termination mode, same signal number, same exit status and
    /// the same bytes printed (if any).
    pub fn assert_same_outcome(
        &self,
        label: &str,
        s1: *const c_char,
        s2: *const c_char,
    ) -> ChildOutcome {
        let c = self.fork_c(s1, s2);
        let r = self.fork_rs(s1, s2);
        assert_eq!(
            (c.signaled, c.termsig, c.exited, c.exit_status),
            (r.signaled, r.termsig, r.exited, r.exit_status),
            "{label}: termination mismatch\n  C    = {c:?}\n  Rust = {r:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&c.printed),
            String::from_utf8_lossy(&r.printed),
            "{label}: printed-output mismatch\n  C    = {c:?}\n  Rust = {r:?}"
        );
        c
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe {
            close(self.cap_fd);
        }
        let _ = std::fs::remove_file(&self.cap_path);
    }
}

static CHILD_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Result of running a batch of cases through one library inside a forked child.
#[derive(Clone, PartialEq, Eq)]
pub struct Capture {
    pub bytes: Vec<u8>,
    pub exited: bool,
    pub exit_status: i32,
    pub signaled: bool,
    pub termsig: i32,
}

impl Capture {
    /// True when the child ran every case and exited 0 (no panic, no signal).
    pub fn clean(&self) -> bool {
        self.exited && self.exit_status == 0
    }
}

impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let shown = if self.bytes.len() > 400 {
            format!(
                "{}...(+{} bytes)",
                String::from_utf8_lossy(&self.bytes[..400]),
                self.bytes.len() - 400
            )
        } else {
            String::from_utf8_lossy(&self.bytes).into_owned()
        };
        write!(
            f,
            "Capture {{ exited: {}, exit_status: {}, signaled: {}, termsig: {}, \
             bytes({}): {:?} }}",
            self.exited,
            self.exit_status,
            self.signaled,
            self.termsig,
            self.bytes.len(),
            shown
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildOutcome {
    pub signaled: bool,
    pub termsig: i32,
    pub exited: bool,
    pub exit_status: i32,
    pub printed: Vec<u8>,
}

fn wif_exited(status: c_int) -> bool {
    (status & 0x7f) == 0
}
fn wexit_status(status: c_int) -> i32 {
    (status >> 8) & 0xff
}
fn wif_signaled(status: c_int) -> bool {
    ((((status & 0x7f) + 1) as i8) >> 1) > 0
}
fn wterm_sig(status: c_int) -> i32 {
    status & 0x7f
}

pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;
pub const SIGABRT: i32 = 6;

/// Whether the Rust `.so` under test was built with `debug_assertions` on
/// (cargo's `dev` profile). Set by `run_all_tests.sh` when it exercises the debug
/// artifact. See `Harness::assert_fault_parity` for why this matters.
pub fn rust_ub_checks_enabled() -> bool {
    matches!(
        std::env::var("DRIVER_RUST_UB_CHECKS").as_deref(),
        Ok("1") | Ok("true")
    )
}

// ---------------------------------------------------------------------------
// A single differential test case: two NUL-terminated byte buffers
// ---------------------------------------------------------------------------

/// Owns both buffers so the raw pointers handed to the libraries stay valid.
/// `s1`/`s2` always include their terminating NUL.
pub struct Case {
    pub s1: Vec<u8>,
    pub s2: Vec<u8>,
    /// Byte offset into `s1` at which the string actually starts (alignment tests).
    pub s1_off: usize,
    pub s2_off: usize,
}

impl Case {
    /// Both arguments given as payload bytes; NUL terminators are appended.
    pub fn new(s1: &[u8], s2: &[u8]) -> Case {
        let mut a = s1.to_vec();
        a.push(0);
        let mut b = s2.to_vec();
        b.push(0);
        Case {
            s1: a,
            s2: b,
            s1_off: 0,
            s2_off: 0,
        }
    }

    /// Buffers supplied verbatim — the caller is responsible for the NULs.
    /// Used for embedded-NUL and aliasing rows.
    pub fn raw(s1: Vec<u8>, s2: Vec<u8>) -> Case {
        Case {
            s1,
            s2,
            s1_off: 0,
            s2_off: 0,
        }
    }

    /// Shifts the visible start of each string inside its buffer, so the
    /// libraries receive deliberately (mis)aligned pointers.
    pub fn with_offsets(mut self, s1_off: usize, s2_off: usize) -> Case {
        self.s1_off = s1_off;
        self.s2_off = s2_off;
        self
    }

    pub fn ptrs(&self) -> (*const c_char, *const c_char) {
        (
            unsafe { self.s1.as_ptr().add(self.s1_off) as *const c_char },
            unsafe { self.s2.as_ptr().add(self.s2_off) as *const c_char },
        )
    }

    fn describe(buf: &[u8], off: usize) -> String {
        let v = &buf[off.min(buf.len())..];
        let end = v.iter().position(|&b| b == 0).unwrap_or(v.len());
        let shown = &v[..end.min(96)];
        format!(
            "len={} ptr_off={} bytes={:02x?}{}",
            end,
            off,
            shown,
            if end > 96 { " ...(truncated)" } else { "" }
        )
    }

    pub fn describe_s1(&self) -> String {
        Self::describe(&self.s1, self.s1_off)
    }
    pub fn describe_s2(&self) -> String {
        Self::describe(&self.s2, self.s2_off)
    }
}

// ---------------------------------------------------------------------------
// Page-guarded buffers, for over-read / page-edge rows
// ---------------------------------------------------------------------------

pub fn page_size() -> usize {
    let v = unsafe { sysconf(SC_PAGESIZE) };
    if v > 0 {
        v as usize
    } else {
        4096
    }
}

/// Two consecutive pages: the first readable/writable, the second `PROT_NONE`.
/// Any read past the end of the first page faults immediately, which is exactly
/// what makes the over-read rows observable.
pub struct GuardedPage {
    base: *mut u8,
    total: usize,
    pub page: usize,
}

impl GuardedPage {
    pub fn new() -> GuardedPage {
        let page = page_size();
        let total = page * 2;
        unsafe {
            let base = mmap(
                std::ptr::null_mut(),
                total,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_ne!(base as isize, MAP_FAILED, "mmap failed");
            assert_eq!(
                mprotect(base.add(page) as *mut c_void, page, PROT_NONE),
                0,
                "mprotect guard page failed"
            );
            GuardedPage {
                base: base as *mut u8,
                total,
                page,
            }
        }
    }

    /// Writable slice covering the whole first (accessible) page.
    pub fn data(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.base, self.page) }
    }

    /// Pointer to byte `off` of the accessible page.
    pub fn ptr_at(&self, off: usize) -> *const c_char {
        assert!(off < self.page);
        unsafe { self.base.add(off) as *const c_char }
    }

    /// Fills the accessible page with `fill` and writes a terminating NUL as its
    /// very last byte, then returns a pointer to a string of exactly `len`
    /// payload bytes that ends flush against the guard page.
    pub fn string_flush_to_edge(&mut self, len: usize, fill: u8) -> *const c_char {
        let page = self.page;
        assert!(len < page);
        let d = self.data();
        for b in d.iter_mut() {
            *b = fill;
        }
        d[page - 1] = 0;
        self.ptr_at(page - 1 - len)
    }

    /// Fills the accessible page entirely with `fill` and returns a pointer such
    /// that there is NO terminating NUL before the guard page.
    pub fn unterminated(&mut self, len: usize, fill: u8) -> *const c_char {
        let page = self.page;
        assert!(len <= page);
        let d = self.data();
        for b in d.iter_mut() {
            *b = fill;
        }
        self.ptr_at(page - len)
    }
}

impl Drop for GuardedPage {
    fn drop(&mut self) {
        unsafe {
            munmap(self.base as *mut c_void, self.total);
        }
    }
}
