//! Shared differential-test harness.
//!
//! Loads BOTH shared objects — the C reference (`c_src/build/lib*.so`) and the
//! Rust translation (`target/<profile>/libto_barycentric_lib.so`) — with
//! `libloading` and calls `to_barycentric` through the dynamic symbol in each.
//! The Rust implementation is never called directly, so the `#[no_mangle]`
//! `extern "C"` wrapper and the struct-passing ABI are exercised too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Mirror of `typedef struct lm_vec2 { float x, y; } lm_vec2;`
#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub const fn bits(x: u32, y: u32) -> Self {
        Vec2 {
            x: f32::from_bits(x),
            y: f32::from_bits(y),
        }
    }
    pub fn to_bits(self) -> (u32, u32) {
        (self.x.to_bits(), self.y.to_bits())
    }
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:e}|{:#010x}, {:e}|{:#010x})",
            self.x,
            self.x.to_bits(),
            self.y,
            self.y.to_bits()
        )
    }
}

pub type ToBarycentric = unsafe extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2;

pub struct Impls {
    pub c: ToBarycentric,
    pub rust: ToBarycentric,
    pub c_addr: usize,
    pub rust_addr: usize,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn find_so(dir: &Path, exact: Option<&str>) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hit = None;
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name()?.to_string_lossy().to_string();
        if !name.starts_with("lib") || !name.ends_with(".so") {
            continue;
        }
        match exact {
            Some(want) if name != want => continue,
            _ => {}
        }
        hit = Some(p);
        break;
    }
    hit
}

/// `<crate>/../c_src/build/lib<project>.so` — the project name is derived from
/// the parent directory name by `CMakeLists.txt`, so it is globbed rather than
/// hard-coded.
fn c_so_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has a parent directory")
        .to_path_buf();
    let build = root.join("c_src").join("build");
    find_so(&build, None).unwrap_or_else(|| {
        panic!(
            "C shared library not found in {}.\nBuild it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// `target/<profile>/libto_barycentric_lib.so`, derived from the location of the
/// running test binary (`target/<profile>/deps/<test>-<hash>`) so that it always
/// matches the profile and feature set under test.
///
/// `cargo test` does not necessarily build the `cdylib` artifact (it is not a
/// dependency of the integration-test binaries, which reach the library through
/// `dlopen` rather than by linking), so if it is missing this shells out to
/// `cargo build` for the same profile once.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<bin>")
        .to_path_buf();
    let want = "libto_barycentric_lib.so";

    if let Some(p) = find_so(&profile_dir, Some(want)) {
        return p;
    }

    let release = profile_dir.file_name().map(|n| n == "release").unwrap_or(false);
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = std::process::Command::new(cargo);
    cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build");
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status();
    if let Some(p) = find_so(&profile_dir, Some(want)) {
        return p;
    }
    panic!(
        "Rust cdylib {} not found in {} (auto-`cargo build` returned {:?}).\n\
         Build it explicitly with: cd translation && cargo build{}",
        want,
        profile_dir.display(),
        status,
        if release { " --release" } else { "" }
    )
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        // The libraries are intentionally leaked: the function pointers taken
        // out of them must stay valid for the whole process lifetime.
        let c_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&c_path).expect("dlopen C .so")
        }));
        let rust_lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&rust_path).expect("dlopen Rust .so")
        }));

        let c_sym: libloading::Symbol<'static, ToBarycentric> = unsafe {
            c_lib
                .get(b"to_barycentric\0")
                .expect("C .so exports to_barycentric")
        };
        let rust_sym: libloading::Symbol<'static, ToBarycentric> = unsafe {
            rust_lib
                .get(b"to_barycentric\0")
                .expect("Rust .so exports to_barycentric")
        };

        let c = *c_sym;
        let rust = *rust_sym;
        Impls {
            c,
            rust,
            c_addr: c as usize,
            rust_addr: rust as usize,
            c_path,
            rust_path,
        }
    })
}

pub fn call_c(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    unsafe { (impls().c)(p1, p2, p3, p) }
}

