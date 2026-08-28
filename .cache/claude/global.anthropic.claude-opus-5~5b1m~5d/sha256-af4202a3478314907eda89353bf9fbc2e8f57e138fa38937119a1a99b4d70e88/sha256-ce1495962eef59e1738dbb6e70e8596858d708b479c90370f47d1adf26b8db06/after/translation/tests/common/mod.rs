//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` and calls `tfm` only through the
//! dynamic symbol, so the `#[no_mangle] extern "C"` export wrapper is what is
//! under test — never the Rust function called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The C ABI of the one and only entry point.
pub type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, i32);

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `translation/` — the crate root.
pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory that holds both `c_src/` and `translation/`.
pub fn work_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("crate root has a parent")
        .to_path_buf()
}

fn first_so_in(dir: &Path) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Path to the C `.so`.
///
/// `c_src/CMakeLists.txt` names the library after the *parent directory* of
/// `c_src`, so the basename is not fixed; glob for it instead of hard-coding.
/// Overridable with `TFM_C_SO`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TFM_C_SO") {
        return PathBuf::from(p);
    }
    let build = work_root().join("c_src").join("build");
    first_so_in(&build).unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path to the Rust `cdylib`, for the *same* cargo profile the test binary was
/// built with (so `cargo test` checks `target/debug/` and `cargo test --release`
/// checks `target/release/`). Overridable with `TFM_RUST_SO`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TFM_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe() == <target>/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test exe lives in <profile>/deps/")
        .to_path_buf();

    let direct = profile_dir.join("libtfm_lib.so");
    if direct.exists() {
        return direct;
    }
    panic!(
        "Rust cdylib not found at {}. Build it first: cargo build (or --release)",
        direct.display()
    );
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// A loaded implementation: the `Library` is leaked so the returned function
/// pointer stays valid for the whole process (and so `dlclose` never races with
/// a still-live symbol).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub tfm: TfmFn,
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let lib: &'static Library = Box::leak(Box::new(lib));
    let sym: Symbol<'static, TfmFn> = unsafe { lib.get(b"tfm\0") }
        .unwrap_or_else(|e| panic!("dlsym(tfm) in {} failed: {e}", path.display()));
    let tfm: TfmFn = *sym;
    Impl { name, path, tfm }
}

/// Both implementations, loaded once per test process.
pub struct Pair {
    pub c: &'static Impl,
    pub rs: &'static Impl,
}

