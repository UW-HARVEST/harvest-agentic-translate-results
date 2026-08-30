//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `dlopen`/`dlsym` via
//! `libloading`. The Rust implementation is NEVER called directly as a Rust
//! function — always through its `#[no_mangle] extern "C"` exports, exactly as
//! an external C consumer would, so the export wrappers are under test too.
//!
//! Every function in this library returns `void` and communicates only through
//! `stdout`, so the harness redirects file descriptor 1 to a scratch file
//! around each call and compares the captured bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::os::unix::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture. These resolve out of the libc the test
// binary already links against -- the same libc both `.so`s use, so there is
// exactly one `stdout` FILE in the process and one `fflush(NULL)` flushes it.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every randomized row is reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { Self::DEFAULT_SEED } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn range_usize(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below((hi_inclusive - lo + 1) as u64) as usize
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}

/// `c_src/build/libdriver.so`, built on demand with cmake if absent.
fn c_so_path() -> PathBuf {
    let c_src = workspace_root().join("c_src");
    let build = c_src.join("build");
    let so = build.join("libdriver.so");
    if so.exists() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let configure = std::process::Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("run cmake configure");
    assert!(
        configure.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&configure.stdout),
        String::from_utf8_lossy(&configure.stderr)
    );
    let build_out = std::process::Command::new("cmake")
        .current_dir(&build)
        .arg("--build")
        .arg(".")
        .output()
        .expect("run cmake build");
    assert!(
        build_out.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );
    assert!(so.exists(), "cmake did not produce {}", so.display());
    so
}

/// The Rust cdylib for whichever profile this test binary was built in.
/// `current_exe()` is `<target>/<profile>/deps/<test>-<hash>`.
///
/// `cargo test` does not produce `cdylib` artifacts, so the `.so` may not exist
/// yet; in that case build it into a private target dir with the matching
/// profile. `DRIVER_RUST_SO` overrides the whole search.
fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={} does not exist", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile> dir")
        .to_path_buf();
    // Prefer the artifact produced by the *same* cargo invocation, which lands
    // next to this test binary in `<profile>/deps/`, then the `cargo build`
    // hardlink in `<profile>/`.
    for cand in [
        exe.parent().map(|d| d.join("libdriver.so")),
        Some(profile_dir.join("libdriver.so")),
    ]
    .into_iter()
    .flatten()
    {
        if cand.exists() {
            return cand;
        }
    }
    let so = profile_dir.join("libdriver.so");

    // `<target>/<profile>` -> the profile cargo was invoked with. Cargo names
    // the release dir "release"; everything else here is the dev profile.
    let is_release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
    let private_target = profile_dir.join("difftest-cdylib");
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.current_dir(manifest_dir())
        .arg("build")
        .arg("--offline")
        .arg("--lib")
        .env("CARGO_TARGET_DIR", &private_target)
        // Do not inherit the outer cargo's RUSTFLAGS/jobserver bookkeeping.
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("RUSTC_WORKSPACE_WRAPPER");
    if is_release {
        cmd.arg("--release");
    }
    let out = cmd.output().expect("spawn cargo build for the cdylib");
    let built = private_target
        .join(if is_release { "release" } else { "debug" })
        .join("libdriver.so");
    assert!(
        out.status.success() && built.exists(),
        "Rust cdylib not found at {} and building it failed.\n\
         Run `cargo build{}` first (that is what scripts/verify.sh does).\n{}\n{}",
        so.display(),
        if is_release { " --release" } else { "" },
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    built
}

// ---------------------------------------------------------------------------
// The loaded library surface.
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub print_line: Symbol<unsafe extern "C" fn(*const c_char)>,
    pub print_hex_char_line: Symbol<unsafe extern "C" fn(c_char)>,
    /// The very same `printHexCharLine` symbol, but declared as taking a full
    /// `int`. Used to drive out-of-`char`-range values across the ABI boundary
    /// so that the truncation the *callee* performs is what gets compared.
    pub print_hex_char_line_as_int: Symbol<unsafe extern "C" fn(c_int)>,
    pub bad: Symbol<unsafe extern "C" fn()>,
    pub good: Symbol<unsafe extern "C" fn()>,
    pub driver: Symbol<unsafe extern "C" fn(c_int)>,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        // RTLD_NOW | RTLD_LOCAL: resolve everything up front (so a missing
        // dependency is a hard error here) and keep the two libraries'
        // identically-named symbols out of the global namespace.
        const RTLD_NOW: c_int = 0x2;
        const RTLD_LOCAL: c_int = 0;
        let lib = unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = unsafe { lib.get($n) }.unwrap_or_else(|e| {
                    panic!(
                        "dlsym {:?} missing from {} ({}): {e}",
                        String::from_utf8_lossy(&$n[..$n.len() - 1]),
                        path.display(),
                        name
                    )
                });
                s
            }};
        }
        Lib {
            name,
            print_line: sym!(unsafe extern "C" fn(*const c_char), b"printLine\0"),
            print_hex_char_line: sym!(unsafe extern "C" fn(c_char), b"printHexCharLine\0"),
            print_hex_char_line_as_int: sym!(unsafe extern "C" fn(c_int), b"printHexCharLine\0"),
            bad: sym!(unsafe extern "C" fn(), b"bad\0"),
            good: sym!(unsafe extern "C" fn(), b"good\0"),
            driver: sym!(unsafe extern "C" fn(c_int), b"driver\0"),
            path,
            _lib: lib,
        }
    }
}

/// The two libraries plus the scratch file used for stdout capture, behind one
/// mutex: fd 1 is process-global, so captures must not overlap.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
    scratch: std::fs::File,
}

