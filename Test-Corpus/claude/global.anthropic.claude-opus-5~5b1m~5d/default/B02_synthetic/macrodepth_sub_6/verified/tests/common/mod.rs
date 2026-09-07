//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and every call
//! goes through `dlsym`, so the Rust `#[no_mangle]` export wrappers are what is
//! under test — no Rust function is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

/* ------------------------------------------------------------------ config */

/// Resolves the build configuration the same way `src/mdconfig.rs` does, so the
/// test picks the C `.so` that was built with the matching `-DOP/-DREPEAT`.
pub fn op_name() -> &'static str {
    if cfg!(feature = "add") {
        "add"
    } else if cfg!(feature = "sub") {
        "sub"
    } else if cfg!(feature = "mul") {
        "mul"
    } else {
        "add" // #ifndef OP -> add
    }
}

pub fn repeat() -> c_int {
    for (i, on) in [
        cfg!(feature = "0"),
        cfg!(feature = "1"),
        cfg!(feature = "2"),
        cfg!(feature = "3"),
        cfg!(feature = "4"),
        cfg!(feature = "5"),
        cfg!(feature = "6"),
        cfg!(feature = "7"),
    ]
    .iter()
    .enumerate()
    {
        if *on {
            return i as c_int;
        }
    }
    5 // #ifndef REPEAT -> 5
}

/// `INIT_FOR(OP)`
pub fn init() -> c_int {
    if op_name() == "mul" {
        1
    } else {
        0
    }
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent dir")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    workspace_root().join(format!(
        "cbuild/libcdriver_{}_{}.so",
        op_name(),
        repeat()
    ))
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_SO") {
        return PathBuf::from(p);
    }
    workspace_root().join(format!("rustbuild/libdriver_{}_{}.so", op_name(), repeat()))
}

pub fn c_exe_path() -> PathBuf {
    workspace_root().join(format!("cbuild/cdriver_{}_{}", op_name(), repeat()))
}

pub fn rust_exe_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFF_RUST_EXE") {
        return PathBuf::from(p);
    }
    workspace_root().join(format!("rustbuild/rdriver_{}_{}", op_name(), repeat()))
}

/* -------------------------------------------------------------- libraries */

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(
            cp.exists(),
            "missing C shared library {} — run ./build_c.sh",
            cp.display()
        );
        assert!(
            rp.exists(),
            "missing Rust shared library {} — run ./build_rust.sh",
            rp.display()
        );
        unsafe {
            Libs {
                c: Library::new(&cp).expect("dlopen C .so"),
                rust: Library::new(&rp).expect("dlopen Rust .so"),
            }
        }
    })
}

pub type Fn2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Fn1 = unsafe extern "C" fn(c_int) -> c_int;

/// `dlsym` a two-argument function from both libraries.
pub fn sym2(name: &str) -> (Fn2, Fn2) {
    let l = libs();
    unsafe {
        let c: libloading::Symbol<Fn2> = l
            .c
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("C .so missing {name}: {e}"));
        let r: libloading::Symbol<Fn2> = l
            .rust
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("Rust .so missing {name}: {e}"));
        (*c, *r)
    }
}

/// `dlsym` a one-argument function from both libraries.
pub fn sym1(name: &str) -> (Fn1, Fn1) {
    let l = libs();
    unsafe {
        let c: libloading::Symbol<Fn1> = l
            .c
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("C .so missing {name}: {e}"));
        let r: libloading::Symbol<Fn1> = l
            .rust
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("Rust .so missing {name}: {e}"));
        (*c, *r)
    }
}

/// `dlsym` the address of an exported data object from both libraries.
pub fn data_sym(name: &str) -> (*mut u8, *mut u8) {
    let l = libs();
    unsafe {
        let c: libloading::Symbol<*mut u8> = l
            .c
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("C .so missing {name}: {e}"));
        let r: libloading::Symbol<*mut u8> = l
            .rust
            .get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("Rust .so missing {name}: {e}"));
        // `Symbol<*mut u8>` derefs to the *value* stored at the symbol; we want
        // the symbol's own address instead.
        (
            c.into_raw().into_raw() as *mut u8,
            r.into_raw().into_raw() as *mut u8,
        )
    }
}

