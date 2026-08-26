//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading`; the
//! Rust functions are never called directly, so the `#[no_mangle] extern "C"`
//! export wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use libloading::{Library, Symbol};

/// `char *bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len);`
pub type Bin2Hex =
    unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Path of the C shared object built by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_CDYLIB") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("c_src/build/libtranslated_rust.so");
    assert!(
        p.exists(),
        "C shared object not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path of the Rust `cdylib`.
///
/// Prefers an up-to-date artifact produced by `cargo build` (`target/{debug,
/// release}/libbin2hex_lib.so`).  `cargo test` does not build `cdylib` targets,
/// so if none is present (or it is older than `src/lib.rs`) the harness compiles
/// one with `rustc`, forwarding the feature cfgs this test binary was built
/// with, and mirroring `[lib] name`/`crate-type` from `Cargo.toml`.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_CDYLIB") {
            return PathBuf::from(p);
        }
        let root = manifest_dir();
        let src = root.join("src/lib.rs");
        let src_mtime = mtime(&src).expect("src/lib.rs must exist");

        let mut best: Option<(SystemTime, PathBuf)> = None;
        for profile in ["debug", "release"] {
            let cand = root.join("target").join(profile).join("libbin2hex_lib.so");
            if let Some(m) = mtime(&cand) {
                if m >= src_mtime && best.as_ref().map_or(true, |(bm, _)| m > *bm) {
                    best = Some((m, cand));
                }
            }
        }
        if let Some((_, p)) = best {
            return p;
        }

        // Fallback: build the cdylib ourselves.
        let out_dir = root.join("target/test-cdylib");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out = out_dir.join("libbin2hex_lib.so");
        let mut cmd = std::process::Command::new("rustc");
        cmd.arg("--edition=2021")
            .arg("--crate-type=cdylib")
            .arg("--crate-name=bin2hex_lib")
            .arg("-Cdebuginfo=0")
            // The C library has no debug instrumentation; Rust's UB checks would
            // turn the C's null-deref UB into a controlled abort instead of a
            // segfault.  Mirrors the `[profile.*]` settings in Cargo.toml.
            .arg("-Cdebug-assertions=off")
            .arg("-Coverflow-checks=off")
            .arg(&src)
            .arg("-o")
            .arg(&out);
        for f in enabled_features() {
            cmd.arg("--cfg").arg(format!("feature=\"{f}\""));
        }
        let st = cmd.status().expect("failed to spawn rustc");
        assert!(st.success(), "rustc failed to build the Rust cdylib: {st:?}");
        out
    })
    .clone()
}

/// Cargo features this test binary was compiled with.  The crate currently
/// declares no `[features]`, so this is empty for every valid configuration;
/// it is kept so the harness stays correct if features are ever added.
pub fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}

pub struct Impls {
    _c_lib: Library,
    _r_lib: Library,
    /// `bin2hex` from the C shared object (ground truth).
    pub c: Bin2Hex,
    /// `bin2hex` from the Rust shared object (under test).
    pub r: Bin2Hex,
}

// `libloading::Library` is Send+Sync; the raw fn pointers are plain data.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

/// Loads both shared objects once per test process.
pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let r_lib = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
        let c_sym: Symbol<Bin2Hex> = c_lib
            .get(b"bin2hex\0")
            .unwrap_or_else(|e| panic!("dlsym bin2hex in C .so: {e}"));
        let r_sym: Symbol<Bin2Hex> = r_lib
            .get(b"bin2hex\0")
            .unwrap_or_else(|e| panic!("dlsym bin2hex in Rust .so: {e}"));
        let c = *c_sym;
        let r = *r_sym;
        Impls { _c_lib: c_lib, _r_lib: r_lib, c, r }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------

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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Value in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.next_u8();
        }
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// One differential invocation.
///
/// * a destination buffer of `buf_total` bytes, pre-filled with `fill`, is
///   allocated separately for C and for Rust;
/// * `hex` points `hex_off` bytes into that buffer;
/// * `bin` points `bin_off` bytes into a copy of `bin_src`;
/// * `bin_len` / `hex_maxlen` are passed through verbatim (they are *not*
///   derived from the slice lengths, so out-of-spec combinations can be tested).
///
/// Asserts that C and Rust agree on the returned pointer, on **every** byte of
/// the destination buffer (including untouched sentinel bytes) and that neither
/// modified its source buffer.
#[allow(clippy::too_many_arguments)]
pub fn diff_call(
    label: &str,
    bin_src: &[u8],
    bin_off: usize,
    bin_len: usize,
    hex_maxlen: usize,
    buf_total: usize,
    hex_off: usize,
    fill: u8,
) {
    let f = impls();

    let c_bin = bin_src.to_vec();
    let r_bin = bin_src.to_vec();
    let mut c_buf = vec![fill; buf_total];
    let mut r_buf = vec![fill; buf_total];

    let (c_ret, r_ret) = unsafe {
        let c_hex = c_buf.as_mut_ptr().add(hex_off) as *mut c_char;
        let r_hex = r_buf.as_mut_ptr().add(hex_off) as *mut c_char;
        let c_ret = (f.c)(c_hex, hex_maxlen, c_bin.as_ptr().add(bin_off), bin_len);
        let r_ret = (f.r)(r_hex, hex_maxlen, r_bin.as_ptr().add(bin_off), bin_len);
        (c_ret as usize - c_buf.as_ptr() as usize, r_ret as usize - r_buf.as_ptr() as usize)
    };

    // C19: `return hex;`
    assert_eq!(
        c_ret, hex_off,
        "[{label}] C returned an unexpected pointer offset (bug in the test)"
    );
    assert_eq!(
        r_ret, c_ret,
        "[{label}] returned pointer mismatch: C -> buf+{c_ret}, Rust -> buf+{r_ret}"
    );

    if c_buf != r_buf {
        let at = c_buf
            .iter()
            .zip(r_buf.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "[{label}] output mismatch at byte {at}\n  bin_len={bin_len} hex_maxlen={hex_maxlen} \
             buf_total={buf_total} hex_off={hex_off} bin_off={bin_off} fill={fill:#04x}\n  \
             bin  = {:02x?}\n  C    = {:02x?}\n  Rust = {:02x?}",
            &bin_src[bin_off.min(bin_src.len())
                ..bin_off.min(bin_src.len()) + bin_len.min(bin_src.len().saturating_sub(bin_off))],
            &c_buf[..],
            &r_buf[..]
        );
    }

    assert_eq!(c_bin, r_bin, "[{label}] source buffers diverged");
    assert_eq!(
        &c_bin[..],
        bin_src,
        "[{label}] C modified its (const) source buffer"
    );
}

