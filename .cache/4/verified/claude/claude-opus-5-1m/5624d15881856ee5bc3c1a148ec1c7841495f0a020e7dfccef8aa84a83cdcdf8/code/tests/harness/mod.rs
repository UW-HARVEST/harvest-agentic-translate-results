//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded as *shared objects* via
//! `libloading` and driven only through their exported C symbols. The Rust
//! functions are never called directly, so the `#[no_mangle] extern "C"`
//! wrappers are part of what is under test.
//!
//! Every function in this library returns `void` and communicates exclusively
//! by writing to `stdout` through libc `printf`, so "compare the outputs"
//! means "capture file descriptor 1 and compare the bytes".

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, CString};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — property-style testing with a fixed seed.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform-ish value in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    /// A random `c_char` covering the whole platform `char` domain.
    pub fn next_c_char(&mut self) -> c_char {
        (self.next_u32() & 0xff) as u8 as c_char
    }

    /// A NUL-free byte string of `len` bytes drawn from `0x01..=0xff`.
    pub fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| 1u8.wrapping_add((self.below(255)) as u8))
            .collect()
    }

    /// A NUL-free printable-ASCII string of `len` bytes (`0x20..=0x7e`).
    pub fn next_ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| 0x20u8 + self.below(0x5f) as u8).collect()
    }
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `"release"` when the tests themselves were built in release mode, so the
/// Rust `.so` under test always matches the profile the suite runs under
/// (this is what makes the `panic = "abort"` / overflow-check rows in
/// CONFIGS.md meaningful).
pub fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// Build the C library with CMake exactly as documented, then return its path.
fn build_c_lib() -> PathBuf {
    let root = manifest_dir().join("c_src");
    let build = root.join("build");
    let so = build.join("libdriver.so");
    if so.exists() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake configure");
    assert!(st.success(), "cmake configure failed");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake build");
    assert!(st.success(), "cmake build failed");
    assert!(so.exists(), "C .so not produced at {}", so.display());
    so
}

/// Build the C library a second time at `-O2`, into the Rust target directory
/// (nothing under `c_src/` is written).
///
/// The default CMake build is unoptimised, and an ABI divergence was found in
/// this library that only appears once the *Rust* side is optimised, so the
/// optimised C build is worth comparing against too: it pins that gcc narrows
/// the `char` parameter defensively (`movsbl %dil,%esi`) and rewrites
/// `printf("%s\n", s)` into `puts(s)` at `-O2`, i.e. exactly what the optimised
/// Rust build does. Returns `None` if no C compiler is available.
fn build_c_lib_o2() -> Option<PathBuf> {
    let out_dir = manifest_dir().join("target").join("c-o2");
    std::fs::create_dir_all(&out_dir).ok()?;
    let so = out_dir.join("libdriver_O2.so");
    let root = manifest_dir().join("c_src");
    let st = Command::new("cc")
        .arg("-O2")
        .arg("-fPIC")
        .arg("-shared")
        .arg("-I")
        .arg(root.join("include"))
        .arg(root.join("src").join("driver.c"))
        .arg("-o")
        .arg(&so)
        .status()
        .ok()?;
    if st.success() && so.exists() {
        Some(so)
    } else {
        None
    }
}

static C_SO_O2: OnceLock<Option<PathBuf>> = OnceLock::new();

/// The `-O2` C build, if a compiler is available.
pub fn c_o2_api() -> Option<&'static Api> {
    static API: OnceLock<Option<Api>> = OnceLock::new();
    API.get_or_init(|| {
        C_SO_O2
            .get_or_init(build_c_lib_o2)
            .clone()
            .map(|p| Api::load("C-O2", p))
    })
    .as_ref()
}

/// Build the Rust `cdylib` and return its path.
///
/// `cargo test` does *not* emit the `cdylib` artifact for a `crate-type =
/// ["cdylib"]` package, so the harness builds it itself. A dedicated
/// `CARGO_TARGET_DIR` is used so this nested build can never contend with the
/// outer `cargo test` invocation's build-directory lock.
fn build_rust_lib() -> PathBuf {
    let target = manifest_dir().join("target").join("ffi-so");
    let so = target.join(profile()).join("libdriver.so");

    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(manifest_dir())
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .args(["build", "--offline", "--no-default-features", "--lib"]);
    if profile() == "release" {
        cmd.arg("--release");
    }
    let st = cmd.status().expect("run cargo build for the cdylib");
    assert!(st.success(), "cargo build of the cdylib failed");
    assert!(
        so.exists(),
        "Rust .so not produced at {} (crate-type must include cdylib)",
        so.display()
    );
    so
}

// ---------------------------------------------------------------------------
// The resolved C ABI surface of one shared object.
// ---------------------------------------------------------------------------

/// Every symbol the C `.so` exports, resolved out of one library.
///
/// `print_hex_char_line_widened` is the *same* symbol as
/// `print_hex_char_line`, resolved through a deliberately widened
/// `extern "C" fn(c_int)` prototype. C callers with a stale/implicit
/// declaration really do this, and it is how the "out-of-range value handed
/// across the FFI boundary" rows of ERRORS.md are exercised: the callee must
/// narrow to the low byte and sign-extend it.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_hex_char_line: unsafe extern "C" fn(c_char),
    pub print_hex_char_line_widened: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(c_int),
}

/// The five symbols `nm -D` reports on the C `.so` (see SYMBOLS.md).
pub const EXPECTED_SYMBOLS: [&str; 5] = ["printLine", "printHexCharLine", "bad", "good", "driver"];

