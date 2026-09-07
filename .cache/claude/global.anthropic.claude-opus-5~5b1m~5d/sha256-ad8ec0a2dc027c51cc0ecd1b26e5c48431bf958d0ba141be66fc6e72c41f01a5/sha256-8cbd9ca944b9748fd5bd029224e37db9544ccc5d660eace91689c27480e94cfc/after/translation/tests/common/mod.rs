//! Shared differential-test harness.
//!
//! BOTH the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the dynamic-symbol table, exactly as an external C caller
//! would do it. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Library location
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C library is named after the *parent directory* of `c_src`
/// (see `cmake_path(GET parent FILENAME project_name)`), so glob for it
/// instead of hard-coding the name.
pub fn c_so_path() -> PathBuf {
    let dir = crate_root().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C lib first.", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one C .so in {}, got {found:?}", dir.display());
    found.pop().unwrap()
}

pub fn rust_so_path() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib is two levels up.
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps
    p.pop(); // <profile>
    let so = p.join("libarity_lib.so");
    assert!(so.exists(), "{} missing; run `cargo build` first", so.display());

    // ------------------------------------------------------------------
    // STALENESS GUARD  (critical!)
    //
    // These tests `dlopen` the cdylib by PATH, so they declare no Cargo
    // dependency on it. That means `cargo test --test <name>` will happily
    // run against a MONTH-OLD `.so`: an injected bug in `src/lib.rs` would
    // silently still "pass". Verified empirically. So refuse to run unless
    // the `.so` is newer than every source input.
    // ------------------------------------------------------------------
    let so_mtime = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
    for src in ["src/lib.rs", "Cargo.toml"] {
        let sp = crate_root().join(src);
        let Ok(m) = std::fs::metadata(&sp).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            m <= so_mtime,
            "\n\n*** STALE RUST .so ***\n\
             {} is newer than {}.\n\
             `cargo test --test <name>` does NOT rebuild a cdylib that the test\n\
             only dlopens by path, so this run would have tested an OLD library.\n\
             Run `cargo build --release` (or ./run_tests.sh) first.\n",
            sp.display(),
            so.display()
        );
    }
    so
}

// ---------------------------------------------------------------------------
// The 9 exported symbols, as raw C function pointers
// ---------------------------------------------------------------------------

pub type FnShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnInitMatrix = unsafe extern "C" fn(*mut c_int);
pub type FnCompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnArity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnArity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnArity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnArity = unsafe extern "C" fn(c_int, *const c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub shift_array: FnShiftArray,
    pub process_string: FnProcessString,
    pub apply_bitmask: FnApplyBitmask,
    pub init_matrix: FnInitMatrix,
    pub compare_allocations: FnCompareAllocations,
    pub arity4: FnArity4,
    pub arity3: FnArity3,
    pub arity2: FnArity2,
    pub arity: FnArity,
}

impl Lib {
    fn open(name: &'static str, path: &std::path::Path) -> Lib {
        // Leaked so the symbols stay valid for the whole test process, and so
        // the library is never `dlclose`d mid-test.
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
        }));
        unsafe fn sym<T: Copy>(lib: &'static Library, n: &[u8], who: &str) -> T {
            let s: Symbol<T> = unsafe { lib.get(n) }
                .unwrap_or_else(|e| panic!("{who}: missing symbol {}: {e}", String::from_utf8_lossy(n)));
            *s
        }
        unsafe {
            Lib {
                name,
                shift_array: sym(lib, b"shift_array", name),
                process_string: sym(lib, b"process_string", name),
                apply_bitmask: sym(lib, b"apply_bitmask", name),
                init_matrix: sym(lib, b"init_matrix", name),
                compare_allocations: sym(lib, b"compare_allocations", name),
                arity4: sym(lib, b"arity4", name),
                arity3: sym(lib, b"arity3", name),
                arity2: sym(lib, b"arity2", name),
                arity: sym(lib, b"arity", name),
            }
        }
    }
}

/// Loads both libraries. `.0` = C (ground truth), `.1` = Rust.
pub fn both() -> (Lib, Lib) {
    (
        Lib::open("C", &c_so_path()),
        Lib::open("Rust", &rust_so_path()),
    )
}

// ---------------------------------------------------------------------------
// Allocator-state normalization  (see CONFIGS.md, axis G)
// ---------------------------------------------------------------------------

/// `compare_allocations()` observably compares the two addresses returned by
/// two consecutive `malloc(sizeof(int))` calls. glibc's tcache is LIFO, so the
/// result depends on process-global allocation *history*, not on the arguments.
///
/// This primes the 32-byte tcache bin so that the next two `malloc(4)` calls
/// return the lower address first (`asc = true` -> `ptr1 < ptr2` -> `result 1`)
/// or the higher address first (`asc = false` -> `ptr1 > ptr2` -> `result 2`).
///
/// Making this explicit turns a nondeterministic artifact into a *controlled
/// input axis*, and lets both pointer-ordering arms be tested deliberately.
///
/// Must be called immediately before the measured call, with no intervening
/// heap traffic.
#[inline(never)]
pub fn normalize_heap(asc: bool) {
    unsafe {
        let a = malloc(4);
        let b = malloc(4);
        assert!(!a.is_null() && !b.is_null());
        let (lo, hi) = if (a as usize) < (b as usize) { (a, b) } else { (b, a) };
        if asc {
            free(hi);
            free(lo);
        } else {
            free(lo);
            free(hi);
        }
    }
}

