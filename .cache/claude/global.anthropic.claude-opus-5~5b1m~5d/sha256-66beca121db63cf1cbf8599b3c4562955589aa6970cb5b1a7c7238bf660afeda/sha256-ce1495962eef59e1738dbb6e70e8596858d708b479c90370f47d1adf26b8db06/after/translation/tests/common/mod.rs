//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading`; the Rust
//! side is **never** called directly as a Rust function, always through the
//! `.so`'s exported symbols, exactly like an external C consumer.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// C ABI mirror of `struct btac1c_idxstate_s` (verified against the C compiler:
// size=74 align=2 idx=0 lpred=2 rpred=4 tag=6 bcfcn=7 bsfcn=8 usefx=9 firfx=10)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdxState {
    pub idx: u16,
    pub lpred: i16,
    pub rpred: i16,
    pub tag: u8,
    pub bcfcn: u8,
    pub bsfcn: u8,
    pub usefx: u8,
    pub firfx: [[i16; 8]; 4],
}

impl IdxState {
    pub fn zeroed() -> Self {
        IdxState {
            idx: 0,
            lpred: 0,
            rpred: 0,
            tag: 0,
            bcfcn: 0,
            bsfcn: 0,
            usefx: 0,
            firfx: [[0i16; 8]; 4],
        }
    }
}

/// `int (*)(int *psamp, int idx, int pfcn, btac1c_idxstate *ridx)`
pub type PredictFn = unsafe extern "C" fn(*mut c_int, c_int, c_int, *mut IdxState) -> c_int;
/// `void *(*)(int pfcn)`
pub type GetPredictFuncFn = unsafe extern "C" fn(c_int) -> *mut std::ffi::c_void;
/// `int (*)(int pfcn)`
pub type CallPredictFn = unsafe extern "C" fn(c_int) -> c_int;

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn target_dir() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(v) => PathBuf::from(v),
        None => manifest_dir().join("target"),
    }
}

/// The C shared library produced by `cmake --build` in `c_src/build`.
///
/// The CMake project name is derived from the *parent directory* name, so the
/// file name is not fixed — glob for the single `.so` in the build dir.
pub fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert!(
        !found.is_empty(),
        "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    found.remove(0)
}

fn rust_so_in(profile: &str) -> PathBuf {
    target_dir().join(profile).join("libcall_predict_lib.so")
}

/// The `--release` Rust cdylib (the shipped artifact).
pub fn rust_so_release() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_SO") {
        return PathBuf::from(p);
    }
    let p = rust_so_in("release");
    assert!(
        p.exists(),
        "{} missing; build it with `cargo build --release --offline`",
        p.display()
    );
    p
}

/// The unoptimised Rust cdylib (keeps the `static` predictors as real, distinct
/// functions so their local symbols can be resolved).
pub fn rust_so_debug() -> Option<PathBuf> {
    let p = rust_so_in("debug");
    if p.exists() { Some(p) } else { None }
}

// ---------------------------------------------------------------------------
// Loaded library + internal (non-exported) symbol resolution
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: String,
    pub path: PathBuf,
    pub lib: Library,
    /// runtime load bias: `runtime_addr = link_addr + bias`
    bias: usize,
    is_rust: bool,
    /// cached `nm --defined-only` output (link addr, symbol name)
    syms: Vec<(usize, String)>,
}

fn nm_symbols(path: &Path) -> Vec<(usize, String)> {
    let out = Command::new("nm")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut v = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let a = match it.next() {
            Some(a) => a,
            None => continue,
        };
        let _ty = match it.next() {
            Some(t) => t,
            None => continue,
        };
        let name = match it.next() {
            Some(n) => n,
            None => continue,
        };
        if let Ok(addr) = usize::from_str_radix(a, 16) {
            v.push((addr, name.to_string()));
        }
    }
    v
}

impl Lib {
    pub fn open(name: &str, path: PathBuf, is_rust: bool) -> Lib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        // Establish the load bias from the one symbol both libraries export.
        let runtime: usize = unsafe {
            let s: Symbol<CallPredictFn> = lib
                .get(b"call_predict\0")
                .unwrap_or_else(|e| panic!("{} has no `call_predict`: {e}", path.display()));
            *s as usize
        };
        let syms = nm_symbols(&path);
        let link = syms
            .iter()
            .find(|(_, n)| n == "call_predict")
            .map(|(a, _)| *a)
            .unwrap_or_else(|| panic!("`call_predict` not in nm output of {}", path.display()));
        Lib {
            name: name.to_string(),
            path,
            lib,
            bias: runtime.wrapping_sub(link),
            is_rust,
            syms,
        }
    }

    pub fn call_predict(&self) -> CallPredictFn {
        unsafe {
            let s: Symbol<CallPredictFn> = self.lib.get(b"call_predict\0").unwrap();
            *s
        }
    }

    /// Resolve a `static` (local, non-exported) function by its *logical* C
    /// name. For the Rust `.so` the name is looked up through its `v0`-less
    /// legacy mangling (`_ZN<len(crate)><crate><len(name)><name>17h<hash>E`).
    pub fn internal_addr(&self, logical: &str) -> Option<usize> {
        let needle = format!("{}{}17h", logical.len(), logical);
        for (addr, n) in &self.syms {
            let hit = if self.is_rust {
                n.contains(&needle)
            } else {
                n == logical
            };
            if hit {
                return Some(self.bias.wrapping_add(*addr));
            }
        }
        None
    }

    pub fn predict_fn(&self, logical: &str) -> Option<PredictFn> {
        self.internal_addr(logical)
            .map(|a| unsafe { std::mem::transmute::<usize, PredictFn>(a) })
    }

    pub fn get_predict_func_fn(&self) -> Option<GetPredictFuncFn> {
        self.internal_addr("BTAC1C2_GetPredictFunc")
            .map(|a| unsafe { std::mem::transmute::<usize, GetPredictFuncFn>(a) })
    }
}