/// Convenience wrapper: exact-minimum destination buffer, zero-filled.
pub fn diff_exact(label: &str, bin: &[u8]) {
    let need = bin.len() * 2 + 1;
    diff_call(label, bin, 0, bin.len(), need, need, 0, 0x00);
}

/// Convenience wrapper: destination buffer with `slack` extra sentinel bytes.
pub fn diff_slack(label: &str, bin: &[u8], slack: usize, fill: u8) {
    let need = bin.len() * 2 + 1;
    diff_call(label, bin, 0, bin.len(), need + slack, need + slack, 0, fill);
}

// ---------------------------------------------------------------------------
// Fork helper for the crashing / aborting paths
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
    Unknown(i32),
}

impl Outcome {
    pub fn describe(self) -> String {
        match self {
            Outcome::Exited(c) => format!("exited({c})"),
            Outcome::Signaled(s) => format!("killed by signal {s} ({})", signame(s)),
            Outcome::Unknown(st) => format!("unknown wait status {st:#x}"),
        }
    }
}

pub fn signame(s: i32) -> &'static str {
    match s {
        libc::SIGABRT => "SIGABRT",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGBUS => "SIGBUS",
        libc::SIGILL => "SIGILL",
        libc::SIGFPE => "SIGFPE",
        libc::SIGKILL => "SIGKILL",
        _ => "other",
    }
}

/// Runs `f` in a forked child (core dumps disabled) and reports how it died.
pub fn forked<F: FnOnce()>(f: F) -> Outcome {
    // Make sure nothing buffered in the parent gets flushed twice.
    use std::io::Write;
    std::io::stdout().flush().ok();
    std::io::stderr().flush().ok();

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let rl = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &rl);
            f();
            libc::_exit(0);
        }
        let mut status: i32 = 0;
        loop {
            let r = libc::waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            if r < 0 && *libc::__errno_location() == libc::EINTR {
                continue;
            }
            panic!("waitpid failed");
        }
        if libc::WIFEXITED(status) {
            Outcome::Exited(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else {
            Outcome::Unknown(status)
        }
    }
}

/// Calls `bin2hex` from the C `.so` in a forked child.
pub fn forked_c(hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize) -> Outcome {
    let f = impls();
    let (hex, bin) = (hex as usize, bin as usize);
    forked(move || unsafe {
        let p = (f.c)(hex as *mut c_char, hex_maxlen, bin as *const u8, bin_len);
        // Keep the call observable so nothing can be optimised away.
        std::ptr::read_volatile(&p);
    })
}

/// Calls `bin2hex` from the Rust `.so` in a forked child.
pub fn forked_r(hex: *mut c_char, hex_maxlen: usize, bin: *const u8, bin_len: usize) -> Outcome {
    let f = impls();
    let (hex, bin) = (hex as usize, bin as usize);
    forked(move || unsafe {
        let p = (f.r)(hex as *mut c_char, hex_maxlen, bin as *const u8, bin_len);
        std::ptr::read_volatile(&p);
    })
}

/// Differential check of a crashing/aborting configuration: C and Rust must die
/// (or survive) in *exactly* the same way.
pub fn diff_outcome(
    label: &str,
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> Outcome {
    let c = forked_c(hex, hex_maxlen, bin, bin_len);
    let r = forked_r(hex, hex_maxlen, bin, bin_len);
    assert_eq!(
        c,
        r,
        "[{label}] outcome mismatch: C {} vs Rust {} (hex={hex:p} hex_maxlen={hex_maxlen} \
         bin={bin:p} bin_len={bin_len})",
        c.describe(),
        r.describe()
    );
    c
}

pub const SIZE_MAX: usize = usize::MAX;
/// `(18446744073709551615UL) / 2` from the C source.
pub const C_LIMIT: usize = 18446744073709551615u64 as usize / 2;
