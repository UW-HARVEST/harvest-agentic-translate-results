//! Shared differential-test harness.
//!
//! Both the C reference and the Rust translation are loaded as **shared
//! objects** via `libloading` and called through their `extern "C"` exports, so
//! the `#[no_mangle]` wrapper is part of what is under test. No Rust function
//! is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `void tfm(float *dest, const float *src, int count)`.
///
/// Declared over `u32` rather than `f32` so that no NaN can be laundered by an
/// incidental floating-point move on the test side. Pointer and `int` ABI are
/// identical.
pub type TfmFn = unsafe extern "C" fn(*mut u32, *const u32, i32);

pub struct Impls {
    c_lib: Library,
    rust_lib: Library,
}

impl Impls {
    pub fn c(&self) -> Symbol<'_, TfmFn> {
        unsafe { self.c_lib.get(b"tfm\0").expect("C .so does not export `tfm`") }
    }

    pub fn rust(&self) -> Symbol<'_, TfmFn> {
        unsafe {
            self.rust_lib
                .get(b"tfm\0")
                .expect("Rust .so does not export `tfm` (missing #[no_mangle]?)")
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn newest_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| {
            std::fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
        .cloned()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TFM_C_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("c_src").join("build");
    let candidates: Vec<PathBuf> = [
        "libtranslated_rust.so",
        "libtranslated_rust.dylib",
        "libc_src.so",
    ]
    .iter()
    .map(|n| base.join(n))
    .collect();
    newest_existing(&candidates).unwrap_or_else(|| {
        panic!(
            "C shared library not found under {}.\n\
             Build it first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            base.display()
        )
    })
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("TFM_RUST_SO") {
        return PathBuf::from(p);
    }
    // Locate the cdylib next to the test binary (target/<profile>/deps/<test>),
    // walking upwards. First match wins, so the profile the test was built with
    // is always preferred over a stale sibling profile.
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        let mut d: Option<&Path> = exe.parent();
        while let Some(dir) = d {
            for n in ["libtfm_lib.so", "libtfm_lib.dylib", "tfm_lib.dll"] {
                candidates.push(dir.join(n));
            }
            if dir.file_name().and_then(|s| s.to_str()) == Some("target") {
                break;
            }
            d = dir.parent();
        }
    }
    let target = manifest_dir().join("target");
    for profile in ["debug", "release"] {
        for n in ["libtfm_lib.so", "libtfm_lib.dylib", "tfm_lib.dll"] {
            candidates.push(target.join(profile).join(n));
            candidates.push(target.join(profile).join("deps").join(n));
        }
    }
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib not found. Build it first with the SAME feature set:\n  \
                 cargo build   (or ./verify.sh)"
            )
        })
}

/// Loads both shared objects exactly once per test binary.
pub fn impls() -> &'static Impls {
    static IMPLS: OnceLock<Impls> = OnceLock::new();
    IMPLS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        let c_lib = unsafe { Library::new(&cp) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", cp.display()));
        let rust_lib = unsafe { Library::new(&rp) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rp.display()));
        Impls { c_lib, rust_lib }
    })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed, no external dependency.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x2b7e_1516_28ae_d2a6;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A finite `f32` normal with an exponent spread over the whole range.
    pub fn normal_f32(&mut self) -> u32 {
        let sign = (self.next_u64() & 1) as u32;
        // Biased exponent 1..=254 (normals only).
        let exp = 1 + self.below(254) as u32;
        let frac = self.next_u32() & 0x007f_ffff;
        (sign << 31) | (exp << 23) | frac
    }
    /// A "tame" finite value in roughly `[-1e3, 1e3]`, so `sqd` stays finite.
    pub fn tame_f32(&mut self) -> u32 {
        let v = (self.unit() * 2.0 - 1.0) * 10f64.powi(self.below(7) as i32 - 3);
        (v as f32).to_bits()
    }
    /// Draws from the full special-value pool (§C of `CONFIGS.md`).
    pub fn pool_f32(&mut self) -> u32 {
        match self.below(16) {
            0 => SPECIALS[self.below(SPECIALS.len())],
            1 => self.next_u32(), // fully random bit pattern
            2 => self.normal_f32(),
            3 => {
                // subnormal
                let sign = (self.next_u64() & 1) as u32;
                let frac = (self.next_u32() & 0x007f_ffff).max(1);
                (sign << 31) | frac
            }
            4 => {
                // NaN with a random payload, random sign, random quiet bit
                let sign = (self.next_u64() & 1) as u32;
                let payload = (self.next_u32() & 0x007f_ffff).max(1);
                (sign << 31) | 0x7f80_0000 | payload
            }
            5 => {
                // huge: squaring overflows
                let sign = (self.next_u64() & 1) as u32;
                let exp = 200 + self.below(55) as u32;
                (sign << 31) | (exp << 23) | (self.next_u32() & 0x007f_ffff)
            }
            _ => self.tame_f32(),
        }
    }
}

