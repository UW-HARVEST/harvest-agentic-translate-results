//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `merge_sort` only through the dynamic symbol, never directly.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Byte-level mirror of the C `spritebatch_sprite_t`.
///
/// ```c
/// typedef struct spritebatch_sprite_t {
///     unsigned long long texture_id;   // offset 0, 8 bytes
///     int sort_bits;                   // offset 8, 4 bytes
///     /* 4 bytes tail padding */
/// } spritebatch_sprite_t;
/// ```
///
/// Represented as raw bytes so that tests can control the tail padding
/// explicitly and compare all 16 bytes.
pub const SPRITE_SIZE: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Sprite(pub [u8; SPRITE_SIZE]);

impl Sprite {
    pub fn new(texture_id: u64, sort_bits: i32, padding: u8) -> Self {
        let mut b = [0u8; SPRITE_SIZE];
        b[0..8].copy_from_slice(&texture_id.to_ne_bytes());
        b[8..12].copy_from_slice(&sort_bits.to_ne_bytes());
        b[12..16].copy_from_slice(&[padding; 4]);
        Sprite(b)
    }
    pub fn texture_id(&self) -> u64 {
        u64::from_ne_bytes(self.0[0..8].try_into().unwrap())
    }
    pub fn sort_bits(&self) -> i32 {
        i32::from_ne_bytes(self.0[8..12].try_into().unwrap())
    }
}

impl std::fmt::Debug for Sprite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sprite {{ tex: {}, bits: {}, pad: {:02x?} }}",
            self.texture_id(),
            self.sort_bits(),
            &self.0[12..16]
        )
    }
}

/// Sanity check that our byte-level view agrees with the compiler's idea of the
/// C struct layout.
pub fn assert_layout() {
    assert_eq!(SPRITE_SIZE, 16, "sizeof(spritebatch_sprite_t)");
    assert_eq!(std::mem::size_of::<Sprite>(), 16);
    assert_eq!(std::mem::align_of::<Sprite>(), 8);
}

type MergeSortFn = unsafe extern "C" fn(*mut Sprite, *mut Sprite, i32);

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    merge_sort: MergeSortFn,
}

impl Impl {
    /// SAFETY: caller must uphold the C contract — `a` and `b` valid,
    /// non-overlapping, at least `size` elements each.
    pub unsafe fn merge_sort(&self, a: *mut Sprite, b: *mut Sprite, size: i32) {
        unsafe { (self.merge_sort)(a, b, size) }
    }
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    // Allow pointing at an alternative C build (e.g. an -O2 one) to confirm the
    // translation matches the C at other optimisation levels too.
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_C_SO does not exist: {}", p.display());
        return p;
    }
    let build = crate_root().join("../c_src/build");
    let entries = std::fs::read_dir(&build).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    });
    let mut found = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("so") {
            found = Some(p);
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn find_rust_so() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so walk up
    // to the profile dir and look for the cdylib there.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("profile dir")
        .to_path_buf();
    for name in ["libmerge_sort_lib.so", "merge_sort_lib.so"] {
        let p = profile_dir.join(name);
        if p.exists() {
            return p;
        }
    }
    // Fall back to the release build.
    for profile in ["release", "debug"] {
        let p = crate_root()
            .join("target")
            .join(profile)
            .join("libmerge_sort_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmerge_sort_lib.so not found near {}. Run `cargo build` first.",
        profile_dir.display()
    );
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    unsafe {
        let lib = Library::new(&path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let sym: Symbol<MergeSortFn> = lib
            .get(b"merge_sort\0")
            .unwrap_or_else(|e| panic!("dlsym merge_sort in {} failed: {e}", path.display()));
        let f = *sym;
        Impl {
            name,
            _lib: lib,
            merge_sort: f,
        }
    }
}

/// The pair under test. Both are loaded through `libloading`; the Rust one is
/// deliberately *not* called as a normal Rust function, so the `#[no_mangle]`
/// `extern "C"` wrapper is exercised too.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

pub fn load_pair() -> Pair {
    assert_layout();
    Pair {
        c: load("C", find_c_so()),
        rust: load("Rust", find_rust_so()),
    }
}

pub fn pair() -> &'static Pair {
    use std::sync::OnceLock;
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(load_pair)
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed so failures reproduce exactly.
// ---------------------------------------------------------------------------

pub const RNG_SEED: u64 = 0x5EED_1234_ABCD_F00D;

pub struct Rng(pub u64);

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
    /// Uniform-ish in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

// ---------------------------------------------------------------------------
// The differential core
// ---------------------------------------------------------------------------

/// A test case: the initial contents of `a` and of the scratch buffer `b`, plus
/// the `size` argument to pass (usually `a.len()`, but kept separate so
/// degenerate sizes can be tested).
#[derive(Clone)]
pub struct Case {
    pub a: Vec<Sprite>,
    pub b: Vec<Sprite>,
    pub size: i32,
    pub label: String,
}

impl Case {
    pub fn new(label: impl Into<String>, a: Vec<Sprite>, b: Vec<Sprite>, size: i32) -> Self {
        Case {
            a,
            b,
            size,
            label: label.into(),
        }
    }
    /// `size` = a.len(), `b` sized to match and filled with `b_fill` pattern.
    pub fn from_a(label: impl Into<String>, a: Vec<Sprite>, b_fill: u8) -> Self {
        let n = a.len();
        let b = vec![Sprite([b_fill; SPRITE_SIZE]); n];
        Case::new(label, a, b, n as i32)
    }
}

fn bytes_of(v: &[Sprite]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast::<u8>(), v.len() * SPRITE_SIZE) }
}

