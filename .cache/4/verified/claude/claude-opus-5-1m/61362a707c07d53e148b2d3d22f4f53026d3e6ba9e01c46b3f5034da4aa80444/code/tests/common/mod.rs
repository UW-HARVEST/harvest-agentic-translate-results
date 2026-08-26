// Shared differential-test harness.
//
// BOTH libraries are loaded as shared objects through `libloading` and every
// call goes through an exported C symbol. The Rust functions are NEVER called
// directly (the crate is `crate-type = ["cdylib"]`, so this also exercises the
// `#[no_mangle] extern "C"` export wrappers exactly as an external C consumer
// would).
#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Function-pointer types (mirror c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------
pub type FnShiftArray = unsafe extern "C" fn(*mut c_int, c_int, c_int);
pub type FnProcessString = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyBitmask = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnInitMatrix = unsafe extern "C" fn(*mut c_int);
pub type FnCompareAllocations = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnArity4 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnArity3 = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnArity2 = unsafe extern "C" fn(c_int, c_int) -> c_int;
// NOTE: `include/lib.h` declares `int arity(int len, int *params)`. The
// definition in `src/lib.c` takes `unsigned char len`; we deliberately call
// through the *public header* prototype (`int`) so that the 8-bit truncation
// performed by the callee is part of what is being compared.
pub type FnArity = unsafe extern "C" fn(c_int, *mut c_int) -> c_int;

