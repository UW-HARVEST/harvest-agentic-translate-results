// Shared differential-testing harness.
//
// Both libraries are loaded through `libloading` and driven ONLY through their
// exported `.so` symbols -- never by calling Rust functions directly -- so the
// `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// `#define ARRAY_SIZE (256 * 1024)` from `c_src/src/long.c:29`.
pub const N: usize = 256 * 1024;
/// `#define ITERATIONS 2000` from `c_src/src/long.c:30`.
pub const ITERATIONS: usize = 2000;
/// Byte size of the exported `array` object: 256 * 1024 * sizeof(int).
pub const ARRAY_BYTES: u64 = (N * 4) as u64;

/// Ground truth measured from gcc: the value a zero element becomes after ONE
/// `perform_expensive_operations` call (i.e. 100 applications of the inner
/// loop). Note that 0 is *not* a fixed point -- `step(0) == -3` -- and the
/// orbit never settles, which is why `long_exec`'s 2000 iterations genuinely
/// have to be run to validate it end to end.
pub const CHURN_OF_ZERO: c_int = -626_538_949;

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture. Declared here so the test crate does
// not need a `libc` dependency (which is not available offline).
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn srand(seed: c_uint);
    fn rand() -> c_int;
}

/// Reference glibc `rand()` sequence, produced by the *same* libc both `.so`s
/// are dynamically linked against.
pub fn libc_rand_fill(seed: c_uint, out: &mut [c_int]) {
    unsafe {
        srand(seed);
        for slot in out.iter_mut() {
            *slot = rand();
        }
    }
}

// ---------------------------------------------------------------------------
// A single loaded library under test.
// ---------------------------------------------------------------------------

pub struct Target {
    pub name: String,
    pub path: PathBuf,
    lib: Library,
}

impl Target {
    fn open(name: &str, path: PathBuf) -> Option<Target> {
        if !path.exists() {
            return None;
        }
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Some(Target {
            name: name.to_string(),
            path,
            lib,
        })
    }

    /// Address of the exported `array` global.
    ///
    /// `libloading`'s `Symbol<T>::deref` reinterprets the *pointer field*, so
    /// `Symbol<*mut c_int>` dereferences to the address of the object -- which
    /// is exactly what `dlsym("array")` returns.
    pub fn array_ptr(&self) -> *mut c_int {
        unsafe {
            let sym: Symbol<*mut c_int> = self
                .lib
                .get(b"array\0")
                .unwrap_or_else(|e| panic!("[{}] dlsym(array): {e}", self.name));
            *sym
        }
    }

    pub fn read_array(&self) -> Vec<c_int> {
        let mut v = vec![0 as c_int; N];
        unsafe { std::ptr::copy_nonoverlapping(self.array_ptr(), v.as_mut_ptr(), N) };
        v
    }