fn first_diff(x: &[u8], y: &[u8]) -> Option<usize> {
    x.iter().zip(y.iter()).position(|(p, q)| p != q)
}

/// Run one case through both implementations and assert byte-identical results
/// in **both** buffers.
#[track_caller]
pub fn assert_same(case: &Case) {
    let p = pair();

    let mut a_c = case.a.clone();
    let mut b_c = case.b.clone();
    let mut a_r = case.a.clone();
    let mut b_r = case.b.clone();

    unsafe {
        p.c.merge_sort(a_c.as_mut_ptr(), b_c.as_mut_ptr(), case.size);
        p.rust
            .merge_sort(a_r.as_mut_ptr(), b_r.as_mut_ptr(), case.size);
    }

    for (which, xc, xr) in [
        ("a (sorted output)", &a_c, &a_r),
        ("b (scratch buffer)", &b_c, &b_r),
    ] {
        let bc = bytes_of(xc);
        let br = bytes_of(xr);
        if let Some(off) = first_diff(bc, br) {
            let idx = off / SPRITE_SIZE;
            panic!(
                "DIVERGENCE in buffer {which}\n\
                 case: {} (size={}, len={})\n\
                 first differing byte: absolute offset {off} \
                 (element {idx}, byte {} within element)\n\
                 C    element[{idx}] = {:?}\n\
                 Rust element[{idx}] = {:?}\n\
                 C    bytes = {:02x?}\n\
                 Rust bytes = {:02x?}\n\
                 input element[{idx}] = {:?}",
                case.label,
                case.size,
                case.a.len(),
                off % SPRITE_SIZE,
                xc[idx],
                xr[idx],
                &xc[idx].0,
                &xr[idx].0,
                case.a.get(idx),
            );
        }
        assert_eq!(bc.len(), br.len(), "length mismatch in buffer {which}");
    }
}

/// Same as [`assert_same`] but the buffers are laid out adjacently in a single
/// allocation (`b` immediately after `a`) so that an off-by-one write past the
/// end of `a` lands in `b` and is caught.
#[track_caller]
pub fn assert_same_adjacent(label: &str, a: &[Sprite], size: i32, guard_fill: u8) {
    let p = pair();
    let n = a.len();
    // Layout: [ a (n) | b (n) | guard (4) ]
    let build = || {
        let mut v: Vec<Sprite> = Vec::with_capacity(2 * n + 4);
        v.extend_from_slice(a);
        v.extend(std::iter::repeat_n(Sprite([guard_fill; SPRITE_SIZE]), n + 4));
        v
    };
    let mut vc = build();
    let mut vr = build();

    unsafe {
        let (ac, bc) = vc.split_at_mut(n);
        p.c.merge_sort(ac.as_mut_ptr(), bc.as_mut_ptr(), size);
        let (ar, br) = vr.split_at_mut(n);
        p.rust.merge_sort(ar.as_mut_ptr(), br.as_mut_ptr(), size);
    }

    let bc = bytes_of(&vc);
    let br = bytes_of(&vr);
    if let Some(off) = first_diff(bc, br) {
        let idx = off / SPRITE_SIZE;
        let region = if idx < n {
            "a"
        } else if idx < 2 * n {
            "b"
        } else {
            "TRAILING GUARD (out-of-bounds write!)"
        };
        panic!(
            "DIVERGENCE (adjacent layout) in region {region}\n\
             case: {label} (size={size}, n={n})\n\
             offset {off}, element {idx}\n\
             C    = {:?}\nRust = {:?}",
            vc[idx], vr[idx]
        );
    }
}