impl Api {
    fn load(name: &'static str, path: PathBuf) -> Api {
        // Leaked so the resolved function pointers are valid for 'static and
        // the library is never unloaded mid-test.
        let lib: &'static Library = Box::leak(Box::new(
            unsafe { Library::new(&path) }
                .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name)),
        ));
        unsafe {
            // Resolve every expected symbol up front so a missing export is a
            // loud failure rather than a silently skipped test.
            for s in EXPECTED_SYMBOLS {
                let mut z = s.as_bytes().to_vec();
                z.push(0);
                let r = lib.get::<unsafe extern "C" fn()>(&z);
                assert!(
                    r.is_ok(),
                    "symbol {s} missing from the {name} .so ({}): {:?}",
                    path.display(),
                    r.err()
                );
            }
            Api {
                name,
                print_line: *lib.get(b"printLine\0").unwrap(),
                print_hex_char_line: *lib.get(b"printHexCharLine\0").unwrap(),
                print_hex_char_line_widened: *lib.get(b"printHexCharLine\0").unwrap(),
                bad: *lib.get(b"bad\0").unwrap(),
                good: *lib.get(b"good\0").unwrap(),
                driver: *lib.get(b"driver\0").unwrap(),
                path,
            }
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The C `.so` and the Rust `.so`, built and `dlopen`ed on first use.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Api::load("C", c_so_path()),
        rust: Api::load("RUST", rust_so_path()),
    })
}

static C_SO: OnceLock<PathBuf> = OnceLock::new();
static RUST_SO: OnceLock<PathBuf> = OnceLock::new();

pub fn c_so_path() -> PathBuf {
    C_SO.get_or_init(build_c_lib).clone()
}

pub fn rust_so_path() -> PathBuf {
    RUST_SO.get_or_init(build_rust_lib).clone()
}

// ---------------------------------------------------------------------------
// stdout capture.
// ---------------------------------------------------------------------------

struct Cap {
    file: File,
}

static CAP: Mutex<Option<Cap>> = Mutex::new(None);

extern "C" {
    /// glibc's `stdout` `FILE *`. The test binary, the C `.so` and the Rust
    /// `.so` all bind to the same `libc.so.6`, so this is the very same `FILE`
    /// object both libraries' `printf` calls write through — which is what
    /// makes flushing it here sufficient for both.
    static mut stdout: *mut libc::FILE;
}

/// Flush the shared libc `stdout` stream.
unsafe fn fflush_libc_stdout() {
    libc::fflush(stdout);
}

/// Run `f` with file descriptor 1 redirected into a scratch file and return
/// everything that was written to it.
///
/// Both libraries write through the *same* process-global libc `stdout`
/// `FILE`, so `fflush` here flushes whichever of them just ran.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut guard = CAP.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        let path = std::env::temp_dir().join(format!("driver_diff_cap_{}.bin", std::process::id()));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("create capture scratch file");
        // Unlink immediately: the fd keeps it alive, nothing is left behind.
        let _ = std::fs::remove_file(&path);
        *guard = Some(Cap { file });
    }
    let cap = guard.as_mut().unwrap();
    let fd = cap.file.as_raw_fd();

    // Push out anything the *Rust* side has buffered (e.g. the test harness's
    // progress line) so it lands on the real stdout, not in our capture.
    let _ = std::io::stdout().flush();

    unsafe {
        fflush_libc_stdout();
        assert_eq!(libc::ftruncate(fd, 0), 0, "ftruncate");
        assert!(libc::lseek(fd, 0, libc::SEEK_SET) >= 0, "lseek");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1)");
        assert!(libc::dup2(fd, 1) >= 0, "dup2");
        f();
        // Flush the library's writes *while* fd 1 is still redirected.
        fflush_libc_stdout();
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore");
        libc::close(saved);
    }

    let mut buf = Vec::new();
    cap.file.seek(SeekFrom::Start(0)).expect("rewind capture");
    cap.file
        .read_to_end(&mut buf)
        .expect("read back capture");
    buf
}

// ---------------------------------------------------------------------------
// The differential assertion.
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(400) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 400 {
        s.push_str(&format!("... ({} bytes total)", b.len()));
    }
    s
}

/// Run the same closure against the C `.so` and the Rust `.so` and require the
/// bytes they emit to be identical. Returns the (shared) output.
pub fn assert_same<F: Fn(&Api)>(label: &str, f: F) -> Vec<u8> {
    let p = pair(); // build/dlopen outside the capture window
    let out_c = capture(|| f(&p.c));
    let out_rust = capture(|| f(&p.rust));
    if out_c != out_rust {
        panic!(
            "DIVERGENCE [{label}]\n  C    ({:>4} bytes): \"{}\"\n  RUST ({:>4} bytes): \"{}\"",
            out_c.len(),
            show(&out_c),
            out_rust.len(),
            show(&out_rust)
        );
    }
    out_c
}

/// `assert_same` plus a check against the byte string the C source says must
/// be produced (used where the C output is fully determined by constants).
pub fn assert_same_and_eq<F: Fn(&Api)>(label: &str, expected: &[u8], f: F) {
    let out = assert_same(label, f);
    assert_eq!(
        out,
        expected,
        "[{label}] both libraries agreed on \"{}\" but the C source mandates \"{}\"",
        show(&out),
        show(expected)
    );
}

/// Helper: `printLine` with a NUL-terminated copy of `bytes`.
pub fn call_print_line(api: &Api, bytes: &[u8]) {
    let cs = CString::new(bytes).expect("payload must not contain an interior NUL");
    unsafe { (api.print_line)(cs.as_ptr()) }
}

pub fn nm_defined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // "<addr> <type> <name>"; keep code/data symbols.
            if f.len() == 3 && matches!(f[1], "T" | "t" | "B" | "D" | "R" | "W" | "V") {
                Some(f[2].to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}
