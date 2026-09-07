//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `cdylib` through `libloading` and calls
//! `tfm` only via `dlsym`, so the `#[no_mangle] extern "C"` export wrapper is
//! part of what is under test. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, i32);

/// Fixed seed so every property-style row is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root().parent().expect("crate has a parent dir").to_path_buf()
}

/// Locate the C shared library built by `c_src/CMakeLists.txt`.
///
/// The CMake project name is derived from the *parent directory* name, so the
/// file name is not fixed; glob the build dir instead of hard-coding it.
fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO_PATH") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    collect_so(&build, &mut candidates);
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found under {}; build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn collect_so(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            // Only one level of nesting is needed, but recursing is harmless.
            collect_so(&p, out);
        } else if p.extension().map(|x| x == "so").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// Locate the Rust `cdylib` **for the profile the test binary itself was built
/// with**. `cargo test` puts the test executable in `target/<profile>/deps/`,
/// so the cdylib is expected in `target/<profile>/` (or that same `deps/` dir).
///
/// This deliberately does NOT fall back to another profile: silently loading a
/// release `.so` while running the debug suite would report a pass for an
/// artifact that was never built, which is exactly the kind of false green this
/// harness exists to prevent.
fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let name = "libtfm_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    let deps = exe.parent().expect("test binary has a parent dir");
    let profile_dir = deps.parent().expect("deps has a parent dir");
    for cand in [deps.join(name), profile_dir.join(name)] {
        if cand.is_file() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib `{name}` not found in {} or {}.\n\
         The test binary was built into {}, so the cdylib for THAT profile must \
         exist. `cargo test` does not always emit the cdylib; build it first, \
         e.g.\n    cargo build{}\n\
         (refusing to fall back to another profile's .so — that would give a \
         false pass)",
        deps.display(),
        profile_dir.display(),
        deps.display(),
        if profile_dir.file_name().map(|s| s == "release").unwrap_or(false) {
            " --release"
        } else {
            ""
        },
    );
}

struct Libs {
    _c: Library,
    _rust: Library,
    c_tfm: TfmFn,
    rust_tfm: TfmFn,
}

// The two `Library` handles stay alive for the whole process (leaked into the
// OnceLock), so the raw fn pointers remain valid. `dlopen`ed code is
// thread-safe here: `tfm` is a pure leaf function with no global state.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        eprintln!("[harness] C    .so = {}", c_path.display());
        eprintln!("[harness] Rust .so = {}", rust_path.display());
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
            let c_sym: Symbol<TfmFn> = c
                .get(b"tfm\0")
                .unwrap_or_else(|e| panic!("dlsym tfm in C .so failed: {e}"));
            let rust_sym: Symbol<TfmFn> = rust
                .get(b"tfm\0")
                .unwrap_or_else(|e| panic!("dlsym tfm in Rust .so failed: {e}"));
            let c_tfm = *c_sym;
            let rust_tfm = *rust_sym;
            Libs { _c: c, _rust: rust, c_tfm, rust_tfm }
        }
    })
}

pub fn c_tfm() -> TfmFn {
    libs().c_tfm
}

pub fn rust_tfm() -> TfmFn {
    libs().rust_tfm
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — no external dev-dependency needed.
// ---------------------------------------------------------------------------

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

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        self.next_u32() % n
    }

    /// A completely arbitrary `f32` bit pattern (NaNs, infinities, subnormals
    /// all reachable).
    pub fn any_bits_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A "reasonable" finite value, log-uniform over a wide exponent range.
    pub fn finite_f32(&mut self) -> f32 {
        loop {
            let mant = self.next_u32() & 0x007F_FFFF;
            let exp = self.below(200) + 20; // biased exponent 20..220
            let sign = self.next_u32() & 1;
            let bits = (sign << 31) | (exp << 23) | mant;
            let v = f32::from_bits(bits);
            if v.is_finite() {
                return v;
            }
        }
    }

    /// Small finite value in roughly `[-4, 4]`, dense around 1.
    pub fn small_f32(&mut self) -> f32 {
        let u = (self.next_u32() as f64) / (u32::MAX as f64);
        let v = (u * 8.0 - 4.0) as f32;
        v
    }

    pub fn special_f32(&mut self) -> f32 {
        let pool: [f32; 24] = [
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            -0.5,
            2.0,
            -2.0,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::from_bits(1),               // smallest subnormal
            f32::from_bits(0x8000_0001),     // -smallest subnormal
            f32::from_bits(0x007F_FFFF),     // largest subnormal
            f32::from_bits(0x807F_FFFF),
            f32::MAX,
            f32::MIN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7FC0_0000),     // canonical qNaN
            f32::from_bits(0xFFC0_0000),     // -qNaN ("indefinite")
            f32::from_bits(0x7FA0_0000),     // sNaN
            f32::from_bits(0xFFA0_0000),     // -sNaN
            f32::from_bits(0x7F80_0001),     // sNaN, payload 1
            3.4028234e38,
        ];
        pool[self.below(pool.len() as u32) as usize]
    }

    /// A NaN with a random payload and sign, quiet or signalling.
    pub fn nan_f32(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let quiet = (self.next_u32() & 1) << 22;
        let mut payload = self.next_u32() & 0x003F_FFFF;
        if quiet == 0 && payload == 0 {
            payload = 1; // mantissa must be non-zero, else it's an infinity
        }
        f32::from_bits(sign | 0x7F80_0000 | quiet | payload)
    }
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Canary value written past the end of both output buffers, so an
/// over-write by either implementation is detected.
const CANARY: u32 = 0xDEAD_BEEF;
const CANARY_LEN: usize = 8;

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Run both `.so` implementations on identical inputs in disjoint buffers and
/// assert the output buffers (plus trailing canaries) are bit-identical.
///
/// `dest_len` is the number of `f32` slots the caller provides for output;
/// normally `2 * count`, but tests pass a different value to probe extents.
#[track_caller]
pub fn diff_disjoint(src: &[f32], count: i32, dest_len: usize, ctx: &str) {
    let mut c_dest = vec![f32::from_bits(0xA5A5_A5A5); dest_len + CANARY_LEN];
    let mut r_dest = c_dest.clone();
    for i in 0..CANARY_LEN {
        c_dest[dest_len + i] = f32::from_bits(CANARY);
        r_dest[dest_len + i] = f32::from_bits(CANARY);
    }
    let c_src = src.to_vec();
    let r_src = src.to_vec();

    unsafe {
        (c_tfm())(c_dest.as_mut_ptr(), c_src.as_ptr(), count);
        (rust_tfm())(r_dest.as_mut_ptr(), r_src.as_ptr(), count);
    }

    assert_eq!(
        bits(&c_src),
        bits(&r_src),
        "{ctx}: input buffer was modified differently (src is `const` and must not be written)"
    );
    compare(&c_dest, &r_dest, src, count, dest_len, ctx);
}

