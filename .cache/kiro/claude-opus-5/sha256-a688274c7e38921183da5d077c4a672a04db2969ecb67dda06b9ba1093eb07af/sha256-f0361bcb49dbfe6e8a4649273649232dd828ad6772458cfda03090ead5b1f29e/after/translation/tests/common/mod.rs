//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and every call goes through
//! the exported `extern "C"` symbols — the Rust crate is never called directly,
//! so the `#[no_mangle]` wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture (declared directly, no `libc` crate).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// `fflush(NULL)` flushes *every* open output stream, which is what we want:
/// it drains whichever `FILE*` the loaded libraries have been writing to.
fn flush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Function pointer types matching the C declarations exactly.
// ---------------------------------------------------------------------------
pub type DriverFn = unsafe extern "C" fn(*const c_int, c_int);
pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub driver: DriverFn,
    pub fma_array: FmaArrayFn,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        // Resolve by exact exported name, exactly as an external consumer would.
        let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{name}: symbol `driver` not exported: {e}"));
        let fma_array: Symbol<FmaArrayFn> = unsafe { lib.get(b"fma_array\0") }
            .unwrap_or_else(|e| panic!("{name}: symbol `fma_array` not exported: {e}"));
        let driver = *driver;
        let fma_array = *fma_array;
        Impl {
            name,
            path,
            _lib: lib,
            driver,
            fma_array,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Locate the Rust `cdylib`. `cargo test` does not always emit the cdylib
/// artifact, so both profile directories are searched; the test runner script
/// runs `cargo build` first.
pub fn rust_so_path() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test-bin>
        if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
            candidates.push(profile_dir.join("libdriver.so"));
        }
    }
    let md = manifest_dir();
    candidates.push(md.join("target/release/libdriver.so"));
    candidates.push(md.join("target/debug/libdriver.so"));

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found. Looked in:\n{}\nBuild it with `cargo build --release`.",
        candidates
            .iter()
            .map(|p| format!("  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", c_so_path()),
        rs: Impl::load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture around an FFI call
// ---------------------------------------------------------------------------

/// Redirect fd 1 to a temp file, run `f`, restore fd 1 and return the raw bytes
/// that were written. Captures whatever the loaded `.so` printed via `printf`.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}_{}.out",
        std::process::id(),
        tag,
        n
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture temp file");

    // Drain anything already pending so it is not attributed to this call.
    flush_all();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    // The library's stdout is fully buffered now that fd 1 is a file; flush it
    // before restoring, otherwise the bytes land in the wrong place.
    flush_all();

    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut buf).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&path);
    buf
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Values sitting on the `int` multiply/add overflow boundaries.
pub const EXTREMES: [i32; 15] = [
    i32::MIN,
    i32::MIN + 1,
    -65536,
    -46341,
    -46340,
    -2,
    -1,
    0,
    1,
    2,
    46340,
    46341,
    65535,
    65536,
    i32::MAX,
];

/// The value-class axis from CONFIGS.md.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Vals {
    FullRandom,
    Zeros,
    SmallPos,
    SmallNeg,
    Extremes,
}

impl Vals {
    pub fn make(self, rng: &mut Rng, len: usize) -> Vec<i32> {
        (0..len)
            .map(|_| match self {
                Vals::FullRandom => rng.next_i32(),
                Vals::Zeros => 0,
                Vals::SmallPos => rng.range(1, 100) as i32,
                Vals::SmallNeg => rng.range(-100, -1) as i32,
                Vals::Extremes => rng.pick(&EXTREMES),
            })
            .collect()
    }
}

/// The pointer-aliasing axis from CONFIGS.md. `map` gives the backing-slot
/// index for `(out, mul1, mul2, add)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alias {
    Disjoint,
    Mul1EqMul2,
    OutEqMul1,
    OutEqAdd,
    AllSame,
}

impl Alias {
    pub fn slots(self) -> usize {
        match self {
            Alias::Disjoint => 4,
            Alias::Mul1EqMul2 | Alias::OutEqMul1 | Alias::OutEqAdd => 3,
            Alias::AllSame => 1,
        }
    }
    pub fn map(self) -> [usize; 4] {
        match self {
            Alias::Disjoint => [0, 1, 2, 3],
            Alias::Mul1EqMul2 => [0, 1, 1, 2],
            Alias::OutEqMul1 => [0, 0, 1, 2],
            Alias::OutEqAdd => [0, 1, 2, 0],
            Alias::AllSame => [0, 0, 0, 0],
        }
    }
}

/// Run `fma_array` in one implementation over freshly-copied backing buffers
/// and return the post-call contents of every backing buffer.
pub fn run_fma(imp: &Impl, alias: Alias, seed_slots: &[Vec<i32>], len: c_int) -> Vec<Vec<i32>> {
    let mut slots: Vec<Vec<i32>> = seed_slots.to_vec();
    let map = alias.map();
    let base: Vec<*mut c_int> = slots.iter_mut().map(|v| v.as_mut_ptr()).collect();
    let out = base[map[0]];
    let mul1 = base[map[1]] as *const c_int;
    let mul2 = base[map[2]] as *const c_int;
    let add = base[map[3]] as *const c_int;
    unsafe { (imp.fma_array)(out, mul1, mul2, add, len) };
    slots
}

/// Differential `fma_array` check for one configuration.
pub fn diff_fma(alias: Alias, seed_slots: &[Vec<i32>], len: c_int, ctx: &str) {
    let p = pair();
    let c_out = run_fma(&p.c, alias, seed_slots, len);
    let rs_out = run_fma(&p.rs, alias, seed_slots, len);
    assert_eq!(
        c_out, rs_out,
        "fma_array divergence [{ctx}] alias={alias:?} len={len}\n  inputs={seed_slots:?}\n  \
         C   ={c_out:?}\n  Rust={rs_out:?}"
    );
}

/// Differential `driver` check: compares captured stdout byte-for-byte.
pub fn diff_driver(data: &[i32], len: c_int, ctx: &str) -> Vec<u8> {
    let p = pair();
    // Each side gets its own copy of the input so a stray write cannot leak
    // between the two runs.
    let c_in = data.to_vec();
    let rs_in = data.to_vec();

    let f = p.c.driver;
    let c_bytes = capture_stdout("c", || unsafe { f(c_in.as_ptr(), len) });
    let g = p.rs.driver;
    let rs_bytes = capture_stdout("rs", || unsafe { g(rs_in.as_ptr(), len) });

    assert_eq!(
        c_in, rs_in,
        "driver mutated its `const` input differently [{ctx}] len={len}"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_bytes),
        String::from_utf8_lossy(&rs_bytes),
        "driver stdout divergence [{ctx}] len={len} data={data:?}"
    );
    assert_eq!(
        c_bytes, rs_bytes,
        "driver stdout byte divergence [{ctx}] len={len}"
    );
    c_bytes
}

/// The length shapes from CONFIGS.md.
pub fn len_shapes(rng: &mut Rng) -> Vec<usize> {
    vec![
        1,
        2,
        rng.range(3, 8) as usize,
        rng.range(64, 512) as usize,
        1024,
    ]
}
