//! Shared differential-test harness.
//!
//! Both the C library and the Rust library are loaded **through `libloading`**
//! and every call goes through a `.so` export. Nothing in this harness calls a
//! Rust function directly, so the `#[no_mangle] extern "C"` wrappers are part of
//! what is under test.
//!
//! ## Why everything is serialized
//!
//! `lib.c` keeps two `static int`s (`global_counter`, `global_accumulator`) that
//! several entry points read and write. The C `.so` and the Rust `.so` each own
//! their own copy. They only stay comparable if *every* call made to one library
//! is mirrored by the identical call to the other, in the same order. So:
//!
//! * the pair of libraries is a process-wide singleton,
//! * `lock()` serializes tests (they would otherwise interleave on the shared
//!   state, since `cargo test` runs `#[test]`s on multiple threads),
//! * the `both_*` helpers always call C first and Rust immediately after, so
//!   mirroring is structural rather than something each test must remember.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type CTimeT = i64;

pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int>;
pub type ModifierFunc = Option<unsafe extern "C" fn(c_int, c_int)>;

/// Mirror of the C `DataRecord` (`int id; int value; time_t timestamp; char name[32];`).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: CTimeT,
    pub name: [c_char; 32],
}

impl DataRecord {
    pub fn zeroed() -> Self {
        DataRecord {
            id: 0,
            value: 0,
            timestamp: 0,
            name: [0; 32],
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// Repository root (parent of the `translation` crate directory).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    // Lets the runner script point the whole suite at a differently-configured
    // C build (e.g. an optimized one) without touching c_src.
    if let Some(p) = std::env::var_os("HATCH_C_SO") {
        return PathBuf::from(p);
    }
    if let Some(p) = find_c_so() {
        return p;
    }
    build_c_library();
    find_c_so().unwrap_or_else(|| {
        panic!(
            "no C .so in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            repo_root().join("c_src").join("build").display()
        )
    })
}

fn find_c_so() -> Option<PathBuf> {
    let build = repo_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.pop()
}

/// Builds `c_src` exactly the way the task documents. Writes only into
/// `c_src/build` (a build artifact directory); no C source is touched.
fn build_c_library() {
    let c_src = repo_root().join("c_src");
    let build = c_src.join("build");
    let _ = std::fs::create_dir_all(&build);
    let cmake = std::process::Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output();
    if let Ok(o) = cmake {
        if !o.status.success() {
            eprintln!("cmake configure failed:\n{}", String::from_utf8_lossy(&o.stderr));
        }
    }
    let _ = std::process::Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output();
}

pub fn rust_so_path() -> PathBuf {
    // `HATCH_RUST_SO` lets the runner script test the debug cdylib, which has
    // `overflow-checks = true` — any non-wrapping arithmetic left in the
    // translation aborts there instead of silently matching.
    if let Some(p) = std::env::var_os("HATCH_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer an already-built cdylib (release first, so `cargo build --release`
    // followed by `cargo test` uses the optimized artifact).
    for r in [
        target.join("release"),
        target.join("debug"),
        target.join("cdylib").join("release"),
    ] {
        let p = r.join("libhatch_lib.so");
        if p.exists() {
            return p;
        }
    }
    // `cargo test` does NOT build the cdylib target (it links the lib as an
    // rlib), so bootstrap it here. A separate --target-dir avoids contending
    // for the build lock held by the outer cargo invocation.
    build_rust_cdylib();
    let p = target.join("cdylib").join("release").join("libhatch_lib.so");
    if p.exists() {
        return p;
    }
    panic!(
        "libhatch_lib.so not found; build it with: cd translation && cargo build --release"
    );
}

fn build_rust_cdylib() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = std::process::Command::new(env!("CARGO"))
        .current_dir(&manifest)
        .args(["build", "--release", "--target-dir"])
        .arg(manifest.join("target").join("cdylib"))
        .output();
    match out {
        Ok(o) if !o.status.success() => {
            panic!(
                "auto-building the cdylib failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            )
        }
        Err(e) => panic!("could not run cargo to build the cdylib: {e}"),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// One loaded library
// ---------------------------------------------------------------------------

/// A `.so` plus resolved pointers to each of the 12 exported symbols.
///
/// The raw `extern "C"` function pointers are extracted once at load time (the
/// `Library` is leaked so they stay valid for the whole process).
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub increment_counter: unsafe extern "C" fn(c_int, c_int),
    pub update_accumulator: unsafe extern "C" fn(c_int, c_int),
    pub apply_operation: unsafe extern "C" fn(OperationFunc, c_int, c_int, c_int) -> c_int,
    pub add_three: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
    pub multiply_add: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
    pub complex_calc: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
    pub shift_array_data: unsafe extern "C" fn(*mut c_int, c_int, c_int),
    pub process_pointer_data: unsafe extern "C" fn(*mut c_int, c_int) -> c_int,
    pub compute_with_dynamic_memory: unsafe extern "C" fn(c_int, c_int) -> c_int,
    pub get_time_based_value: unsafe extern "C" fn(c_int) -> c_int,
    pub manipulate_records: unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int,
    pub hatch: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

macro_rules! sym {
    ($lib:expr, $path:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("{} missing {}: {e}", $path.display(), $name));
        *s
    }};
}

impl Lib {
    pub fn load(name: &'static str, path: PathBuf) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        // Leaked on purpose: the extracted fn pointers must outlive this scope.
        let lib: &'static Library = Box::leak(Box::new(lib));

        Lib {
            name,
            increment_counter: sym!(lib, path, "increment_counter", unsafe extern "C" fn(c_int, c_int)),
            update_accumulator: sym!(lib, path, "update_accumulator", unsafe extern "C" fn(c_int, c_int)),
            apply_operation: sym!(
                lib,
                path,
                "apply_operation",
                unsafe extern "C" fn(OperationFunc, c_int, c_int, c_int) -> c_int
            ),
            add_three: sym!(lib, path, "add_three", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int),
            multiply_add: sym!(lib, path, "multiply_add", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int),
            complex_calc: sym!(lib, path, "complex_calc", unsafe extern "C" fn(c_int, c_int, c_int) -> c_int),
            shift_array_data: sym!(lib, path, "shift_array_data", unsafe extern "C" fn(*mut c_int, c_int, c_int)),
            process_pointer_data: sym!(
                lib,
                path,
                "process_pointer_data",
                unsafe extern "C" fn(*mut c_int, c_int) -> c_int
            ),
            compute_with_dynamic_memory: sym!(
                lib,
                path,
                "compute_with_dynamic_memory",
                unsafe extern "C" fn(c_int, c_int) -> c_int
            ),
            get_time_based_value: sym!(lib, path, "get_time_based_value", unsafe extern "C" fn(c_int) -> c_int),
            manipulate_records: sym!(
                lib,
                path,
                "manipulate_records",
                unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int
            ),
            hatch: sym!(lib, path, "hatch", unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int),
            path,
        }
    }
}

// ---------------------------------------------------------------------------
// The singleton pair + serialization lock
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

// Safety: the two `Lib`s hold only plain `extern "C"` fn pointers. Access is
// serialized through `lock()`, matching the (non-thread-safe) C library.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Lib::load("C", c_so_path()),
        r: Lib::load("Rust", rust_so_path()),
    })
}

