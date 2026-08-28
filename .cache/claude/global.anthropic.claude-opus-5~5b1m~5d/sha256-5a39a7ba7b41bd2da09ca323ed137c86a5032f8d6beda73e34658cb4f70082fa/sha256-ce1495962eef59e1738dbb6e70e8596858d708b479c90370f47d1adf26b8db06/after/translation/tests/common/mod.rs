//! Shared differential-test harness.
//!
//! Loads BOTH shared objects (the C one built by CMake and the Rust `cdylib`)
//! through `libloading` and resolves the `float2half` symbol from each. No Rust
//! function is ever called directly: every comparison goes through the dynamic
//! symbol, so the `#[no_mangle] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// The one and only exported signature: `uint16_t float2half(float)`.
pub type Float2Half = unsafe extern "C" fn(f32) -> u16;

pub struct Libs {
    // Kept alive: the function pointers below borrow from these.
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: Float2Half,
    rust_fn: Float2Half,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`.
///
/// CMake derives the library name from the name of the directory *above*
/// `c_src`, so the file name is environment-specific. Glob for `lib*.so`
/// instead of hard-coding it.
fn find_c_lib() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB") {
        return PathBuf::from(p);
    }
    let root = crate_root();
    let repo = root.parent().expect("crate root has a parent");
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in [
        repo.join("c_src").join("build"),
        repo.join("c_src").join("build").join("lib"),
    ] {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.starts_with("lib") && name.ends_with(".so") && p.is_file() {
                    candidates.push(p);
                }
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "could not find the C shared library under {}/c_src/build.\n\
             Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
             (or set the C_LIB environment variable)",
            repo.display()
        )
    })
}

/// Locate the Rust `cdylib`.
///
/// Honours `CARGO_TARGET_DIR`, and deliberately prefers the artifact built with
/// the SAME profile as this test binary. That matters: a debug-profile `.so`
/// has `debug_assertions` and integer-overflow checks enabled, so running the
/// suite under `cargo test` (debug) genuinely exercises the "does the Rust
/// introduce a panic the C does not have?" rows of `ERRORS.md`, while
/// `cargo test --release` exercises the shipped `panic = "abort"` build.
fn find_rust_lib() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB") {
        return PathBuf::from(p);
    }
    const LIB_NAME: &str = "libfloat2half_lib.so";
    let target_dir = match std::env::var("CARGO_TARGET_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => crate_root().join("target"),
    };

    // Same profile as this test binary first, then the other as a fallback.
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target_dir.join(profile).join(LIB_NAME);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "could not find {} under {}/{{debug,release}}. Build it with: \
         cargo build [--release]",
        LIB_NAME,
        target_dir.display()
    )
}

unsafe fn load(path: &Path) -> (Library, Float2Half) {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    let f = {
        let sym: Symbol<Float2Half> = unsafe { lib.get(b"float2half\0") }.unwrap_or_else(|e| {
            panic!("symbol `float2half` not found in {}: {e}", path.display())
        });
        // Copy the raw fn pointer out; `lib` is moved into the returned tuple
        // and kept alive for as long as the pointer is used.
        *sym
    };
    (lib, f)
}

impl Libs {
    pub fn load() -> Self {
        let c_path = find_c_lib();
        let rust_path = find_rust_lib();
        let (c_lib, c_fn) = unsafe { load(&c_path) };
        let (rust_lib, rust_fn) = unsafe { load(&rust_path) };
        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c_fn,
            rust_fn,
            c_path,
            rust_path,
        }
    }

    /// Call the C implementation through its `.so`.
    #[inline]
    pub fn c(&self, x: f32) -> u16 {
        unsafe { (self.c_fn)(x) }
    }

    /// Call the Rust implementation through its `.so`.
    #[inline]
    pub fn rust(&self, x: f32) -> u16 {
        unsafe { (self.rust_fn)(x) }
    }

    /// The raw C function pointer, for handing to worker threads.
    ///
    /// Function pointers are `Copy + Send + Sync`; the caller must keep this
    /// `Libs` alive for as long as the pointer is used (a scoped thread does).
    pub fn c_raw(&self) -> Float2Half {
        self.c_fn
    }

    /// The raw Rust function pointer, for handing to worker threads.
    pub fn rust_raw(&self) -> Float2Half {
        self.rust_fn
    }
}

/// Deterministic PRNG (SplitMix64) so every "randomized" run is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x0024_0611_C0FF_EEEE;

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Assemble a `f32` from its three IEEE-754 binary32 fields.
///
/// `sign` in 0..=1, `exp` in 0..=255, `mantissa` in 0..=0x7FFFFF.
#[inline]
pub fn make_f32(sign: u32, exp: u32, mantissa: u32) -> f32 {
    debug_assert!(sign <= 1 && exp <= 0xFF && mantissa <= 0x7F_FFFF);
    f32::from_bits((sign << 31) | (exp << 23) | mantissa)
}

/// The table index the C code computes: `j = (bits >> 23) & 0x1ff`.
#[inline]
pub fn index_of(bits: u32) -> u32 {
    (bits >> 23) & 0x1ff
}

/// Compare both implementations on one raw bit pattern.
///
/// Passing the value as raw bits (rather than as a parsed literal) guarantees
/// NaN payloads and signalling NaNs cross the FFI boundary untouched.
#[track_caller]
pub fn check_bits(libs: &Libs, bits: u32, ctx: &str) {
    let x = f32::from_bits(bits);
    let c = libs.c(x);
    let r = libs.rust(x);
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: input bits 0x{bits:08X} (f32 {x:e}, sign={} exp={} mant=0x{:06X}, j={}) \
         -> C returned 0x{c:04X}, Rust returned 0x{r:04X}",
        bits >> 31,
        (bits >> 23) & 0xFF,
        bits & 0x7F_FFFF,
        index_of(bits),
    );
}

/// Compare on an assembled `(sign, exp, mantissa)` triple.
#[track_caller]
pub fn check_fields(libs: &Libs, sign: u32, exp: u32, mantissa: u32, ctx: &str) {
    check_bits(libs, (sign << 31) | (exp << 23) | mantissa, ctx);
}

/// Mantissa shapes that probe a shift of `shift` bits: the exact cut points.
pub fn boundary_mantissas(shift: u32) -> Vec<u32> {
    let mut v = vec![0u32, 1, 2, 0x7F_FFFF, 0x7F_FFFE, 0x40_0000];
    if shift < 23 {
        let unit = 1u32 << shift;
        v.extend_from_slice(&[
            unit.wrapping_sub(1),
            unit,
            unit.wrapping_add(1),
            unit.wrapping_mul(2).wrapping_sub(1),
            unit.wrapping_mul(2),
        ]);
    }
    v.retain(|&m| m <= 0x7F_FFFF);
    v.sort_unstable();
    v.dedup();
    v
}

/// Parse `m__base` and `m__shift` straight out of the **C source text**.
///
/// The expected values in the tests are therefore derived from the C, not from
/// the Rust translation, so a test cannot be satisfied by a Rust table that
/// merely agrees with itself.
pub fn read_c_tables() -> ([u16; 512], [u8; 512]) {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate root has a parent")
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");

    fn grab(src: &str, start: &str) -> Vec<u32> {
        let i = src.find(start).expect("table start marker") + start.len();
        let j = i + src[i..].find("};").expect("table end marker");
        let mut out = Vec::new();
        let mut rest = &src[i..j];
        while let Some(p) = rest.find("0x") {
            rest = &rest[p + 2..];
            let end = rest
                .find(|c: char| !c.is_ascii_hexdigit())
                .unwrap_or(rest.len());
            out.push(u32::from_str_radix(&rest[..end], 16).expect("hex literal"));
            rest = &rest[end..];
        }
        out
    }

    let b = grab(&src, "m__base[512] = {");
    let s = grab(&src, "m__shift[512] = {");
    assert_eq!(b.len(), 512, "parsed m__base should have 512 entries");
    assert_eq!(s.len(), 512, "parsed m__shift should have 512 entries");

    let mut base = [0u16; 512];
    let mut shift = [0u8; 512];
    for i in 0..512 {
        assert!(b[i] <= 0xFFFF, "m__base[{i}] out of u16 range");
        assert!(s[i] <= 0xFF, "m__shift[{i}] out of u8 range");
        base[i] = b[i] as u16;
        shift[i] = s[i] as u8;
    }
    (base, shift)
}

/// Number of randomized samples per (exponent, sign) cell. Override with
/// `DIFF_SAMPLES` to turn the dial up.
pub fn samples_per_cell(default: u32) -> u32 {
    std::env::var("DIFF_SAMPLES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
