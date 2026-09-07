// Shared differential-test harness.
//
// BOTH libraries are loaded as shared objects via `libloading` and every call
// goes through the exported C ABI symbols -- the Rust functions are never called
// directly, so the `#[no_mangle]`/`extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_double, c_int, c_long};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type TimeT = c_long;

pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

macro_rules! getsym {
    ($self:ident, $name:literal, $t:ty) => {{
        let s: Symbol<$t> = unsafe {
            $self
                .lib
                .get($name)
                .unwrap_or_else(|e| {
                    panic!(
                        "{}: missing symbol {}: {e}",
                        $self.name,
                        String::from_utf8_lossy(&$name[..$name.len() - 1])
                    )
                })
        };
        *s
    }};
}

impl Lib {
    pub fn classify_mode(&self, s: *const c_char) -> c_int {
        let f = getsym!(self, b"classify_mode\0", unsafe extern "C" fn(*const c_char) -> c_int);
        unsafe { f(s) }
    }
    pub fn classify_mode_bytes(&self, s: &[u8]) -> c_int {
        // caller must supply a NUL-terminated buffer
        assert_eq!(s.last(), Some(&0u8), "test bug: string not NUL terminated");
        self.classify_mode(s.as_ptr() as *const c_char)
    }
    pub fn apply_multiplier(&self, base: c_int, level: c_int) -> c_int {
        let f = getsym!(self, b"apply_multiplier\0", unsafe extern "C" fn(c_int, c_int) -> c_int);
        unsafe { f(base, level) }
    }
    pub fn convert_time_factor(&self, factor: c_double) -> c_int {
        let f = getsym!(self, b"convert_time_factor\0", unsafe extern "C" fn(c_double) -> c_int);
        unsafe { f(factor) }
    }
    pub fn convert_negative_overflow(&self, v: c_double) -> c_int {
        let f = getsym!(
            self,
            b"convert_negative_overflow\0",
            unsafe extern "C" fn(c_double) -> c_int
        );
        unsafe { f(v) }
    }
    pub fn get_modified_time(&self, days: c_int, hours: c_int) -> TimeT {
        let f = getsym!(self, b"get_modified_time\0", unsafe extern "C" fn(c_int, c_int) -> TimeT);
        unsafe { f(days, hours) }
    }
    pub fn hash_time_value(&self, t: TimeT) -> c_int {
        let f = getsym!(self, b"hash_time_value\0", unsafe extern "C" fn(TimeT) -> c_int);
        unsafe { f(t) }
    }
    pub fn modeselect(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        let f = getsym!(
            self,
            b"modeselect\0",
            unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
        );
        unsafe { f(a, b, c, d) }
    }
    pub fn has(&self, sym: &[u8]) -> bool {
        unsafe { self.lib.get::<*const ()>(sym).is_ok() }
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    // Escape hatch used by run_all.sh to re-run the whole suite against a C
    // library built at a different optimisation level / with a different
    // compiler, proving the Rust matches the C *semantics* and not one
    // particular codegen accident.
    if let Some(p) = std::env::var_os("DIFF_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DIFF_C_SO points at a missing file: {}", p.display());
        return p;
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/, so walk up to the profile dir.
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // deps/
    let deps = dir.clone();
    dir.pop(); // <profile>/
    let target = workspace_root().join("translation/target");
    // Prefer the profile the tests were built with, then any sibling profile.
    let bases = [
        dir.clone(),
        deps,
        target.join("debug"),
        target.join("release"),
    ];
    for base in bases {
        let p = base.join("libmodeselect_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmodeselect_lib.so not found near {}. `cargo test` does not build the \
         cdylib -- run `cargo build` (same profile) first, or use ./run_all.sh.",
        dir.display()
    );
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| {
        let cp = find_c_so();
        let rp = find_rust_so();
        let c = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", cp.display()));
        let rust = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rp.display()));
        Pair {
            c: Lib { name: "C", lib: c },
            rust: Lib { name: "Rust", lib: rust },
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) -- fixed seed => reproducible test inputs.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A random f64 spread over many magnitudes (both signs), always finite.
    pub fn next_f64_magnitudes(&mut self, max_exp: i32) -> f64 {
        let mantissa = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        let exp = (self.below((2 * max_exp as u64) + 1) as i32) - max_exp;
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        sign * mantissa * 2f64.powi(exp)
    }
    /// Arbitrary bit pattern reinterpreted as f64: NaNs, infinities, denormals.
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers with informative messages.
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(ctx: impl std::fmt::Display, c: c_int, r: c_int) {
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: C returned {c} (0x{c:X}) but Rust returned {r} (0x{r:X})"
    );
}

#[track_caller]
pub fn eq_i64(ctx: impl std::fmt::Display, c: TimeT, r: TimeT) {
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: C returned {c} (0x{c:X}) but Rust returned {r} (0x{r:X})"
    );
}

pub fn cstring(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

pub const MODES: [&str; 4] = ["standard", "enhanced", "turbo", "extreme"];

// ---------------------------------------------------------------------------
// fork()-based helpers.
//
// Needed for two things the in-process harness cannot do:
//   * capture the raw fd-1 output that the libraries' `printf` calls produce
//     (cargo's test harness only captures Rust's own print macros);
//   * observe fatal signals (the C `strcmp(NULL, ..)` path) without killing the
//     test process.
// ---------------------------------------------------------------------------

/// Outcome of a forked child.
#[derive(Debug, PartialEq, Eq)]
pub enum Child {
    Exited(i32),
    Signaled(i32),
}

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let n = std::process::id();
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.join(format!("difftest-{tag}-{n}-{uniq}.bin"))
}

/// Run `f` in a forked child with fd 1 redirected to a fresh file; return the
/// bytes written plus how the child terminated.
pub fn capture_stdout(tag: &str, f: impl FnOnce()) -> (Vec<u8>, Child) {
    use std::os::unix::io::AsRawFd;

    let path = temp_path(tag);
    let file = std::fs::File::create(&path).expect("create capture file");

    // Flush the parent's own stdio so the child does not re-emit buffered bytes.
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            if libc::dup2(file.as_raw_fd(), 1) < 0 {
                libc::_exit(101);
            }
        }
        f();
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    }

    // ---- parent ----
    drop(file);
    let mut status: i32 = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    let outcome = if libc::WIFEXITED(status) {
        Child::Exited(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        Child::Signaled(libc::WTERMSIG(status))
    } else {
        Child::Exited(-1)
    };
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (bytes, outcome)
}

/// Run `f` in a forked child with fd 1 and fd 2 sent to /dev/null; report only
/// how the child terminated. Used for the crash-parity checks.
pub fn run_isolated(f: impl FnOnce()) -> Child {
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 1);
                libc::dup2(devnull, 2);
            }
        }
        f();
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    }
    let mut status: i32 = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    if libc::WIFEXITED(status) {
        Child::Exited(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        Child::Signaled(libc::WTERMSIG(status))
    } else {
        Child::Exited(-1)
    }
}