impl Rng {
    /// A rich generator biased towards the interesting corners of the input
    /// space: specials, uniform bit patterns, the underflow boundary, nearly
    /// equal operand pairs (catastrophic cancellation), infinities/zeros, and
    /// the overflow boundary.
    pub fn candidate(&mut self) -> (u32, u32, u32) {
        match self.below(6) {
            0 => (self.pool_f32(), self.pool_f32(), self.pool_f32()),
            1 => (self.next_u32(), self.next_u32(), self.next_u32()),
            2 => {
                // near the f32 underflow boundary (sqd can go subnormal / negative)
                let e = 30 + self.below(50);
                let s = |r: &mut Rng| {
                    let v = (r.unit() * 2.0 - 1.0) * 2f64.powi(-(e as i32));
                    (v as f32).to_bits()
                };
                (s(self), s(self), s(self))
            }
            3 => {
                // src[0] and src[1] within a few ULPs -> catastrophic cancellation
                let base = self.normal_f32();
                let d = self.below(9) as i64 - 4;
                let j = (base as i64 + d) as u32;
                (base, j, self.pool_f32())
            }
            4 => {
                const V: [u32; 10] = [
                    0x7f80_0000, 0xff80_0000, 0x0000_0000, 0x8000_0000, 0x3f80_0000,
                    0xbf80_0000, 0x7f7f_ffff, 0xff7f_ffff, 0x5f00_0000, 0xdf00_0000,
                ];
                (
                    V[self.below(10)],
                    V[self.below(10)],
                    V[self.below(10)],
                )
            }
            _ => {
                // near the f32 overflow boundary (squares overflow to inf)
                let e = 60 + self.below(70);
                let s = |r: &mut Rng| {
                    let v = (r.unit() * 2.0 - 1.0) * 2f64.powi(e as i32);
                    (v as f32).to_bits()
                };
                (s(self), s(self), s(self))
            }
        }
    }
}

/// Searches the input space for triples satisfying `pred` (classified by the
/// value-level [`trace`] oracle) and runs a bit-exact C-vs-Rust differential on
/// every hit. Returns the number of hits.
///
/// Asserts that at least `min_hits` matching inputs were found, so a row can
/// never silently pass by never reaching its own condition.
pub fn diff_matching(
    ctx: &str,
    seed: u64,
    budget: usize,
    min_hits: usize,
    pred: impl Fn(&Trace) -> bool,
) -> usize {
    let mut rng = Rng::new(seed);
    let mut hits = 0usize;
    for i in 0..budget {
        let (a, b, c) = rng.candidate();
        if pred(&trace(a, b, c)) {
            diff(&format!("{ctx} hit#{hits} (iter {i})"), &[a, b, c], 1, 2);
            hits += 1;
        }
    }
    // Also sweep the deterministic specials cross-product for this condition.
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                if pred(&trace(a, b, c)) {
                    diff(&format!("{ctx} specials-hit#{hits}"), &[a, b, c], 1, 2);
                    hits += 1;
                }
            }
        }
    }
    assert!(
        hits >= min_hits,
        "{ctx}: only {hits} inputs satisfied the row's condition \
         (needed >= {min_hits}); the row would pass vacuously"
    );
    hits
}