/// Serializes a test against every other test. Hold it for the whole test body:
/// the libraries carry mutable global state that must advance in lockstep.
pub fn lock() -> MutexGuard<'static, ()> {
    match LOCK.lock() {
        Ok(g) => g,
        // A previously panicking test poisoned the lock; the panic already
        // reported the divergence, so keep going rather than cascading.
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — property-style inputs, fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Rng {
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
    /// Uniform over the whole `i32` domain (so `INT_MIN`/`INT_MAX` neighbourhoods
    /// and overflowing operands really do occur).
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small magnitudes, both signs — the values a "normal" caller passes.
    pub fn i32_small(&mut self) -> i32 {
        (self.next_u32() % 2001) as i32 - 1000
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// A grab-bag biased toward interesting `int` values.
    pub fn i32_interesting(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            3 => -1,
            4 => 1,
            5 => self.i32_small(),
            _ => self.i32_any(),
        }
    }
}

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub fn rng() -> Rng {
    Rng::new(SEED)
}

/// Boundary `i32` values used for exhaustive small cross-products.
pub const EDGE: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// Mirrored call helpers: C first, Rust immediately after, then compare.
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_eq_ctx(ctx: impl std::fmt::Display, c: c_int, r: c_int) {
    assert_eq!(
        c, r,
        "DIVERGENCE at {ctx}: C returned {c} (0x{c:08x}) but Rust returned {r} (0x{r:08x})"
    );
}

#[track_caller]
pub fn both_add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.add_three)(a, b, c) };
    let rv = unsafe { (p.r.add_three)(a, b, c) };
    assert_eq_ctx(format!("add_three({a}, {b}, {c})"), cv, rv);
    cv
}

#[track_caller]
pub fn both_multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.multiply_add)(a, b, c) };
    let rv = unsafe { (p.r.multiply_add)(a, b, c) };
    assert_eq_ctx(format!("multiply_add({a}, {b}, {c})"), cv, rv);
    cv
}

#[track_caller]
pub fn both_complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.complex_calc)(a, b, c) };
    let rv = unsafe { (p.r.complex_calc)(a, b, c) };
    assert_eq_ctx(format!("complex_calc({a}, {b}, {c})"), cv, rv);
    cv
}

#[track_caller]
pub fn both_increment_counter(value: c_int, unused: c_int) {
    let p = libs();
    unsafe { (p.c.increment_counter)(value, unused) };
    unsafe { (p.r.increment_counter)(value, unused) };
}

