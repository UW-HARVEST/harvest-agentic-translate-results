//! Shared harness for the differential tests.
//!
//! Both sides are always reached through `dlopen` + `dlsym` (`libloading`): the C
//! reference `.so` built from `c_src/src/mdcore.c`, and the Rust `cdylib` built
//! from this crate. Nothing here calls a Rust function from the translation
//! directly — every comparison goes through the `#[no_mangle] extern "C"` exports,
//! so the wrappers themselves are under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Build configuration, mirroring src/mdmacros.rs's feature precedence exactly.
// ---------------------------------------------------------------------------

/// `-DOP=` value this test binary was compiled for.
pub const OP_TAG: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// `-DREPEAT=` value this test binary was compiled for.
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
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

/// `INIT_FOR(OP)`.
pub const INIT: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

pub fn config_tag() -> String {
    format!("{}_{}", OP_TAG, REPEAT)
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

/// Repository root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

/// The C reference `.so` for this build configuration, compiling it on demand.
///
/// `c_src/CMakeLists.txt` only declares `add_executable`, so the shared-library
/// form is produced here from `mdcore.c` (the half without `main`) using the same
/// `-D` flags cmake would pass. `c_src/` is only ever read.
pub fn c_lib_path() -> PathBuf {
    let root = repo_root();
    let out_dir = root.join("cbuild");
    let so = out_dir.join(format!("libcdriver_{}.so", config_tag()));
    if !so.exists() {
        std::fs::create_dir_all(&out_dir).expect("create cbuild/");
        let status = Command::new("gcc")
            .args([
                "-O2",
                "-fPIC",
                "-shared",
                &format!("-DOP={}", OP_TAG),
                &format!("-DREPEAT={}", REPEAT),
            ])
            .arg(format!("-I{}", root.join("c_src/src").display()))
            .arg("-o")
            .arg(&so)
            .arg(root.join("c_src/src/mdcore.c"))
            .status()
            .expect("run gcc");
        assert!(status.success(), "gcc failed for config {}", config_tag());
    }
    so
}

/// The Rust `cdylib` for this build configuration.
///
/// `cargo test` builds the crate's rlib and the test harness but *not* the
/// `cdylib` (adding `rlib` to `crate-type` does not change that), so the `.so` has
/// to be built separately. `build_so.sh` does it and stamps a per-configuration
/// copy, and both `sweep_so.sh` and `mutation_check.sh` point the tests straight at
/// that copy via `MD_RUST_SO`. Resolution order:
///
/// 1. `$MD_RUST_SO` — an explicit path, which is what the scripts always pass.
/// 2. `target/<profile>/libdriver_<op>_<repeat>.so` — the stamped copy.
/// 3. `target/<profile>/libdriver.so` — a bare `cargo build`. This path is shared
///    by every feature set, so it is probed for the expected configuration; a
///    mismatch is reported as such rather than as a mysterious divergence.
pub fn rust_lib_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(explicit) = std::env::var_os("MD_RUST_SO") {
            let p = PathBuf::from(explicit);
            assert!(p.exists(), "MD_RUST_SO points at a missing file: {}", p.display());
            return p;
        }

        let exe = std::env::current_exe().expect("current_exe");
        // target/<profile>/deps/<test>-<hash>  ->  target/<profile>
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("test exe lives in target/<profile>/deps");

        let stamped = profile_dir.join(format!("libdriver_{}.so", config_tag()));
        if stamped.exists() {
            return stamped;
        }

        let bare = profile_dir.join("libdriver.so");
        assert!(
            bare.exists(),
            "no Rust cdylib for the {}/{} configuration.\n\
             `cargo test` does not build a cdylib; build it first, e.g.\n\
             \n    ./build_so.sh --no-default-features --features {},{}\n\n\
             or just run ./verify.sh, which does the whole sweep.",
            OP_TAG,
            REPEAT,
            OP_TAG,
            REPEAT
        );
        assert!(
            built_for_this_config(&bare),
            "{} does not behave like the {}/{} configuration this test binary was \
             compiled for.\nEither it is a stale artifact from a different feature set \
             (target/<profile>/libdriver.so is shared by all of them) or the \
             translation genuinely diverges. Re-run ./verify.sh, which builds and \
             stamps a per-configuration copy, to tell the two apart.",
            bare.display(),
            OP_TAG,
            REPEAT
        );
        bare
    })
    .clone()
}

/// `RUN_LOOP(OP, acc, REPEAT)` starting from `INIT_FOR(OP)` — what
/// `helper_call(0, 0)` returns, since `OP_FN(0, 0)` is 0 for all three ops.
pub fn expected_run_loop_acc() -> c_int {
    match OP_TAG {
        "add" => (0..REPEAT).sum(),
        "sub" => -(0..REPEAT).sum::<c_int>(),
        "mul" => (0..REPEAT).fold(1, |acc, i| acc.wrapping_mul(i + 1)),
        other => panic!("unexpected OP {other}"),
    }
}

