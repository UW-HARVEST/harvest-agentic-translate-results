//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded through `libloading` — the Rust side is loaded from
//! its built `cdylib` exactly as an external C consumer would, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are part of what is under test.
//! No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

pub type FnI3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnV2 = unsafe extern "C" fn(c_int, c_int);
pub type FnApply = unsafe extern "C" fn(Option<FnI3>, c_int, c_int, c_int) -> c_int;
pub type FnShift = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnPtrData = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnDyn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnTime = unsafe extern "C" fn(c_int) -> c_int;
pub type FnRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
pub type FnHatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Mirrors the C `DataRecord`: `int id; int value; time_t timestamp; char name[32];`
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: i64,
    pub name: [i8; 32],
}

impl Default for DataRecord {
    fn default() -> Self {
        DataRecord { id: 0, value: 0, timestamp: 0, name: [0; 32] }
    }
}

impl std::fmt::Debug for DataRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataRecord {{ id: {}, value: {}, timestamp: {}, name: {:?} }}",
            self.id, self.value, self.timestamp, self.name
        )
    }
}

/// The 12 exported entry points of one library, resolved by symbol name.
pub struct Lib {
    pub tag: &'static str,
    _lib: Library,
    pub increment_counter: FnV2,
    pub update_accumulator: FnV2,
    pub apply_operation: FnApply,
    pub add_three: FnI3,
    pub multiply_add: FnI3,
    pub complex_calc: FnI3,
    pub shift_array_data: FnShift,
    pub process_pointer_data: FnPtrData,
    pub compute_with_dynamic_memory: FnDyn,
    pub get_time_based_value: FnTime,
    pub manipulate_records: FnRecords,
    pub hatch: FnHatch,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Fails if `artifact` is older than `source`. Without this, `cargo test`
/// (which builds the *debug* test binaries) would happily run against a stale
/// *release* `.so` and report a false pass after a source edit.
fn assert_fresh(artifact: &std::path::Path, source: &std::path::Path, how: &str) {
    let meta = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("cannot stat {}: {e}", p.display()))
    };
    let a = meta(artifact);
    let s = meta(source);
    assert!(
        a >= s,
        "STALE ARTIFACT: {} is older than {}. Rebuild with `{}` before testing, \
         otherwise the differential result is meaningless.",
        artifact.display(),
        source.display(),
        how
    );
}

pub fn c_so_path() -> PathBuf {
    let build = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one C .so in {}, found {found:?}", build.display());
    let so = found.pop().unwrap();
    assert_fresh(&so, &repo_root().join("c_src/src/lib.c"), "cmake --build c_src/build");
    so
}

pub fn rust_so_path() -> PathBuf {
    let p = repo_root().join("translation/target/release/libhatch_lib.so");
    assert!(p.exists(), "{} missing — run `cargo build --release` first", p.display());
    assert_fresh(&p, &repo_root().join("translation/src/lib.rs"), "cargo build --release");
    p
}

macro_rules! resolve {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("symbol `{}` not found: {e}", $name));
        unsafe { *s.into_raw() }
    }};
}

impl Lib {
    pub fn open(tag: &'static str, path: &std::path::Path) -> Lib {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let l = Lib {
            tag,
            increment_counter: resolve!(lib, "increment_counter", FnV2),
            update_accumulator: resolve!(lib, "update_accumulator", FnV2),
            apply_operation: resolve!(lib, "apply_operation", FnApply),
            add_three: resolve!(lib, "add_three", FnI3),
            multiply_add: resolve!(lib, "multiply_add", FnI3),
            complex_calc: resolve!(lib, "complex_calc", FnI3),
            shift_array_data: resolve!(lib, "shift_array_data", FnShift),
            process_pointer_data: resolve!(lib, "process_pointer_data", FnPtrData),
            compute_with_dynamic_memory: resolve!(lib, "compute_with_dynamic_memory", FnDyn),
            get_time_based_value: resolve!(lib, "get_time_based_value", FnTime),
            manipulate_records: resolve!(lib, "manipulate_records", FnRecords),
            hatch: resolve!(lib, "hatch", FnHatch),
            _lib: lib,
        };
        l
    }
}

/// A C/Rust library pair. NOTE: `dlopen` de-duplicates by path, so there is
/// exactly ONE copy of each library (and therefore ONE copy of each file-scope
/// `static`) per test process. Tests must therefore (a) hold [`lock`] for their
/// whole body and (b) call [`Pair::reset`] if they need pristine statics.
pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    /// Reads `global_counter` out of a library. `complex_calc(0,0,0)` is
    /// `(0-0)*0 + global_counter`, i.e. the raw value.
    pub fn counter(l: &Lib) -> c_int {
        unsafe { (l.complex_calc)(0, 0, 0) }
    }
    /// Reads `global_accumulator`. `process_pointer_data(&0, 0)` is
    /// `0*0 + global_accumulator`.
    pub fn accumulator(l: &Lib) -> c_int {
        let mut zero: c_int = 0;
        unsafe { (l.process_pointer_data)(&mut zero, 0) }
    }
    /// Forces both statics of both libraries back to 0, using only exported
    /// symbols: `increment_counter(-x)` cancels the counter, and
    /// `update_accumulator(-(2*acc))` cancels the accumulator because
    /// `acc*2 + (-(acc*2)) == 0` under two's-complement wrapping.
    pub fn reset(&self) {
        for l in [&self.c, &self.r] {
            let cnt = Self::counter(l);
            unsafe { (l.increment_counter)(cnt.wrapping_neg(), 0) };
            let acc = Self::accumulator(l);
            unsafe { (l.update_accumulator)(acc.wrapping_mul(2).wrapping_neg(), 0) };
            assert_eq!(Self::counter(l), 0, "{} global_counter reset failed", l.tag);
            assert_eq!(Self::accumulator(l), 0, "{} global_accumulator reset failed", l.tag);
        }
    }
}