    pub fn write_array(&self, src: &[c_int]) {
        assert_eq!(src.len(), N, "fill must cover the whole array");
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), self.array_ptr(), N) };
    }

    /// Call the exported `perform_expensive_operations` (no args, no return).
    pub fn peo(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self
                .lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| {
                    panic!("[{}] dlsym(perform_expensive_operations): {e}", self.name)
                });
            f();
        }
    }

    /// Call the exported `long_exec(unsigned int)`.
    pub fn long_exec(&self, seed: c_uint) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_uint)> = self
                .lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("[{}] dlsym(long_exec): {e}", self.name));
            f(seed);
        }
    }

    /// Call `long_exec` through a deliberately *mismatched* prototype that
    /// passes surplus register arguments (ERRORS.md row 6).
    pub fn long_exec_extra_args(&self, seed: c_uint, junk: u64) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_uint, u64, u64, u64)> = self
                .lib
                .get(b"long_exec\0")
                .unwrap_or_else(|e| panic!("[{}] dlsym(long_exec): {e}", self.name));
            f(seed, junk, junk ^ 0xa5a5_a5a5, !junk);
        }
    }

    /// Call `perform_expensive_operations` through a prototype that passes a
    /// null pointer, exercising the "null pointer" boundary against a function
    /// that declares no parameters (ERRORS.md row 7).
    pub fn peo_with_null_arg(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*mut std::ffi::c_void) -> ()> = self
                .lib
                .get(b"perform_expensive_operations\0")
                .unwrap_or_else(|e| panic!("[{}] dlsym: {e}", self.name));
            f(std::ptr::null_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// The harness: one C target plus every Rust target that has been built.
// ---------------------------------------------------------------------------

pub struct Harness {
    pub c: Target,
    pub rust: Vec<Target>,
}

impl Harness {
    /// Every target, C first.
    pub fn all(&self) -> impl Iterator<Item = &Target> {
        std::iter::once(&self.c).chain(self.rust.iter())
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("LONG_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/liblong.so")
}

fn build_harness() -> Harness {
    let cpath = c_so_path();
    let c = Target::open("C", cpath.clone()).unwrap_or_else(|| {
        panic!(
            "C shared library not found at {}.\n\
             Build it with:\n  cd c_src && mkdir -p build && cd build && \\\n\
             \x20   cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            cpath.display()
        )
    });

    let mut rust = Vec::new();
    if let Ok(p) = std::env::var("LONG_RUST_SO") {
        rust.push(Target::open("rust", PathBuf::from(&p)).expect("LONG_RUST_SO does not exist"));
    } else {
        // Use every profile that has been built. The debug profile is the
        // interesting one for UB parity: it has `overflow-checks = true`, so
        // any arithmetic the translation did NOT spell out as `wrapping_*`
        // aborts the process instead of wrapping like the C does.
        for (name, rel) in [
            ("rust-debug", "target/debug/liblong.so"),
            ("rust-release", "target/release/liblong.so"),
        ] {
            if let Some(t) = Target::open(name, manifest_dir().join(rel)) {
                rust.push(t);
            }
        }
    }
    assert!(
        !rust.is_empty(),
        "no Rust liblong.so found; run `cargo build` and/or `cargo build --release`"
    );
    Harness { c, rust }
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Acquire the harness.
///
/// A process-wide lock is mandatory: `dlopen` of the same path returns the same
/// handle, so all tests share one `array` per library, and `srand`/`rand` plus
/// `stdout` are process-global too. Tests must therefore not run concurrently.
/// Poisoning is ignored so one failing row does not cascade into false
/// failures in the others.
pub fn harness() -> MutexGuard<'static, Harness> {
    HARNESS
        .get_or_init(|| Mutex::new(build_harness()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seeds keep every row reproducible.
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
    /// Uniform over all 2^32 bit patterns of `int`.
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform over `[0, 2^31)` -- the shape glibc `rand()` produces.
    pub fn next_nonneg(&mut self) -> c_int {
        (self.next_u32() >> 1) as c_int
    }
    /// Uniform over `[INT_MIN, 0)`.
    pub fn next_neg(&mut self) -> c_int {
        !((self.next_u32() >> 1) as c_int)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers.
// ---------------------------------------------------------------------------

fn describe_mismatch(label: &str, cname: &str, rname: &str, c: &[c_int], r: &[c_int]) -> String {
    let mut diffs = Vec::new();
    for i in 0..N {
        if c[i] != r[i] {
            if diffs.len() < 8 {
                diffs.push(format!("  [{i}]: {cname}={} {rname}={}", c[i], r[i]));
            }
        }
    }
    let total = (0..N).filter(|&i| c[i] != r[i]).count();
    format!(
        "{label}: {rname} diverges from {cname} in {total}/{N} elements\nfirst mismatches:\n{}",
        diffs.join("\n")
    )
}

/// Write `input` into every library's `array`, then apply
/// `perform_expensive_operations` `calls` times, asserting after EVERY call
/// that all Rust libraries still agree with C byte-for-byte.
pub fn diff_peo(h: &Harness, label: &str, input: &[c_int], calls: usize) {
    for t in h.all() {
        t.write_array(input);
    }
    // `calls == 0` still checks that the plain write/read round trip agrees.
    for step in 0..=calls {
        if step > 0 {
            for t in h.all() {
                t.peo();
            }
        }
        let c = h.c.read_array();
        for t in &h.rust {
            let r = t.read_array();
            assert!(
                c == r,
                "{}",
                describe_mismatch(
                    &format!("{label} after {step} perform_expensive_operations call(s)"),
                    &h.c.name,
                    &t.name,
                    &c,
                    &r
                )
            );
        }
    }
}

/// `xor_result` fold exactly as `long_exec` performs it (`long.c:60-63`).
pub fn xor_fold(a: &[c_int]) -> c_int {
    let mut acc: c_int = 0;
    for v in a {
        acc ^= *v;
    }
    acc
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level.
//
// `long_exec` uses libc `printf`, which writes to fd 1; Rust's test harness
// only captures Rust-level `print!`, so fd 1 has to be redirected by hand.
// ---------------------------------------------------------------------------

pub fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("long_stdout_{}.txt", std::process::id()));
    let out = String::from_utf8(capture_stdout_to(&path, f)).expect("stdout was not valid UTF-8");
    let _ = std::fs::remove_file(&path);
    out
}

fn capture_stdout_to<F: FnOnce()>(path: &Path, f: F) -> Vec<u8> {
    use std::os::unix::io::AsRawFd;
    let file = std::fs::File::create(path).expect("cannot create capture file");
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);
    std::fs::read(path).expect("cannot read capture file")
}

// ---------------------------------------------------------------------------
// Edge-case value table used by several rows.
// ---------------------------------------------------------------------------

pub const EDGE_VALUES: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    3,
    -3,
    4,
    -4,
    6,
    -6,
    7,
    -7,
    8,
    -8,
    13,
    -13,
    49,
    -49,
    1 << 29,
    -(1 << 29),
    1 << 30,
    -(1 << 30),
    (1 << 30) + 1,
    -((1 << 30) + 1),
    c_int::MAX,
    c_int::MAX - 1,
    c_int::MAX / 3,
    c_int::MAX / 3 + 1,
    c_int::MIN,
    c_int::MIN + 1,
    c_int::MIN / 3,
    c_int::MIN / 3 - 1,
    -2147483647,
    1431655765,
    -1431655765,
    715827883,
    -715827883,
];

/// Fill `buf` with `value` everywhere.
pub fn fill_uniform(buf: &mut [c_int], value: c_int) {
    for slot in buf.iter_mut() {
        *slot = value;
    }
}

/// Fill `buf` by cycling `table`, so consecutive 8-lane groups (the Rust
/// worker's `chunks_exact(8)` batches) see different mixes of edge cases.
pub fn fill_cycle(buf: &mut [c_int], table: &[c_int], offset: usize) {
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = table[(i + offset) % table.len()];
    }
}
