//! Shared differential-test harness.
//!
//! Everything here is deliberately *external*: the Rust implementation is only
//! ever reached through `dlopen` + `dlsym` on the built `cdylib`, exactly like
//! the C implementation. No function of this crate is ever called directly, so
//! the `#[no_mangle] extern "C"` export wrappers are part of what gets tested.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

/* ------------------------------------------------------------------ */
/* The configuration this test binary was compiled for                */
/* ------------------------------------------------------------------ */

/// Mirrors the `#[cfg]` precedence in `src/mdmacros.rs` (`sub` > `mul` >
/// `add`-fallback), which in turn mirrors `#ifndef OP / #define OP add`.
pub const OP: &str = if cfg!(feature = "sub") {
    "sub"
} else if cfg!(feature = "mul") {
    "mul"
} else {
    "add"
};

/// Mirrors `src/mdmacros.rs` (`repeat_0` > 1 > 2 > 3 > 4 > 6 > 7 > `5`-fallback),
/// which mirrors `#ifndef REPEAT / #define REPEAT 5`.
pub const REPEAT: c_int = if cfg!(feature = "repeat_0") {
    0
} else if cfg!(feature = "repeat_1") {
    1
} else if cfg!(feature = "repeat_2") {
    2
} else if cfg!(feature = "repeat_3") {
    3
} else if cfg!(feature = "repeat_4") {
    4
} else if cfg!(feature = "repeat_6") {
    6
} else if cfg!(feature = "repeat_7") {
    7
} else {
    5
};

/// `INIT_FOR(OP)`: `INIT_add`/`INIT_sub` = 0, `INIT_mul` = 1.
pub fn init_for_op() -> c_int {
    if OP == "mul" {
        1
    } else {
        0
    }
}

/// `STEP_OP(OP, acc, i)`.
pub fn step(acc: c_int, i: c_int) -> c_int {
    match OP {
        "add" => acc.wrapping_add(i),
        "sub" => acc.wrapping_sub(i),
        _ => acc.wrapping_mul(i.wrapping_add(1)),
    }
}

pub fn tag() -> String {
    format!("{}_{}", OP, REPEAT)
}

/* ------------------------------------------------------------------ */
/* Building the two artifacts under test                              */
/* ------------------------------------------------------------------ */

pub struct Artifacts {
    /// `libcdriver.so` built from `c_src/src/mdcore.c`.
    pub c_lib: PathBuf,
    /// `libdriver.so`, the Rust `cdylib`.
    pub r_lib: PathBuf,
    /// Directory containing the C `driver` executable, invoked as `./driver`.
    pub c_bin_dir: PathBuf,
    /// Directory containing the Rust `driver` executable, invoked as `./driver`.
    pub r_bin_dir: PathBuf,
    /// `execve(prog, {NULL}, {NULL})` launcher, for the `argc == 0` row.
    pub argc0: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_src_dir() -> PathBuf {
    manifest_dir().join("../c_src/src").canonicalize().expect(
        "c_src/src must sit next to the translation crate; \
         the C sources are the ground truth for this test suite",
    )
}

fn run(cmd: &mut Command) {
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {cmd:?}: {e}"));
    if !out.status.success() {
        panic!(
            "command failed: {cmd:?}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// `c_src/CMakeLists.txt` sets `CMAKE_C_FLAGS` to exactly
/// `-DOP=${OP} -DREPEAT=${REPEAT}` and never sets `CMAKE_BUILD_TYPE`, so the
/// reference build carries **no** `-O` flag. We reproduce that verbatim; the
/// only additions are `-shared -fPIC` (CMake gets those from
/// `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` plus the library link step) because
/// `CMakeLists.txt` itself only declares `add_executable`.
fn build_c(out_dir: &Path) -> (PathBuf, PathBuf) {
    let src = c_src_dir();
    let lib = out_dir.join("libcdriver.so");
    let bin_dir = out_dir.join("cbin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    run(Command::new("gcc").args([
        "-shared",
        "-fPIC",
        &format!("-DOP={OP}"),
        &format!("-DREPEAT={REPEAT}"),
        "-o",
        lib.to_str().unwrap(),
        src.join("mdcore.c").to_str().unwrap(),
    ]));

    run(Command::new("gcc").args([
        &format!("-DOP={OP}"),
        &format!("-DREPEAT={REPEAT}"),
        "-o",
        bin_dir.join("driver").to_str().unwrap(),
        src.join("mdcore.c").to_str().unwrap(),
        src.join("mdmain.c").to_str().unwrap(),
    ]));

    (lib, bin_dir)
}

/// Builds the Rust `cdylib` + `driver` with the *same* feature set this test
/// binary was compiled with. A separate `CARGO_TARGET_DIR` is used so the
/// nested invocation does not contend with the outer `cargo test`'s lock, and
/// so switching feature combinations never serves a stale `.so`.
fn build_rust(out_dir: &Path) -> (PathBuf, PathBuf) {
    let features = format!("{OP},repeat_{REPEAT}");
    let target_dir = out_dir.join("rust");
    run(Command::new(env!("CARGO"))
        .current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target_dir)
        .args([
            "build",
            "--release",
            "--quiet",
            "--no-default-features",
            "--features",
            &features,
        ]));

    let lib = target_dir.join("release/libdriver.so");
    assert!(lib.is_file(), "cdylib not produced at {}", lib.display());

    let bin_dir = out_dir.join("rbin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::copy(target_dir.join("release/driver"), bin_dir.join("driver")).unwrap();

    (lib, bin_dir)
}

fn build_argc0_launcher(out_dir: &Path) -> PathBuf {
    let src = out_dir.join("argc0.c");
    std::fs::write(
        &src,
        b"#include <unistd.h>\n\
          int main(int argc, char **argv) {\n\
          \x20   char *a[] = {0};\n\
          \x20   char *e[] = {0};\n\
          \x20   execve(argv[1], a, e);\n\
          \x20   return 127;\n\
          }\n" as &[u8],
    )
    .unwrap();
    let exe = out_dir.join("argc0");
    run(Command::new("gcc").args(["-o", exe.to_str().unwrap(), src.to_str().unwrap()]));
    exe
}

pub fn artifacts() -> &'static Artifacts {
    static A: OnceLock<Artifacts> = OnceLock::new();
    A.get_or_init(|| {
        let out_dir = manifest_dir().join("target/difftest").join(tag());
        std::fs::create_dir_all(&out_dir).unwrap();
        let (c_lib, c_bin_dir) = build_c(&out_dir);
        let (r_lib, r_bin_dir) = build_rust(&out_dir);
        let argc0 = build_argc0_launcher(&out_dir);
        Artifacts {
            c_lib,
            r_lib,
            c_bin_dir,
            r_bin_dir,
            argc0,
        }
    })
}

/* ------------------------------------------------------------------ */
/* Loading both shared objects                                        */
/* ------------------------------------------------------------------ */

pub type Op2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Op1 = unsafe extern "C" fn(c_int) -> c_int;

pub struct Both {
    pub c: Library,
    pub r: Library,
}

/// `libloading` uses `RTLD_LAZY | RTLD_LOCAL`, so the two libraries' identically
/// named exports (`op_add`, `G_OP`, ...) do not collide with each other.
pub fn both() -> &'static Both {
    static B: OnceLock<Both> = OnceLock::new();
    B.get_or_init(|| {
        let a = artifacts();
        unsafe {
            Both {
                c: Library::new(&a.c_lib)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", a.c_lib.display())),
                r: Library::new(&a.r_lib)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", a.r_lib.display())),
            }
        }
    })
}