#[track_caller]
pub fn both_update_accumulator(value: c_int, unused: c_int) {
    let p = libs();
    unsafe { (p.c.update_accumulator)(value, unused) };
    unsafe { (p.r.update_accumulator)(value, unused) };
}

#[track_caller]
pub fn both_process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    let p = libs();
    let mut cbuf: c_int = value;
    let mut rbuf: c_int = value;
    let cv = unsafe { (p.c.process_pointer_data)(&mut cbuf, multiplier) };
    let rv = unsafe { (p.r.process_pointer_data)(&mut rbuf, multiplier) };
    assert_eq_ctx(
        format!("process_pointer_data(*ptr={value}, {multiplier})"),
        cv,
        rv,
    );
    assert_eq!(cbuf, rbuf, "process_pointer_data must not modify *ptr");
    assert_eq!(cbuf, value, "process_pointer_data must not modify *ptr");
    cv
}

#[track_caller]
pub fn both_compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.compute_with_dynamic_memory)(base, count) };
    let rv = unsafe { (p.r.compute_with_dynamic_memory)(base, count) };
    assert_eq_ctx(
        format!("compute_with_dynamic_memory({base}, {count})"),
        cv,
        rv,
    );
    cv
}

#[track_caller]
pub fn both_get_time_based_value(seed: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.get_time_based_value)(seed) };
    let rv = unsafe { (p.r.get_time_based_value)(seed) };
    assert_eq_ctx(format!("get_time_based_value({seed})"), cv, rv);
    cv
}

#[track_caller]
pub fn both_hatch(p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.hatch)(p1, p2, p3, p4) };
    let rv = unsafe { (p.r.hatch)(p1, p2, p3, p4) };
    assert_eq_ctx(format!("hatch({p1}, {p2}, {p3}, {p4})"), cv, rv);
    cv
}

#[track_caller]
pub fn both_apply_operation_with(
    label: &str,
    c_op: OperationFunc,
    r_op: OperationFunc,
    a: c_int,
    b: c_int,
    c: c_int,
) -> c_int {
    let p = libs();
    let cv = unsafe { (p.c.apply_operation)(c_op, a, b, c) };
    let rv = unsafe { (p.r.apply_operation)(r_op, a, b, c) };
    assert_eq_ctx(format!("apply_operation({label}, {a}, {b}, {c})"), cv, rv);
    cv
}

/// `shift_array_data` on identical buffers; compares the *whole* buffer
/// afterwards, including any slack past `size`, so stray writes are caught.
#[track_caller]
pub fn both_shift_array_data(data: &[c_int], size: c_int, shift_by: c_int) -> Vec<c_int> {
    let p = libs();
    let mut cbuf = data.to_vec();
    let mut rbuf = data.to_vec();
    unsafe { (p.c.shift_array_data)(cbuf.as_mut_ptr(), size, shift_by) };
    unsafe { (p.r.shift_array_data)(rbuf.as_mut_ptr(), size, shift_by) };
    assert_eq!(
        cbuf, rbuf,
        "DIVERGENCE in shift_array_data(len={}, size={size}, shift_by={shift_by}) buffer state\n  C:    {:?}\n  Rust: {:?}",
        data.len(), cbuf, rbuf
    );
    cbuf
}

/// `manipulate_records` on identical buffers; compares return value *and* the
/// full 48-byte-per-record post-state.
#[track_caller]
pub fn both_manipulate_records(
    records: &[DataRecord],
    num_records: c_int,
    shift: c_int,
) -> (c_int, Vec<DataRecord>) {
    let p = libs();
    let mut cbuf = records.to_vec();
    let mut rbuf = records.to_vec();
    let cv = unsafe { (p.c.manipulate_records)(cbuf.as_mut_ptr(), num_records, shift) };
    let rv = unsafe { (p.r.manipulate_records)(rbuf.as_mut_ptr(), num_records, shift) };
    assert_eq_ctx(
        format!(
            "manipulate_records(len={}, num_records={num_records}, shift={shift})",
            records.len()
        ),
        cv,
        rv,
    );
    for i in 0..cbuf.len() {
        assert_eq!(
            cbuf[i], rbuf[i],
            "DIVERGENCE in manipulate_records(len={}, num_records={num_records}, shift={shift}) at record {i}",
            records.len()
        );
    }
    (cv, cbuf)
}

// ---------------------------------------------------------------------------
// Misc helpers
// ---------------------------------------------------------------------------

pub fn random_records(rng: &mut Rng, n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut name = [0 as c_char; 32];
            for b in name.iter_mut() {
                // Non-zero payload so a short/wrong memmove is visible.
                *b = (rng.byte() | 1) as c_char;
            }
            DataRecord {
                id: i as c_int,
                value: rng.i32_small(),
                timestamp: rng.next_u64() as i64,
                name,
            }
        })
        .collect()
}

pub fn as_void(f: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int) -> *mut c_void {
    f as *mut c_void
}
