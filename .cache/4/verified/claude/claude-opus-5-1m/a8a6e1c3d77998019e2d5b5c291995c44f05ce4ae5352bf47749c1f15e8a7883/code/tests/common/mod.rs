// Shared differential-test harness.
//
// Both the C shared object and the Rust shared object are loaded with
// `libloading` (i.e. `dlopen`) and driven *only* through their exported
// symbols, exactly the way an external C consumer would.  Rust functions of
// the crate are never called directly, so the `#[unsafe(no_mangle)]`
// `extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `#define ARRAY_SIZE (256 * 1024)`
pub const ARRAY_SIZE: usize = 256 * 1024;
/// `#define ITERATIONS 2000`
pub const ITERATIONS: usize = 2000;

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (never used to implement the library)
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn srand(seed: c_uint);
    fn rand() -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// One loaded shared object
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    array: *mut c_int,
    peo: unsafe extern "C" fn(),
    long_exec_fn: unsafe extern "C" fn(c_uint),
    _lib: Library,
}

// Access is serialised through `harness()`'s mutex; the pointers point into the
// (process-wide) mapping of the shared object.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    pub fn open(path: &Path, name: &'static str) -> Lib {
        assert!(
            path.exists(),
            "shared object {} does not exist — build it first",
            path.display()
        );
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

            // `int array[ARRAY_SIZE]` — a data symbol: take the *address* of
            // the symbol, not its contents.
            // (`Symbol<T>`'s `T` only has to be pointer-sized here — we take
            // the raw `dlsym` result, which for a data symbol is its address.)
            let arr_sym: Symbol<*mut c_int> = lib
                .get(b"array\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(array) failed: {e}"));
            let array = arr_sym.into_raw().into_raw() as *mut c_int;

            let peo_sym: Symbol<unsafe extern "C" fn()> = lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| {
                    panic!("{name}: dlsym(perform_expensive_operations) failed: {e}")
                });
            let peo = *peo_sym;

            let le_sym: Symbol<unsafe extern "C" fn(c_uint)> = lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("{name}: dlsym(long_exec) failed: {e}"));
            let long_exec_fn = *le_sym;

            Lib {
                name,
                path: path.to_path_buf(),
                array,
                peo,
                long_exec_fn,
                _lib: lib,
            }
        }
    }

    // --- the exported data object -----------------------------------------

    pub fn array_ptr(&self) -> *mut c_int {
        self.array
    }

    pub fn read_array(&self) -> Vec<c_int> {
        unsafe { std::slice::from_raw_parts(self.array, ARRAY_SIZE).to_vec() }
    }

    pub fn read_bytes(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.array as *const u8, ARRAY_SIZE * 4).to_vec() }
    }

    pub fn write_array(&self, data: &[c_int]) {
        assert!(data.len() <= ARRAY_SIZE);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.array, data.len()) }
    }

    pub fn set(&self, index: usize, value: c_int) {
        assert!(index < ARRAY_SIZE);
        unsafe { *self.array.add(index) = value }
    }

    pub fn get(&self, index: usize) -> c_int {
        assert!(index < ARRAY_SIZE);
        unsafe { *self.array.add(index) }
    }

    pub fn zero_array(&self) {
        unsafe { std::ptr::write_bytes(self.array as *mut u8, 0, ARRAY_SIZE * 4) }
    }

    /// XOR reduction exactly as `long_exec` computes it.
    pub fn xor_array(&self) -> c_int {
        let mut x: c_int = 0;
        unsafe {
            for i in 0..ARRAY_SIZE {
                x ^= *self.array.add(i);
            }
        }
        x
    }

    // --- the exported functions -------------------------------------------

    pub fn perform_expensive_operations(&self) {
        unsafe { (self.peo)() }
    }

    pub fn perform_expensive_operations_n(&self, n: usize) {
        for _ in 0..n {
            unsafe { (self.peo)() }
        }
    }

    pub fn long_exec(&self, seed: c_uint) {
        unsafe { (self.long_exec_fn)(seed) }
    }

    /// Call `perform_expensive_operations` through a deliberately *wrong*
    /// prototype: the C definition `void perform_expensive_operations()` has an
    /// unspecified parameter list, so extra arguments are legal at the ABI
    /// level and must be ignored.
    pub fn peo_with_extra_args(&self, a: c_int, b: *const c_char, c: f64) {
        let f: unsafe extern "C" fn(c_int, *const c_char, f64) =
            unsafe { std::mem::transmute(self.peo) };
        unsafe { f(a, b, c) }
    }

    /// Call `long_exec` through a signed prototype (same ABI register).
    pub fn long_exec_signed(&self, seed: c_int) {
        let f: unsafe extern "C" fn(c_int) = unsafe { std::mem::transmute(self.long_exec_fn) };
        unsafe { f(seed) }
    }
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/liblong.so")
}

/// `target/<profile>/liblong.so` for the profile the test binary itself was
/// built with (test executables live in `target/<profile>/deps/`).
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();
    profile_dir.join("liblong.so")
}

pub fn rust_so_path_for(profile: &str) -> PathBuf {
    manifest_dir().join("target").join(profile).join("liblong.so")
}