/// Best-effort probe for the `OP`/`REPEAT` a `.so` was compiled with, used only on
/// the shared unstamped path.
///
/// `REPEAT` 0 and 1 are indistinguishable by construction (`REP0` expands to
/// nothing and `REP1` applies the identity step `+0` / `-0` / `*1`), which is a
/// property of the C header, not a gap in the probe.
fn built_for_this_config(so: &Path) -> bool {
    let probe = Impl::open("probe", so);
    if probe.g_op_name() != OP_TAG.as_bytes() {
        return false;
    }
    let (acc, _) = capture_stdout(|| probe.helper_call(0, 0));
    acc == expected_run_loop_acc()
}

/// The C reference `driver` executable for this configuration, built on demand.
pub fn c_exe_path() -> PathBuf {
    let root = repo_root();
    let dir = root.join("cbuild").join(format!("exe_{}", config_tag()));
    let exe = dir.join("driver");
    if !exe.exists() {
        std::fs::create_dir_all(&dir).expect("create exe dir");
        let status = Command::new("gcc")
            .args([
                "-O2",
                &format!("-DOP={}", OP_TAG),
                &format!("-DREPEAT={}", REPEAT),
            ])
            .arg(format!("-I{}", root.join("c_src/src").display()))
            .arg("-o")
            .arg(&exe)
            .arg(root.join("c_src/src/mdcore.c"))
            .arg(root.join("c_src/src/mdmain.c"))
            .status()
            .expect("run gcc");
        assert!(status.success(), "gcc failed for exe {}", config_tag());
    }
    exe
}

/// The Rust `driver` executable for this configuration.
///
/// `cargo test` does build the `[[bin]]` target with the current feature set, so
/// this path is current; it is still probed for the expected `op=` name so a stale
/// binary cannot quietly stand in.
pub fn rust_exe_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(|p| p.parent())
            .expect("test exe lives in target/<profile>/deps");
        let driver = profile_dir.join("driver");
        assert!(driver.exists(), "driver binary missing at {}", driver.display());
        let out = Command::new(&driver)
            .args(["0", "0"])
            .output()
            .expect("run driver probe");
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            text.contains(&format!("op={} ", OP_TAG))
                && text.contains(&format!("acc={} ", expected_run_loop_acc())),
            "driver at {} was built for a different configuration than {}/{}: {:?}",
            driver.display(),
            OP_TAG,
            REPEAT,
            text
        );
        driver
    })
    .clone()
}

// ---------------------------------------------------------------------------
// Symbol signatures
// ---------------------------------------------------------------------------

pub type OpFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type AccumFn = unsafe extern "C" fn(c_int) -> c_int;

/// One loaded implementation, C or Rust, reached only through `dlsym`.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn open(name: &'static str, path: &Path) -> Impl {
        // SAFETY: both objects are plain C-ABI libraries with no initializers
        // beyond the constant data slots `G_OP` / `G_OP_NAME`.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name));
        Impl { name, lib }
    }

    fn func(&self, sym: &[u8]) -> Symbol<'_, OpFn> {
        unsafe { self.lib.get::<OpFn>(sym) }.unwrap_or_else(|e| {
            panic!(
                "{}: missing exported symbol {}: {e}",
                self.name,
                String::from_utf8_lossy(sym)
            )
        })
    }

    pub fn op_add(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.func(b"op_add\0"))(a, b) }
    }
    pub fn op_sub(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.func(b"op_sub\0"))(a, b) }
    }
    pub fn op_mul(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.func(b"op_mul\0"))(a, b) }
    }
    pub fn helper_call(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.func(b"helper_call\0"))(a, b) }
    }
    pub fn helper_ptr(&self, a: c_int, b: c_int) -> c_int {
        unsafe { (self.func(b"helper_ptr\0"))(a, b) }
    }
    pub fn use_generated(&self, n: c_int) -> c_int {
        let f: Symbol<'_, AccumFn> = unsafe { self.lib.get::<AccumFn>(b"use_generated\0") }
            .unwrap_or_else(|e| panic!("{}: missing use_generated: {e}", self.name));
        unsafe { f(n) }
    }

    /// Address of an exported `op_*`, for comparing against `G_OP`'s contents.
    pub fn op_addr(&self, which: &str) -> usize {
        let sym: &[u8] = match which {
            "add" => b"op_add\0",
            "sub" => b"op_sub\0",
            "mul" => b"op_mul\0",
            other => panic!("no op_{other}"),
        };
        *self.func(sym) as usize
    }

    /// Reads the `G_OP` data slot (`int (*G_OP)(int,int)`).
    pub fn g_op(&self) -> OpFn {
        let sym: Symbol<'_, *mut OpFn> = unsafe { self.lib.get::<*mut OpFn>(b"G_OP\0") }
            .unwrap_or_else(|e| panic!("{}: missing G_OP: {e}", self.name));
        unsafe { **sym }
    }

    /// Overwrites `G_OP`; the C global is not `const`, so this is legal for a
    /// caller and both sides must dispatch through the stored pointer.
    pub fn set_g_op(&self, f: OpFn) {
        let sym: Symbol<'_, *mut OpFn> = unsafe { self.lib.get::<*mut OpFn>(b"G_OP\0") }
            .unwrap_or_else(|e| panic!("{}: missing G_OP: {e}", self.name));
        unsafe { **sym = f };
    }

    /// Reads `G_OP_NAME` and copies the NUL-terminated bytes it points at.
    pub fn g_op_name(&self) -> Vec<u8> {
        let sym: Symbol<'_, *mut *const c_char> =
            unsafe { self.lib.get::<*mut *const c_char>(b"G_OP_NAME\0") }
                .unwrap_or_else(|e| panic!("{}: missing G_OP_NAME: {e}", self.name));
        let p = unsafe { **sym };
        assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.name);
        let mut out = Vec::new();
        let mut i = 0isize;
        loop {
            let byte = unsafe { *p.offset(i) } as u8;
            if byte == 0 {
                break;
            }
            out.push(byte);
            i += 1;
            assert!(i < 64, "{}: G_OP_NAME not NUL-terminated", self.name);
        }
        out
    }
}