// ---------------------------------------------------------------------------
// Data-shape generators (the axes from CONFIGS.md)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Axis B: already ascending by `sort_bits`.
    Ascending,
    /// Axis B: already descending by `sort_bits`.
    Descending,
    /// Axis B/D: every `sort_bits` identical, `texture_id` ascending.
    AllEqualTexAsc,
    /// Axis D: every `sort_bits` identical, `texture_id` DESCENDING — the input
    /// the dead tiebreak would have reordered.
    AllEqualTexDesc,
    /// Axis B: only `buckets` distinct `sort_bits` values → many ties.
    FewBuckets(u32),
    /// Axis C: fully random over the whole i32 range.
    Random,
    /// Axis C: all `sort_bits` strictly negative.
    AllNegative,
    /// Axis C: `sort_bits` from {i32::MIN, -1, 0, 1, i32::MAX}.
    BoundaryBits,
    /// Axis D: `texture_id` from {0,1,MAX/2,MAX-1,MAX}, `sort_bits` all equal.
    BoundaryTex,
}

impl Shape {
    pub fn name(&self) -> String {
        format!("{self:?}")
    }
    /// The six shapes used for the full cross-product sweep (row 25).
    pub fn sweep_set() -> [Shape; 6] {
        [
            Shape::Ascending,
            Shape::Descending,
            Shape::AllEqualTexAsc,
            Shape::FewBuckets(3),
            Shape::Random,
            Shape::BoundaryBits,
        ]
    }
}

/// Build `n` sprites of the given shape. `pad` controls the tail padding bytes
/// (axis G): `None` → zero, `Some(p)` → that byte pattern.
pub fn make_sprites(shape: Shape, n: usize, rng: &mut Rng, pad: Option<u8>) -> Vec<Sprite> {
    let padbyte = |rng: &mut Rng| match pad {
        None => 0u8,
        Some(0xFF) => rng.next_u8(), // 0xFF sentinel = random padding
        Some(p) => p,
    };
    let mut out = Vec::with_capacity(n);
    match shape {
        Shape::Ascending => {
            let mut cur: i32 = rng.next_i32() / 4;
            for i in 0..n {
                out.push(Sprite::new(rng.next_u64(), cur, padbyte(rng)));
                cur = cur.saturating_add((rng.below(3) + 1) as i32);
                let _ = i;
            }
        }
        Shape::Descending => {
            let mut cur: i32 = rng.next_i32() / 4;
            for _ in 0..n {
                out.push(Sprite::new(rng.next_u64(), cur, padbyte(rng)));
                cur = cur.saturating_sub((rng.below(3) + 1) as i32);
            }
        }
        Shape::AllEqualTexAsc => {
            let bits = rng.next_i32();
            for i in 0..n {
                out.push(Sprite::new(i as u64, bits, padbyte(rng)));
            }
        }
        Shape::AllEqualTexDesc => {
            let bits = rng.next_i32();
            for i in 0..n {
                out.push(Sprite::new((n - i) as u64, bits, padbyte(rng)));
            }
        }
        Shape::FewBuckets(k) => {
            let base = rng.next_i32() / 2;
            for _ in 0..n {
                let bits = base.wrapping_add(rng.below(k as usize) as i32);
                out.push(Sprite::new(rng.next_u64(), bits, padbyte(rng)));
            }
        }
        Shape::Random => {
            for _ in 0..n {
                out.push(Sprite::new(rng.next_u64(), rng.next_i32(), padbyte(rng)));
            }
        }
        Shape::AllNegative => {
            for _ in 0..n {
                let bits = -((rng.next_u32() % (i32::MAX as u32)) as i32) - 1;
                out.push(Sprite::new(rng.next_u64(), bits, padbyte(rng)));
            }
        }
        Shape::BoundaryBits => {
            const BITS: [i32; 5] = [i32::MIN, -1, 0, 1, i32::MAX];
            for _ in 0..n {
                let bits = BITS[rng.below(BITS.len())];
                out.push(Sprite::new(rng.next_u64(), bits, padbyte(rng)));
            }
        }
        Shape::BoundaryTex => {
            const TEX: [u64; 5] = [0, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX];
            let bits = rng.next_i32();
            for _ in 0..n {
                out.push(Sprite::new(TEX[rng.below(TEX.len())], bits, padbyte(rng)));
            }
        }
    }
    out
}
