//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and called
//! only through their exported `merge_sort` symbol. The Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `void merge_sort(spritebatch_sprite_t *a, spritebatch_sprite_t *b, int size)`
pub type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, i32);

/// Byte-exact mirror of the C `spritebatch_sprite_t`.
///
/// The C struct has 4 bytes of *implicit* trailing padding (verified with
/// `offsetof`/`sizeof` against gcc: size 16, align 8, `texture_id` @0,
/// `sort_bits` @8). Here the padding is made *explicit* so the test can seed it
/// with known values and compare all 16 bytes of every element — the C
/// `b[k] = a[i]` struct assignment compiles to two 8-byte `mov`s and therefore
/// does propagate the padding, so it is part of the observable output.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Sprite {
    pub texture_id: u64,
    pub sort_bits: i32,
    pub pad: u32,
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{tex:{:#x} bits:{} pad:{:#x}}}",
            self.texture_id, self.sort_bits, self.pad
        )
    }
}

pub const SPRITE_SIZE: usize = 16;

/// Deterministic PRNG (splitmix64) — no external rand dependency, so every run
/// is bit-for-bit reproducible from the seed.
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
    /// Uniform-ish value in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// The C `.so` produced by CMake. `CMakeLists.txt` names the project after the
/// *parent directory* of `c_src`, so the file name is not fixed — glob for it.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// The Rust `cdylib`. Defaults to the release artifact (what a real consumer
/// links against, and the profile that has `panic = "abort"` plus optimisation),
/// falling back to debug. Override with `RUST_LIB_PATH`.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let name = "libmerge_sort_lib.so";
    let target = crate_root().join("target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust .so found under {}. Build it with: cargo build --release",
        target.display()
    );
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Both implementations, loaded and resolved.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: MergeSortFn,
    pub rust: MergeSortFn,
}

impl Pair {
    pub fn load() -> Self {
        let c_lib = load(&c_lib_path());
        let rust_lib = load(&rust_lib_path());
        let c: MergeSortFn = unsafe {
            let s: Symbol<MergeSortFn> = c_lib
                .get(b"merge_sort\0")
                .expect("C .so does not export `merge_sort`");
            *s
        };
        let rust: MergeSortFn = unsafe {
            let s: Symbol<MergeSortFn> = rust_lib
                .get(b"merge_sort\0")
                .expect("Rust .so does not export `merge_sort`");
            *s
        };
        Pair { _c_lib: c_lib, _rust_lib: rust_lib, c, rust }
    }
}

/// One shared `Pair` for the whole test binary (dlopen once).
pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(Pair::load)
}

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Raw bytes of a sprite slice, for byte-exact comparison (padding included).
pub fn bytes(v: &[Sprite]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

fn hexdump(v: &[Sprite]) -> String {
    let b = bytes(v);
    let mut s = String::new();
    for (i, chunk) in b.chunks(SPRITE_SIZE).enumerate() {
        s.push_str(&format!("  [{i:4}] "));
        for byte in chunk {
            s.push_str(&format!("{byte:02x}"));
        }
        s.push('\n');
        if i >= 63 {
            s.push_str("  ...\n");
            break;
        }
    }
    s
}

/// Result of running one implementation: the final contents of both buffers.
pub struct Outcome {
    pub a: Vec<Sprite>,
    pub b: Vec<Sprite>,
}

/// Run `f` on fresh copies of `a_in`/`b_in` with the given `size`.
pub fn run_one(f: MergeSortFn, a_in: &[Sprite], b_in: &[Sprite], size: i32) -> Outcome {
    let mut a = a_in.to_vec();
    let mut b = b_in.to_vec();
    unsafe { f(a.as_mut_ptr(), b.as_mut_ptr(), size) };
    Outcome { a, b }
}

/// Run `f` with `a` and `b` aliased to the SAME buffer (`ERRORS.md` #18).
pub fn run_one_aliased(f: MergeSortFn, a_in: &[Sprite], size: i32) -> Vec<Sprite> {
    let mut a = a_in.to_vec();
    let p = a.as_mut_ptr();
    unsafe { f(p, p, size) };
    a
}

/// Core assertion: C and Rust must agree byte-for-byte on BOTH buffers.
pub fn assert_same(ctx: &str, a_in: &[Sprite], b_in: &[Sprite], size: i32) {
    let c = run_one(pair().c, a_in, b_in, size);
    let r = run_one(pair().rust, a_in, b_in, size);
    if bytes(&c.a) != bytes(&r.a) {
        panic!(
            "{ctx}: buffer `a` diverged (size={size}, a_len={}, b_len={})\n\
             --- input a ---\n{}--- C a ---\n{}--- Rust a ---\n{}",
            a_in.len(),
            b_in.len(),
            hexdump(a_in),
            hexdump(&c.a),
            hexdump(&r.a)
        );
    }
    if bytes(&c.b) != bytes(&r.b) {
        panic!(
            "{ctx}: buffer `b` diverged (size={size}, a_len={}, b_len={})\n\
             --- input a ---\n{}--- input b ---\n{}--- C b ---\n{}--- Rust b ---\n{}",
            a_in.len(),
            b_in.len(),
            hexdump(a_in),
            hexdump(b_in),
            hexdump(&c.b),
            hexdump(&r.b)
        );
    }
}

pub fn assert_same_aliased(ctx: &str, a_in: &[Sprite], size: i32) {
    let c = run_one_aliased(pair().c, a_in, size);
    let r = run_one_aliased(pair().rust, a_in, size);
    assert!(
        bytes(&c) == bytes(&r),
        "{ctx}: aliased buffer diverged (size={size})\n\
         --- input ---\n{}--- C ---\n{}--- Rust ---\n{}",
        hexdump(a_in),
        hexdump(&c),
        hexdump(&r)
    );
}

// ---------------------------------------------------------------------------
// Input generators — the axes from CONFIGS.md
// ---------------------------------------------------------------------------

/// `sort_bits` patterns (axis K).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum K {
    Eq,
    Asc,
    Desc,
    Rand,
    Few,
    Alt,
    Neg,
    Ext,
    One,
    SortedDups,
}