// ---------------------------------------------------------------------------
// Globally serialised access (each `.so` has ONE global `array`)
// ---------------------------------------------------------------------------

static LOCK: Mutex<()> = Mutex::new(());
static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub struct Harness {
    _guard: MutexGuard<'static, ()>,
    pub c: &'static Lib,
    pub rust: &'static Lib,
}

/// Load both libraries (once per process) and take the global lock.
pub fn harness() -> Harness {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let c = C_LIB.get_or_init(|| Lib::open(&c_so_path(), "C"));
    let rust = RUST_LIB.get_or_init(|| Lib::open(&rust_so_path(), "Rust"));
    Harness {
        _guard: guard,
        c,
        rust,
    }
}

impl Harness {
    pub fn libs(&self) -> [&'static Lib; 2] {
        [self.c, self.rust]
    }

    /// Write the same input into both libraries' `array`.
    pub fn write_both(&self, data: &[c_int]) {
        self.c.write_array(data);
        self.rust.write_array(data);
    }

    pub fn zero_both(&self) {
        self.c.zero_array();
        self.rust.zero_array();
    }

    /// Compare the two 1 MiB `array` objects byte-for-byte and report the first
    /// difference.
    pub fn assert_arrays_equal(&self, ctx: &str) {
        assert_arrays_equal_libs(self.c, self.rust, ctx);
    }
}

pub fn assert_arrays_equal_libs(a: &Lib, b: &Lib, ctx: &str) {
    let av = a.read_bytes();
    let bv = b.read_bytes();
    if av == bv {
        return;
    }
    let ai = a.read_array();
    let bi = b.read_array();
    let mut diffs = 0usize;
    let mut first = String::new();
    for i in 0..ARRAY_SIZE {
        if ai[i] != bi[i] {
            if diffs == 0 {
                first = format!(
                    "index {i}: {} = {} (0x{:08x}) vs {} = {} (0x{:08x})",
                    a.name, ai[i], ai[i] as u32, b.name, bi[i], bi[i] as u32
                );
            }
            diffs += 1;
        }
    }
    panic!("[{ctx}] array mismatch between {} and {}: {diffs} of {ARRAY_SIZE} elements differ; first {first}",
           a.name, b.name);
}

// ---------------------------------------------------------------------------
// Deterministic RNG for the property-style rows (fixed seed => reproducible)
// ---------------------------------------------------------------------------

pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        SplitMix64(seed)
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// A full-size array of uniformly random `i32`.
pub fn random_array(rng: &mut SplitMix64) -> Vec<c_int> {
    (0..ARRAY_SIZE).map(|_| rng.next_i32()).collect()
}

/// A full-size array of non-negative values, the shape `rand()` produces.
pub fn random_nonnegative_array(rng: &mut SplitMix64) -> Vec<c_int> {
    (0..ARRAY_SIZE)
        .map(|_| (rng.next_u32() >> 1) as c_int)
        .collect()
}

/// Exactly the array `long_exec` builds: `srand(seed)` then `ARRAY_SIZE`
/// consecutive `rand()` values from the platform PRNG.
pub fn libc_rand_array(seed: c_uint) -> Vec<c_int> {
    unsafe {
        srand(seed);
        (0..ARRAY_SIZE).map(|_| rand()).collect()
    }
}

/// Values that stress every sign / overflow / division / modulo boundary of
/// `x*3+7`, `x^(x>>3)`, `x-(x<<1)`, `x/2 + x%7`.
pub fn boundary_values() -> Vec<c_int> {
    let mut v: Vec<c_int> = Vec::new();
    v.extend([
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        8,
        -8,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX - 2,
        i32::MIN + 2,
    ]);
    for k in -16..=16 {
        v.push(k);
    }
    for b in 0..31 {
        let p = 1i32 << b;
        v.push(p);
        v.push(p.wrapping_sub(1));
        v.push(p.wrapping_add(1));
        v.push(p.wrapping_neg());
        v.push(p.wrapping_neg().wrapping_sub(1));
        v.push(p.wrapping_neg().wrapping_add(1));
    }
    for m in 0..40 {
        let s = 7i32.wrapping_mul(m);
        v.push(s);
        v.push(s + 1);
        v.push(s - 1);
        v.push(-s);
        v.push(-s + 1);
        v.push(-s - 1);
    }
    for k in 0..9 {
        v.push(i32::MIN + k);
        v.push(i32::MAX - k);
    }
    v
}

/// Tile `values` over a full-size array.
pub fn tile(values: &[c_int]) -> Vec<c_int> {
    (0..ARRAY_SIZE).map(|i| values[i % values.len()]).collect()
}

// ---------------------------------------------------------------------------
// stdout capture (the `printf("%d\n", ...)` channel of `long_exec`)
// ---------------------------------------------------------------------------

/// Run `f` with the process' file descriptor 1 redirected into a temporary
/// file, then return the bytes that were written.  This captures what the
/// *shared object* wrote through libc `printf`, which is the observable output
/// of `long_exec`.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "long_stdout_{}_{}_{}.txt",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let out = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        std::fs::read(&path).expect("read capture file")
    };
    let _ = std::fs::remove_file(&path);
    out
}