/// All nine exported entry points of one implementation.
pub struct Api {
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
    _lib: &'static Library,
}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        assert!(
            path.exists(),
            "missing shared library {}\n\
             Build both libraries first:\n  ./run_tests.sh\n\
             or manually:\n\
             \x20 (cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)\n\
             \x20 cargo build",
            path.display()
        );
        // Leaked so the fn pointers copied out below stay valid for the whole
        // test run (the library is never dlclose'd).
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen({}): {e}", path.display()))
        }));
        unsafe {
            macro_rules! sym {
                ($t:ty, $n:literal) => {
                    *lib.get::<$t>($n).unwrap_or_else(|e| {
                        panic!("dlsym({}, {}): {e}", path.display(), stringify!($n))
                    })
                };
            }
            Api {
                name,
                shift_array: sym!(FnShiftArray, b"shift_array\0"),
                process_string: sym!(FnProcessString, b"process_string\0"),
                apply_bitmask: sym!(FnApplyBitmask, b"apply_bitmask\0"),
                init_matrix: sym!(FnInitMatrix, b"init_matrix\0"),
                compare_allocations: sym!(FnCompareAllocations, b"compare_allocations\0"),
                arity4: sym!(FnArity4, b"arity4\0"),
                arity3: sym!(FnArity3, b"arity3\0"),
                arity2: sym!(FnArity2, b"arity2\0"),
                arity: sym!(FnArity, b"arity\0"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

/// Locate the Rust cdylib next to the running test executable
/// (`target/<profile>/deps/<test>` -> `target/<profile>/libarity_lib.so`).
pub fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    if let Some(profile_dir) = exe.parent().and_then(|d| d.parent()) {
        let p = profile_dir.join("libarity_lib.so");
        if p.exists() {
            return p;
        }
    }
    manifest_dir().join("target/debug/libarity_lib.so")
}

/// Load the C implementation and the Rust implementation.
pub fn load_both() -> (Api, Api) {
    (
        Api::load("C", &c_lib_path()),
        Api::load("Rust", &rust_lib_path()),
    )
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed => reproducible runs
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x2545_F491;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform-ish value in `0..n`.
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// An `i32` with `v % 4 == target` for `target in -3..=3`.
    pub fn i32_with_mod4(&mut self, target: i32) -> i32 {
        loop {
            let v = self.next_i32();
            if v.wrapping_rem(4) == target {
                return v;
            }
        }
    }
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// Interesting `i32` boundary values used for cross-product rows.
pub const BOUNDARY: [i32; 10] = [
    i32::MIN,
    i32::MIN + 1,
    -1,
    0,
    1,
    i32::MAX - 1,
    i32::MAX,
    100,
    -100,
    50,
];

// ---------------------------------------------------------------------------
// Allocator-state-aware comparison  (see ERRORS.md Note B)
// ---------------------------------------------------------------------------
//
// `compare_allocations` compares the ADDRESSES returned by two consecutive
// `malloc(sizeof(int))` calls, so its return value depends on the state of the
// glibc tcache, not only on its arguments. Because the tcache free-list is
// LIFO, consecutive calls alternate (…11, 12, 11, 12…) in BOTH libraries.
//
// Therefore a naive `assert_eq!(c(x), rust(x))` would compare two *different*
// allocator states and report a false divergence. Two consecutive calls restore
// the allocator to its previous state, so a 2-call batch is parity-neutral:
// the C pair and the Rust pair start from the same state and must match
// element-wise. Nothing is allocated between the calls below.

#[inline]
pub fn batch2<T: Copy, F: FnMut() -> T, G: FnMut() -> T>(c: &mut F, r: &mut G) -> ([T; 2], [T; 2]) {
    let c0 = c();
    let c1 = c();
    let r0 = r();
    let r1 = r();
    ([c0, c1], [r0, r1])
}

/// Differential assertion for allocator-state-dependent functions.
///
/// Retries a few times so that a stray allocation from unrelated code (which
/// would flip tcache parity mid-batch) cannot cause a flaky failure; a genuine
/// translation bug diverges on every attempt.
#[track_caller]
pub fn assert_alloc_eq<T, F, G>(label: &str, mut c: F, mut r: G)
where
    T: Copy + PartialEq + std::fmt::Debug,
    F: FnMut() -> T,
    G: FnMut() -> T,
{
    let mut last = None;
    for _ in 0..4 {
        let (a, b) = batch2(&mut c, &mut r);
        if a == b {
            return;
        }
        last = Some((a, b));
    }
    let (a, b) = last.unwrap();
    panic!("DIVERGENCE [{label}]\n  C   -> {a:?}\n  Rust-> {b:?}");
}

/// Report the current allocator "state tag" (the address-order term, 11 or 12
/// for `val1 > 0`) WITHOUT net-changing it: two calls are parity-neutral and the
/// first one reveals the state.
pub fn probe_alloc_state(api: &Api) -> c_int {
    let a = unsafe { (api.compare_allocations)(5, 5) };
    let _b = unsafe { (api.compare_allocations)(5, 5) };
    a
}

/// Force the process allocator into the state `want` (as reported by
/// `probe_alloc_state`). Used to give a Rust call sequence the same allocator
/// starting state the C sequence had -- for `compare_allocations` the allocator
/// state is effectively an extra input, so it must be equalised rather than
/// assumed. Counting calls is NOT sufficient (chunks belonging to the same
/// size class can enter or leave the tcache from unrelated allocations), so the
/// state is measured.
pub fn align_alloc_state(api: &Api, want: c_int) {
    if probe_alloc_state(api) != want {
        unsafe { (api.compare_allocations)(5, 5) };
    }
    debug_assert_eq!(probe_alloc_state(api), want);
}

/// Differential assertion for pure (deterministic) functions.
#[track_caller]
pub fn assert_eq_diff<T: PartialEq + std::fmt::Debug>(label: &str, c: T, r: T) {
    if c != r {
        panic!("DIVERGENCE [{label}]\n  C   -> {c:?}\n  Rust-> {r:?}");
    }
}

// ---------------------------------------------------------------------------
// Guarded buffers: detect out-of-bounds writes by the library under test
// ---------------------------------------------------------------------------
pub const GUARD: c_int = 0x5A5A_5A5A;
pub const GUARD_N: usize = 4;

/// A buffer of `len` ints surrounded by `GUARD_N` sentinel ints on both sides.
pub struct Guarded {
    pub buf: Vec<c_int>,
    pub len: usize,
}

impl Guarded {
    pub fn new(contents: &[c_int]) -> Guarded {
        let mut buf = vec![GUARD; contents.len() + 2 * GUARD_N];
        buf[GUARD_N..GUARD_N + contents.len()].copy_from_slice(contents);
        Guarded {
            buf,
            len: contents.len(),
        }
    }
    pub fn ptr(&mut self) -> *mut c_int {
        unsafe { self.buf.as_mut_ptr().add(GUARD_N) }
    }
    pub fn data(&self) -> &[c_int] {
        &self.buf[GUARD_N..GUARD_N + self.len]
    }
    /// Whole buffer including guards -- compared byte-for-byte between C/Rust.
    pub fn all(&self) -> &[c_int] {
        &self.buf
    }
    pub fn guards_intact(&self) -> bool {
        self.buf[..GUARD_N].iter().all(|&v| v == GUARD)
            && self.buf[GUARD_N + self.len..].iter().all(|&v| v == GUARD)
    }
}

// ---------------------------------------------------------------------------
// Crash-parity helpers (for C code paths that are UB: NULL dereferences)
// ---------------------------------------------------------------------------
//
// The C library has no NULL checks, so `process_string(NULL)` etc. segfault.
// To compare that behaviour without killing the test runner, the test binary
// re-executes ITSELF with `DIFFTEST_CRASH_CASE=<case>` set; the child performs
// the raw call and dies, and the parent compares the termination signals of the
// C child and the Rust child.
pub const CRASH_ENV: &str = "DIFFTEST_CRASH_CASE";

pub fn crash_case() -> Option<String> {
    std::env::var(CRASH_ENV).ok()
}

/// How a child process terminated.
#[derive(Debug, PartialEq, Eq)]
pub enum Term {
    Signal(i32),
    Exit(i32),
}

/// Env var used to ask a child process to run a long call SEQUENCE against
/// exactly one library and print the results.
///
/// Long sequences cannot be compared inside a single process: `compare_allocations`
/// reads the glibc tcache, and after the first sequence the tcache holds a
/// different set of chunks than it did at the start (unrelated same-size-class
/// allocations move in and out), so the second sequence does not see the same
/// allocator evolution. Running each library in its OWN fresh process gives both
/// sides a pristine, identical allocator and makes the comparison deterministic.
pub const SEQ_ENV: &str = "DIFFTEST_SEQ_LIB";

pub fn seq_child_lib() -> Option<String> {
    std::env::var(SEQ_ENV).ok()
}

/// Run `test_name` in a fresh child process with `SEQ_ENV=which`, and return the
/// sequence line the child printed.
pub fn run_seq_child(test_name: &str, which: &str) -> String {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
        .env(SEQ_ENV, which)
        .output()
        .expect("spawn sequence child");
    assert!(
        out.status.success(),
        "sequence child ({which}) failed: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // With --nocapture libtest prefixes the line with "test <name> ... ", so
    // locate the marker anywhere in the line and keep everything from it on.
    let line = stdout
        .lines()
        .find(|l| l.contains("SEQ:"))
        .unwrap_or_else(|| {
            panic!(
                "child ({which}) printed no SEQ: line\n--- stdout ---\n{}",
                &stdout[..stdout.len().min(2000)]
            )
        });
    let idx = line.find("SEQ:").unwrap();
    line[idx..].to_string()
}

pub fn run_crash_child(test_name: &str, case: &str) -> Term {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args([test_name, "--exact", "--nocapture", "--test-threads=1"])
        .env(CRASH_ENV, case)
        .output()
        .expect("spawn crash child");
    match out.status.signal() {
        Some(s) => Term::Signal(s),
        None => Term::Exit(out.status.code().unwrap_or(-1)),
    }
}
