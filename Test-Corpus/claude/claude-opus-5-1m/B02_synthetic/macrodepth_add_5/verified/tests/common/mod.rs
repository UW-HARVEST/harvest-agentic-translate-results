//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are always reached through their **shared objects**,
//! loaded with `libloading` and called by exported symbol name — never by
//! calling the Rust functions directly. That way the `#[no_mangle]`/`extern "C"`
//! wrappers, the ABI and the data-symbol storage classes are all under test,
//! exactly as an external consumer would see them.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/* ------------------------------------------------------------------ */
/* The configuration this test binary was compiled for                */
/* ------------------------------------------------------------------ */

/// `OP`, resolved with the same precedence the library uses (mul > sub > add),
/// falling back to the `#ifndef OP` default `add` when no feature is selected.
pub const OP: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// `REPEAT`, first-feature-wins like the library, `#ifndef REPEAT` default `5`.
pub const REPEAT: c_int = if cfg!(feature = "0") {
    0
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

/// `true` when no `OP` feature was given, i.e. we are testing the
/// `#ifndef OP` fallback. The C side is then compiled with **no** `-DOP`.
pub const OP_UNSET: bool =
    !cfg!(feature = "add") && !cfg!(feature = "sub") && !cfg!(feature = "mul");

/// `true` when no `REPEAT` feature was given (`#ifndef REPEAT` fallback).
pub const REPEAT_UNSET: bool = !cfg!(feature = "0")
    && !cfg!(feature = "1")
    && !cfg!(feature = "2")
    && !cfg!(feature = "3")
    && !cfg!(feature = "4")
    && !cfg!(feature = "5")
    && !cfg!(feature = "6")
    && !cfg!(feature = "7");

/// `INIT_FOR(OP)`
pub const fn init_for_op() -> c_int {
    match OP.as_bytes()[0] {
        b'm' => 1, // INIT_mul
        _ => 0,    // INIT_add / INIT_sub
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("ctest");
    std::fs::create_dir_all(&d).expect("create target/ctest");
    d
}

/* ------------------------------------------------------------------ */
/* Building the C artifacts                                          */
/* ------------------------------------------------------------------ */

/// The `-D` flags CMake would pass: `CMAKE_C_FLAGS "-DOP=${OP} -DREPEAT=${REPEAT}"`.
/// When the corresponding Cargo feature is unset we deliberately omit the flag so
/// that the C preprocessor takes its own `#ifndef` fallback, mirroring the Rust.
fn c_defines() -> Vec<String> {
    let mut v = Vec::new();
    if !OP_UNSET {
        v.push(format!("-DOP={OP}"));
    }
    if !REPEAT_UNSET {
        v.push(format!("-DREPEAT={REPEAT}"));
    }
    v
}

fn cfg_tag() -> String {
    format!(
        "{}_{}",
        if OP_UNSET {
            "opdefault".to_string()
        } else {
            OP.to_string()
        },
        if REPEAT_UNSET {
            "repdefault".to_string()
        } else {
            REPEAT.to_string()
        }
    )
}

/// Compile `srcs` with gcc into `out`, atomically (unique temp + rename) so that
/// concurrently running test binaries cannot observe a half-written file.
fn gcc_build(out: &Path, shared: bool, srcs: &[&str]) {
    if out.exists() {
        return;
    }
    let tmp = out.with_extension(format!("tmp{}", std::process::id()));
    let mut cmd = Command::new("gcc");
    if shared {
        cmd.arg("-shared");
    }
    cmd.arg("-fPIC");
    for d in c_defines() {
        cmd.arg(d);
    }
    cmd.arg("-o").arg(&tmp);
    let root = manifest_dir();
    for s in srcs {
        cmd.arg(root.join(s));
    }
    let st = cmd.output().expect("run gcc");
    assert!(
        st.status.success(),
        "gcc failed for {:?}:\n{}",
        out,
        String::from_utf8_lossy(&st.stderr)
    );
    // rename is atomic on the same filesystem; a losing racer just overwrites
    // with an identical artifact.
    let _ = std::fs::rename(&tmp, out);
    assert!(out.exists(), "gcc produced no output at {out:?}");
}

/// Path to the C **library** `.so` (the `mdcore.c` translation unit — the set of
/// entities declared `extern` in the public header `mdmacros.h`).
pub fn c_lib_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let out = out_dir().join(format!("libcdriver_{}.so", cfg_tag()));
        gcc_build(&out, true, &["c_src/src/mdcore.c"]);
        out
    })
}

/// Path to the C **executable** (`mdcore.c` + `mdmain.c`).
pub fn c_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let out = out_dir().join(format!("cdriver_{}", cfg_tag()));
        gcc_build(&out, false, &["c_src/src/mdcore.c", "c_src/src/mdmain.c"]);
        out
    })
}