/// Like [`diff_matching`] but asserts the condition is **unreachable** through
/// the public API, while still differentially checking the entire search space
/// (so C/Rust agreement is proven over a superset of the row).
pub fn assert_unreachable(
    ctx: &str,
    seed: u64,
    budget: usize,
    pred: impl Fn(&Trace) -> bool,
) {
    let mut rng = Rng::new(seed);
    let mut checked = 0usize;
    for _ in 0..budget {
        let (a, b, c) = rng.candidate();
        let t = trace(a, b, c);
        assert!(
            !pred(&t),
            "{ctx}: condition documented as unreachable was reached by \
             ({a:#010x}, {b:#010x}, {c:#010x}) -> {t:?}"
        );
        // Differentially check every candidate anyway.
        if checked < 20_000 {
            diff(&format!("{ctx} coverage#{checked}"), &[a, b, c], 1, 2);
            checked += 1;
        }
    }
    for &a in SPECIALS {
        for &b in SPECIALS {
            for &c in SPECIALS {
                let t = trace(a, b, c);
                assert!(
                    !pred(&t),
                    "{ctx}: condition documented as unreachable was reached by \
                     the specials triple ({a:#010x}, {b:#010x}, {c:#010x}) -> {t:?}"
                );
                diff(&format!("{ctx} specials-coverage"), &[a, b, c], 1, 2);
            }
        }
    }
}

/// Deterministic table of interesting `f32` bit patterns.
pub const SPECIALS: &[u32] = &[
    0x0000_0000, // +0.0
    0x8000_0000, // -0.0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007f_ffff, // largest positive subnormal
    0x807f_ffff, // largest negative subnormal
    0x0080_0000, // FLT_MIN
    0x8080_0000, // -FLT_MIN
    0x3f80_0000, // 1.0
    0xbf80_0000, // -1.0
    0x4000_0000, // 2.0
    0x3f00_0000, // 0.5
    0x4080_0000, // 4.0
    0x5f00_0000, // 9.22e18  (square overflows)
    0x7f7f_ffff, // FLT_MAX
    0xff7f_ffff, // -FLT_MAX
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // +qNaN (default)
    0xffc0_0000, // -qNaN (x86 indefinite)
    0x7fc0_1234, // +qNaN, payload
    0xffca_bcde, // -qNaN, payload
    0x7f80_0001, // +sNaN, minimal payload
    0xffbf_ffff, // -sNaN, maximal payload
];

// ---------------------------------------------------------------------------
// Differential driver
// ---------------------------------------------------------------------------

/// Guard bytes written around the output region so an over-write is detected.
const CANARY: u32 = 0xdead_beef;
const GUARD: usize = 4;

/// Runs `tfm` on both implementations with identical inputs and asserts the
/// entire destination buffer (plus guards) is bit-identical.
///
/// `src` is the exact input buffer; `count` is passed through verbatim.
pub fn diff(ctx: &str, src: &[u32], count: i32, out_elems: usize) {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    let total = out_elems + 2 * GUARD;
    let mut dc = vec![CANARY; total];
    let mut dr = vec![CANARY; total];

    let sc = src.to_vec();
    let sr = src.to_vec();

    unsafe {
        c(dc.as_mut_ptr().add(GUARD), sc.as_ptr(), count);
        r(dr.as_mut_ptr().add(GUARD), sr.as_ptr(), count);
    }

    if dc != dr {
        report_mismatch(ctx, src, count, &dc, &dr, GUARD);
    }
    assert_eq!(
        sc, sr,
        "{ctx}: the two implementations left the *input* buffer in different states"
    );
    for i in 0..GUARD {
        assert_eq!(
            dc[i], CANARY,
            "{ctx}: C wrote before the start of dest (guard {i})"
        );
        assert_eq!(
            dc[total - 1 - i], CANARY,
            "{ctx}: C wrote past the end of dest (guard {i})"
        );
    }
}

