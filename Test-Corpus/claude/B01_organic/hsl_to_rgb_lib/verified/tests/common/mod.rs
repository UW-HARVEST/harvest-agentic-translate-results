//! Shared differential-test harness.
//!
//! Both the C reference (`c_src/build/libtranslated_rust.so`) and the Rust
//! translation (`libhsl_to_rgb_lib.so`) are loaded with `libloading` and driven
//! exclusively through their exported `hsl_to_rgb` symbol, so the `#[no_mangle]`
//! `extern "C"` wrapper is part of what gets tested. The Rust crate is never
//! linked into the test binary and no Rust function is ever called directly.
//!
//! The Rust `cdylib` is exercised in **both** the `dev` and the `release`
//! profile, because optimisation level is the only remaining build-time axis
//! that could perturb floating point codegen (`Cargo.toml` declares no
//! `[features]`, so there is exactly one feature combination).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// ABI of the single exported symbol: `void hsl_to_rgb(float *dest, const float *src);`
pub type HslFn = unsafe extern "C" fn(*mut f32, *const f32);

// ---------------------------------------------------------------------------
// Locating / building the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cargo_bin() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// Build `c_src` with CMake (exactly the command from the task description) and
/// return the path to the resulting `.so`.
fn c_so_path() -> PathBuf {
    let root = manifest_dir();
    let build = root.join("c_src/build");
    let so = build.join("libtranslated_rust.so");
    if so.exists() {
        return so;
    }
    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
        .status()
        .expect("run cmake");
    assert!(st.success(), "cmake configure failed");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake --build");
    assert!(st.success(), "cmake build failed");
    assert!(so.exists(), "expected {} after building c_src", so.display());
    so
}

/// Build the Rust `cdylib` for `profile` into a private target directory (so the
/// nested `cargo` invocation cannot deadlock against the `cargo test` that is
/// running us) and return the path to the resulting `.so`.
fn rust_so_path(profile: &str) -> PathBuf {
    let root = manifest_dir();
    let target_dir = root.join("target").join(format!("ffi-{profile}"));
    let so = target_dir
        .join(if profile == "dev" { "debug" } else { profile })
        .join("libhsl_to_rgb_lib.so");

    let mut cmd = Command::new(cargo_bin());
    cmd.current_dir(&root)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        // The C reference is compiled by CMake with no build type, i.e. plain
        // `-O0` with no instrumentation. Rust's `dev` profile additionally turns
        // on `debug-assertions`, which injects `assert_unsafe_precondition!`
        // alignment/null checks into every raw-pointer dereference and makes the
        // library *abort* on the two undefined-behaviour rows of ERRORS.md
        // (null pointers, mis-aligned buffers) instead of doing what the
        // hardware — and therefore the C — does. Turning them off is what makes
        // the unoptimized Rust build an apples-to-apples counterpart of the
        // unoptimized C build. Optimisation level is still the axis under test.
        .env("RUSTFLAGS", "-Cdebug-assertions=off")
        .args(["build", "--no-default-features", "--profile", profile])
        .args(["--target-dir", target_dir.to_str().unwrap()]);
    let out = cmd.output().expect("run cargo build for the cdylib");
    assert!(
        out.status.success(),
        "cargo build --profile {profile} failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(so.exists(), "expected {} after cargo build", so.display());
    so
}

fn load(path: &Path) -> (libloading::Library, HslFn) {
    // SAFETY: both libraries are plain leaf libraries with no initialisers that
    // run arbitrary code, and the signature below matches `lib.h` exactly.
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let sym: libloading::Symbol<HslFn> = lib
            .get(b"hsl_to_rgb\0")
            .unwrap_or_else(|e| panic!("dlsym hsl_to_rgb in {}: {e}", path.display()));
        let f: HslFn = *sym;
        (lib, f)
    }
}

pub struct Harness {
    pub c: HslFn,
    /// `(label, function)` for every Rust build variant under test.
    pub rust: Vec<(&'static str, HslFn)>,
    pub c_so: PathBuf,
    pub rust_so: PathBuf,
}

static HARNESS: OnceLock<Harness> = OnceLock::new();

pub fn harness() -> &'static Harness {
    HARNESS.get_or_init(|| {
        let c_so = c_so_path();
        let (c_lib, c) = load(&c_so);
        std::mem::forget(c_lib); // keep the mapping alive for the whole process

        let mut rust = Vec::new();
        let mut release_so = None;
        for profile in ["dev", "release"] {
            let p = rust_so_path(profile);
            let (lib, f) = load(&p);
            std::mem::forget(lib);
            rust.push((if profile == "dev" { "rust-debug" } else { "rust-release" }, f));
            if profile == "release" {
                release_so = Some(p);
            }
        }

        Harness { c, rust, c_so, rust_so: release_so.unwrap() }
    })
}

// ---------------------------------------------------------------------------
// Calling convention used by every test
// ---------------------------------------------------------------------------

