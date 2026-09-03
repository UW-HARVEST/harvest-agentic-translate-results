//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared objects* via `libloading` and
//! driven only through their exported C symbols — the Rust crate is never
//! called directly, so the `#[no_mangle] extern "C"` wrappers are part of what
//! is under test.
//!
//! * the C `.so` is produced on demand with `gcc -fPIC -shared` from
//!   `c_src/src/mdcore.c` (`c_src` itself is never modified; `CMakeLists.txt`
//!   only builds an executable, and `mdmain.c` is the half that holds `main`);
//! * the Rust `.so` is the crate's own `cdylib`, located relative to the test
//!   executable so it always matches the feature set the test was built with.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/* ------------------------------------------------------------------ */
/* Build-time configuration, resolved exactly like `src/mdconfig.rs`. */
/* ------------------------------------------------------------------ */

/// `STR(OP)` for the currently selected feature set.
/// Priority mirrors `mdconfig.rs`: `add > sub > mul`, default `add`.
pub const OP: &str = if cfg!(feature = "add") {
    "add"
} else if cfg!(feature = "sub") {
    "sub"
} else if cfg!(feature = "mul") {
    "mul"
} else {
    "add"
};

/// `REPEAT` for the currently selected feature set.
/// Priority mirrors `mdconfig.rs`: lowest explicitly selected value wins,
/// default 5.
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

/// `INIT_FOR(OP)`
pub const INIT: c_int = if OP.as_bytes()[0] == b'm' { 1 } else { 0 };

/* ------------------------------------------------------------------ */
/* Locating / building the two shared objects.                        */
/* ------------------------------------------------------------------ */

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_src_root() -> PathBuf {
    crate_root().parent().unwrap().join("c_src")
}

/// `target/<profile>/` — derived from the test executable (`.../deps/foo-hash`).
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    if deps.file_name().map(|n| n == "deps").unwrap_or(false) {
        deps.parent().expect("profile dir").to_path_buf()
    } else {
        deps.to_path_buf()
    }
}

/// The crate's own `cdylib`, built with the same features as this test.
pub fn rust_lib_path() -> PathBuf {
    let p = target_profile_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {}. Run `cargo build` for the same feature set first.",
        p.display()
    );
    p
}

/// Compile `c_src/src/mdcore.c` into a shared object for the current
/// `(OP, REPEAT)` configuration. Cached per configuration on disk.
pub fn c_lib_path() -> PathBuf {
    let dir = target_profile_dir().join("cdiff");
    std::fs::create_dir_all(&dir).expect("create cdiff dir");
    let out = dir.join(format!("libcmd_{}_{}.so", OP, REPEAT));

    let src = c_src_root().join("src").join("mdcore.c");
    assert!(src.exists(), "missing {}", src.display());

    if needs_rebuild(&out, &src) {
        let status = Command::new(cc())
            .arg("-O2")
            .arg("-fPIC")
            .arg("-shared")
            .arg(format!("-DOP={}", OP))
            .arg(format!("-DREPEAT={}", REPEAT))
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .status()
            .expect("failed to spawn C compiler");
        assert!(status.success(), "compiling {} failed", src.display());
    }
    out
}

/// Build the C `driver` executable (`mdcore.c` + `mdmain.c`) for the current
/// configuration, for whole-program comparison against the Rust `driver` bin.
pub fn c_exe_path() -> PathBuf {
    let dir = target_profile_dir().join("cdiff");
    std::fs::create_dir_all(&dir).expect("create cdiff dir");
    let out = dir.join(format!("cdriver_{}_{}", OP, REPEAT));

    let core = c_src_root().join("src").join("mdcore.c");
    let main = c_src_root().join("src").join("mdmain.c");

    if needs_rebuild(&out, &core) || needs_rebuild(&out, &main) {
        let status = Command::new(cc())
            .arg("-O2")
            .arg(format!("-DOP={}", OP))
            .arg(format!("-DREPEAT={}", REPEAT))
            .arg("-o")
            .arg(&out)
            .arg(&core)
            .arg(&main)
            .status()
            .expect("failed to spawn C compiler");
        assert!(status.success(), "compiling the C driver failed");
    }
    out
}

/// The Rust `driver` executable built with this test's feature set.
pub fn rust_exe_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
    assert!(p.exists(), "Rust driver binary not found at {}", p.display());
    p
}

fn cc() -> String {
    std::env::var("CC").unwrap_or_else(|_| "cc".to_string())
}

