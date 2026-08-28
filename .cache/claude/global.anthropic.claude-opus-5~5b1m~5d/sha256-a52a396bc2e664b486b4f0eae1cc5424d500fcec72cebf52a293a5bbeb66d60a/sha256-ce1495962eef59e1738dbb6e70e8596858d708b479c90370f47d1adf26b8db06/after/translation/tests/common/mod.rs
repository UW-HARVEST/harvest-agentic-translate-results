// Differential-test harness.
//
// Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
// exclusively through their exported symbols — the Rust crate is NEVER linked
// or called directly, so the `#[no_mangle] extern "C"` wrappers are part of
// what is under test.
//
// IMPORTANT: `lib.c` keeps two `static int`s (`global_counter`,
// `global_accumulator`) that are read and written by several entry points, so
// results depend on call history. Every test therefore
//   * takes the global `LOCK` (so the two libraries always observe the exact
//     same *sequence* of calls), and
//   * normalises the hidden state with `Libs::set_state(..)` on entry.
//
// The hidden state is observable/settable purely through the public API:
//   read  counter      = complex_calc(0, 0, 0)              == global_counter
//   read  accumulator  = process_pointer_data(&any, 0)      == global_accumulator
//   write counter      = increment_counter(target - cur, _)
//   write accumulator  = update_accumulator(target - 2*cur, _)

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type CTimeT = i64;

/// Mirror of the `DataRecord` struct in `c_src/src/lib.c`.
/// Verified against gcc on this host: size 48, align 8, offsets 0/4/8/16.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: CTimeT,
    pub name: [c_char; 32],
}

impl DataRecord {
    pub fn zeroed() -> Self {
        DataRecord { id: 0, value: 0, timestamp: 0, name: [0; 32] }
    }
}

pub type FnMod = unsafe extern "C" fn(c_int, c_int);
pub type FnOp3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnApply = unsafe extern "C" fn(*const c_void, c_int, c_int, c_int) -> c_int;
pub type FnShift = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcPtr = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnCwdm = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnTime = unsafe extern "C" fn(c_int) -> c_int;
pub type FnRecords = unsafe extern "C" fn(*mut DataRecord, c_int, c_int) -> c_int;
pub type FnHatch = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// The 12 exported entry points of one shared object.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
    pub increment_counter: FnMod,
    pub update_accumulator: FnMod,
    pub apply_operation: FnApply,
    pub add_three: FnOp3,
    pub multiply_add: FnOp3,
    pub complex_calc: FnOp3,
    pub shift_array_data: FnShift,
    pub process_pointer_data: FnProcPtr,
    pub compute_with_dynamic_memory: FnCwdm,
    pub get_time_based_value: FnTime,
    pub manipulate_records: FnRecords,
    pub hatch: FnHatch,
}