/* ------------------------------------------------------------------ */
/* Locating the Rust artifacts                                       */
/* ------------------------------------------------------------------ */

fn rust_artifact(name: &str) -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib and the driver binary are two levels up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join(name);
    assert!(
        p.exists(),
        "missing Rust artifact {p:?}.\n\
         Build it for the same feature set first, e.g.\n  \
         cargo build --no-default-features --features <combo> --lib --bin driver\n\
         (./check_all.sh does this automatically)."
    );
    assert_fresh(&p);
    p
}

/// Refuse to test a Rust artifact older than the newest source file.
///
/// `cargo test` does not necessarily relink the `cdylib`/`bin` targets, so
/// without this guard an edit that was never compiled — or a leftover artifact
/// whose source has since been restored with its original timestamps — would be
/// tested instead of the current source, and the suite would report a pass for
/// code that is not the code under test.
fn assert_fresh(artifact: &Path) {
    fn mtime(p: &Path) -> Option<std::time::SystemTime> {
        std::fs::metadata(p).ok()?.modified().ok()
    }
    let art = match mtime(artifact) {
        Some(t) => t,
        None => return,
    };

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let root = manifest_dir();
    let mut stack = vec![root.join("src")];
    let mut files = vec![root.join("Cargo.toml")];
    while let Some(d) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    files.push(p);
                }
            }
        }
    }
    for f in files {
        if let Some(t) = mtime(&f) {
            if newest.as_ref().is_none_or(|(bt, _)| t > *bt) {
                newest = Some((t, f));
            }
        }
    }

    if let Some((t, f)) = newest {
        assert!(
            art >= t,
            "stale artifact: {artifact:?} is older than {f:?}.\n\
             Rebuild for this feature set before testing:\n  \
             cargo build --no-default-features --features <combo> --lib --bin driver"
        );
    }
}

/// Path to the Rust `cdylib`.
pub fn rust_lib_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| rust_artifact("libdriver.so"))
}

/// Path to the Rust `driver` executable.
pub fn rust_exe_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| rust_artifact("driver"))
}

/* ------------------------------------------------------------------ */
/* Loaded library wrapper                                            */
/* ------------------------------------------------------------------ */

pub type BinFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type UnFn = unsafe extern "C" fn(c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    lib: libloading::Library,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {path:?} ({name}): {e}"));
        Lib { name, lib }
    }

    /// Function symbol lookup by exported name.
    pub fn func2(&self, sym: &str) -> BinFn {
        unsafe {
            *self
                .lib
                .get::<BinFn>(format!("{sym}\0").as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol {sym}: {e}", self.name))
                .into_raw()
        }
    }

    pub fn func1(&self, sym: &str) -> UnFn {
        unsafe {
            *self
                .lib
                .get::<UnFn>(format!("{sym}\0").as_bytes())
                .unwrap_or_else(|e| panic!("{}: missing symbol {sym}: {e}", self.name))
                .into_raw()
        }
    }

    /// Address of the writable `int (*G_OP)(int,int)` global.
    pub fn g_op(&self) -> *mut BinFn {
        unsafe {
            *self
                .lib
                .get::<*mut BinFn>(b"G_OP\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol G_OP: {e}", self.name))
        }
    }

    /// Address of the writable `const char *G_OP_NAME` global.
    pub fn g_op_name(&self) -> *mut *const c_char {
        unsafe {
            *self
                .lib
                .get::<*mut *const c_char>(b"G_OP_NAME\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol G_OP_NAME: {e}", self.name))
        }
    }

    /// The NUL-terminated string currently referenced by `G_OP_NAME`.
    pub fn g_op_name_str(&self) -> Vec<u8> {
        unsafe {
            let p = *self.g_op_name();
            assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.name);
            let mut v = Vec::new();
            let mut i = 0isize;
            loop {
                let b = *p.offset(i) as u8;
                if b == 0 {
                    break;
                }
                v.push(b);
                i += 1;
                assert!(i < 4096, "unterminated G_OP_NAME");
            }
            v
        }
    }
}

/// The C library and the Rust library, both loaded (`RTLD_LOCAL`, so their
/// identically named symbols cannot shadow one another).
pub fn libs() -> &'static (Lib, Lib) {
    static L: OnceLock<(Lib, Lib)> = OnceLock::new();
    L.get_or_init(|| {
        (
            Lib::open("C", c_lib_path()),
            Lib::open("Rust", rust_lib_path()),
        )
    })
}