pub const BOTH_ORDERS: [bool; 2] = [true, false];

// ---------------------------------------------------------------------------
// Serialization + retry for heap-sensitive differential calls
// ---------------------------------------------------------------------------

fn heap_mutex() -> &'static std::sync::Mutex<()> {
    static M: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    M.get_or_init(|| std::sync::Mutex::new(()))
}

/// Differential call for any entry point that reaches `compare_allocations`.
///
/// `normalize_heap` assumes the next two `malloc(4)` calls hand back exactly
/// the two chunks it just freed. That holds within a thread, but glibc refills
/// an empty per-thread tcache bin from the SHARED arena, so a concurrent test
/// thread can steal the chunk in between. Measured: single-threaded runs are
/// 100% stable, `--test-threads=8` intermittently reports a bogus off-by-one.
///
/// So: hold a process-wide lock across the whole normalize/call/normalize/call
/// critical section, and retry on mismatch. A transient heap perturbation
/// cannot masquerade as a divergence, while a genuine translation bug is
/// deterministic and mismatches on every attempt.
pub fn diff_norm(
    row: &str,
    ctx: impl std::fmt::Debug,
    asc: bool,
    cf: &mut dyn FnMut() -> c_int,
    rf: &mut dyn FnMut() -> c_int,
) -> c_int {
    let mut last = (0, 0);
    for _ in 0..8 {
        let g = heap_mutex().lock().unwrap_or_else(|e| e.into_inner());
        normalize_heap(asc);
        let cv = cf();
        normalize_heap(asc);
        let rv = rf();
        drop(g);
        if cv == rv {
            return cv;
        }
        last = (cv, rv);
    }
    eq_i32(row, ctx, last.0, last.1);
    unreachable!("eq_i32 must have panicked")
}

/// Same, for a whole replayed SEQUENCE of calls (see `CONFIGS.md` row 22/42).
pub fn diff_norm_seq<T: PartialEq + std::fmt::Debug>(
    row: &str,
    asc: bool,
    cf: &mut dyn FnMut() -> Vec<T>,
    rf: &mut dyn FnMut() -> Vec<T>,
) -> Vec<T> {
    let mut last: Option<(Vec<T>, Vec<T>)> = None;
    for _ in 0..8 {
        let g = heap_mutex().lock().unwrap_or_else(|e| e.into_inner());
        normalize_heap(asc);
        let cs = cf();
        normalize_heap(asc);
        let rs = rf();
        drop(g);
        if cs == rs {
            return cs;
        }
        last = Some((cs, rs));
    }
    let (cs, rs) = last.unwrap();
    let i = cs.iter().zip(&rs).position(|(a, b)| a != b);
    panic!(
        "\n[{row}] C/Rust SEQUENCE DIVERGENCE (persisted across 8 retries)\n  \
         first differing index: {i:?}\n  C len={} Rust len={}\n  C    : {:?}\n  Rust : {:?}\n",
        cs.len(),
        rs.len(),
        i.map(|i| &cs[i.saturating_sub(2)..(i + 3).min(cs.len())]),
        i.map(|i| &rs[i.saturating_sub(2)..(i + 3).min(rs.len())]),
    );
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD;

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
    /// Uniform over the whole `i32` range (including `INT_MIN`/`INT_MAX`).
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() % (hi - lo + 1) as u64) as usize
    }
    pub fn u8_nonzero(&mut self) -> u8 {
        (self.next_u64() % 255) as u8 + 1
    }
    /// A value biased towards interesting boundaries.
    pub fn spicy_i32(&mut self) -> i32 {
        const SPICE: [i32; 17] = [
            0, 1, -1, 2, -2, 3, -3, 4, -4, 100, -100, 101, -101, 99, -99,
            i32::MIN, i32::MAX,
        ];
        if self.next_u64() % 3 == 0 {
            SPICE[(self.next_u64() % SPICE.len() as u64) as usize]
        } else {
            self.i32()
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_i32(row: &str, ctx: impl std::fmt::Debug, c: c_int, r: c_int) {
    assert_eq!(
        c, r,
        "\n[{row}] C/Rust DIVERGENCE\n  input : {ctx:?}\n  C     : {c}\n  Rust  : {r}\n"
    );
}

#[track_caller]
pub fn eq_slice(row: &str, ctx: impl std::fmt::Debug, c: &[c_int], r: &[c_int]) {
    assert_eq!(
        c, r,
        "\n[{row}] C/Rust DIVERGENCE\n  input : {ctx:?}\n  C     : {c:?}\n  Rust  : {r:?}\n"
    );
}

/// A NUL-terminated C string buffer that also keeps trailing bytes, so tests
/// can place an interior NUL and still observe what lies beyond it.
pub fn cbuf(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}