fn report_mismatch(
    ctx: &str,
    src: &[u32],
    count: i32,
    dc: &[u32],
    dr: &[u32],
    guard: usize,
) -> ! {
    let mut msg = format!("{ctx}: C/Rust divergence (count = {count})\n");
    msg.push_str("  input triples:\n");
    for (i, ch) in src.chunks(3).enumerate() {
        msg.push_str(&format!("    [{i}]"));
        for &b in ch {
            msg.push_str(&format!(" {:#010x}({:e})", b, f32::from_bits(b)));
        }
        msg.push('\n');
        if i >= 7 {
            msg.push_str("    ...\n");
            break;
        }
    }
    msg.push_str("  first differing output slots (index is relative to dest[0]):\n");
    let mut shown = 0;
    for i in 0..dc.len() {
        if dc[i] != dr[i] {
            msg.push_str(&format!(
                "    dest[{}]: C = {:#010x} ({:e})  Rust = {:#010x} ({:e})\n",
                i as isize - guard as isize,
                dc[i],
                f32::from_bits(dc[i]),
                dr[i],
                f32::from_bits(dr[i]),
            ));
            shown += 1;
            if shown >= 12 {
                msg.push_str("    ...\n");
                break;
            }
        }
    }
    panic!("{msg}");
}

/// Convenience: one element triple, `count = 1`.
pub fn diff1(ctx: &str, s0: u32, s1: u32, s2: u32) {
    diff(ctx, &[s0, s1, s2], 1, 2);
}

/// Number of randomized samples per `CONFIGS.md` row.
pub const SAMPLES: usize = 512;

/// Differential call with **null** `dest` and/or `src`. Only meaningful for
/// `count <= 0`, where the C never dereferences either pointer.
pub fn diff_null(ctx: &str, dest_null: bool, src_null: bool, count: i32) {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    let mut dc = vec![CANARY; 8];
    let mut dr = vec![CANARY; 8];
    let sc = vec![0x3f80_0000u32; 12];

    let dpc = if dest_null {
        std::ptr::null_mut()
    } else {
        dc.as_mut_ptr()
    };
    let dpr = if dest_null {
        std::ptr::null_mut()
    } else {
        dr.as_mut_ptr()
    };
    let sp = if src_null {
        std::ptr::null()
    } else {
        sc.as_ptr()
    };

    unsafe {
        c(dpc, sp, count);
        r(dpr, sp, count);
    }
    assert_eq!(dc, dr, "{ctx}: dest buffers diverged");
    assert!(
        dc.iter().all(|&x| x == CANARY),
        "{ctx}: C wrote to dest for count = {count}"
    );
    assert!(
        dr.iter().all(|&x| x == CANARY),
        "{ctx}: Rust wrote to dest for count = {count}"
    );
}

/// Differential call where `dest` and `src` point into the **same** buffer, at
/// the given element offsets (aliasing / overlap).
pub fn diff_alias(ctx: &str, buf: &[u32], count: i32, dest_off: usize, src_off: usize) {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    let mut bc = buf.to_vec();
    let mut br = buf.to_vec();

    unsafe {
        c(bc.as_mut_ptr().add(dest_off), bc.as_ptr().add(src_off), count);
        r(br.as_mut_ptr().add(dest_off), br.as_ptr().add(src_off), count);
    }

    if bc != br {
        let mut msg = format!(
            "{ctx}: aliasing divergence (count = {count}, dest_off = {dest_off}, \
             src_off = {src_off})\n  input: {:?}\n",
            buf.iter().map(|b| format!("{b:#010x}")).collect::<Vec<_>>()
        );
        for i in 0..bc.len() {
            if bc[i] != br[i] {
                msg.push_str(&format!(
                    "    buf[{i}]: C = {:#010x} ({:e})  Rust = {:#010x} ({:e})\n",
                    bc[i],
                    f32::from_bits(bc[i]),
                    br[i],
                    f32::from_bits(br[i]),
                ));
            }
        }
        panic!("{msg}");
    }
}

