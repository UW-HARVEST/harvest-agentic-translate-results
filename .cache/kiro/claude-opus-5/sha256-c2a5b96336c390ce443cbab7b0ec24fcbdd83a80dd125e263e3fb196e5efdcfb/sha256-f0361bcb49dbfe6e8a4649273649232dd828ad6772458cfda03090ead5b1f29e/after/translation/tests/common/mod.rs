//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and exposes them behind an identical calling interface.
//!
//! Nothing here calls a Rust function directly — every `bin2hex` invocation
//! goes through a `dlsym`'d symbol, so the `#[no_mangle] extern "C"` export
//! wrapper is under test just like the C one.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

/// `char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len)`
pub type Bin2HexFn = unsafe extern "C" fn(*mut i8, usize, *const u8, usize) -> *mut i8;

/// Repository root (the directory containing `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {}. Build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // `cargo test` puts the integration-test binary in target/<profile>/deps/,
    // and the cdylib in target/<profile>/.
    //
    // Important: `cargo test` does NOT build the `cdylib` crate-type — the
    // integration tests link the `rlib`, so the `.so` is left untouched. That
    // means a plain `cargo test` would happily run against a MISSING or STALE
    // `.so` and report green for code that no longer exists. To make the
    // harness trustworthy no matter how it is invoked, we build the cdylib
    // ourselves whenever it is absent or older than `src/lib.rs`.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let is_release = profile_dir.file_name().and_then(|s| s.to_str()) == Some("release");
    let so = profile_dir.join("libbin2hex_lib.so");

    if !is_fresh(&so, &manifest.join("src").join("lib.rs")) {
        build_cdylib(&manifest, is_release);
    }

    assert!(
        so.is_file(),
        "libbin2hex_lib.so was not produced at {}. Build it manually:\n  \
         cd translation && cargo build{}",
        so.display(),
        if is_release { " --release" } else { "" }
    );
    assert!(
        is_fresh(&so, &manifest.join("src").join("lib.rs")),
        "STALE ARTIFACT: {} is older than src/lib.rs even after rebuilding.\n\
         The tests would be comparing against an out-of-date Rust library.",
        so.display()
    );
    so
}

/// `true` when `artifact` exists and is at least as new as `src`.
fn is_fresh(artifact: &std::path::Path, src: &std::path::Path) -> bool {
    let a = match std::fs::metadata(artifact).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    match std::fs::metadata(src).and_then(|m| m.modified()) {
        Ok(s) => a >= s,
        Err(_) => true,
    }
}

fn build_cdylib(manifest: &std::path::Path, release: bool) {
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"));
    if release {
        cmd.arg("--release");
    }
    // Avoid inheriting cargo's own "we are inside a build" plumbing.
    cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
    let out = cmd.output().expect("failed to spawn `cargo build --lib`");
    assert!(
        out.status.success(),
        "`cargo build --lib` failed while producing the cdylib under test:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

pub struct Libs {
    _c: libloading::Library,
    _rust: libloading::Library,
    pub c_bin2hex: Bin2HexFn,
    pub rust_bin2hex: Bin2HexFn,
}

// The function pointers point into the two libraries, which are leaked for the
// lifetime of the process (they live in a `OnceLock`), so sharing is fine.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c = libloading::Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust = libloading::Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));

            let c_sym: libloading::Symbol<Bin2HexFn> = c
                .get(b"bin2hex\0")
                .unwrap_or_else(|e| panic!("dlsym bin2hex in C .so: {e}"));
            let rust_sym: libloading::Symbol<Bin2HexFn> = rust
                .get(b"bin2hex\0")
                .unwrap_or_else(|e| panic!("dlsym bin2hex in Rust .so: {e}"));

            let c_bin2hex = *c_sym;
            let rust_bin2hex = *rust_sym;

            Libs {
                _c: c,
                _rust: rust,
                c_bin2hex,
                rust_bin2hex,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// Differential driver for the VALID path (no abort expected)
// ---------------------------------------------------------------------------

/// Sentinel byte used to fill output buffers so that any stray write past
/// `bin_len * 2 + 1` shows up in the byte-for-byte comparison.
pub const SENTINEL: u8 = 0xA5;

/// Result of one side's invocation.
pub struct Outcome {
    pub buf: Vec<u8>,
    /// `true` when the returned pointer was exactly the `hex` argument.
    pub returned_hex_ptr: bool,
}

/// Run one side (`f`) with a freshly sentinel-filled buffer of `buf_len` bytes.
///
/// `hex_off` / `bin_off` place the pointers at an offset inside their
/// allocations (axis F). `hex_maxlen` is passed through verbatim so callers can
/// lie about the buffer size (E19/C21).
pub fn run_one(
    f: Bin2HexFn,
    buf_len: usize,
    hex_off: usize,
    hex_maxlen: usize,
    bin: &[u8],
    bin_off: usize,
    bin_len: usize,
) -> Outcome {
    let mut buf = vec![SENTINEL; buf_len];
    let hex_ptr = unsafe { buf.as_mut_ptr().add(hex_off) } as *mut i8;
    let bin_ptr = if bin.is_empty() {
        std::ptr::null()
    } else {
        unsafe { bin.as_ptr().add(bin_off) }
    };
    let ret = unsafe { f(hex_ptr, hex_maxlen, bin_ptr, bin_len) };
    Outcome {
        buf,
        returned_hex_ptr: ret == hex_ptr,
    }
}

/// Compare C and Rust on one valid-path configuration; panics with a detailed
/// diff on the first divergence.
#[allow(clippy::too_many_arguments)]
pub fn assert_same(
    label: &str,
    buf_len: usize,
    hex_off: usize,
    hex_maxlen: usize,
    bin: &[u8],
    bin_off: usize,
    bin_len: usize,
) {
    let l = libs();
    let c = run_one(l.c_bin2hex, buf_len, hex_off, hex_maxlen, bin, bin_off, bin_len);
    let r = run_one(
        l.rust_bin2hex,
        buf_len,
        hex_off,
        hex_maxlen,
        bin,
        bin_off,
        bin_len,
    );

    assert_eq!(
        c.returned_hex_ptr, r.returned_hex_ptr,
        "[{label}] return-pointer identity differs: C={} Rust={}",
        c.returned_hex_ptr, r.returned_hex_ptr
    );
    assert!(
        c.returned_hex_ptr,
        "[{label}] sanity: C must return the hex argument"
    );

    if c.buf != r.buf {
        let first = c
            .buf
            .iter()
            .zip(r.buf.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "[{label}] output differs at byte {first}\n  \
             bin_len={bin_len} hex_maxlen={hex_maxlen} buf_len={buf_len} \
             hex_off={hex_off} bin_off={bin_off}\n  \
             bin   = {:02x?}\n  C     = {:02x?}\n  Rust  = {:02x?}",
            &bin[bin_off..bin_off + bin_len.min(bin.len().saturating_sub(bin_off))],
            &c.buf[..c.buf.len().min(96)],
            &r.buf[..r.buf.len().min(96)],
        );
    }
}

/// Convenience wrapper for the common case: tight buffer, no offsets.
pub fn assert_same_simple(label: &str, bin: &[u8], hex_maxlen: usize, slack: usize) {
    let buf_len = bin.len() * 2 + 1 + slack;
    assert_same(label, buf_len, 0, hex_maxlen, bin, 0, bin.len());
}

// ---------------------------------------------------------------------------
// Differential driver for the ERROR path (abort / fatal signal expected)
// ---------------------------------------------------------------------------

/// How a forked child ended.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Term {
    Exited(i32),
    Signaled(i32),
}

/// Disable core dumps for this process (and therefore for every child it
/// forks). Without this, each `abort()`/`SIGSEGV` in a child is handed to the
/// system core-dump helper (`/proc/sys/kernel/core_pattern` is a pipe to
/// `systemd-coredump` here), which costs ~0.5s per case and makes the
/// error-path sweeps take tens of minutes.
fn disable_core_dumps() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        let rl = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        libc::setrlimit(libc::RLIMIT_CORE, &rl);
    });
}