/// Sentinel written into `dest` before the call: if a library fails to write a
/// component, the sentinel shows up in the comparison instead of a stale value.
pub const UNWRITTEN: u32 = 0xCAFE_BABE;
/// Guard words placed immediately before and after both buffers.
pub const GUARD_LO: u32 = 0xDEAD_BEEF;
pub const GUARD_HI: u32 = 0xFEED_FACE;

/// Everything observable about one invocation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Obs {
    /// `dest[0..3]` as raw bit patterns.
    pub rgb: [u32; 3],
    /// `dest[-1]`, `dest[3]` — must be untouched.
    pub dest_guards: [u32; 2],
    /// `src[-1]`, `src[0..3]`, `src[3]` after the call.
    pub src_after: [u32; 5],
}

/// Invoke `f` on freshly guarded, disjoint buffers holding the raw bit patterns
/// `src_bits = [h, s, l]`.
pub fn call(f: HslFn, src_bits: [u32; 3]) -> Obs {
    let mut dest: [u32; 5] = [GUARD_LO, UNWRITTEN, UNWRITTEN, UNWRITTEN, GUARD_HI];
    // `src` is declared `mut` and handed over through `as_mut_ptr()` so that the
    // "did the callee mutate its const input?" check below is a real observation
    // and not something the optimiser is free to constant-fold away.
    let mut src: [u32; 5] = [GUARD_LO, src_bits[0], src_bits[1], src_bits[2], GUARD_HI];
    // SAFETY: `dest[1..4]` and `src[1..4]` are three consecutive, 4-byte
    // aligned `f32`-sized slots, exactly what `hsl_to_rgb` requires.
    unsafe {
        f(
            dest.as_mut_ptr().add(1).cast::<f32>(),
            src.as_mut_ptr().add(1).cast::<f32>(),
        );
    }
    Obs {
        rgb: [dest[1], dest[2], dest[3]],
        dest_guards: [dest[0], dest[4]],
        src_after: src,
    }
}

/// Invoke `f` with `dest` and `src` pointing into the same buffer, `dest`
/// displaced by `dest_off` `f32` slots relative to `src`.
///
/// The buffer is 9 slots wide; `src` starts at slot 3, so offsets in
/// `-3..=3` stay in bounds. Returns the whole buffer plus its guards.
pub fn call_overlapping(f: HslFn, src_bits: [u32; 3], dest_off: isize) -> [u32; 11] {
    let mut buf: [u32; 11] = [UNWRITTEN; 11];
    buf[0] = GUARD_LO;
    buf[10] = GUARD_HI;
    // src occupies slots 4,5,6 (indices into `buf`)
    buf[4] = src_bits[0];
    buf[5] = src_bits[1];
    buf[6] = src_bits[2];
    // SAFETY: `dest_off` is constrained to -3..=3 by the callers, so both
    // three-slot windows stay inside `buf[1..10]`.
    unsafe {
        let src = buf.as_ptr().add(4).cast::<f32>();
        let dest = buf.as_mut_ptr().add(4).offset(dest_off).cast::<f32>();
        f(dest, src);
    }
    buf
}