/// The C library and the release Rust library, ready for differential calls.
pub fn open_pair() -> (Lib, Lib) {
    (
        Lib::open("C", c_so_path(), false),
        Lib::open("Rust(release)", rust_so_release(), true),
    )
}

/// The C library and the *debug* Rust library (internal symbols available).
pub fn open_pair_debug() -> Option<(Lib, Lib)> {
    let r = rust_so_debug()?;
    Some((
        Lib::open("C", c_so_path(), false),
        Lib::open("Rust(debug)", r, true),
    ))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed -> reproducible property tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn next_i16(&mut self) -> i16 {
        self.next_u64() as u16 as i16
    }
    /// uniform in `[lo, hi]`
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

// ---------------------------------------------------------------------------
// Input shape generators (CONFIGS.md axis A3 / A4)
// ---------------------------------------------------------------------------

/// The fixed, hand-picked `psamp` shapes the C code special-cases in effect
/// (all-zero, constant, ramp, alternating, impulse, negatives, saturation,
/// `i32` extremes).
pub fn psamp_shapes() -> Vec<[c_int; 8]> {
    let mut v: Vec<[c_int; 8]> = vec![
        [0; 8],
        [1; 8],
        [-1; 8],
        [0, 1, 2, 3, 4, 5, 6, 7],
        [7, 6, 5, 4, 3, 2, 1, 0],
        [0, -1, -2, -3, -4, -5, -6, -7],
        [1, -1, 1, -1, 1, -1, 1, -1],
        [-1, 1, -1, 1, -1, 1, -1, 1],
        [1000, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 1000],
        [-1000, 0, 0, 0, 0, 0, 0, 0],
        [32767; 8],
        [-32768; 8],
        [32767, -32768, 32767, -32768, 32767, -32768, 32767, -32768],
        [i32::MAX; 8],
        [i32::MIN; 8],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX, i32::MIN, i32::MAX],
        [-3, -3, -3, -3, -3, -3, -3, -3],
        [3, 3, 3, 3, 3, 3, 3, 3],
        [-7, -5, -3, -1, 1, 3, 5, 7],
        [i32::MAX / 2, i32::MIN / 2, 1, -1, 0, 12345, -54321, 7],
    ];
    // random shapes: small magnitudes (typical audio), i16 range, full i32
    let mut r = Rng::new(0xC0FFEE_1234_5678);
    for _ in 0..24 {
        let mut a = [0i32; 8];
        for x in a.iter_mut() {
            *x = r.range_i32(-64, 64);
        }
        v.push(a);
    }
    for _ in 0..24 {
        let mut a = [0i32; 8];
        for x in a.iter_mut() {
            *x = r.range_i32(-32768, 32767);
        }
        v.push(a);
    }
    for _ in 0..24 {
        let mut a = [0i32; 8];
        for x in a.iter_mut() {
            *x = r.next_i32();
        }
        v.push(a);
    }
    v
}

/// `idx` values: the natural 0..7 window plus values that must be masked.
pub fn idx_shapes() -> Vec<c_int> {
    let mut v: Vec<c_int> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 63, 64, -1, -2, -7, -8, -9, -16, -63, -64, 1000,
        -1000, i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, i32::MAX / 2, i32::MIN / 2,
    ];
    let mut r = Rng::new(0xBEEF_0F0F_0F0F);
    for _ in 0..24 {
        v.push(r.next_i32());
    }
    v
}

/// `firfx` coefficient rows (axis A4).
pub fn firfx_shapes() -> Vec<[[i16; 8]; 4]> {
    let mut v: Vec<[[i16; 8]; 4]> = Vec::new();
    let presets: [[i16; 8]; 8] = [
        [0; 8],
        [256, 0, 0, 0, 0, 0, 0, 0],
        [i16::MAX; 8],
        [i16::MIN; 8],
        [i16::MAX, i16::MIN, i16::MAX, i16::MIN, i16::MAX, i16::MIN, i16::MAX, i16::MIN],
        [1, -2, 3, -4, 5, -6, 7, -8],
        [-1, -1, -1, -1, -1, -1, -1, -1],
        [512, -256, 128, -64, 32, -16, 8, -4],
    ];
    for p in presets.iter() {
        v.push([*p; 4]);
    }
    // distinct rows so that a wrong `pfcn - 12` row index is caught
    v.push([presets[1], presets[5], presets[7], presets[3]]);
    v.push([presets[3], presets[7], presets[5], presets[1]]);
    let mut r = Rng::new(0xFEED_FACE_CAFE);
    for _ in 0..24 {
        let mut m = [[0i16; 8]; 4];
        for row in m.iter_mut() {
            for c in row.iter_mut() {
                *c = r.next_i16();
            }
        }
        v.push(m);
    }
    v
}