/* ------------------------------------------------------------------ */
/* stdout capture (the functions communicate through printf)         */
/* ------------------------------------------------------------------ */

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// fd 1 is process-global, so captures must not overlap.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` with file descriptor 1 redirected to a temp file and return
/// `(f's value, bytes written to stdout)`.
///
/// Both libraries call the very same glibc `printf`, so this compares the real
/// stdout bytes, including formatting and buffering behaviour.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom, Write};

    // fd 1 is redirected for the whole process, so libtest must not be printing
    // progress from another thread while we hold it -- that would leak harness
    // text into the captured bytes. `.cargo/config.toml` sets RUST_TEST_THREADS=1
    // for exactly this reason.
    assert_eq!(
        std::env::var("RUST_TEST_THREADS").as_deref(),
        Ok("1"),
        "these differential tests capture file descriptor 1 and therefore require \
         single-threaded execution.\nRun them via ./check_all.sh, or with \
         RUST_TEST_THREADS=1 / `cargo test -- --test-threads=1`."
    );

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Push libtest's own pending, newline-less progress text ("test foo ... ")
    // out of Rust's userspace buffer before fd 1 changes underneath it.
    let _ = std::io::stdout().flush();

    let mut tmp = tempfile();
    let fd = {
        use std::os::unix::io::AsRawFd;
        tmp.as_raw_fd()
    };

    let saved = unsafe {
        fflush(std::ptr::null_mut()); // flush everything pending first
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");
        saved
    };

    let out = f();

    unsafe {
        fflush(std::ptr::null_mut()); // stdout is a file now => fully buffered
        dup2(saved, 1);
        close(saved);
    }

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read capture");
    (out, buf)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let p = out_dir().join(format!(
        "cap_{}_{}_{}.txt",
        std::process::id(),
        cfg_tag(),
        n
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .expect("open capture temp");
    let _ = std::fs::remove_file(&p); // unlinked but still open
    f
}

/* ------------------------------------------------------------------ */
/* Differential helpers                                              */
/* ------------------------------------------------------------------ */

/// Call `sym(a, b)` in both libraries and assert the return value *and* the
/// printed bytes are identical.
pub fn diff2(sym: &str, a: c_int, b: c_int) -> c_int {
    let (c, r) = libs();
    let cf = c.func2(sym);
    let rf = r.func2(sym);
    let (cv, cout) = capture(|| unsafe { cf(a, b) });
    let (rv, rout) = capture(|| unsafe { rf(a, b) });
    assert_eq!(
        cv, rv,
        "{sym}({a}, {b}) return mismatch: C={cv} Rust={rv} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        show(&cout),
        show(&rout),
        "{sym}({a}, {b}) stdout mismatch [OP={OP} REPEAT={REPEAT}]"
    );
    cv
}

/// Call `sym(n)` in both libraries and assert return value + printed bytes match.
pub fn diff1(sym: &str, n: c_int) -> c_int {
    let (c, r) = libs();
    let cf = c.func1(sym);
    let rf = r.func1(sym);
    let (cv, cout) = capture(|| unsafe { cf(n) });
    let (rv, rout) = capture(|| unsafe { rf(n) });
    assert_eq!(
        cv, rv,
        "{sym}({n}) return mismatch: C={cv} Rust={rv} [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        show(&cout),
        show(&rout),
        "{sym}({n}) stdout mismatch [OP={OP} REPEAT={REPEAT}]"
    );
    cv
}

/// Lossless-enough rendering for assertion messages.
pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/* ------------------------------------------------------------------ */
/* Deterministic randomness                                          */
/* ------------------------------------------------------------------ */

/// Fixed seed => reproducible runs.
pub const SEED: u64 = 0x5EED_1234_5EED_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(if s == 0 { SEED } else { s })
    }
    /// xorshift64*
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// A mixture of small and full-range values, so both ordinary arithmetic and
    /// overflowing arithmetic get exercised.
    pub fn next_mixed(&mut self) -> c_int {
        let v = self.next_u64();
        match v % 4 {
            0 => (v >> 32) as u32 as c_int,        // full range
            1 => ((v >> 8) % 201) as c_int - 100,  // small, signed
            2 => ((v >> 8) % 65536) as c_int,      // medium
            _ => {
                // near the int boundaries
                let d = ((v >> 8) % 5) as c_int;
                if v & 0x10 != 0 {
                    c_int::MAX - d
                } else {
                    c_int::MIN + d
                }
            }
        }
    }
    pub fn range(&mut self, lo: c_int, hi: c_int) -> c_int {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as c_int
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new()
    }
}

/// The fixed boundary set used for cross-products.
pub const BOUNDARIES: [c_int; 9] = [
    0,
    1,
    -1,
    2,
    -2,
    c_int::MAX,
    c_int::MIN,
    c_int::MAX - 1,
    c_int::MIN + 1,
];

/// Every public function of the library surface.
pub const BIN_FUNCS: [&str; 5] = ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"];