fn needs_rebuild(out: &Path, src: &Path) -> bool {
    let om = out.metadata().and_then(|m| m.modified()).ok();
    let sm = src.metadata().and_then(|m| m.modified()).ok();
    match (om, sm) {
        (Some(o), Some(s)) => s > o,
        _ => true,
    }
}

/* ------------------------------------------------------------------ */
/* The loaded pair.                                                   */
/* ------------------------------------------------------------------ */

type Bin2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Un1 = unsafe extern "C" fn(c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn bin2(&self, sym: &str) -> Symbol<'_, Bin2> {
        unsafe { self.lib.get(sym.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, sym, e))
    }

    pub fn un1(&self, sym: &str) -> Symbol<'_, Un1> {
        unsafe { self.lib.get(sym.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, sym, e))
    }

    /// Address stored *in* the `G_OP` data slot (a function pointer variable).
    pub fn g_op(&self) -> Bin2 {
        let slot: Symbol<'_, *mut Bin2> = unsafe { self.lib.get(b"G_OP") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `G_OP`: {}", self.name, e));
        unsafe { **slot }
    }

    /// Raw address of a symbol, for pointer-identity checks.
    pub fn addr(&self, sym: &str) -> usize {
        let s: Symbol<'_, *mut c_void> = unsafe { self.lib.get(sym.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol `{}`: {}", self.name, sym, e));
        unsafe { s.into_raw().into_raw() as usize }
    }

    /// Bytes of the C string that `G_OP_NAME` points at, including the NUL.
    pub fn g_op_name(&self) -> Vec<u8> {
        let slot: Symbol<'_, *mut *const c_char> = unsafe { self.lib.get(b"G_OP_NAME") }
            .unwrap_or_else(|e| panic!("{}: missing symbol `G_OP_NAME`: {}", self.name, e));
        unsafe {
            let p = **slot;
            assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.name);
            let mut v = Vec::new();
            let mut i = 0isize;
            loop {
                let b = *p.offset(i) as u8;
                v.push(b);
                if b == 0 {
                    break;
                }
                i += 1;
                assert!(i < 64, "{}: G_OP_NAME not NUL-terminated", self.name);
            }
            v
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub r: Impl,
}

/// Load both shared objects. `RTLD_LOCAL` (libloading's default) keeps the two
/// symbol namespaces separate so `op_add` from one cannot shadow the other.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        let c = unsafe { Library::new(&cp) }.expect("dlopen C .so");
        let r = unsafe { Library::new(&rp) }.expect("dlopen Rust .so");
        Pair {
            c: Impl { name: "C", lib: c },
            r: Impl { name: "Rust", lib: r },
        }
    })
}

/* ------------------------------------------------------------------ */
/* stdout capture (the helpers printf, so output is part of the ABI).  */
/* ------------------------------------------------------------------ */

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Run `f` with fd 1 redirected to a temp file and return everything written.
///
/// `fflush(NULL)` flushes *every* libc stream, which is what makes the C `.so`'s
/// buffered `printf` output observable; the Rust side flushes explicitly inside
/// `out()`, so nothing is left behind on either side.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};

    // fd 1 is process-global: only one capture may be active at a time.
    // (Run with `--test-threads=1` for fully deterministic captures.)
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = target_profile_dir().join("cdiff");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join(format!(
        "cap_{}_{}_{:?}.txt",
        OP,
        REPEAT,
        std::thread::current().id()
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let out = unsafe {
        use std::os::unix::io::AsRawFd;
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        let ret = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        ret
    };

    file.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).expect("read capture");
    let _ = std::fs::remove_file(&path);
    (out, buf)
}

/* ------------------------------------------------------------------ */
/* Deterministic PRNG (SplitMix64) — fixed seeds, reproducible runs.  */
/* ------------------------------------------------------------------ */

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

    /// Full-range `i32`, biased towards small magnitudes and the extremes so
    /// that both ordinary and overflowing operands are hit often.
    pub fn next_i32(&mut self) -> c_int {
        let v = self.next_u64();
        match v & 7 {
            0 => (v >> 3) as i16 as c_int,
            1 => ((v >> 3) & 0xFF) as c_int,
            2 => -(((v >> 3) & 0xFF) as c_int),
            3 => i32::MAX - ((v >> 3) & 3) as c_int,
            4 => i32::MIN + ((v >> 3) & 3) as c_int,
            _ => (v >> 3) as u32 as c_int,
        }
    }
}

/// The full set of interesting `i32` boundary values.
pub const EDGE_I32: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    7,
    -7,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x7FFF,
    -0x8000,
    0x10000,
    -0x10000,
    65535,
    46341,
    -46341,
];