macro_rules! sym {
    ($lib:expr, $ty:ty, $n:literal) => {{
        let s: libloading::Symbol<$ty> = unsafe { $lib.get($n) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", stringify!($n)));
        *s
    }};
}

impl Api {
    fn open(name: &'static str, path: &Path) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let api = Api {
            name,
            path: path.to_path_buf(),
            increment_counter: sym!(lib, FnMod, b"increment_counter\0"),
            update_accumulator: sym!(lib, FnMod, b"update_accumulator\0"),
            apply_operation: sym!(lib, FnApply, b"apply_operation\0"),
            add_three: sym!(lib, FnOp3, b"add_three\0"),
            multiply_add: sym!(lib, FnOp3, b"multiply_add\0"),
            complex_calc: sym!(lib, FnOp3, b"complex_calc\0"),
            shift_array_data: sym!(lib, FnShift, b"shift_array_data\0"),
            process_pointer_data: sym!(lib, FnProcPtr, b"process_pointer_data\0"),
            compute_with_dynamic_memory: sym!(lib, FnCwdm, b"compute_with_dynamic_memory\0"),
            get_time_based_value: sym!(lib, FnTime, b"get_time_based_value\0"),
            manipulate_records: sym!(lib, FnRecords, b"manipulate_records\0"),
            hatch: sym!(lib, FnHatch, b"hatch\0"),
            lib,
        };
        api
    }

    /// Raw address of an exported symbol, for use as a `operation_func` argument.
    pub fn addr(&self, sym: &[u8]) -> *const c_void {
        let s: libloading::Symbol<*const c_void> = unsafe { self.lib.get(sym) }
            .unwrap_or_else(|e| panic!("missing symbol {sym:?}: {e}"));
        unsafe { s.into_raw().into_raw() as *const c_void }
    }

    /// `global_counter`, read through `complex_calc(0,0,0) == (0-0)*0 + counter`.
    pub fn read_counter(&self) -> c_int {
        unsafe { (self.complex_calc)(0, 0, 0) }
    }

    /// `global_accumulator`, read through
    /// `process_pointer_data(&v, 0) == v*0 + accumulator`.
    pub fn read_accumulator(&self) -> c_int {
        let mut v: c_int = 0;
        unsafe { (self.process_pointer_data)(&mut v, 0) }
    }

    /// Drive the hidden state to an exact value using only public entry points.
    pub fn set_state(&self, counter: c_int, accumulator: c_int) {
        let cur_c = self.read_counter();
        unsafe { (self.increment_counter)(counter.wrapping_sub(cur_c), 0) };
        let cur_a = self.read_accumulator();
        unsafe { (self.update_accumulator)(accumulator.wrapping_sub(cur_a.wrapping_mul(2)), 0) };
        assert_eq!(self.read_counter(), counter, "{}: set_state counter", self.name);
        assert_eq!(
            self.read_accumulator(),
            accumulator,
            "{}: set_state accumulator",
            self.name
        );
    }
}

pub struct Libs {
    pub c: Api,
    pub r: Api,
}

impl Libs {
    /// Normalise the hidden state of *both* libraries and cross-check that the
    /// two read the back the same values.
    pub fn set_state(&self, counter: c_int, accumulator: c_int) {
        self.c.set_state(counter, accumulator);
        self.r.set_state(counter, accumulator);
        assert_eq!(self.c.read_counter(), self.r.read_counter());
        assert_eq!(self.c.read_accumulator(), self.r.read_accumulator());
    }

    pub fn reset(&self) {
        self.set_state(0, 0);
    }
}

static LIBS: OnceLock<Mutex<Libs>> = OnceLock::new();