static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();

pub fn pair() -> MutexGuard<'static, Pair> {
    let m = PAIR.get_or_init(|| {
        let scratch_path = std::env::temp_dir().join(format!(
            "driver-difftest-{}-{}.out",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let scratch = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&scratch_path)
            .expect("open stdout scratch file");
        let _ = std::fs::remove_file(&scratch_path); // unlink; fd keeps it alive
        Mutex::new(Pair {
            c: Lib::open("C", c_so_path()),
            rust: Lib::open("Rust", rust_so_path()),
            scratch,
        })
    });
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Pair {
    /// Run `f` with fd 1 redirected to the scratch file and return everything
    /// the C stdio layer wrote.
    fn capture(&mut self, f: &mut dyn FnMut()) -> Vec<u8> {
        unsafe {
            // Flush whatever the harness itself may have buffered so it does
            // not leak into the capture.
            fflush(std::ptr::null_mut());
            let _ = std::io::Write::flush(&mut std::io::stdout());

            self.scratch.set_len(0).expect("truncate scratch");
            self.scratch
                .seek(SeekFrom::Start(0))
                .expect("rewind scratch");

            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(self.scratch.as_raw_fd(), 1) >= 0, "dup2 failed");

            f();

            // The library under test uses C stdio; fd 1 is a file here so the
            // stream is fully buffered. Flush before restoring.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
            close(saved);
        }

        self.scratch
            .seek(SeekFrom::Start(0))
            .expect("rewind scratch for read");
        let mut buf = Vec::new();
        self.scratch
            .read_to_end(&mut buf)
            .expect("read back scratch");
        buf
    }

    /// Capture the output of an arbitrary closure (used when a single capture
    /// must drive both libraries, e.g. interleaving tests).
    pub fn capture_raw(&mut self, f: &mut dyn FnMut()) -> Vec<u8> {
        self.capture(f)
    }

    /// Capture only the C library's output for `run`.
    pub fn capture_c(&mut self, mut run: impl FnMut(&Lib)) -> Vec<u8> {
        let c: *const Lib = &self.c;
        self.capture(&mut || run(unsafe { &*c }))
    }

    /// Capture only the Rust library's output for `run`.
    pub fn capture_rust(&mut self, mut run: impl FnMut(&Lib)) -> Vec<u8> {
        let r: *const Lib = &self.rust;
        self.capture(&mut || run(unsafe { &*r }))
    }

    /// Capture C's output and Rust's output for the same operation and assert
    /// they are byte-identical. `run` receives the library to drive.
    pub fn assert_same(&mut self, what: &str, mut run: impl FnMut(&Lib)) {
        // SAFETY-of-borrowing dance: capture() needs &mut self while `run`
        // needs &self.c / &self.rust. Copy the symbol handles out first.
        let c_out = {
            let c: *const Lib = &self.c;
            self.capture(&mut || run(unsafe { &*c }))
        };
        let r_out = {
            let r: *const Lib = &self.rust;
            self.capture(&mut || run(unsafe { &*r }))
        };
        if c_out != r_out {
            panic!(
                "DIVERGENCE [{what}]\n  C    ({} bytes): {}\n  Rust ({} bytes): {}\n  first diff at byte {:?}",
                c_out.len(),
                render(&c_out),
                r_out.len(),
                render(&r_out),
                c_out.iter().zip(r_out.iter()).position(|(a, b)| a != b),
            );
        }
    }
}

/// Readable, bounded rendering of a captured byte stream for panic messages.
pub fn render(bytes: &[u8]) -> String {
    const LIMIT: usize = 220;
    let shown = &bytes[..bytes.len().min(LIMIT)];
    let mut s = String::from("\"");
    for &b in shown {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    if bytes.len() > LIMIT {
        s.push_str(&format!(" ...(+{} bytes)", bytes.len() - LIMIT));
    }
    s
}

/// Build a NUL-terminated C buffer from raw bytes.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// The randomized mixed-pipeline program used by the composed-pipeline rows.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Op {
    Driver(c_int),
    Good,
    Bad,
    PrintLine(Vec<u8>), // already NUL-terminated
    PrintLineNull,
    PrintHex(c_char),
    PrintHexInt(c_int),
}

pub fn random_program(rng: &mut Rng, len: usize) -> Vec<Op> {
    let mut ops = Vec::with_capacity(len);
    for _ in 0..len {
        ops.push(match rng.below(7) {
            0 => Op::Driver(rng.next_i32()),
            1 => Op::Good,
            2 => Op::Bad,
            3 => {
                let n = rng.range_usize(0, 48);
                let mut s: Vec<u8> = (0..n)
                    .map(|_| {
                        let b = rng.next_u8();
                        if b == 0 {
                            1
                        } else {
                            b
                        }
                    })
                    .collect();
                s.push(0);
                Op::PrintLine(s)
            }
            4 => Op::PrintLineNull,
            5 => Op::PrintHex(rng.next_u8() as c_char),
            _ => Op::PrintHexInt(rng.next_i32()),
        });
    }
    ops
}

pub unsafe fn run_program(lib: &Lib, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Driver(v) => (lib.driver)(*v),
            Op::Good => (lib.good)(),
            Op::Bad => (lib.bad)(),
            Op::PrintLine(s) => (lib.print_line)(s.as_ptr() as *const c_char),
            Op::PrintLineNull => (lib.print_line)(std::ptr::null()),
            Op::PrintHex(v) => (lib.print_hex_char_line)(*v),
            Op::PrintHexInt(v) => (lib.print_hex_char_line_as_int)(*v),
        }
    }
}