/// Fork, invoke `f` with raw arguments in the child, and report how the child
/// terminated. The child calls `_exit(0)` if the call returns normally, so
/// `Exited(0)` means "no rejection".
///
/// Forking is the only way to observe `abort()` differentially: both the C and
/// the Rust implementation terminate the process, so they must be run in
/// throwaway children.
pub fn term_of(f: Bin2HexFn, hex: *mut i8, hex_maxlen: usize, bin: *const u8, bin_len: usize) -> Term {
    disable_core_dumps();
    unsafe {
        // Flush so the child does not duplicate buffered test output.
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Suppress core-dump generation for THIS child only. The host's
            // `/proc/sys/kernel/core_pattern` pipes to `systemd-coredump`,
            // which costs ~0.3-0.5s per abort even with RLIMIT_CORE=0 and
            // would make the error-path sweeps take tens of minutes.
            // `PR_SET_DUMPABLE = 0` makes the kernel skip the dump while
            // leaving the termination signal itself untouched, so
            // `WIFSIGNALED`/`WTERMSIG` still report the true signal.
            libc::prctl(libc::PR_SET_DUMPABLE, 0);
            let _ = f(hex, hex_maxlen, bin, bin_len);
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Term::Signaled(libc::WTERMSIG(status))
        } else if libc::WIFEXITED(status) {
            Term::Exited(libc::WEXITSTATUS(status))
        } else {
            panic!("child neither exited nor signaled: status={status}")
        }
    }
}

/// Describes how the `hex` pointer for an error-path case is produced.
#[derive(Clone, Copy)]
pub enum Ptr {
    /// A real, writable buffer of the given size.
    Buf(usize),
    Null,
}

fn materialize(p: Ptr) -> (Option<Vec<u8>>, *mut u8) {
    match p {
        Ptr::Null => (None, std::ptr::null_mut()),
        Ptr::Buf(n) => {
            let mut v = vec![SENTINEL; n.max(1)];
            let ptr = v.as_mut_ptr();
            (Some(v), ptr)
        }
    }
}

/// Run one error-path row against both `.so`s and assert identical termination.
pub fn assert_same_term(
    label: &str,
    hex: Ptr,
    hex_maxlen: usize,
    bin: Ptr,
    bin_len: usize,
    expected: Term,
) {
    let l = libs();

    let (_hk, hp) = materialize(hex);
    let (_bk, bp) = materialize(bin);
    let c = term_of(l.c_bin2hex, hp as *mut i8, hex_maxlen, bp as *const u8, bin_len);

    let (_hk2, hp2) = materialize(hex);
    let (_bk2, bp2) = materialize(bin);
    let r = term_of(
        l.rust_bin2hex,
        hp2 as *mut i8,
        hex_maxlen,
        bp2 as *const u8,
        bin_len,
    );

    assert_eq!(
        c, r,
        "[{label}] termination differs: C={c:?} Rust={r:?} (hex_maxlen={hex_maxlen} bin_len={bin_len})"
    );
    assert_eq!(
        c, expected,
        "[{label}] C behaviour is not what ERRORS.md claims: got {c:?}, expected {expected:?}"
    );
}

pub const SIGABRT: i32 = 6;
pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

/// `18446744073709551615UL / 2`
pub const LIMIT: usize = 9223372036854775807;