pub fn fn2(lib: &'static Library, name: &str) -> Op2 {
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    unsafe {
        let s: Symbol<Op2> = lib
            .get(&sym)
            .unwrap_or_else(|e| panic!("dlsym {name}: {e}"));
        *s
    }
}

pub fn fn1(lib: &'static Library, name: &str) -> Op1 {
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    unsafe {
        let s: Symbol<Op1> = lib
            .get(&sym)
            .unwrap_or_else(|e| panic!("dlsym {name}: {e}"));
        *s
    }
}

/// Address of the exported `G_OP` *object* (not the function it points at).
pub fn g_op_slot(lib: &'static Library) -> *mut Op2 {
    unsafe {
        let s: Symbol<*mut Op2> = lib.get(b"G_OP\0").expect("dlsym G_OP");
        *s
    }
}

/// Address of the exported `G_OP_NAME` *object*.
pub fn g_op_name_slot(lib: &'static Library) -> *mut *const c_char {
    unsafe {
        let s: Symbol<*mut *const c_char> = lib.get(b"G_OP_NAME\0").expect("dlsym G_OP_NAME");
        *s
    }
}

pub fn read_g_op_name(lib: &'static Library) -> Vec<u8> {
    unsafe {
        let p = *g_op_name_slot(lib);
        assert!(!p.is_null(), "G_OP_NAME must not be NULL");
        CStr::from_ptr(p).to_bytes().to_vec()
    }
}

/// The list of dynamic symbols the C header declares as the public surface.
pub const PUBLIC_FNS_2: [&str; 5] = ["op_add", "op_sub", "op_mul", "helper_call", "helper_ptr"];

/* ------------------------------------------------------------------ */
/* stdout capture across the FFI boundary                             */
/* ------------------------------------------------------------------ */

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

static CAPTURE: Mutex<()> = Mutex::new(());

/// Runs `f` with file descriptor 1 pointed at a temporary file and returns
/// whatever `f` wrote there.
///
/// This is the only way to compare the two implementations' `printf` output:
/// the C `.so` writes through glibc's `stdout` FILE (fully buffered when fd 1
/// is not a tty, hence the `fflush(NULL)`), while the Rust `.so` carries its own
/// `std` instance whose `Stdout` is a `LineWriter` writing straight to fd 1.
/// Neither is visible to the Rust test harness's own output capture.
pub fn capture_fd1<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = CAPTURE.lock().unwrap_or_else(|e| e.into_inner());

    let dir = manifest_dir().join("target/difftest").join(tag());
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("cap-{}.txt", std::process::id()));

    let _ = std::io::stdout().flush();
    let file = std::fs::File::create(&path).unwrap();

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let r = f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    (r, bytes)
}

