//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and exposes each exported symbol as a matched (C, Rust) pair.
//!
//! Nothing here calls the Rust crate directly. Every Rust call goes through
//! `dlopen`/`dlsym` on `target/<profile>/libarity_lib.so`, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;

pub type FnShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnInitMatrix = unsafe extern "C" fn(*mut [c_int; 4]);
pub type FnCompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnArity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnArity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnArity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
/// Declared as the public header declares it: `int arity(int len, int *params)`.
/// The C *definition* takes `unsigned char`; calling through the header's type
/// is exactly what a real consumer does, and is the behaviour under test.
pub type FnArity = unsafe extern "C" fn(c_int, *const c_int) -> c_int;

/// One side of the differential: a loaded library plus its resolved symbols.
pub struct Side {
    _lib: Library,
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

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    unsafe {
        let s: Symbol<T> = lib
            .get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
        *s
    }
}

impl Side {
    unsafe fn open(path: &PathBuf, name: &'static str) -> Side {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
        };
        unsafe {
            Side {
                name,
                shift_array: sym(&lib, b"shift_array\0"),
                process_string: sym(&lib, b"process_string\0"),
                apply_bitmask: sym(&lib, b"apply_bitmask\0"),
                init_matrix: sym(&lib, b"init_matrix\0"),
                compare_allocations: sym(&lib, b"compare_allocations\0"),
                arity4: sym(&lib, b"arity4\0"),
                arity3: sym(&lib, b"arity3\0"),
                arity2: sym(&lib, b"arity2\0"),
                arity: sym(&lib, b"arity\0"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Side,
    pub rust: Side,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    let build = crate_root().join("../c_src/build");
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\nBuild the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib built by the same `cargo test` invocation is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let direct = profile_dir.join("libarity_lib.so");
    if direct.exists() {
        return direct;
    }
    for p in ["target/release/libarity_lib.so", "target/debug/libarity_lib.so"] {
        let cand = crate_root().join(p);
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "libarity_lib.so not found near {}; run `cargo build` first",
        profile_dir.display()
    )
}

/// Load both libraries. Called once per test; `dlopen` refcounts so the
/// underlying mapping is shared across tests in the same process.
pub fn load() -> Pair {
    unsafe {
        Pair {
            c: Side::open(&find_c_so(), "C"),
            rust: Side::open(&find_rust_so(), "Rust"),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, so every "randomized" row is
// exactly reproducible across runs and across the C/Rust sides.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Full-range i32, including `INT_MIN`/`INT_MAX` neighbourhoods.
    pub fn i32_full(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small signed value in `[-lim, lim]`, to hit the dense low-magnitude paths.
    pub fn i32_small(&mut self, lim: i32) -> i32 {
        let span = (lim as i64) * 2 + 1;
        ((self.next_u64() % span as u64) as i64 - lim as i64) as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Mix of full-range and small values: value-dependent bugs hide at both
    /// extremes, and pure full-range sampling almost never produces a small
    /// number.
    pub fn i32_mixed(&mut self) -> i32 {
        match self.next_u64() % 4 {
            0 => self.i32_small(4),
            1 => self.i32_small(200),
            2 => [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0, -1, 1, 100, -100]
                [(self.next_u64() % 9) as usize],
            _ => self.i32_full(),
        }
    }
}

// ---------------------------------------------------------------------------
// Phase-neutral comparison for the allocator-dependent functions.
//
// `compare_allocations` returns 1 or 2 depending on which of two `malloc(4)`
// results has the lower address. glibc's tcache is LIFO, so the
// malloc,malloc,free,free sequence swaps the two chunks and the answer
// alternates 1,2,1,2,... That state is process-global and SHARED between the C
// .so and the Rust .so (both call the same libc malloc), so calling C once then
// Rust once is guaranteed to disagree even though both are correct.
//
// Two consecutive calls restore the entry state, so comparing two-call
// SEQUENCES is phase-independent *and* observes both heap phases per input.
// ---------------------------------------------------------------------------

/// Call `f` twice and return both results, leaving heap parity as found.
///
/// The two calls are deliberately adjacent with no intervening allocation, so
/// the C measurement and the Rust measurement of the same input always start
/// from the same parity.
pub fn pair2<F: FnMut() -> c_int>(mut f: F) -> [c_int; 2] {
    let a = f();
    let b = f();
    [a, b]
}

fn sorted2(mut x: [c_int; 2]) -> [c_int; 2] {
    x.sort();
    x
}

/// Serialises every allocator-sensitive measurement.
///
/// Why this is needed: glibc's `arena_get` will fall back to a *different*
/// arena when the thread's preferred arena is contended. Under `cargo test`'s
/// parallel harness that can make `ptr1` and `ptr2` inside
/// `compare_allocations` come from two different arenas, at which point their
/// address ordering is genuinely nondeterministic rather than the clean
/// tcache-LIFO alternation. Holding this lock keeps one measurement at a time.
fn heap_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Measure an allocator-dependent function across both heap phases, retrying
/// until the measurement is self-consistent.
///
/// Returns the sorted two-phase multiset. Panics if the allocator environment
/// never stabilises, which is reported as an environment problem rather than a
/// divergence so it can never be mistaken for a passing test.
pub fn measure_pair<F: FnMut() -> c_int>(ctx: &str, mut f: F) -> [c_int; 2] {
    let _g = heap_lock();
    let mut seen = Vec::new();
    for _ in 0..32 {
        let a = sorted2(pair2(&mut f));
        let b = sorted2(pair2(&mut f));
        if a == b {
            return a;
        }
        seen.push((a, b));
    }
    panic!("[{ctx}] allocator state never stabilised across 32 attempts: {seen:?}");
}

/// The core allocator-aware differential assertion.
///
/// Takes both sides as closures so the C and Rust measurements happen
/// **adjacently, under the heap lock, with no intervening allocation**. Because
/// two consecutive calls restore the tcache to its entry state, the Rust pair
/// starts from the same phase the C pair did, so the two pairs are compared
/// **in order** — which is strictly stronger than a multiset compare (an
/// implementation that swapped the `ptr1 < ptr2` / `ptr1 > ptr2` arms would
/// produce the same multiset but the reversed order, and must be caught).
///
/// The C side is measured again afterwards and must reproduce its own ordered
/// pair before any divergence is reported. That makes the assertion immune to
/// glibc arena-contention noise (where `ptr1` and `ptr2` can come from
/// different arenas and the ordering stops alternating cleanly) without ever
/// hiding a genuine divergence: a C measurement that reproduces itself is still
/// compared strictly and ordered.
pub fn assert_alloc_diff<C, R>(ctx: &str, mut cf: C, mut rf: R)
where
    C: FnMut() -> c_int,
    R: FnMut() -> c_int,
{
    let _g = heap_lock();
    let mut last = None;
    for _ in 0..32 {
        let c1 = pair2(&mut cf);
        let r = pair2(&mut rf);
        let c2 = pair2(&mut cf);
        if c1 == c2 {
            assert_eq!(
                c1, r,
                "divergence [{ctx}]: C={c1:?} Rust={r:?} \
                 (ordered pair over both allocator phases; C reproduced itself)"
            );
            return;
        }
        last = Some((c1, r, c2));
    }
    panic!(
        "[{ctx}] allocator state never stabilised across 32 attempts \
         (last C/Rust/C: {last:?}) — environment problem, not a divergence"
    );
}

/// Assert that C and Rust agree on a phase-neutral two-call sequence.
///
/// Compared as a **multiset** (sorted), not as an ordered sequence: which of
/// the two allocator phases is observed first is an artifact of heap state.
/// Prefer [`assert_alloc_diff`], which additionally serialises the measurement
/// and proves the C side reproducible.
pub fn assert_pair_eq(ctx: &str, c: [c_int; 2], r: [c_int; 2]) {
    let cs = sorted2(c);
    let rs = sorted2(r);
    assert_eq!(
        cs, rs,
        "divergence [{ctx}]: C returned {c:?} (sorted {cs:?}), \
         Rust returned {r:?} (sorted {rs:?}) \
         — phase-neutral two-call multiset over both heap phases"
    );
}
