//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded with `libloading` and every call goes through an
//! exported symbol looked up by name -- the Rust functions are never called
//! directly, so the `#[no_mangle]` / `extern "C"` wrappers are under test too.
//!
//! Paths and the active build configuration arrive through the environment,
//! set by `run_tests.sh`:
//!
//! ```text
//! MD_C_SO     cbuild/<op>_<rep>/libmdcore.so
//! MD_RUST_SO  cbuild/<op>_<rep>/libdriver.so   (copy of target/release/libdriver.so)
//! MD_C_BIN    cbuild/<op>_<rep>/driver
//! MD_RUST_BIN cbuild/<op>_<rep>/driver_rs
//! MD_OP       add | sub | mul
//! MD_REPEAT   0 .. 7
//! ```

#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

use libloading::{Library, Symbol};

pub type Op2 = extern "C" fn(c_int, c_int) -> c_int;
pub type Op1 = extern "C" fn(c_int) -> c_int;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C output stream -- needed because the C
    /// `.so`'s `stdout` is fully buffered once fd 1 points at a regular file.
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Outcome of a forked probe: either a clean exit code or a fatal signal.
#[derive(Debug, PartialEq, Eq)]
pub enum Probe {
    Exited(c_int),
    Signaled(c_int),
}

/// Runs `f` in a forked child and reports how the child terminated.
///
/// Used to compare *fatal* behaviour (e.g. storing into a `.rodata` string) that
/// cannot be observed in-process. The child does nothing but run `f` and `_exit`,
/// so the usual post-`fork` restrictions are respected.
pub fn probe_in_child<F: FnOnce()>(f: F) -> Probe {
    // SAFETY: single `fork`; the child performs only `f` and `_exit`, never
    // returning into the test harness or touching its allocator locks.
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    // SAFETY: `pid` is our direct child; `status` is a valid out-param.
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    if status & 0x7f != 0 {
        Probe::Signaled(status & 0x7f)
    } else {
        Probe::Exited((status >> 8) & 0xff)
    }
}

fn env_path(key: &str) -> PathBuf {
    PathBuf::from(
        std::env::var(key)
            .unwrap_or_else(|_| panic!("{key} must be set; run the suite via ./run_tests.sh")),
    )
}

/// The build configuration the C `.so` and the Rust `.so` were both built for.
pub struct Config {
    pub op: String,
    pub repeat: c_int,
}

impl Config {
    pub fn from_env() -> Self {
        let op = std::env::var("MD_OP").unwrap_or_else(|_| "add".to_string());
        let repeat: c_int = std::env::var("MD_REPEAT")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .expect("MD_REPEAT must be an integer");
        assert!(
            matches!(op.as_str(), "add" | "sub" | "mul"),
            "MD_OP must be add/sub/mul, got {op}"
        );
        assert!(
            (0..=7).contains(&repeat),
            "REPEAT is bounded by the REP0..REP7 macros; got {repeat}"
        );
        Config { op, repeat }
    }

    /// `INIT_FOR(OP)` -- `INIT_mul` is 1, the others 0 (`mdmacros.h:56-58`).
    pub fn init(&self) -> c_int {
        if self.op == "mul" {
            1
        } else {
            0
        }
    }
}