/// The two implementations, opened once per test process.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::open("C", &c_lib_path()),
        rust: Impl::open("Rust", &rust_lib_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture (the helpers print, so return-value parity is not enough)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every libc output stream, which is how the C
    /// `.so`'s fully-buffered `printf` output is forced out before fd 1 is
    /// restored. The Rust `.so` uses a `LineWriter` and self-flushes on `\n`.
    fn fflush(stream: *mut c_void) -> c_int;
}

fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// fd 1 is process-global and libtest writes its own progress lines to it, so
/// capture is only sound when the test threads run serially. `.cargo/config.toml`
/// sets `RUST_TEST_THREADS = "1"` for every Cargo-launched process; this check
/// turns a lost setting into an explicit failure instead of a mysterious
/// "stdout differs" containing `test cfg_... ok`.
fn require_serial() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let flagged = std::env::args().any(|a| a == "--test-threads=1")
            || std::env::args()
                .zip(std::env::args().skip(1))
                .any(|(a, b)| a == "--test-threads" && b == "1");
        let env_serial = std::env::var("RUST_TEST_THREADS").ok().as_deref() == Some("1");
        let single_cpu = std::thread::available_parallelism()
            .map(|n| n.get() == 1)
            .unwrap_or(false);
        assert!(
            flagged || env_serial || single_cpu,
            "these tests capture fd 1 and must run serially; \
             re-run with `RUST_TEST_THREADS=1` (set by translation/.cargo/config.toml) \
             or `cargo test ... -- --test-threads=1`"
        );
    });
}

/// Runs `body` with fd 1 pointed at a temporary file and returns both the
/// closure's value and the bytes written to stdout.
///
/// Serialised through a mutex because fd 1 is process-global.
pub fn capture_stdout<T>(body: impl FnOnce() -> T) -> (T, Vec<u8>) {
    require_serial();
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "mdcap_{}_{}_{:?}.bin",
        std::process::id(),
        config_tag(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");

    unsafe {
        fflush(std::ptr::null_mut());
    }
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let value = body();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);

    (value, bytes)
}

/// Calls the same operation on both implementations and asserts the return value
/// and the stdout bytes match exactly.
pub fn assert_same<F>(what: &str, mut call: F)
where
    F: FnMut(&Impl) -> c_int,
{
    let p = pair();
    let (cv, cout) = capture_stdout(|| call(&p.c));
    let (rv, rout) = capture_stdout(|| call(&p.rust));
    assert_eq!(
        cv,
        rv,
        "[{}/{}] {}: return value C={} Rust={}",
        OP_TAG,
        REPEAT,
        what,
        cv,
        rv
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "[{}/{}] {}: stdout differs",
        OP_TAG,
        REPEAT,
        what
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — property-style inputs, fixed seed
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }
    /// A mix of full-range values and small values, so both wrapping arithmetic
    /// and the ordinary small-integer paths get hit.
    pub fn next_operand(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 4 {
            0 => (r >> 8) as u32 as i32,
            1 => ((r >> 8) % 65) as i64 as i32 - 32,
            2 => ((r >> 8) % 131_073) as i64 as i32 - 65_536,
            _ => (r >> 32) as u32 as i32,
        }
    }
}

/// Boundary `(a, b)` pairs every `op_*` row is driven with.
pub const BOUNDARY_PAIRS: &[(c_int, c_int)] = &[
    (0, 0),
    (0, 1),
    (1, 0),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
    (i32::MAX, 0),
    (0, i32::MAX),
    (i32::MAX, 1),
    (1, i32::MAX),
    (i32::MAX, -1),
    (i32::MIN, 0),
    (0, i32::MIN),
    (i32::MIN, 1),
    (i32::MIN, -1),
    (-1, i32::MIN),
    (i32::MAX, i32::MAX),
    (i32::MIN, i32::MIN),
    (i32::MAX, i32::MIN),
    (i32::MIN, i32::MAX),
    (65_536, 65_536),
    (-65_536, 65_536),
    (46_341, 46_341),
    (2, i32::MAX),
    (i32::MAX / 2, 3),
];
