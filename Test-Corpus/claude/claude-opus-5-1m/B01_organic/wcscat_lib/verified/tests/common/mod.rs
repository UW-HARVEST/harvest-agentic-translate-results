//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called *only* through their exported `wcscat` symbol, so the `#[no_mangle]`
//! `extern "C"` wrapper is part of what gets tested.
//!
//! * C   : `c_src/build/libtranslated_rust.so`
//! * Rust: `target/<profile>/libwcscat_lib.so`

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `wchar_t` on this target: 4-byte signed int (verified in `SYMBOLS.md`).
pub type Wchar = i32;

pub type WcscatFn = unsafe extern "C" fn(*mut Wchar, usize, *const Wchar) -> i32;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    pub wcscat: WcscatFn,
    // Keep the handle alive for the whole process lifetime.
    _lib: Library,
}

pub struct Libs {
    pub c: Impl,
    pub rust: Impl,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Tag identifying the feature set this test binary was compiled with, so the
/// Rust `.so` we load is built with exactly the same configuration.
fn feature_tag() -> String {
    let mut tag = String::new();
    if cfg!(feature = "default") {
        tag.push_str("default");
    }
    if tag.is_empty() {
        tag.push_str("nofeatures");
    }
    tag
}

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]`-only library
/// (integration tests cannot link it, so it is not a dependency of the test
/// target). Loading `target/<profile>/libwcscat_lib.so` directly would
/// therefore happily test a stale artifact and pass even when `src/lib.rs` is
/// wrong. To make the harness trustworthy we build the cdylib ourselves, into
/// a private target directory (separate from the one the outer `cargo test`
/// holds a lock on), with the same feature set as this test binary.
///
/// Set `DIFFTEST_PROFILE=release` to verify the optimized artifact instead
/// (the profile that also enables `panic = "abort"`); `run_all_features.sh`
/// runs both.
fn build_rust_so() -> PathBuf {
    let release = std::env::var("DIFFTEST_PROFILE").as_deref() == Ok("release");
    let profile_dir = if release { "release" } else { "debug" };
    let manifest = manifest_dir();
    let target = manifest
        .join("target")
        .join("difftest")
        .join(feature_tag());
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = Command::new(&cargo);
    cmd.current_dir(&manifest)
        .env("CARGO_TARGET_DIR", &target)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .args(["build", "--offline", "--quiet", "--lib", "--no-default-features"]);
    if release {
        cmd.arg("--release");
    }
    if cfg!(feature = "default") {
        cmd.args(["--features", "default"]);
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build`: {e}"));
    assert!(
        out.status.success(),
        "building the Rust cdylib failed ({}):\n{}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    target.join(profile_dir).join("libwcscat_lib.so")
}

fn load(name: &'static str, path: PathBuf) -> Impl {
    assert!(
        path.exists(),
        "{name} shared object not found at {}.\n\
         Build the C library with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust library with:\n  cargo build --offline",
        path.display()
    );
    // SAFETY: loading a plain C shared library with no constructors.
    let lib = unsafe { Library::new(&path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    let wcscat: WcscatFn = unsafe {
        let sym: Symbol<WcscatFn> = lib
            .get(b"wcscat\0")
            .unwrap_or_else(|e| panic!("dlsym wcscat in {} failed: {e}", path.display()));
        *sym
    };
    Impl {
        name,
        path,
        wcscat,
        _lib: lib,
    }
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(build_rust_so).clone()
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let l = Libs {
            c: load("C", c_so_path()),
            rust: load("Rust", rust_so_path()),
        };
        // The two entry points must be genuinely different code objects; if
        // `dlsym` had resolved both to the same definition (e.g. glibc's
        // 2-argument `wcscat`) every differential assertion would be vacuous.
        assert_ne!(
            l.c.wcscat as usize, l.rust.wcscat as usize,
            "C and Rust `wcscat` resolved to the same address"
        );
        l
    })
}