/// Same, but the caller supplies the (possibly aliasing) buffer layout: `buf`
/// is the full backing store, `dest_off`/`src_off` are element offsets into it.
#[track_caller]
pub fn diff_aliased(buf: &[f32], dest_off: usize, src_off: usize, count: i32, ctx: &str) {
    let mut c_buf = buf.to_vec();
    let mut r_buf = buf.to_vec();
    unsafe {
        (c_tfm())(
            c_buf.as_mut_ptr().add(dest_off),
            c_buf.as_ptr().add(src_off),
            count,
        );
        (rust_tfm())(
            r_buf.as_mut_ptr().add(dest_off),
            r_buf.as_ptr().add(src_off),
            count,
        );
    }
    let cb = bits(&c_buf);
    let rb = bits(&r_buf);
    if cb != rb {
        let i = cb.iter().zip(&rb).position(|(a, b)| a != b).unwrap();
        panic!(
            "{ctx}: aliased buffer mismatch at slot {i}\n  \
             C    = 0x{:08X} ({:e})\n  Rust = 0x{:08X} ({:e})\n  \
             count={count} dest_off={dest_off} src_off={src_off}\n  input = {}",
            cb[i],
            f32::from_bits(cb[i]),
            rb[i],
            f32::from_bits(rb[i]),
            hexdump(buf)
        );
    }
}

#[track_caller]
fn compare(
    c_dest: &[f32],
    r_dest: &[f32],
    src: &[f32],
    count: i32,
    dest_len: usize,
    ctx: &str,
) {
    let cb = bits(c_dest);
    let rb = bits(r_dest);
    if cb == rb {
        // Also assert neither implementation ran past the end.
        for i in 0..CANARY_LEN {
            assert_eq!(
                cb[dest_len + i], CANARY,
                "{ctx}: C wrote past the end of dest (canary {i} clobbered)"
            );
        }
        return;
    }
    let i = cb.iter().zip(&rb).position(|(a, b)| a != b).unwrap();
    let kind = if i >= dest_len { "CANARY (buffer overrun)" } else { "output" };
    panic!(
        "{ctx}: {kind} mismatch at dest slot {i} (element {}, lane {})\n  \
         C    = 0x{:08X} ({:e})\n  Rust = 0x{:08X} ({:e})\n  \
         count = {count}\n  src   = {}\n  C dest   = {}\n  R dest   = {}",
        i / 2,
        i % 2,
        cb[i],
        f32::from_bits(cb[i]),
        rb[i],
        f32::from_bits(rb[i]),
        hexdump(src),
        hexdump(&c_dest[..dest_len.min(c_dest.len())]),
        hexdump(&r_dest[..dest_len.min(r_dest.len())]),
    );
}

pub fn hexdump(v: &[f32]) -> String {
    let mut s = String::from("[");
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        if i == 24 {
            s.push_str("...");
            break;
        }
        s.push_str(&format!("0x{:08X}", x.to_bits()));
    }
    s.push(']');
    s
}

/// Convenience: build a `src` buffer of `3*count` floats from a generator and
/// diff it. Repeated `iters` times with the seeded PRNG.
#[track_caller]
pub fn diff_rows(
    rng: &mut Rng,
    iters: usize,
    ctx: &str,
    mut gen: impl FnMut(&mut Rng, usize) -> (Vec<f32>, i32),
) {
    for it in 0..iters {
        let (src, count) = gen(rng, it);
        let dest_len = (count.max(0) as usize) * 2;
        diff_disjoint(&src, count, dest_len, &format!("{ctx} [iter {it}]"));
    }
}