/// `texture_id` patterns (axis T).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum T {
    Zero,
    Rand,
    Ext,
    Anti,
}

/// Padding patterns (axis P).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P {
    Zero,
    Garbage,
}

/// Scratch-buffer pre-fill (axis F).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum F {
    Zero,
    Sentinel,
}

pub const ALL_K: [K; 10] = [
    K::Eq,
    K::Asc,
    K::Desc,
    K::Rand,
    K::Few,
    K::Alt,
    K::Neg,
    K::Ext,
    K::One,
    K::SortedDups,
];
pub const ALL_T: [T; 4] = [T::Zero, T::Rand, T::Ext, T::Anti];
pub const ALL_P: [P; 2] = [P::Zero, P::Garbage];
pub const ALL_F: [F; 2] = [F::Zero, F::Sentinel];

/// Every `size` from `CONFIGS.md` axis S.
pub const ALL_SIZES: [i32; 22] = [
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 100, 255, 256, 257, 1000, 4096, 4097,
];

const INT_EXT: [i32; 4] = [i32::MIN, i32::MAX, 0, -1];
const TEX_EXT: [u64; 4] = [0, u64::MAX, 1, u64::MAX - 1];

pub fn gen_sort_bits(k: K, n: usize, rng: &mut Rng) -> Vec<i32> {
    let mut v = Vec::with_capacity(n);
    match k {
        K::Eq => {
            let c = rng.next_i32();
            v.resize(n, c);
        }
        K::Asc => {
            // Strictly ascending without overflowing i32.
            let start = i32::MIN / 2 + rng.below(1000) as i32;
            for i in 0..n {
                v.push(start.wrapping_add(i as i32 * 3));
            }
        }
        K::Desc => {
            let start = i32::MAX / 2 - rng.below(1000) as i32;
            for i in 0..n {
                v.push(start.wrapping_sub(i as i32 * 3));
            }
        }
        K::Rand => {
            for _ in 0..n {
                v.push(rng.next_i32());
            }
        }
        K::Few => {
            let alphabet_len = 2 + rng.below(3); // 2..=4
            let alphabet: Vec<i32> = (0..alphabet_len).map(|_| rng.next_i32()).collect();
            for _ in 0..n {
                v.push(alphabet[rng.below(alphabet_len)]);
            }
        }
        K::Alt => {
            let hi = rng.next_i32();
            let lo = rng.next_i32();
            for i in 0..n {
                v.push(if i % 2 == 0 { hi } else { lo });
            }
        }
        K::Neg => {
            for _ in 0..n {
                v.push(-1 - (rng.next_u32() >> 1) as i32);
            }
        }
        K::Ext => {
            for _ in 0..n {
                v.push(INT_EXT[rng.below(INT_EXT.len())]);
            }
        }
        K::One => {
            for i in 0..n {
                v.push(i as i32 * 2);
            }
            if n >= 2 {
                let from = rng.below(n);
                let to = rng.below(n);
                v.swap(from, to);
                // Also make one element wildly out of place.
                v[rng.below(n)] = rng.next_i32();
            }
        }
        K::SortedDups => {
            let mut cur: i32 = i32::MIN / 2;
            let mut i = 0;
            while i < n {
                let run = 1 + rng.below(4);
                for _ in 0..run {
                    if i >= n {
                        break;
                    }
                    v.push(cur);
                    i += 1;
                }
                cur = cur.wrapping_add(1 + rng.below(5) as i32);
            }
        }
    }
    debug_assert_eq!(v.len(), n);
    v
}

