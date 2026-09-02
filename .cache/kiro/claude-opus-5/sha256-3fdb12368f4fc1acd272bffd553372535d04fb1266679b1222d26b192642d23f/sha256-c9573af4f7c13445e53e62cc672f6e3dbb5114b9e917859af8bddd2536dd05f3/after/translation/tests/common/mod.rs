//! Shared differential-test harness.
//!
//! Loads BOTH the C shared library and the Rust `cdylib` through `libloading`
//! and exposes typed wrappers for every exported symbol, so the Rust side is
//! always exercised through its real `#[no_mangle] extern "C"` ABI exports —
//! never by calling Rust functions directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type FnShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnInitMatrix = unsafe extern "C" fn(*mut c_int);
pub type FnCompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnArity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnArity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnArity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
/// The public header declares `int arity(int len, int *params)`.
pub type FnArity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
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

impl Impl {
    pub fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
            let me = Impl {
                name,
                shift_array: sym(&lib, b"shift_array"),
                process_string: sym(&lib, b"process_string"),
                apply_bitmask: sym(&lib, b"apply_bitmask"),
                init_matrix: sym(&lib, b"init_matrix"),
                compare_allocations: sym(&lib, b"compare_allocations"),
                arity4: sym(&lib, b"arity4"),
                arity3: sym(&lib, b"arity3"),
                arity2: sym(&lib, b"arity2"),
                arity: sym(&lib, b"arity"),
                path,
                _lib: lib,
            };
            me
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`. Its file name is derived from the parent directory name
/// by `c_src/CMakeLists.txt`, so glob the build directory instead of hardcoding.
/// If it is absent, run the documented CMake build (no `c_src` source file is
/// ever modified — only the `build/` artifact directory is populated).
pub fn c_so_path() -> PathBuf {
    // Escape hatch used by scripts/check_c_optimization_levels.sh to point the
    // suite at a C .so built with different optimization flags.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_C_SO={} does not exist", p.display());
        return p;
    }
    let c_src = manifest_dir().parent().unwrap().join("c_src");
    let build = c_src.join("build");
    if find_so(&build).is_none() {
        std::fs::create_dir_all(&build).expect("mkdir c_src/build");
        let cmake = Command::new("cmake")
            .current_dir(&build)
            .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .output()
            .expect("run cmake (is cmake installed?)");
        assert!(
            cmake.status.success(),
            "cmake configure failed:\n{}",
            String::from_utf8_lossy(&cmake.stderr)
        );
        let build_out = Command::new("cmake")
            .current_dir(&build)
            .args(["--build", "."])
            .output()
            .expect("run cmake --build");
        assert!(
            build_out.status.success(),
            "cmake build failed:\n{}",
            String::from_utf8_lossy(&build_out.stderr)
        );
    }
    find_so(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with: cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next()
}

/// Path to the Rust `cdylib`, BUILT ON DEMAND.
///
/// This must not simply look inside `target/`: `cargo test` does **not** build a
/// `crate-type = ["cdylib"]` artifact (integration tests do not link against it),
/// so a path lookup silently picks up a stale `.so` from an earlier
/// `cargo build` — or none at all — and the whole differential suite then
/// verifies old code. That failure mode is silent and total, so the harness
/// builds the `cdylib` itself, into a dedicated target directory so it cannot
/// contend with the `cargo test` invocation that is running us.
///
/// The profile matches the test profile (`--release` when `debug_assertions` is
/// off) so the release build's `panic = "abort"` and optimizations are exercised.
/// Feature selection is forwarded through `HARVEST_SO_FEATURES` (set by
/// `scripts/check_features.sh`), because integration tests cannot otherwise
/// observe which features the lib was built with.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(build_rust_cdylib).clone()
}

fn build_rust_cdylib() -> PathBuf {
    let manifest = manifest_dir();
    let target_dir = manifest.join("target").join("ffi-cdylib");
    let release = !cfg!(debug_assertions);
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = Command::new(cargo);
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir)
        // A nested cargo inherits the outer invocation's env; clear the pieces
        // that would redirect or confuse the child build.
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC")
        .env_remove("RUSTDOC")
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("CARGO_PRIMARY_PACKAGE")
        .env_remove("CARGO_UNSTABLE_BUILD_STD");
    if release {
        cmd.arg("--release");
    }
    if let Ok(extra) = std::env::var("HARVEST_SO_FEATURES") {
        for tok in extra.split_whitespace() {
            cmd.arg(tok);
        }
    }

    let out = cmd.output().expect("failed to spawn cargo to build the cdylib");
    assert!(
        out.status.success(),
        "building the Rust cdylib failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let so = target_dir
        .join(if release { "release" } else { "debug" })
        .join("libarity_lib.so");
    assert!(
        so.exists(),
        "cargo reported success but {} does not exist",
        so.display()
    );
    // Guard against ever testing a stale artifact again.
    let src = manifest.join("src/lib.rs");
    if let (Ok(sm), Ok(om)) = (
        std::fs::metadata(&src).and_then(|m| m.modified()),
        std::fs::metadata(&so).and_then(|m| m.modified()),
    ) {
        assert!(
            om >= sm,
            "{} is older than src/lib.rs — the cdylib is stale",
            so.display()
        );
    }
    so
}

pub fn load_c() -> Impl {
    Impl::load("C", c_so_path())
}

pub fn load_rust() -> Impl {
    Impl::load("Rust", rust_so_path())
}

/// Both implementations, loaded once.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    Pair {
        c: load_c(),
        rust: load_rust(),
    }
}

/// Deterministic xorshift PRNG so every property-style test is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Small-magnitude value, the range where the arithmetic does not overflow.
    pub fn small_i32(&mut self) -> i32 {
        (self.next_u64() % 4001) as i32 - 2000
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Pick one of a set of interesting boundary values, or a random one.
    pub fn interesting_i32(&mut self) -> i32 {
        const POOL: [i32; 24] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            -3,
            4,
            -4,
            5,
            -5,
            15,
            16,
            100,
            -100,
            99,
            -99,
            255,
            256,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            0x7fff_0000,
        ];
        match self.next_u64() % 3 {
            0 => POOL[(self.next_u64() as usize) % POOL.len()],
            1 => self.small_i32(),
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Heap-ordering control
// ---------------------------------------------------------------------------
//
// `compare_allocations` — and therefore every `arityN` — observes *real*
// `malloc`/`free` behavior: it allocates two 4-byte blocks, compares their
// addresses, then frees both. glibc's tcache is LIFO, so which of the two
// addresses is the lower one depends on the process-wide allocator state, not
// only on the arguments. Both `.so`s share one libc heap inside the test
// process, so naively calling C once and then Rust once compares two *different*
// heap states and is meaningless.
//
// Instead the test drives that state explicitly. Freeing a known
// higher-addressed chunk and then a known lower-addressed one leaves the
// lower one at the head of the 32-byte tcache bin, so the next two `malloc(4)`
// calls hand back `(lo, hi)` — i.e. `ptr1 < ptr2`, the `result = 1` branch.
// Reversing the two `free`s selects `ptr1 > ptr2`, the `result = 2` branch.
// Seeding before each measured call makes both implementations fully
// deterministic AND lets every heap-dependent row be tested under BOTH
// orderings, which is how the `result = 1` and `result = 2` branches are both
// reached on purpose.

use std::ffi::c_void;

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Heap {
    /// Next `malloc` pair returns ascending addresses → `ptr1 < ptr2` → `1`.
    Ascending,
    /// Next `malloc` pair returns descending addresses → `ptr1 > ptr2` → `2`.
    Descending,
}

pub const HEAP_STATES: [Heap; 2] = [Heap::Ascending, Heap::Descending];

/// glibc's per-bin tcache capacity (`mp_.tcache_count`, default 7). A `free`
/// beyond this goes to the fastbin instead of the tcache, which would silently
/// defeat naive seeding — so the seed drains the bin first.
const TCACHE_COUNT: usize = 7;

/// Force the 32-byte tcache bin into the requested ordering, unconditionally and
/// independently of whatever the bin held before.
///
/// `malloc(4)` rounds up to the 32-byte minimum chunk, so every allocation in
/// this library (and every 16-byte `Vec` the test harness makes) lands in the
/// same bin. The seed therefore:
///
///  1. drains the bin with `TCACHE_COUNT` allocations (after which the bin is
///     provably empty, since that is its capacity);
///  2. frees the surplus chunks first, so the bin cannot overflow;
///  3. frees the two chunks that will be handed to the next `malloc` pair LAST,
///     in the order that puts the desired one at the head of the LIFO list.
///
/// Must be called immediately before the measured call, with no intervening
/// allocation.
pub fn seed_heap(order: Heap) {
    unsafe {
        let mut chunks = [std::ptr::null_mut::<c_void>(); TCACHE_COUNT];
        for slot in chunks.iter_mut() {
            *slot = malloc(4);
            assert!(!slot.is_null(), "seed malloc failed");
        }
        chunks.sort_unstable_by_key(|p| *p as usize);
        // Surplus first: the bin ends at exactly TCACHE_COUNT, never above it.
        for &p in &chunks[2..] {
            free(p);
        }
        let (lo, hi) = (chunks[0], chunks[1]);
        match order {
            // head = lo, next = hi  =>  ptr1 = lo < ptr2 = hi  =>  result 1
            Heap::Ascending => {
                free(hi);
                free(lo);
            }
            // head = hi, next = lo  =>  ptr1 = hi > ptr2 = lo  =>  result 2
            Heap::Descending => {
                free(lo);
                free(hi);
            }
        }
    }
}

/// Run `f` against C and against Rust under one heap ordering, seeding
/// immediately before each call so both observe the same allocator state.
/// `f` MUST perform exactly one allocating library call.
pub fn run_seeded<F: Fn(&Impl) -> i32>(p: &Pair, order: Heap, f: &F) -> (i32, i32) {
    seed_heap(order);
    let c = f(&p.c);
    seed_heap(order);
    let r = f(&p.rust);
    (c, r)
}

/// `[(c, rust)]` for both heap orderings. Allocation-free on the success path.
pub fn run_both_heaps<F: Fn(&Impl) -> i32>(p: &Pair, f: F) -> [(i32, i32); 2] {
    let asc = run_seeded(p, Heap::Ascending, &f);
    let desc = run_seeded(p, Heap::Descending, &f);
    [asc, desc]
}

/// Assert C and Rust agree under both heap orderings.
#[track_caller]
pub fn assert_both_heaps<F: Fn(&Impl) -> i32>(p: &Pair, ctx: &str, f: F) {
    for order in HEAP_STATES {
        let (c, r) = run_seeded(p, order, &f);
        assert_eq!(c, r, "{ctx} [heap={order:?}]: C={c} Rust={r}");
    }
}

/// Assert C and Rust agree for a call that does not touch the heap.
#[track_caller]
pub fn assert_pure_eq<F: Fn(&Impl) -> i32>(p: &Pair, ctx: &str, f: F) {
    let c = f(&p.c);
    let r = f(&p.rust);
    assert_eq!(c, r, "{ctx}: C={c} Rust={r}");
}