/// Function-pointer address of an exported function (for identity checks).
pub fn fn_addr(name: &str) -> (usize, usize) {
    let (c, r) = sym2(name);
    (c as usize, r as usize)
}

/* ------------------------------------------------------- stdout capturing */

static CAPTURE: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 redirected to a temporary file and returns whatever `f`
/// produced together with the exact bytes it wrote to stdout.
///
/// `fflush(NULL)` is issued afterwards so glibc's fully-buffered `stdout`
/// (the C library uses `printf`) is drained before the fd is restored.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let guard = CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "diff_capture_{}_{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    let (result, bytes) = unsafe {
        libc::fflush(std::ptr::null_mut());
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open temp capture file failed");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");
        let result = f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);
        let bytes = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (result, bytes)
    };
    drop(guard);
    (result, bytes)
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/* ---------------------------------------------------------------- helpers */

/// Reads a NUL-terminated string from a pointer.
pub unsafe fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return b"<null>".to_vec();
    }
    let mut v = Vec::new();
    let mut q = p;
    loop {
        let b = *q as u8;
        if b == 0 {
            break;
        }
        v.push(b);
        q = q.add(1);
    }
    v
}

/// Deterministic SplitMix64 PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        Rng(0x5eed_1234_dead_beef)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Full-range `int`.
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    /// Small-magnitude `int` (keeps `mul` results interesting but bounded).
    pub fn next_small(&mut self) -> c_int {
        (self.next_u64() % 2001) as c_int - 1000
    }
}

/// The boundary operand set every row is exercised with.
pub const BOUNDARY: [c_int; 12] = [
    0,
    1,
    -1,
    2,
    -2,
    7,
    42,
    c_int::MAX,
    c_int::MIN,
    c_int::MAX / 2,
    c_int::MIN / 2,
    65536,
];

/// Calls `name(a, b)` in both libraries and asserts the return value *and* the
/// stdout bytes match.
pub fn assert_fn2_eq(name: &str, a: c_int, b: c_int) {
    let (cf, rf) = sym2(name);
    let (cr, cout) = capture(|| unsafe { cf(a, b) });
    let (rr, rout) = capture(|| unsafe { rf(a, b) });
    assert_eq!(
        cr,
        rr,
        "{name}({a}, {b}) return mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{name}({a}, {b}) stdout mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
}

/// Calls `name(n)` in both libraries and asserts return value + stdout match.
pub fn assert_fn1_eq(name: &str, n: c_int) {
    let (cf, rf) = sym1(name);
    let (cr, cout) = capture(|| unsafe { cf(n) });
    let (rr, rout) = capture(|| unsafe { rf(n) });
    assert_eq!(
        cr,
        rr,
        "{name}({n}) return mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{name}({n}) stdout mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
}

/* ------------------------------------------------------- driver execution */

pub struct Run {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: Option<i32>,
}

/// Runs an executable copied to `<tmp>/driver` and invoked as `./driver`, so
/// `argv[0]` is byte-identical for the C and the Rust binary.
pub fn run_driver(exe: &Path, args: &[&str]) -> Run {
    let dir = std::env::temp_dir().join(format!(
        "diff_run_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let dst = dir.join("driver");
    std::fs::copy(exe, &dst).unwrap_or_else(|e| panic!("copy {} : {e}", exe.display()));
    let out = std::process::Command::new("./driver")
        .args(args)
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn {} : {e}", dst.display()));
    let _ = std::fs::remove_dir_all(&dir);
    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        status: out.status.code(),
    }
}

pub fn assert_driver_eq(args: &[&str]) {
    let c = run_driver(&c_exe_path(), args);
    let r = run_driver(&rust_exe_path(), args);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "driver {args:?} stdout mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "driver {args:?} stderr mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
    assert_eq!(
        c.status,
        r.status,
        "driver {args:?} exit status mismatch [OP={} REPEAT={}]",
        op_name(),
        repeat()
    );
}