fn open_pair_raw() -> Pair {
    Pair { c: Lib::open("C", &c_so_path()), r: Lib::open("RUST", &rust_so_path()) }
}

/// The single process-wide pair. Always use this; never `dlopen` again.
pub fn shared() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(open_pair_raw)
}

/// Serialises access to the shared global state of the two libraries.
pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Acquire the lock, get the shared pair, and zero both libraries' statics.
pub fn fresh() -> (std::sync::MutexGuard<'static, ()>, &'static Pair) {
    let g = lock();
    let p = shared();
    p.reset();
    (g, p)
}

/// Acquire the lock and get the shared pair. Both libraries' statics are zeroed
/// first, so every test starts from an identical, known state regardless of
/// which tests ran before it (and regardless of whether one of them failed
/// part-way through and left the two libraries out of step).
pub fn locked() -> (std::sync::MutexGuard<'static, ()>, &'static Pair) {
    fresh()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

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
    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Mix of small values, boundary values, and full-range values — so a row
    /// exercises both the plain-arithmetic and the wrapping code paths.
    pub fn i32_mixed(&mut self) -> i32 {
        const BOUNDS: [i32; 12] = [
            0, 1, -1, 2, -2, 3, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 100, -100,
        ];
        match self.next_u64() % 4 {
            0 => self.range(-1000, 1000),
            1 => BOUNDS[(self.next_u64() % BOUNDS.len() as u64) as usize],
            _ => self.i32_any(),
        }
    }
    pub fn i64_any(&mut self) -> i64 {
        self.next_u64() as i64
    }
    pub fn i8_any(&mut self) -> i8 {
        self.next_u64() as i8
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(what: &str, ctx: impl std::fmt::Debug, c: c_int, r: c_int) {
    assert_eq!(c, r, "{what} diverged for input {ctx:?}: C returned {c}, Rust returned {r}");
}

#[track_caller]
pub fn eq_bytes<T: Copy + PartialEq + std::fmt::Debug>(
    what: &str,
    ctx: impl std::fmt::Debug,
    c: &[T],
    r: &[T],
) {
    assert_eq!(c.len(), r.len(), "{what}: buffer length mismatch for {ctx:?}");
    if c != r {
        for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
            assert!(a == b, "{what} buffer diverged at index {i} for input {ctx:?}: C={a:?}, Rust={b:?}");
        }
    }
}

/// Raw-byte view of a slice, for byte-for-byte struct comparison including any
/// padding the C compiler may leave in `DataRecord`.
pub fn as_raw_bytes<T>(s: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(s.as_ptr() as *const u8, std::mem::size_of_val(s)) }
}

// ---------------------------------------------------------------------------
// Crash-equivalence helper: run a closure in a forked child and report how it
// died, so UB paths (NULL deref, allocation failure) can be compared without
// taking down the test process.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Forks, runs `f` in the child, and reports the child's termination status.
/// The child `_exit(0)`s if `f` returns normally.
pub fn outcome_of<F: FnOnce()>(f: F) -> Outcome {
    outcome_of_limited(0, f)
}

/// As [`outcome_of`], but if `as_limit_bytes != 0` the child's `RLIMIT_AS` is
/// lowered to that value first, so that `malloc` genuinely fails for large
/// requests. Used to exercise the allocation-failure path of
/// `compute_with_dynamic_memory` identically in both libraries.
pub fn outcome_of_limited<F: FnOnce()>(as_limit_bytes: u64, f: F) -> Outcome {
    // Flush so the child does not duplicate buffered output.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: silence the crash report, apply the limit, run, then hard-exit.
        unsafe {
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull >= 0 {
                libc::dup2(devnull, 2);
            }
            // Crash rows are expected; don't pay for a core dump each time.
            let no_core = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            libc::setrlimit(libc::RLIMIT_CORE, &no_core);
            if as_limit_bytes != 0 {
                let rl = libc::rlimit {
                    rlim_cur: as_limit_bytes as libc::rlim_t,
                    rlim_max: as_limit_bytes as libc::rlim_t,
                };
                libc::setrlimit(libc::RLIMIT_AS, &rl);
            }
        }
        f();
        unsafe { libc::_exit(0) };
    }
    let mut status: c_int = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    if libc::WIFSIGNALED(status) {
        Outcome::Signaled(libc::WTERMSIG(status))
    } else if libc::WIFEXITED(status) {
        Outcome::Exited(libc::WEXITSTATUS(status))
    } else {
        panic!("unexpected wait status {status}");
    }
}

/// Keeps the compiler from optimising away a value read in a crash probe.
pub fn blackhole(v: c_int) {
    unsafe { std::ptr::write_volatile(&raw mut BH, v) };
}
static mut BH: c_int = 0;

pub fn null_int() -> *mut c_int {
    std::ptr::null_mut()
}
pub fn null_rec() -> *mut DataRecord {
    std::ptr::null_mut()
}
pub fn _unused(_: *mut c_void) {}
