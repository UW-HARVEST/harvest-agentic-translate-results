//! Shared harness for the differential tests.
//!
//! BOTH implementations are exercised strictly through their shared objects:
//! the C `.so` produced by `c_src/CMakeLists.txt` and the Rust `cdylib`
//! produced by this crate. No Rust function is ever called directly, so the
//! `#[no_mangle]` / `extern "C"` export wrappers are part of what is tested.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// Function-pointer view of every symbol the C library exports.
pub struct Api {
    pub name: &'static str,
    pub shift_array: unsafe extern "C" fn(*mut c_int, c_int, c_int),
    pub process_string: unsafe extern "C" fn(*const c_char) -> c_int,
    pub apply_bitmask: unsafe extern "C" fn(c_int, c_int) -> c_int,
    pub init_matrix: unsafe extern "C" fn(*mut c_int),
    pub compare_allocations: unsafe extern "C" fn(c_int, c_int) -> c_int,
    pub arity4: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
    pub arity3: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int,
    pub arity2: unsafe extern "C" fn(c_int, c_int) -> c_int,
    pub arity: unsafe extern "C" fn(c_int, *const c_int) -> c_int,
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &[u8]) -> T {
    let s: Symbol<T> = unsafe {
        lib.get(name)
            .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)))
    };
    *s
}

fn load(path: &Path, name: &'static str) -> Api {
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("cannot load {}: {e}", path.display()))
    }));
    unsafe {
        Api {
            name,
            shift_array: sym(lib, b"shift_array"),
            process_string: sym(lib, b"process_string"),
            apply_bitmask: sym(lib, b"apply_bitmask"),
            init_matrix: sym(lib, b"init_matrix"),
            compare_allocations: sym(lib, b"compare_allocations"),
            arity4: sym(lib, b"arity4"),
            arity3: sym(lib, b"arity3"),
            arity2: sym(lib, b"arity2"),
            arity: sym(lib, b"arity"),
        }
    }
}

fn workspace_root() -> PathBuf {
    // .../translation/tests/common/mod.rs  ->  .../translation  ->  ...
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Locate the C shared library. Its file name is derived from the name of the
/// project directory by `CMakeLists.txt`, so it is discovered by globbing
/// rather than hard-coded.
fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "{} not readable ({e}); build the C library first (see task README)",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// Locate the Rust `cdylib`. `cargo test` places the test executable in
/// `target/<profile>/deps/`, next to the `cdylib` in `target/<profile>/`.
fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>/deps/<exe>")
        .to_path_buf();
    let p = profile_dir.join("libarity_lib.so");
    assert!(p.exists(), "{} does not exist", p.display());
    p
}

pub fn c_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| load(&c_lib_path(), "C"))
}

pub fn rust_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| load(&rust_lib_path(), "Rust"))
}

/// Both implementations, in a fixed order (C first) so that any global state
/// (e.g. the process-wide malloc arena) is touched in a deterministic order.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Making `compare_allocations` deterministic.
//
// `compare_allocations()` compares the addresses returned by two consecutive
// `malloc(sizeof(int))` calls, so its result is a function of the state of the
// process-wide glibc allocator — the same C library called twice in a row can
// return different values (see `tests/probe_alloc.rs`, which shows two dlopens
// of the *same* C `.so` diverging from each other).
//
// Rather than assume anything about that state, the tests *canonicalise* it
// before every measurement: `normalize_allocator` takes `TCACHE_COUNT` chunks of
// exactly `sizeof(int)` out of the allocator and returns them in a chosen
// address order. Since `free` pushes onto the thread-local tcache bin and
// `malloc` pops from its head (LIFO), the order in which they are released fixes
// the addresses the *next* two allocations will receive:
//
//   * freeing them from the highest address down  -> head is the lowest address
//     -> the library sees `ptr1 < ptr2`  (`compare_allocations` -> 1)
//   * freeing them from the lowest address up     -> head is the highest address
//     -> the library sees `ptr1 > ptr2`  (`compare_allocations` -> 2)
//
// The tcache is thread-local, so no other test thread can perturb it; calling
// this immediately before each library call makes the comparison fully
// deterministic and lets both address orderings be exercised on purpose.
// (`ptr1 == ptr2` cannot be produced by a real allocator; it is covered by the
// LD_PRELOAD test `e24_pointer_order_branches`.)
// ---------------------------------------------------------------------------

/// glibc's default `tcache_count` for small bins.
const TCACHE_COUNT: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocOrder {
    /// The next two `malloc(sizeof(int))` calls return increasing addresses.
    Increasing,
    /// The next two `malloc(sizeof(int))` calls return decreasing addresses.
    Decreasing,
}

impl AllocOrder {
    /// The value `compare_allocations` yields for this ordering, before the
    /// `val1 > 0` bonus.
    pub fn expected_branch(self) -> c_int {
        match self {
            AllocOrder::Increasing => 1,
            AllocOrder::Decreasing => 2,
        }
    }

    pub fn both() -> [AllocOrder; 2] {
        [AllocOrder::Increasing, AllocOrder::Decreasing]
    }
}

/// Put the `sizeof(int)` tcache bin into a canonical state. Must be called
/// immediately before the library call being measured: nothing may allocate in
/// between.
#[inline(never)]
pub fn normalize_allocator(order: AllocOrder) {
    let mut p = [core::ptr::null_mut::<c_void>(); TCACHE_COUNT];
    unsafe {
        for slot in p.iter_mut() {
            *slot = malloc(core::mem::size_of::<c_int>());
            assert!(!slot.is_null(), "test harness malloc failed");
        }
        // Insertion sort by address (ascending); allocation-free.
        for i in 1..TCACHE_COUNT {
            let mut j = i;
            while j > 0 && (p[j - 1] as usize) > (p[j] as usize) {
                p.swap(j - 1, j);
                j -= 1;
            }
        }
        match order {
            // Release highest first => LIFO head is the lowest address.
            AllocOrder::Increasing => {
                for i in (0..TCACHE_COUNT).rev() {
                    free(p[i]);
                }
            }
            // Release lowest first => LIFO head is the highest address.
            AllocOrder::Decreasing => {
                for i in 0..TCACHE_COUNT {
                    free(p[i]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) so every "randomized" test is reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Values biased towards the interesting ones: small magnitudes, powers of
    /// two, boundary values, plus fully random 32-bit patterns.
    pub fn interesting_i32(&mut self) -> i32 {
        let r = self.next_u64();
        match r % 8 {
            0 => (r >> 8) as i8 as i32,                    // -128..=127
            1 => ((r >> 8) % 8) as i32,                    // 0..=7
            2 => -(((r >> 8) % 8) as i32),                 // -7..=0
            3 => 1i32.wrapping_shl(((r >> 8) % 32) as u32),// powers of two
            4 => [i32::MIN, i32::MAX, 0, -1, 1, 100, -100, 4][((r >> 8) % 8) as usize],
            _ => self.next_i32(),
        }
    }
    pub fn range(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// The number of randomized inputs used per `CONFIGS.md` row.
pub const ITERS: usize = 400;
