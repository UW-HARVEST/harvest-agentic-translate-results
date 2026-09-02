//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called only through their exported `merge_sort` symbol, exactly as an
//! external consumer would. Nothing is called directly on the Rust crate, so
//! the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Raw, alignment-correct storage for `spritebatch_sprite_t`
// ---------------------------------------------------------------------------

/// One `spritebatch_sprite_t` as raw bytes.
///
/// `sizeof(spritebatch_sprite_t) == 16`, `alignof == 8`:
///
/// * bytes `0..8`   — `unsigned long long texture_id`
/// * bytes `8..12`  — `int sort_bits`
/// * bytes `12..16` — tail padding (observable: the C struct copy is 16 bytes
///   wide, so garbage padding is propagated rather than normalised)
///
/// `align(8)` guarantees a `Vec<Sprite>` allocation is correctly aligned for
/// the C struct.
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Sprite(pub [u8; 16]);

impl Sprite {
    pub const fn zeroed() -> Self {
        Sprite([0u8; 16])
    }

    pub fn texture_id(&self) -> u64 {
        u64::from_ne_bytes(self.0[0..8].try_into().unwrap())
    }

    pub fn sort_bits(&self) -> i32 {
        i32::from_ne_bytes(self.0[8..12].try_into().unwrap())
    }

    pub fn padding(&self) -> [u8; 4] {
        self.0[12..16].try_into().unwrap()
    }

    pub fn set_texture_id(&mut self, v: u64) {
        self.0[0..8].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn set_sort_bits(&mut self, v: i32) {
        self.0[8..12].copy_from_slice(&v.to_ne_bytes());
    }

    pub fn set_padding(&mut self, v: [u8; 4]) {
        self.0[12..16].copy_from_slice(&v);
    }

    pub fn new(texture_id: u64, sort_bits: i32, padding: [u8; 4]) -> Self {
        let mut s = Sprite::zeroed();
        s.set_texture_id(texture_id);
        s.set_sort_bits(sort_bits);
        s.set_padding(padding);
        s
    }
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{tex:{:#x} bits:{} pad:{:02x?}}}",
            self.texture_id(),
            self.sort_bits(),
            self.padding()
        )
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0FF_EE00_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn bytes4(&mut self) -> [u8; 4] {
        (self.next_u32()).to_ne_bytes()
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, i32);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf()
}

/// Locate the C `.so`. Its name is derived from the parent directory name by
/// `c_src/CMakeLists.txt`, so it is discovered rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("lib") && name.ends_with(".so")
        })
        .collect();
    found.sort();
    match found.len() {
        0 => panic!("no lib*.so found in {}", build_dir.display()),
        _ => found.remove(0),
    }
}

/// Locate the Rust `.so`. Prefers `target/release` (the profile that carries
/// `panic = "abort"`, i.e. the real shipping artifact) and falls back to debug.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libmerge_sort_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmerge_sort_lib.so not found under {}; run `cargo build --release`",
        base.display()
    );
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Both implementations, loaded through `dlopen` and reached only via the
/// exported `merge_sort` symbol.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: MergeSortFn,
    pub rust: MergeSortFn,
}

impl Pair {
    pub fn load() -> Self {
        let c_lib = load(&c_so_path());
        let rust_lib = load(&rust_so_path());
        let c = unsafe {
            let s: Symbol<MergeSortFn> = c_lib
                .get(b"merge_sort\0")
                .expect("C .so does not export merge_sort");
            *s
        };
        let rust = unsafe {
            let s: Symbol<MergeSortFn> = rust_lib
                .get(b"merge_sort\0")
                .expect("Rust .so does not export merge_sort");
            *s
        };
        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

fn dump(v: &[Sprite]) -> String {
    let mut s = String::new();
    for (i, e) in v.iter().enumerate() {
        s.push_str(&format!("  [{i}] {e:?}\n"));
    }
    s
}

/// Run both implementations on identical copies of `(a_in, b_in)` with the
/// given `size` and assert the resulting byte images of BOTH buffers match.
///
/// `size` is passed through verbatim so that callers can hand the C an `int`
/// that disagrees with the actual buffer lengths.
pub fn diff_with_size(label: &str, a_in: &[Sprite], b_in: &[Sprite], size: i32) {
    let pair = Pair::load();
    diff_with_size_on(&pair, label, a_in, b_in, size);
}

pub fn diff_with_size_on(pair: &Pair, label: &str, a_in: &[Sprite], b_in: &[Sprite], size: i32) {
    let mut a_c = a_in.to_vec();
    let mut b_c = b_in.to_vec();
    let mut a_r = a_in.to_vec();
    let mut b_r = b_in.to_vec();

    let ap_c = if a_c.is_empty() { std::ptr::null_mut() } else { a_c.as_mut_ptr() };
    let bp_c = if b_c.is_empty() { std::ptr::null_mut() } else { b_c.as_mut_ptr() };
    let ap_r = if a_r.is_empty() { std::ptr::null_mut() } else { a_r.as_mut_ptr() };
    let bp_r = if b_r.is_empty() { std::ptr::null_mut() } else { b_r.as_mut_ptr() };

    unsafe { (pair.c)(ap_c, bp_c, size) };
    unsafe { (pair.rust)(ap_r, bp_r, size) };

    if a_c != a_r || b_c != b_r {
        panic!(
            "DIVERGENCE [{label}] size={size}\n\
             --- input a ---\n{}\
             --- input b ---\n{}\
             --- C out a ---\n{}\
             --- R out a ---\n{}\
             --- C out b ---\n{}\
             --- R out b ---\n{}",
            dump(a_in),
            dump(b_in),
            dump(&a_c),
            dump(&a_r),
            dump(&b_c),
            dump(&b_r),
        );
    }
}

/// Convenience: `size` equals `a.len()`.
pub fn diff(label: &str, a_in: &[Sprite], b_in: &[Sprite]) {
    diff_with_size(label, a_in, b_in, a_in.len() as i32);
}

pub fn diff_on(pair: &Pair, label: &str, a_in: &[Sprite], b_in: &[Sprite]) {
    diff_with_size_on(pair, label, a_in, b_in, a_in.len() as i32);
}

// ---------------------------------------------------------------------------
// Input generators (one per data-shape axis in CONFIGS.md)
// ---------------------------------------------------------------------------

/// Scratch buffer prefilled with random garbage — its final content is part of
/// the observable behaviour.
pub fn garbage_scratch(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| Sprite::new(rng.next_u64(), rng.next_i32(), rng.bytes4()))
        .collect()
}

pub fn zero_scratch(n: usize) -> Vec<Sprite> {
    vec![Sprite::zeroed(); n]
}

pub fn gen_full_random(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| Sprite::new(rng.next_u64(), rng.next_i32(), rng.bytes4()))
        .collect()
}