/// Take the global harness lock. Poisoning is ignored so that one failing test
/// does not turn every later test into an unrelated "poisoned" failure.
pub fn libs() -> MutexGuard<'static, Libs> {
    LIBS.get_or_init(|| {
        Mutex::new(Libs {
            c: Api::open("C", &c_so_path()),
            r: Api::open("Rust", &rust_so_path()),
        })
    })
    .lock()
    .unwrap_or_else(|e| e.into_inner())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — CMake derives the project (and therefore the
/// library) name from the name of the directory containing `c_src`, so it is
/// discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HATCH_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") && p.is_file() {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// The Rust cdylib to test.
///
/// `cargo test` does NOT build a `cdylib`-only library target, so the `.so` is
/// always produced by a separate `cargo build`. Resolution is therefore explicit
/// and deterministic: `$HATCH_RUST_SO`, else `target/release`, else
/// `target/debug`. `run_all_features.sh` sets `$HATCH_RUST_SO` so it can pin a
/// specific profile.
///
/// A STALENESS CHECK guards against the trap of silently testing a leftover
/// artifact from an earlier build (which would make the whole suite pass
/// vacuously): if the chosen `.so` is older than `src/lib.rs`, the harness
/// panics instead of reporting green.
pub fn rust_so_path() -> PathBuf {
    let chosen = if let Ok(p) = std::env::var("HATCH_RUST_SO") {
        PathBuf::from(p)
    } else {
        let candidates = [
            manifest_dir().join("target/release/libhatch_lib.so"),
            manifest_dir().join("target/debug/libhatch_lib.so"),
        ];
        match candidates.iter().find(|c| c.is_file()) {
            Some(c) => c.clone(),
            None => panic!(
                "no Rust cdylib found (tried {candidates:?}); build it with `cargo build --release`"
            ),
        }
    };
    assert!(
        chosen.is_file(),
        "Rust cdylib {} does not exist; run `cargo build --release`",
        chosen.display()
    );
    assert_fresh(&chosen);
    chosen
}

/// Refuse to test a `.so` that predates the source it is supposed to contain.
fn assert_fresh(so: &Path) {
    let src = manifest_dir().join("src/lib.rs");
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    let (t_so, t_src) = (mtime(so), mtime(&src));
    assert!(
        t_so >= t_src,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not build a cdylib-only lib target, so the .so must be \
         rebuilt explicitly:\n    cargo build --release\n\
         (or point $HATCH_RUST_SO at the .so you mean to test)",
        so.display(),
        src.display()
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep every test reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// `int` values the C code is most likely to treat specially.
pub const EXTREMES: [c_int; 13] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    7,
    c_int::MAX,
    c_int::MIN,
    c_int::MAX / 2,
    c_int::MIN / 2,
    65536,
];

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
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
    /// Uniform full-range `i32`.
    pub fn i32_full(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform in `[lo, hi]` (inclusive), `lo <= hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn range_i32(&mut self, lo: c_int, hi: c_int) -> c_int {
        self.range(lo as i64, hi as i64) as c_int
    }
    /// Small magnitude value, the "ordinary" case.
    pub fn small(&mut self) -> c_int {
        self.range_i32(-1000, 1000)
    }
    /// Mixture: 1/3 extreme, 1/3 small, 1/3 full range — hits both the ordinary
    /// and the wrap-around code paths.
    pub fn interesting(&mut self) -> c_int {
        match self.next_u64() % 3 {
            0 => EXTREMES[(self.next_u64() % EXTREMES.len() as u64) as usize],
            1 => self.small(),
            _ => self.i32_full(),
        }
    }
    pub fn bytes(&mut self, out: &mut [u8]) {
        for b in out.iter_mut() {
            *b = (self.next_u64() >> 33) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Buffer helpers: identical buffers are handed to each library so that both
// the return value AND every mutated byte can be compared.
// ---------------------------------------------------------------------------

/// Random `int` array of `len` elements plus `red` trailing red-zone elements
/// (which the callee must never touch).
pub fn rand_int_buf(rng: &mut Rng, len: usize, red: usize) -> Vec<c_int> {
    (0..len + red).map(|_| rng.interesting()).collect()
}

pub fn rand_record(rng: &mut Rng) -> DataRecord {
    let mut name = [0i8; 32];
    let mut raw = [0u8; 32];
    rng.bytes(&mut raw);
    for (d, s) in name.iter_mut().zip(raw.iter()) {
        *d = *s as c_char;
    }
    DataRecord {
        id: rng.interesting(),
        value: rng.interesting(),
        timestamp: rng.next_u64() as CTimeT,
        name,
    }
}

pub fn rand_record_buf(rng: &mut Rng, len: usize) -> Vec<DataRecord> {
    (0..len).map(|_| rand_record(rng)).collect()
}

/// Raw byte image of a slice, for byte-for-byte buffer comparison.
pub fn bytes_of<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// Compare two byte images and report the first differing offset.
pub fn assert_bytes_eq(a: &[u8], b: &[u8], ctx: &str) {
    assert_eq!(a.len(), b.len(), "{ctx}: length mismatch");
    if a != b {
        let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap();
        panic!("{ctx}: buffers differ at byte {i}: C={:#04x} Rust={:#04x}", a[i], b[i]);
    }
}