// ---------------------------------------------------------------------------
// Case description
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dst {
    Null,
    /// Initial contents of the destination allocation. Its length is the
    /// *real* allocation size, which may differ from `num_elem`.
    Buf(Vec<Wchar>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Src {
    Null,
    /// A separate allocation.
    External(Vec<Wchar>),
    /// `src = dst + offset` (aliasing; only valid with `Dst::Buf`).
    AliasDst(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub dst: Dst,
    pub num_elem: usize,
    pub src: Src,
}

impl Case {
    pub fn new(dst: Dst, num_elem: usize, src: Src) -> Self {
        Case {
            dst,
            num_elem,
            src,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    /// Destination allocation after the call (`None` when `dst` was NULL).
    pub dst_after: Option<Vec<Wchar>>,
    /// External source allocation after the call (`None` otherwise) — proves
    /// neither implementation writes through `src`.
    pub src_after: Option<Vec<Wchar>>,
}

/// Run one case against one implementation.
pub fn run(imp: &Impl, case: &Case) -> Outcome {
    match (&case.dst, &case.src) {
        (Dst::Null, Src::Null) => {
            let ret = unsafe { (imp.wcscat)(ptr::null_mut(), case.num_elem, ptr::null()) };
            Outcome {
                ret,
                dst_after: None,
                src_after: None,
            }
        }
        (Dst::Null, Src::External(s)) => {
            let mut src = s.clone();
            let ret = unsafe { (imp.wcscat)(ptr::null_mut(), case.num_elem, src.as_ptr()) };
            let _ = &mut src;
            Outcome {
                ret,
                dst_after: None,
                src_after: Some(src),
            }
        }
        (Dst::Null, Src::AliasDst(_)) => panic!("AliasDst requires a non-NULL dst"),
        (Dst::Buf(b), Src::Null) => {
            let mut dst = b.clone();
            let ret = unsafe { (imp.wcscat)(dst.as_mut_ptr(), case.num_elem, ptr::null()) };
            Outcome {
                ret,
                dst_after: Some(dst),
                src_after: None,
            }
        }
        (Dst::Buf(b), Src::External(s)) => {
            let mut dst = b.clone();
            let mut src = s.clone();
            let ret =
                unsafe { (imp.wcscat)(dst.as_mut_ptr(), case.num_elem, src.as_ptr()) };
            let _ = &mut src;
            Outcome {
                ret,
                dst_after: Some(dst),
                src_after: Some(src),
            }
        }
        (Dst::Buf(b), Src::AliasDst(off)) => {
            let mut dst = b.clone();
            assert!(*off <= dst.len(), "alias offset out of allocation");
            let ret = unsafe {
                let base = dst.as_mut_ptr();
                (imp.wcscat)(base, case.num_elem, base.add(*off) as *const Wchar)
            };
            Outcome {
                ret,
                dst_after: Some(dst),
                src_after: None,
            }
        }
    }
}

/// Run a case against both implementations and assert byte-identical results.
/// Returns the (shared) outcome so callers can additionally assert the exact
/// return code, proving the intended code path was really taken.
#[track_caller]
pub fn assert_same(case: &Case) -> Outcome {
    let l = libs();
    let a = run(&l.c, case);
    let b = run(&l.rust, case);
    if a == b {
        return a;
    }
    {
        panic!(
            "DIVERGENCE\ncase: num_elem={} ({:#x})\n  dst_in : {}\n  src_in : {}\n\
             C   -> ret={} dst={} src={}\n\
             Rust -> ret={} dst={} src={}\n",
            case.num_elem,
            case.num_elem,
            describe_dst(&case.dst),
            describe_src(&case.src),
            a.ret,
            opt(&a.dst_after),
            opt(&a.src_after),
            b.ret,
            opt(&b.dst_after),
            opt(&b.src_after),
        );
    }
}

/// `assert_same` plus an assertion on the exact return code.
#[track_caller]
pub fn assert_same_ret(case: &Case, expected: i32) {
    let o = assert_same(case);
    assert_eq!(
        o.ret,
        expected,
        "both implementations agreed on ret={} but the test intended {} \
         (case: num_elem={}, dst_in={}, src_in={})",
        o.ret,
        expected,
        case.num_elem,
        describe_dst(&case.dst),
        describe_src(&case.src),
    );
}

fn describe_dst(d: &Dst) -> String {
    match d {
        Dst::Null => "NULL".to_string(),
        Dst::Buf(v) => format!("len={} {:?}", v.len(), Trunc(v)),
    }
}

fn describe_src(s: &Src) -> String {
    match s {
        Src::Null => "NULL".to_string(),
        Src::External(v) => format!("external len={} {:?}", v.len(), Trunc(v)),
        Src::AliasDst(o) => format!("dst+{o}"),
    }
}

fn opt(v: &Option<Vec<Wchar>>) -> String {
    match v {
        None => "-".to_string(),
        Some(v) => format!("{:?}", Trunc(v)),
    }
}

struct Trunc<'a>(&'a [Wchar]);
impl std::fmt::Debug for Trunc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX: usize = 40;
        if self.0.len() <= MAX {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{:?}..(+{})", &self.0[..MAX], self.0.len() - MAX)
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
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
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as usize
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.range(0, xs.len() - 1)]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// wchar_t value classes
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Class {
    /// Printable ASCII.
    Ascii,
    /// Astral-plane code points (> 0xFFFF).
    Wide,
    /// Strictly negative values (`wchar_t` is signed here).
    Negative,
    /// Hand-picked extremes and non-characters.
    Extreme,
    /// Any non-zero `i32`.
    Full,
}

pub const EXTREMES: &[Wchar] = &[
    i32::MAX,
    i32::MIN,
    -1,
    1,
    0x7F,
    0x80,
    0xFFFF,
    0x1_0000,
    0x10_FFFF,
    0x11_0000,
    0xD800,
    0xDFFF,
    0xFFFE,
    0xFFFD,
    0x8000_0000u32 as i32,
    0x8000_0001u32 as i32,
];

/// A random **non-zero** `wchar_t` from `class`.
pub fn nonzero(rng: &mut Rng, class: Class) -> Wchar {
    loop {
        let v = match class {
            Class::Ascii => rng.range(0x20, 0x7E) as Wchar,
            Class::Wide => rng.range(0x1_0000, 0x10_FFFF) as Wchar,
            Class::Negative => -(rng.range(1, i32::MAX as usize) as i64) as Wchar,
            Class::Extreme => rng.pick(EXTREMES),
            Class::Full => rng.next_u32() as Wchar,
        };
        if v != 0 {
            return v;
        }
    }
}

/// Any `wchar_t` (may be zero) — used for "garbage" fill.
pub fn any(rng: &mut Rng, class: Class) -> Wchar {
    if rng.range(0, 15) == 0 {
        0
    } else {
        nonzero(rng, class)
    }
}

// ---------------------------------------------------------------------------
// Buffer builders
// ---------------------------------------------------------------------------

/// Destination allocation of `alloc` elements holding a NUL-terminated string
/// of length `k` (`k < alloc`), followed by random garbage.
pub fn make_dst(rng: &mut Rng, alloc: usize, k: usize, class: Class) -> Vec<Wchar> {
    assert!(k < alloc, "k={k} alloc={alloc}");
    let mut v = Vec::with_capacity(alloc);
    for _ in 0..k {
        v.push(nonzero(rng, class));
    }
    v.push(0);
    while v.len() < alloc {
        v.push(any(rng, class));
    }
    v
}

/// Destination allocation of `alloc` elements with **no** NUL anywhere.
pub fn make_dst_unterminated(rng: &mut Rng, alloc: usize, class: Class) -> Vec<Wchar> {
    (0..alloc).map(|_| nonzero(rng, class)).collect()
}

/// NUL-terminated source of `len` non-zero elements, plus `garbage` extra
/// elements after the terminator (to catch over-reads).
pub fn make_src(rng: &mut Rng, len: usize, garbage: usize, class: Class) -> Vec<Wchar> {
    let mut v: Vec<Wchar> = (0..len).map(|_| nonzero(rng, class)).collect();
    v.push(0);
    for _ in 0..garbage {
        v.push(any(rng, class));
    }
    v
}

/// A source with **no** terminator within `len` elements (all non-zero).
pub fn make_src_unterminated(rng: &mut Rng, len: usize, class: Class) -> Vec<Wchar> {
    (0..len).map(|_| nonzero(rng, class)).collect()
}

pub const ALL_CLASSES: &[Class] = &[
    Class::Ascii,
    Class::Wide,
    Class::Negative,
    Class::Extreme,
    Class::Full,
];