/// Random fields, but padding forced to zero.
pub fn gen_clean_padding(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| Sprite::new(rng.next_u64(), rng.next_i32(), [0; 4]))
        .collect()
}

pub fn gen_all_bits_equal(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let bits = rng.next_i32();
    (0..n)
        .map(|_| Sprite::new(rng.next_u64(), bits, rng.bytes4()))
        .collect()
}

pub fn gen_total_duplicates(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let s = Sprite::new(rng.next_u64(), rng.next_i32(), rng.bytes4());
    vec![s; n]
}

pub fn gen_ascending(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let mut bits: i32 = -(n as i32) / 2;
    (0..n)
        .map(|_| {
            let s = Sprite::new(rng.next_u64(), bits, rng.bytes4());
            bits = bits.wrapping_add(1 + (rng.below(3) as i32));
            s
        })
        .collect()
}

pub fn gen_descending(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let mut bits: i32 = (n as i32) / 2;
    (0..n)
        .map(|_| {
            let s = Sprite::new(rng.next_u64(), bits, rng.bytes4());
            bits = bits.wrapping_sub(1 + (rng.below(3) as i32));
            s
        })
        .collect()
}

pub fn gen_two_valued(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let (x, y) = (rng.next_i32(), rng.next_i32());
    (0..n)
        .map(|_| {
            let bits = if rng.next_u64() & 1 == 0 { x } else { y };
            Sprite::new(rng.next_u64(), bits, rng.bytes4())
        })
        .collect()
}

pub fn gen_small_range(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| Sprite::new(rng.next_u64(), rng.below(4) as i32, rng.bytes4()))
        .collect()
}

/// Full-range random `sort_bits` with the signed extremes injected.
pub fn gen_extreme_bits(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    let choices = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    (0..n)
        .map(|i| {
            let bits = if i % 3 == 0 {
                choices[(rng.below(choices.len() as u64)) as usize]
            } else {
                rng.next_i32()
            };
            Sprite::new(rng.next_u64(), bits, rng.bytes4())
        })
        .collect()
}

/// Only `INT_MIN` / `INT_MAX` — maximal-distance signed pairs.
pub fn gen_minmax_bits(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| {
            let bits = if rng.next_u64() & 1 == 0 { i32::MIN } else { i32::MAX };
            Sprite::new(rng.next_u64(), bits, rng.bytes4())
        })
        .collect()
}

/// `texture_id` restricted to the u64 boundary values.
pub fn gen_extreme_texture(rng: &mut Rng, n: usize) -> Vec<Sprite> {
    (0..n)
        .map(|_| {
            let tex = match rng.below(4) {
                0 => 0u64,
                1 => u64::MAX,
                2 => 1u64,
                _ => u64::MAX - 1,
            };
            Sprite::new(tex, rng.next_i32(), rng.bytes4())
        })
        .collect()
}

/// The `size` values enumerated in CONFIGS.md.
pub const SIZES: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255,
    256, 1000, 1024,
];

pub const POW2_SIZES: &[usize] = &[4, 8, 16, 32, 64, 128, 1024];
pub const NON_POW2_SIZES: &[usize] = &[5, 7, 9, 15, 17, 31, 33, 63, 100, 127, 129, 1000, 4096];

/// Iterations per (generator, size) cell.
pub const REPS: usize = 12;