/// Calls `name(a, b)` in both libraries and asserts the return value *and* the
/// printed bytes match.
#[track_caller]
pub fn assert_fn2_matches(name: &str, a: c_int, b: c_int) {
    let bo = both();
    let cf = fn2(&bo.c, name);
    let rf = fn2(&bo.r, name);
    let (cr, cout) = capture_fd1(|| unsafe { cf(a, b) });
    let (rr, rout) = capture_fd1(|| unsafe { rf(a, b) });
    assert_eq!(
        cr, rr,
        "[{}] {name}({a}, {b}) return value: C={cr} Rust={rr}",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{}] {name}({a}, {b}) stdout bytes",
        tag()
    );
}

/// Calls `name(n)` in both libraries and asserts return value + printed bytes.
#[track_caller]
pub fn assert_fn1_matches(name: &str, n: c_int) {
    let bo = both();
    let cf = fn1(&bo.c, name);
    let rf = fn1(&bo.r, name);
    let (cr, cout) = capture_fd1(|| unsafe { cf(n) });
    let (rr, rout) = capture_fd1(|| unsafe { rf(n) });
    assert_eq!(cr, rr, "[{}] {name}({n}) return value", tag());
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{}] {name}({n}) stdout bytes",
        tag()
    );
}

/// Same, for a call through the `G_OP` global.
#[track_caller]
pub fn assert_g_op_matches(a: c_int, b: c_int) {
    let bo = both();
    let cf = unsafe { *g_op_slot(&bo.c) };
    let rf = unsafe { *g_op_slot(&bo.r) };
    let cr = unsafe { cf(a, b) };
    let rr = unsafe { rf(a, b) };
    assert_eq!(cr, rr, "[{}] G_OP({a}, {b})", tag());
}

/* ------------------------------------------------------------------ */
/* Driving the two executables                                        */
/* ------------------------------------------------------------------ */

pub struct RunOut {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
}

/// Both executables are invoked as `./driver` from their own directory, so
/// `argv[0]` — which `mdmain.c` prints in the usage message — is byte-identical.
pub fn run_driver(dir: &Path, args: &[&str]) -> RunOut {
    let out = Command::new("./driver")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn ./driver in {}: {e}", dir.display()));
    RunOut {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
    }
}

#[track_caller]
pub fn assert_driver_matches(args: &[&str]) {
    let a = artifacts();
    let c = run_driver(&a.c_bin_dir, args);
    let r = run_driver(&a.r_bin_dir, args);
    assert_eq!(
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
        "[{}] driver {args:?} stdout",
        tag()
    );
    assert_eq!(
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        "[{}] driver {args:?} stderr",
        tag()
    );
    assert_eq!(c.code, r.code, "[{}] driver {args:?} exit status", tag());
}

/* ------------------------------------------------------------------ */
/* Deterministic randomness                                           */
/* ------------------------------------------------------------------ */

/// SplitMix64. Fixed seed, so every failure is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED;

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
    /// Uniform over the whole `i32` range.
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }
    pub fn next_in(&mut self, lo: i64, hi: i64) -> c_int {
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as c_int
    }
}

/// The boundary set every row is additionally exercised over, squared.
pub const BOUNDARY: [c_int; 7] = [
    c_int::MIN,
    c_int::MIN + 1,
    -1,
    0,
    1,
    c_int::MAX - 1,
    c_int::MAX,
];

pub fn boundary_pairs() -> Vec<(c_int, c_int)> {
    let mut v = Vec::with_capacity(49);
    for &a in &BOUNDARY {
        for &b in &BOUNDARY {
            v.push((a, b));
        }
    }
    v
}

/// Pairs chosen to overflow each of the three operations.
pub fn overflow_pairs() -> Vec<(c_int, c_int)> {
    vec![
        (c_int::MAX, 1),
        (c_int::MIN, -1),
        (c_int::MAX, c_int::MAX),
        (c_int::MIN, c_int::MIN),
        (c_int::MAX, c_int::MIN),
        (c_int::MIN, c_int::MAX),
        (2, c_int::MAX),
        (-1, c_int::MIN),
        (123_456_789, 123_456_789),
        (65_536, 65_536),
        (46_341, 46_341),
        (-46_341, 46_341),
    ]
}

pub fn random_pairs(n: usize, seed: u64) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::new(seed);
    (0..n).map(|_| (rng.next_i32(), rng.next_i32())).collect()
}

pub fn random_pairs_in(n: usize, seed: u64, lo: i64, hi: i64) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| (rng.next_in(lo, hi), rng.next_in(lo, hi)))
        .collect()
}