pub fn pair() -> Pair {
    static C: OnceLock<Impl> = OnceLock::new();
    static RS: OnceLock<Impl> = OnceLock::new();
    Pair {
        c: C.get_or_init(|| load("C", c_so_path())),
        rs: RS.get_or_init(|| load("Rust", rust_so_path())),
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison helpers
// ---------------------------------------------------------------------------

/// Base of the sentinel used to pre-fill `dest`, so "was not written" is
/// distinguishable from "was written with some value". Index-dependent so a
/// misplaced write of the *right* value at the *wrong* offset is still caught.
pub const CANARY: u32 = 0xDEAD_0000;

pub fn canary_bits(i: usize) -> u32 {
    CANARY | ((i as u32).wrapping_mul(0x9E37) & 0xFFFF)
}

pub fn canary_buf(len: usize) -> Vec<f32> {
    (0..len).map(|i| f32::from_bits(canary_bits(i))).collect()
}

pub fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

pub fn fmt_f32(x: f32) -> String {
    format!("{:#010x}({})", x.to_bits(), x)
}

pub fn fmt_slice(v: &[f32]) -> String {
    let n = v.len().min(24);
    let mut s = v[..n]
        .iter()
        .map(|x| fmt_f32(*x))
        .collect::<Vec<_>>()
        .join(", ");
    if v.len() > n {
        s.push_str(&format!(", … (+{} more)", v.len() - n));
    }
    s
}

/// Assert two `f32` slices are bit-identical, reporting the first divergence.
pub fn assert_bits_eq(ctx: &str, c: &[f32], rs: &[f32]) {
    assert_eq!(c.len(), rs.len(), "{ctx}: length mismatch (harness bug)");
    for i in 0..c.len() {
        if c[i].to_bits() != rs[i].to_bits() {
            let lo = i.saturating_sub(2);
            let hi = (i + 3).min(c.len());
            panic!(
                "{ctx}\n  first divergence at output index {i}\n    \
                 C   = {}\n    Rust= {}\n  context C   [{lo}..{hi}] = {}\n  \
                 context Rust[{lo}..{hi}] = {}",
                fmt_f32(c[i]),
                fmt_f32(rs[i]),
                fmt_slice(&c[lo..hi]),
                fmt_slice(&rs[lo..hi]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The core differential driver
// ---------------------------------------------------------------------------

/// Run `tfm(dest, src, count)` on both impls with **disjoint** buffers and
/// compare `dest` bit-for-bit.
///
/// `dest_len` is the number of floats handed to each implementation; it is
/// pre-filled with canaries so unwritten tail elements are checked too.
pub fn diff_disjoint(ctx: &str, p: &Pair, src: &[f32], count: i32, dest_len: usize) {
    // 4 guard floats on each side of the `dest` window, pre-filled with
    // index-dependent canaries so stray writes are localised.
    const G: usize = 4;
    let total = dest_len + 2 * G;
    let mut gc = canary_buf(total);
    let mut gr = canary_buf(total);

    let sc = src.to_vec();
    let sr = src.to_vec();

    unsafe {
        (p.c.tfm)(gc.as_mut_ptr().add(G), sc.as_ptr(), count);
        (p.rs.tfm)(gr.as_mut_ptr().add(G), sr.as_ptr(), count);
    }

    assert_bits_eq(
        &format!(
            "{ctx} [count={count}, dest_len={dest_len}]\n  src = {}",
            fmt_slice(src)
        ),
        &gc,
        &gr,
    );

    // `src` is `const float *`; neither impl may modify it.
    assert_eq!(bits(&sc), bits(src), "{ctx}: C modified its src buffer");
    assert_eq!(bits(&sr), bits(src), "{ctx}: Rust modified its src buffer");

    // Guards outside the dest window must still hold their canaries, for both.
    for i in (0..G).chain(G + dest_len..total) {
        assert_eq!(
            gc[i].to_bits(),
            canary_bits(i),
            "{ctx}: C wrote outside dest at guard index {i}"
        );
        assert_eq!(
            gr[i].to_bits(),
            canary_bits(i),
            "{ctx}: Rust wrote outside dest at guard index {i}"
        );
    }
}

/// Convenience: exactly-sized `dest` (`2 * count` floats).
pub fn diff(ctx: &str, p: &Pair, src: &[f32], count: i32) {
    let dest_len = if count > 0 { 2 * count as usize } else { 2 };
    diff_disjoint(ctx, p, src, count, dest_len);
}

/// Differential run over a **single shared allocation**, so `src` and `dest`
/// alias exactly as the caller arranges. `buf` is the initial contents;
/// `src_off` / `dest_off` are float offsets into it.
pub fn diff_aliased(
    ctx: &str,
    p: &Pair,
    buf: &[f32],
    src_off: usize,
    dest_off: usize,
    count: i32,
) {
    let mut bc = buf.to_vec();
    let mut br = buf.to_vec();
    unsafe {
        (p.c.tfm)(bc.as_mut_ptr().add(dest_off), bc.as_ptr().add(src_off), count);
        (p.rs.tfm)(br.as_mut_ptr().add(dest_off), br.as_ptr().add(src_off), count);
    }
    assert_bits_eq(
        &format!(
            "{ctx} [aliased: src_off={src_off}, dest_off={dest_off}, count={count}]\n  \
             initial buf = {}",
            fmt_slice(buf)
        ),
        &bc,
        &br,
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external dev-dep, fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[-1, 1]`.
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }
    /// Any bit pattern reinterpreted as `f32` (includes inf/NaN/subnormals).
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A finite normal with exponent spread over the whole range.
    pub fn wild_normal(&mut self) -> f32 {
        let sign = self.next_u32() & 1;
        // biased exponent 1..=254 -> normal (never 0/subnormal, never inf/NaN)
        let exp = 1 + self.below(254);
        let mant = self.next_u32() & 0x007F_FFFF;
        f32::from_bits((sign << 31) | (exp << 23) | mant)
    }
    /// A subnormal (or signed zero) value.
    pub fn subnormal(&mut self) -> f32 {
        let sign = self.next_u32() & 1;
        let mant = self.next_u32() & 0x007F_FFFF;
        f32::from_bits((sign << 31) | mant)
    }
    /// A huge normal, `|x| ∈ [2^100, FLT_MAX]`, overflow-prone when squared.
    pub fn huge(&mut self) -> f32 {
        let sign = self.next_u32() & 1;
        let exp = 227 + self.below(254 - 227 + 1); // 227..=254
        let mant = self.next_u32() & 0x007F_FFFF;
        f32::from_bits((sign << 31) | (exp << 23) | mant)
    }
    /// A NaN with a random (possibly non-canonical, possibly signalling)
    /// payload and random sign.
    pub fn any_nan(&mut self) -> f32 {
        let sign = self.next_u32() & 1;
        let mut payload = self.next_u32() & 0x007F_FFFF;
        if payload == 0 {
            payload = 1; // payload 0 would be an infinity
        }
        f32::from_bits((sign << 31) | (0xFF << 23) | payload)
    }
}

// ---------------------------------------------------------------------------
// Special-value alphabet used by the exhaustive cross-product rows
// ---------------------------------------------------------------------------

/// 24 values covering every IEEE-754 binary32 class, both signs.
pub const ALPHABET: [u32; 24] = [
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // +MIN_SUBNORMAL
    0x8000_0001, // -MIN_SUBNORMAL
    0x007F_FFFF, // +MAX_SUBNORMAL
    0x0080_0000, // +FLT_MIN
    0x8080_0000, // -FLT_MIN
    0x3F00_0000, // +0.5
    0xBF00_0000, // -0.5
    0x3F80_0000, // +1.0
    0xBF80_0000, // -1.0
    0x4000_0000, // +2.0
    0xC000_0000, // -2.0
    0x7F7F_FFFF, // +FLT_MAX
    0xFF7F_FFFF, // -FLT_MAX
    0x7149_F2CA, // +1e30 (squares to +inf)
    0xF149_F2CA, // -1e30
    0x7F80_0000, // +inf
    0xFF80_0000, // -inf
    0x7FC0_0000, // +qNaN canonical
    0xFFC0_0000, // -qNaN canonical (x86 "indefinite")
    0x7FA0_0000, // +sNaN
    0xFFA0_0000, // -sNaN
    0x7F80_0001, // +NaN, minimal non-canonical signalling payload
];

pub fn alphabet_f32() -> Vec<f32> {
    ALPHABET.iter().copied().map(f32::from_bits).collect()
}

// ---------------------------------------------------------------------------
// `dest`/`src` at arbitrary float offsets inside one larger allocation
// ---------------------------------------------------------------------------

/// Differential run where `src` and `dest` live in *separate* allocations but at
/// caller-chosen float offsets (exercises non-16-byte-aligned pointers).
pub fn diff_offsets(
    ctx: &str,
    p: &Pair,
    src_data: &[f32],
    src_off: usize,
    dest_off: usize,
    count: i32,
) {
    let src_len = src_off + 3 * count.max(0) as usize + 4;
    let dest_len = dest_off + 2 * count.max(0) as usize + 4;

    let mut sc = canary_buf(src_len);
    sc[src_off..src_off + src_data.len()].copy_from_slice(src_data);
    let sr = sc.clone();
    let src_snapshot = sc.clone();

    let mut dc = canary_buf(dest_len);
    let mut dr = dc.clone();

    unsafe {
        (p.c.tfm)(dc.as_mut_ptr().add(dest_off), sc.as_ptr().add(src_off), count);
        (p.rs.tfm)(dr.as_mut_ptr().add(dest_off), sr.as_ptr().add(src_off), count);
    }

    let full = format!("{ctx} [src_off={src_off}, dest_off={dest_off}, count={count}]");
    assert_bits_eq(&full, &dc, &dr);
    assert_eq!(bits(&sc), bits(&src_snapshot), "{full}: C modified src");
    assert_eq!(bits(&sr), bits(&src_snapshot), "{full}: Rust modified src");
    // Nothing before dest_off, nothing after the written window.
    for i in (0..dest_off).chain(dest_off + 2 * count.max(0) as usize..dest_len) {
        assert_eq!(dc[i].to_bits(), canary_bits(i), "{full}: C stray write at {i}");
        assert_eq!(dr[i].to_bits(), canary_bits(i), "{full}: Rust stray write at {i}");
    }
}

// ---------------------------------------------------------------------------
// Mirror of the C discriminant, used ONLY to *search* for inputs that land in a
// given `sqd` regime. It is never used as an oracle: every assertion compares
// the C `.so` against the Rust `.so`.
// ---------------------------------------------------------------------------

/// `(dy2*dy2) - (2.0f*dx2*dy2) + (dx2*dx2) + (4.0f*dxy*dxy)` in C's evaluation
/// order (`c_src/src/lib.c` lines 12–13 / 22–23).
pub fn sqd_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    let acc = (dy2 * dy2 - (2.0f32 * dx2) * dy2) + dx2 * dx2;
    acc + (4.0f32 * dxy) * dxy
}

/// Apply the C branch selection of `c_src/src/lib.c` line 8 and return
/// `(dx2, dy2, dxy)` plus whether the `if` branch was taken.
pub fn roles(src: [f32; 3]) -> (f32, f32, f32, bool) {
    if src[0] < src[1] {
        (src[0], src[1], src[2], true)
    } else {
        (src[1], src[0], src[2], false)
    }
}

/// The `sqd` that `tfm` will compute for this triple.
pub fn sqd_for_triple(src: [f32; 3]) -> f32 {
    let (dx2, dy2, dxy, _) = roles(src);
    sqd_of(dx2, dy2, dxy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SqdRegime {
    PosNormal,
    PosZero,
    NegZero,
    Negative,
    PosInf,
    Nan,
}

pub fn classify_sqd(s: f32) -> SqdRegime {
    if s.is_nan() {
        SqdRegime::Nan
    } else if s == f32::INFINITY {
        SqdRegime::PosInf
    } else if s == 0.0 {
        if s.is_sign_negative() {
            SqdRegime::NegZero
        } else {
            SqdRegime::PosZero
        }
    } else if s < 0.0 {
        SqdRegime::Negative
    } else {
        SqdRegime::PosNormal
    }
}

// ---------------------------------------------------------------------------
// Reusable search for triples landing in a given `sqd` regime (shared by
// Phase B and Phase C). Used ONLY to construct inputs, never as an oracle.
// ---------------------------------------------------------------------------

/// Nearly-equal `f32` pairs that provoke catastrophic cancellation in
/// `dy2*dy2 - 2*dx2*dy2 + dx2*dx2`, whose rounded residual lands on both sides
/// of zero. `1 + p*2^-23` vs `1 + q*2^-23`: the residual is
/// `(rn(p²/2²³) + rn(q²/2²³) - 2*rn(pq/2²³)) * 2^-23`.
pub fn near_equal_pairs() -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for base in [0x3F80_0000u32, 0x4000_0000, 0x3F00_0000, 0x4180_0000] {
        for p in 1u32..400 {
            for dq in 1u32..5 {
                let a = f32::from_bits(base + p * 8);
                let b = f32::from_bits(base + p * 8 + dq * 7);
                out.push((a, b));
                out.push((b, a));
                out.push((-a, -b));
                out.push((-b, -a));
            }
        }
    }
    out
}

/// Search widely for triples in each `sqd` regime; returns up to `want` per
/// regime keyed by `SqdRegime`.
pub fn find_sqd_regimes(want: usize, seed: u64) -> std::collections::HashMap<SqdRegime, Vec<[f32; 3]>> {
    let mut map: std::collections::HashMap<SqdRegime, Vec<[f32; 3]>> = Default::default();
    {
        let push = |t: [f32; 3], m: &mut std::collections::HashMap<SqdRegime, Vec<[f32; 3]>>| {
            let e = m.entry(classify_sqd(sqd_for_triple(t))).or_default();
            if e.len() < want {
                e.push(t);
            }
        };

        let small_dxy = [
            0.0f32,
            -0.0f32,
            f32::from_bits(0x0000_0001),
            f32::from_bits(0x0000_00FF),
            f32::from_bits(0x0080_0000),
            1e-30,
            -1e-30,
        ];
        for (a, b) in near_equal_pairs() {
            for &d in &small_dxy {
                push([a, b, d], &mut map);
                push([b, a, d], &mut map);
            }
        }

        let alpha = alphabet_f32();
        for &x in &alpha {
            for &y in &alpha {
                for &z in &alpha {
                    push([x, y, z], &mut map);
                }
            }
        }

        let mut rng = Rng::new(seed);
        for _ in 0..400_000 {
            let t = match rng.below(5) {
                0 => [rng.signed_unit(), rng.signed_unit(), rng.signed_unit()],
                1 => [rng.wild_normal(), rng.wild_normal(), rng.wild_normal()],
                2 => [rng.huge(), rng.huge(), rng.huge()],
                3 => [rng.subnormal(), rng.subnormal(), rng.subnormal()],
                _ => [rng.any_bits_f32(), rng.any_bits_f32(), rng.any_bits_f32()],
            };
            push(t, &mut map);
            let a = rng.wild_normal();
            let bump = rng.below(64) as i32 - 32;
            let b = f32::from_bits((a.to_bits() as i32).wrapping_add(bump) as u32);
            push([a, b, rng.subnormal()], &mut map);
        }
    }
    map
}

/// Which C branch the *observed output* proves was taken, deduced from the fact
/// that `dxy` (= `src[2]`) is written **verbatim** to `dest[1]` in the `if`
/// branch and to `dest[0]` in the `else` branch (a plain load/store, not an FP
/// op, so the bits are preserved exactly — including sNaN payloads).
///
/// Returns `Some(true)` for the `if` branch, `Some(false)` for `else`, and
/// `None` when the two are indistinguishable (both slots hold `src[2]`'s bits).
pub fn observed_branch(src: [f32; 3], dest: [f32; 2]) -> Option<bool> {
    let d = src[2].to_bits();
    match (dest[1].to_bits() == d, dest[0].to_bits() == d) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

/// Run one triple through one impl and return the 2 outputs.
pub fn run_one(f: TfmFn, src: [f32; 3]) -> [f32; 2] {
    let mut dest = [f32::from_bits(canary_bits(0)), f32::from_bits(canary_bits(1))];
    unsafe { f(dest.as_mut_ptr(), src.as_ptr(), 1) };
    dest
}

/// Load an arbitrary `.so` that exports `tfm` (used to compare against C builds
/// at other optimization levels).
pub fn load_impl(name: &'static str, path: PathBuf) -> &'static Impl {
    Box::leak(Box::new(load(name, path)))
}

/// True when no lane is a NaN.
pub fn nan_free(t: &[f32; 3]) -> bool {
    t.iter().all(|x| !x.is_nan())
}