pub fn call_rust(p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Vec2 {
    unsafe { (impls().rust)(p1, p2, p3, p) }
}

/// One differential comparison. Returns `Err(report)` on any bit difference so
/// callers can accumulate several failures before panicking.
pub fn diff(row: &str, p1: Vec2, p2: Vec2, p3: Vec2, p: Vec2) -> Result<Vec2, String> {
    let c = call_c(p1, p2, p3, p);
    let r = call_rust(p1, p2, p3, p);
    if c.to_bits() == r.to_bits() {
        Ok(c)
    } else {
        Err(format!(
            "[{row}] DIVERGENCE\n  p1={p1:?}\n  p2={p2:?}\n  p3={p3:?}\n  p ={p:?}\n  \
             C   = {c:?}\n  Rust= {r:?}\n  x bits: C {:#010x} vs Rust {:#010x}\n  \
             y bits: C {:#010x} vs Rust {:#010x}",
            c.x.to_bits(),
            r.x.to_bits(),
            c.y.to_bits(),
            r.y.to_bits()
        ))
    }
}

/// Runs a row: `n` randomized cases from `gen`, reporting at most the first few
/// divergences plus a total count.
pub fn run_row<F>(row: &str, n: usize, mut gen: F)
where
    F: FnMut(&mut Rng) -> [Vec2; 4],
{
    let mut rng = Rng::for_row(row);
    let mut failures = 0usize;
    let mut first: Vec<String> = Vec::new();
    for _ in 0..n {
        let [p1, p2, p3, p] = gen(&mut rng);
        if let Err(e) = diff(row, p1, p2, p3, p) {
            failures += 1;
            if first.len() < 3 {
                first.push(e);
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "row {row}: {failures}/{n} randomized cases diverged\n{}",
        first.join("\n---\n")
    );
}

/// Runs a row over an explicit, exhaustive list of cases.
pub fn run_cases(row: &str, cases: &[[Vec2; 4]]) {
    let mut failures = 0usize;
    let mut first: Vec<String> = Vec::new();
    for &[p1, p2, p3, p] in cases {
        if let Err(e) = diff(row, p1, p2, p3, p) {
            failures += 1;
            if first.len() < 3 {
                first.push(e);
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "row {row}: {failures}/{} explicit cases diverged\n{}",
        cases.len(),
        first.join("\n---\n")
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_DEAD_BEEF;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    /// Seed = global SEED mixed with a per-row salt derived from the row name,
    /// so each row gets its own reproducible stream.
    pub fn for_row(row: &str) -> Self {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for b in row.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        Rng(SEED ^ h)
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
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Uniform in `[-1, 1)` with 24 bits of mantissa entropy.
    pub fn unit(&mut self) -> f32 {
        let m = (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32;
        if self.bool() {
            m
        } else {
            -m
        }
    }

    /// Uniform in `[0, 1)`.
    pub fn unit01(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// `unit()` scaled by `scale`.
    pub fn scaled(&mut self, scale: f32) -> f32 {
        self.unit() * scale
    }

    /// Fully random 32-bit pattern reinterpreted as `f32` (any float class).
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// Small integer in `[-lim, lim]`, exactly representable in binary32.
    pub fn small_int(&mut self, lim: i32) -> f32 {
        let span = (2 * lim + 1) as u32;
        (self.below(span) as i32 - lim) as f32
    }

    /// `±2^k` for `k` in `[lo, hi]`.
    pub fn pow2(&mut self, lo: i32, hi: i32) -> f32 {
        let k = lo + self.below((hi - lo + 1) as u32) as i32;
        let v = (2.0f64).powi(k) as f32;
        if self.bool() {
            v
        } else {
            -v
        }
    }

    pub fn vec_with(&mut self, mut f: impl FnMut(&mut Rng) -> f32) -> Vec2 {
        let x = f(self);
        let y = f(self);
        Vec2 { x, y }
    }
}

// ---------------------------------------------------------------------------
// Interesting float constants.
// ---------------------------------------------------------------------------

pub const QNAN: f32 = f32::from_bits(0x7fc0_0000);
pub const QNAN_NEG: f32 = f32::from_bits(0xffc0_0000);
pub const SNAN: f32 = f32::from_bits(0x7f80_0001);
pub const SNAN_NEG: f32 = f32::from_bits(0xff80_0001);
pub const NAN_A: f32 = f32::from_bits(0x7fc0_00aa);
pub const NAN_B: f32 = f32::from_bits(0x7fc0_00bb);
pub const NAN_C: f32 = f32::from_bits(0xffc0_00cc);
pub const NAN_D: f32 = f32::from_bits(0x7fd0_00dd);
pub const SUBNORMAL_MIN: f32 = f32::from_bits(0x0000_0001);
pub const SUBNORMAL_MAX: f32 = f32::from_bits(0x007f_ffff);

/// Every "interesting" scalar, used for saturating small exhaustive sweeps.
pub const SPECIALS: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    0.5,
    -0.5,
    3.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    SUBNORMAL_MIN,
    -SUBNORMAL_MIN,
    SUBNORMAL_MAX,
    -SUBNORMAL_MAX,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    QNAN,
    QNAN_NEG,
    SNAN,
    SNAN_NEG,
    NAN_A,
    NAN_B,
    NAN_C,
    NAN_D,
];

/// Replaces component `slot` (0..8, ordered p1.x, p1.y, p2.x, … p.y) of the
/// four vectors with `v`.
pub fn set_slot(vs: &mut [Vec2; 4], slot: usize, v: f32) {
    let (i, comp) = (slot / 2, slot % 2);
    if comp == 0 {
        vs[i].x = v;
    } else {
        vs[i].y = v;
    }
}

// ---------------------------------------------------------------------------
// ABI trampoline: calls `to_barycentric` with the upper 64 bits of xmm0..xmm3
// filled with garbage, to prove that neither implementation reads beyond the
// 8-byte `lm_vec2` payload the SysV ABI defines.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".text",
    ".p2align 4",
    ".globl diffharness_dirty_call",
    ".hidden diffharness_dirty_call",
    ".type diffharness_dirty_call,@function",
    "diffharness_dirty_call:",
    // rdi = target fn, rsi = ptr to 4 x 16 bytes of argument data, rdx = out ptr
    "push rbx",
    "mov rbx, rdx",
    "movups xmm0, [rsi]",
    "movups xmm1, [rsi + 16]",
    "movups xmm2, [rsi + 32]",
    "movups xmm3, [rsi + 48]",
    "call rdi",
    "movups [rbx], xmm0",
    "pop rbx",
    "ret",
    ".size diffharness_dirty_call,.-diffharness_dirty_call",
);

#[cfg(target_arch = "x86_64")]
extern "C" {
    fn diffharness_dirty_call(f: usize, args: *const u32, out: *mut u32);
}

/// Calls `f` with dirty XMM upper halves; returns the low 8 bytes of xmm0 as a
/// `Vec2` plus the garbage the callee left in the upper half (ignored).
#[cfg(target_arch = "x86_64")]
pub fn call_dirty(f: usize, vs: [Vec2; 4], garbage: [u32; 2]) -> Vec2 {
    let mut args = [0u32; 16];
    for (i, v) in vs.iter().enumerate() {
        args[i * 4] = v.x.to_bits();
        args[i * 4 + 1] = v.y.to_bits();
        args[i * 4 + 2] = garbage[0];
        args[i * 4 + 3] = garbage[1];
    }
    let mut out = [0u32; 4];
    // SAFETY: `diffharness_dirty_call` reads exactly 64 bytes from `args`,
    // writes exactly 16 bytes to `out`, and calls `f`, which is a
    // `to_barycentric` pointer obtained from a loaded `.so`.
    unsafe { diffharness_dirty_call(f, args.as_ptr(), out.as_mut_ptr()) };
    Vec2::bits(out[0], out[1])
}