/// One handle pair. `Library` must outlive every `Symbol` taken from it, so
/// callers borrow symbols inside a closure/scope rather than storing them.
pub struct Pair {
    pub c: Library,
    pub rust: Library,
    pub cfg: Config,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = env_path("MD_C_SO");
        let rust_path = env_path("MD_RUST_SO");
        // SAFETY: both paths are build artifacts of this repository; loading them
        // runs their (trivial, statically initialized) constructors.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Pair {
            c,
            rust,
            cfg: Config::from_env(),
        }
    }

    pub fn fn2(&self, which: Side, name: &str) -> Symbol<'_, Op2> {
        let lib = self.lib(which);
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        // SAFETY: every one of op_add/op_sub/op_mul/helper_call/helper_ptr is
        // `int (*)(int, int)` in `mdmacros.h`.
        unsafe { lib.get(&sym) }.unwrap_or_else(|e| panic!("{which:?} {name}: {e}"))
    }

    pub fn fn1(&self, which: Side, name: &str) -> Symbol<'_, Op1> {
        let lib = self.lib(which);
        let mut sym = name.as_bytes().to_vec();
        sym.push(0);
        // SAFETY: `use_generated` is `int (int)`.
        unsafe { lib.get(&sym) }.unwrap_or_else(|e| panic!("{which:?} {name}: {e}"))
    }

    /// Reads the `G_OP` data export: `int (*G_OP)(int,int)`.
    pub fn g_op(&self, which: Side) -> Op2 {
        let lib = self.lib(which);
        // SAFETY: libloading hands back the symbol's address reinterpreted as
        // `*mut T`; `G_OP` is a single function-pointer-sized slot. `Option<fn>`
        // is used so a null global would be observable rather than UB.
        let sym: Symbol<'_, *mut Option<Op2>> =
            unsafe { lib.get(b"G_OP\0") }.unwrap_or_else(|e| panic!("{which:?} G_OP: {e}"));
        unsafe { **sym }.expect("G_OP must not be NULL")
    }

    /// Reads the `G_OP_NAME` data export: `const char *G_OP_NAME`, returned as
    /// the NUL-terminated bytes it points at (NUL excluded).
    pub fn g_op_name(&self, which: Side) -> Vec<u8> {
        let lib = self.lib(which);
        // SAFETY: as above; `G_OP_NAME` is one `const char *` slot.
        let sym: Symbol<'_, *mut *const c_char> = unsafe { lib.get(b"G_OP_NAME\0") }
            .unwrap_or_else(|e| panic!("{which:?} G_OP_NAME: {e}"));
        let p = unsafe { **sym };
        assert!(!p.is_null(), "{which:?} G_OP_NAME must not be NULL");
        let mut out = Vec::new();
        let mut i = 0isize;
        loop {
            // SAFETY: the target is a NUL-terminated string literal produced by
            // `STR(OP)`; the loop stops at the terminator.
            let b = unsafe { *p.offset(i) } as u8;
            if b == 0 {
                break;
            }
            out.push(b);
            i += 1;
            assert!(i < 4096, "G_OP_NAME is not NUL-terminated");
        }
        out
    }

    fn lib(&self, which: Side) -> &Library {
        match which {
            Side::C => &self.c,
            Side::Rust => &self.rust,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Side {
    C,
    Rust,
}

/// Runs `f` with fd 1 pointed at a fresh temp file and returns everything
/// written to it, so the `printf` output of each `.so` can be diffed too.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "mddiff-{}-{}-{}.out",
        std::process::id(),
        tag,
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    // SAFETY: plain fd juggling; `saved` is restored before returning and the
    // suite is single-threaded per test binary invocation for these captures.
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    // The C library's stdout is fully buffered against a regular file.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// SplitMix64 -- fixed seed so any divergence reproduces exactly.
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        Rng(0x5EED_1234_ABCD_0001)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// A mix of full-range values and small ones, so both the wide arithmetic
    /// and the near-zero paths get hit.
    pub fn next_mixed_i32(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 4 {
            0 => (r >> 32) as u32 as c_int,
            1 => ((r >> 32) as u32 % 65) as c_int - 32,
            2 => c_int::MAX - ((r >> 32) as u32 % 8) as c_int,
            _ => c_int::MIN + ((r >> 32) as u32 % 8) as c_int,
        }
    }
}

/// The boundary grid used by every `op_*` / `helper_*` row.
pub const GRID: [c_int; 13] = [
    c_int::MIN,
    c_int::MIN + 1,
    -65536,
    -46341,
    -2,
    -1,
    0,
    1,
    2,
    46341,
    65536,
    c_int::MAX - 1,
    c_int::MAX,
];

pub fn c_bin() -> PathBuf {
    env_path("MD_C_BIN")
}

pub fn rust_bin() -> PathBuf {
    env_path("MD_RUST_BIN")
}