/// Invoke `f` on buffers deliberately mis-aligned by `byte_off` bytes.
pub fn call_misaligned(f: HslFn, src_bits: [u32; 3], byte_off: usize) -> [u32; 3] {
    assert!(byte_off < 4);
    let mut sbuf = [0u8; 16];
    let mut dbuf = [0u8; 16];
    for (i, w) in src_bits.iter().enumerate() {
        sbuf[byte_off + 4 * i..byte_off + 4 * i + 4].copy_from_slice(&w.to_le_bytes());
    }
    // SAFETY: `byte_off + 12 <= 16`, so the three unaligned `f32` slots are in
    // bounds. Both libraries access them with plain `movss`, which permits
    // arbitrary alignment.
    unsafe {
        f(
            dbuf.as_mut_ptr().add(byte_off).cast::<f32>(),
            sbuf.as_ptr().add(byte_off).cast::<f32>(),
        );
    }
    let mut out = [0u32; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let mut w = [0u8; 4];
        w.copy_from_slice(&dbuf[byte_off + 4 * i..byte_off + 4 * i + 4]);
        *o = u32::from_le_bytes(w);
    }
    out
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn fmt(bits: u32) -> String {
    format!("{:#010x} ({:e})", bits, f32::from_bits(bits))
}

fn fmt3(b: [u32; 3]) -> String {
    format!("[{}, {}, {}]", fmt(b[0]), fmt(b[1]), fmt(b[2]))
}

/// Core differential assertion: run C and every Rust variant on `src_bits` and
/// require bit-identical output, untouched guard words and an untouched `src`.
pub fn assert_same(ctx: &str, src_bits: [u32; 3]) {
    let h = harness();
    let c = call(h.c, src_bits);

    assert_eq!(
        c.dest_guards,
        [GUARD_LO, GUARD_HI],
        "{ctx}: the C library itself wrote outside dest[0..3] for src={}",
        fmt3(src_bits)
    );

    for (label, f) in &h.rust {
        let r = call(*f, src_bits);
        assert_eq!(
            c.rgb,
            r.rgb,
            "{ctx}: output mismatch for src(h,s,l)={}\n  C   -> {}\n  {label} -> {}",
            fmt3(src_bits),
            fmt3(c.rgb),
            fmt3(r.rgb)
        );
        assert_eq!(
            r.dest_guards,
            [GUARD_LO, GUARD_HI],
            "{ctx}: {label} wrote outside dest[0..3] for src={}",
            fmt3(src_bits)
        );
        assert_eq!(
            r.src_after, c.src_after,
            "{ctx}: {label} mutated the source buffer differently from C for src={}",
            fmt3(src_bits)
        );
        assert_eq!(
            c.src_after,
            [GUARD_LO, src_bits[0], src_bits[1], src_bits[2], GUARD_HI],
            "{ctx}: the C library mutated its const source buffer for src={}",
            fmt3(src_bits)
        );
    }
}

/// `assert_same` over an iterator of inputs.
pub fn assert_same_all<I: IntoIterator<Item = [u32; 3]>>(ctx: &str, inputs: I) {
    let mut n = 0usize;
    for s in inputs {
        assert_same(ctx, s);
        n += 1;
    }
    assert!(n > 0, "{ctx}: no inputs were generated");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + float generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// Fixed seed => reproducible failures.
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
    /// Uniform in `[0, 1)` with 24 bits of entropy.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
    /// A float built from a random exponent and mantissa: covers subnormals,
    /// tiny and huge magnitudes far better than a uniform draw.
    pub fn log_uniform(&mut self, negative_allowed: bool) -> f32 {
        let w = self.next_u32();
        let exp = (w >> 24) & 0xFE; // 0..=254, never 0xFF (no inf/NaN)
        let mant = self.next_u32() & 0x007F_FFFF;
        let sign = if negative_allowed && (w & 1) != 0 { 0x8000_0000 } else { 0 };
        f32::from_bits(sign | (exp << 23) | mant)
    }
    /// A raw, completely unconstrained `f32` bit pattern.
    pub fn raw(&mut self) -> u32 {
        self.next_u32()
    }
    /// A random NaN: both signs, quiet and signalling, random non-zero payload.
    pub fn nan(&mut self) -> u32 {
        let w = self.next_u32();
        let sign = w & 0x8000_0000;
        let quiet = (w & 1) != 0;
        let mut payload = self.next_u32() & 0x003F_FFFF;
        if payload == 0 && !quiet {
            payload = 1; // a signalling NaN needs a non-zero payload
        }
        sign | 0x7F80_0000 | if quiet { 0x0040_0000 } else { 0 } | payload
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

/// The six sector guards of the C `if`/`else if` chain, plus the two
/// out-of-range classes. `(label, lo, hi)`, half-open.
pub const SECTORS: [(&str, f32, f32); 6] = [
    ("h[0,60)", 0.0, 60.0),
    ("h[60,120)", 60.0, 120.0),
    ("h[120,180)", 120.0, 180.0),
    ("h[180,240)", 180.0, 240.0),
    ("h[240,300)", 240.0, 300.0),
    ("h[300,360)", 300.0, 360.0),
];

pub const BOUNDARIES: [f32; 7] = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];

/// A pool of every interesting `f32` value the C code can distinguish.
pub fn interesting_floats() -> Vec<u32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),  // smallest positive subnormal
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF), // largest subnormal
        30.0,
        90.0,
        150.0,
        210.0,
        270.0,
        330.0,
        400.0,
        720.0,
        -30.0,
        -400.0,
        1e-30,
        1e30,
        -1e30,
    ];
    for b in BOUNDARIES {
        v.push(b);
        v.push(next_after(b, f32::NEG_INFINITY));
        v.push(next_after(b, f32::INFINITY));
    }
    let mut bits: Vec<u32> = v.iter().map(|f| f.to_bits()).collect();
    // NaNs: quiet/signalling, both signs, min/max payload.
    bits.extend_from_slice(&[
        0x7FC0_0000, 0xFFC0_0000, 0x7FC0_0001, 0xFFC0_0001, 0x7F80_0001, 0xFF80_0001,
        0x7FBF_FFFF, 0xFFFF_FFFF, 0x7FFF_FFFF,
    ]);
    bits.sort_unstable();
    bits.dedup();
    bits
}

/// `nextafterf` for the finite cases we need.
pub fn next_after(x: f32, toward: f32) -> f32 {
    if x.is_nan() || toward.is_nan() {
        return f32::NAN;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        return f32::from_bits(if toward > 0.0 { 1 } else { 0x8000_0001 });
    }
    let b = x.to_bits();
    let up = (x < toward) == (x > 0.0);
    f32::from_bits(if up { b + 1 } else { b - 1 })
}