pub fn gen_texture_ids(t: T, sort_bits: &[i32], rng: &mut Rng) -> Vec<u64> {
    let n = sort_bits.len();
    let mut v = Vec::with_capacity(n);
    match t {
        T::Zero => v.resize(n, 0),
        T::Rand => {
            for _ in 0..n {
                v.push(rng.next_u64());
            }
        }
        T::Ext => {
            for _ in 0..n {
                v.push(TEX_EXT[rng.below(TEX_EXT.len())]);
            }
        }
        T::Anti => {
            // Descending texture_id. Where sort_bits tie (which is exactly where
            // the C's dead line-9 `texture_id` test would have mattered), the
            // output ordering must still ignore texture_id entirely.
            for i in 0..n {
                v.push(u64::MAX - i as u64);
            }
        }
    }
    debug_assert_eq!(v.len(), n);
    v
}

/// Build an `a` buffer from the K/T/P axes.
pub fn gen_input(k: K, t: T, p: P, n: usize, rng: &mut Rng) -> Vec<Sprite> {
    let bits = gen_sort_bits(k, n, rng);
    let texs = gen_texture_ids(t, &bits, rng);
    (0..n)
        .map(|i| Sprite {
            texture_id: texs[i],
            sort_bits: bits[i],
            pad: match p {
                P::Zero => 0,
                // Distinct non-zero padding per element, so a mis-propagated
                // padding word is immediately visible and traceable.
                P::Garbage => 0x8000_0000 | (i as u32).wrapping_mul(0x0101_0101) | 1,
            },
        })
        .collect()
}

/// Build a scratch buffer of `n` elements from the F axis.
pub fn gen_scratch(f: F, n: usize) -> Vec<Sprite> {
    let s = match f {
        F::Zero => Sprite { texture_id: 0, sort_bits: 0, pad: 0 },
        F::Sentinel => Sprite {
            texture_id: 0xAAAA_AAAA_AAAA_AAAA,
            sort_bits: 0xAAAA_AAAAu32 as i32,
            pad: 0xAAAA_AAAA,
        },
    };
    vec![s; n]
}

/// Number of randomized trials per (row, size) pair. Override with `TRIALS`.
pub fn trials() -> usize {
    std::env::var("TRIALS").ok().and_then(|s| s.parse().ok()).unwrap_or(12)
}

/// Base seed shared by all rows (fixed → reproducible).
/// Fixed by default so every run is reproducible; override with `SEED=<u64>` to
/// explore a different randomized sample of each configuration row.
#[allow(non_snake_case)]
pub fn SEED() -> u64 {
    std::env::var("SEED")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0x5EED_5F12_3ABC_DEF0)
}

/// Drive one `CONFIGS.md` row: for each `size`, run `trials()` randomized
/// inputs through both `.so`s and assert byte-identical `a` and `b`.
pub fn run_row(row: &str, sizes: &[i32], k: K, t: T, p: P, f: F) {
    let mut rng = Rng::new(SEED() ^ (row.len() as u64) << 32 ^ hash_str(row));
    for &size in sizes {
        let n = size.max(0) as usize;
        for trial in 0..trials() {
            let a = gen_input(k, t, p, n, &mut rng);
            let b = gen_scratch(f, n);
            let ctx = format!(
                "row {row} [size={size} trial={trial} K={k:?} T={t:?} P={p:?} F={f:?}]"
            );
            assert_same(&ctx, &a, &b, size);
        }
    }
}

pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}