/// Differential call with `dest`/`src` deliberately misaligned by a byte offset.
pub fn diff_unaligned(
    ctx: &str,
    src: &[u32],
    count: i32,
    out_elems: usize,
    dest_byte_off: usize,
    src_byte_off: usize,
) {
    let im = impls();
    let c = im.c();
    let r = im.rust();

    let src_bytes = 4 * src.len();
    let dst_bytes = 4 * (out_elems + 2 * GUARD);

    let mut sbuf = vec![0u8; src_bytes + 8];
    for (i, &w) in src.iter().enumerate() {
        sbuf[src_byte_off + 4 * i..src_byte_off + 4 * i + 4].copy_from_slice(&w.to_le_bytes());
    }
    let mut dc = vec![0u8; dst_bytes + 8];
    let mut dr = vec![0u8; dst_bytes + 8];
    for chunk in dc.chunks_mut(4) {
        if chunk.len() == 4 {
            chunk.copy_from_slice(&CANARY.to_le_bytes());
        }
    }
    dr.copy_from_slice(&dc);

    unsafe {
        c(
            dc.as_mut_ptr().add(dest_byte_off + 4 * GUARD) as *mut u32,
            sbuf.as_ptr().add(src_byte_off) as *const u32,
            count,
        );
        r(
            dr.as_mut_ptr().add(dest_byte_off + 4 * GUARD) as *mut u32,
            sbuf.as_ptr().add(src_byte_off) as *const u32,
            count,
        );
    }
    assert_eq!(
        dc, dr,
        "{ctx}: unaligned divergence (dest_byte_off = {dest_byte_off}, \
         src_byte_off = {src_byte_off}, count = {count})"
    );
}

// ---------------------------------------------------------------------------
// Value-level oracle, used ONLY to *classify* generated inputs into the
// `CONFIGS.md` / `ERRORS.md` rows. It is never used as an expected result;
// every assertion compares the C `.so` against the Rust `.so`.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Trace {
    pub arm_if: bool,
    pub dx2: f32,
    pub dy2: f32,
    pub dxy: f32,
    pub dy2_sq: f32,
    pub two_dx2_dy2: f32,
    pub after_sub: f32,
    pub dx2_sq: f32,
    pub term4: f32,
    pub sqd: f32,
    pub clamped: f32,
    pub root: f32,
    pub sum: f32,
    pub lambda: f32,
}

/// Recomputes the C expression tree in the C's operand order (values only —
/// NaN payloads are irrelevant for classification).
pub fn trace(s0: u32, s1: u32, s2: u32) -> Trace {
    let a = f32::from_bits(s0);
    let b = f32::from_bits(s1);
    let arm_if = a < b; // ordered `<`: false when either is NaN
    let (dx2, dy2) = if arm_if { (a, b) } else { (b, a) };
    let dxy = f32::from_bits(s2);

    let dy2_sq = dy2 * dy2;
    let two_dx2_dy2 = (dx2 + dx2) * dy2;
    let after_sub = dy2_sq - two_dx2_dy2;
    let dx2_sq = dx2 * dx2;
    let acc = after_sub + dx2_sq;
    let term4 = (4.0f32 * dxy) * dxy;
    let sqd = term4 + acc;
    let clamped = if sqd < 0.0 { 0.0f32 } else { sqd };
    let root = if clamped.is_nan() {
        f32::NAN
    } else {
        clamped.sqrt()
    };
    let sum = (dy2 + dx2) + root;
    let lambda = 0.5f32 * sum;

    Trace {
        arm_if,
        dx2,
        dy2,
        dxy,
        dy2_sq,
        two_dx2_dy2,
        after_sub,
        dx2_sq,
        term4,
        sqd,
        clamped,
        root,
        sum,
        lambda,
    }
}
