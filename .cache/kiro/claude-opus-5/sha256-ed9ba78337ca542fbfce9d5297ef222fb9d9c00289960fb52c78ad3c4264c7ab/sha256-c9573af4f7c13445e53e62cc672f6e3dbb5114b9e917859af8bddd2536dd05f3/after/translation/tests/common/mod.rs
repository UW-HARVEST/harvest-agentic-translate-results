//! Shared differential-testing harness.
//!
//! Both the C shared object and the Rust shared object are loaded with
//! `libloading` and driven exclusively through their exported `wcscat` symbol.
//! No Rust function is ever called directly, so the `#[no_mangle]`/`extern "C"`
//! wrapper is part of what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;

/// `wchar_t` on Linux/glibc: signed 32-bit int.
pub type WcharT = i32;

/// ABI of the function under test.
pub type WcscatFn = unsafe extern "C" fn(*mut WcharT, usize, *const WcharT) -> i32;

/// Sentinel written past `numElem` so an out-of-bounds write shows up as a diff.
pub const GUARD_VAL: WcharT = 0x7F5A_A5F7u32 as i32;
/// Number of guard elements appended to every destination allocation.
pub const GUARD: usize = 8;

/// Fixed seed so every property-style run is reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Impls {
    pub c: WcscatFn,
    pub rust: WcscatFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let root = workspace_root();
    let build = root.join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    // Some CMake generators drop the artifact in a config subdirectory.
    if candidates.is_empty() {
        for sub in ["Debug", "Release", "lib"] {
            if let Ok(rd) = std::fs::read_dir(build.join(sub)) {
                for e in rd.flatten() {
                    let p = e.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("lib") && name.ends_with(".so") {
                        candidates.push(p);
                    }
                }
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C shared library found under {}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // current_exe is <...>/target/<profile>/deps/<test-bin>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");

    let mut dirs = vec![profile_dir.to_path_buf(), deps.to_path_buf()];
    // Also try the sibling profile dirs, in case the cdylib was only built once.
    if let Some(target) = profile_dir.parent() {
        for p in ["debug", "release"] {
            dirs.push(target.join(p));
        }
    }
    for d in &dirs {
        let p = d.join("libwcscat_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libwcscat_lib.so not found; looked in: {:?}. Run `cargo build` first.",
        dirs
    );
}

fn load(path: &Path) -> WcscatFn {
    // Leak the Library so the code stays mapped for the whole process.
    let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
        libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()))
    }));
    let sym: libloading::Symbol<'static, WcscatFn> = unsafe {
        lib.get(b"wcscat\0")
            .unwrap_or_else(|e| panic!("dlsym wcscat in {} failed: {e}", path.display()))
    };
    *sym
}

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let c = load(&c_path);
        let rust = load(&rust_path);
        Impls {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// Case description
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Src {
    /// `src == NULL`
    Null,
    /// A separate, disjoint allocation. Must be NUL-terminated.
    Own(Vec<WcharT>),
    /// `src` points into the destination buffer at the given element offset.
    IntoDst(usize),
}

#[derive(Clone, Debug)]
pub struct Case {
    pub dst_null: bool,
    /// Physical initial contents of the destination allocation. May be longer
    /// or shorter than `num_elem` on purpose.
    pub dst_data: Vec<WcharT>,
    pub num_elem: usize,
    pub src: Src,
}

impl Case {
    pub fn new(dst_data: Vec<WcharT>, num_elem: usize, src: Src) -> Self {
        Case {
            dst_null: false,
            dst_data,
            num_elem,
            src,
        }
    }
    pub fn null_dst(num_elem: usize, src: Src) -> Self {
        Case {
            dst_null: true,
            dst_data: Vec::new(),
            num_elem,
            src,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i32,
    /// Destination allocation (initial contents + guard region) after the call.
    pub dst: Vec<WcharT>,
    /// Source allocation after the call, to catch writes through `src`.
    pub src: Vec<WcharT>,
}

fn exec(f: WcscatFn, c: &Case) -> Outcome {
    let mut dbuf: Vec<WcharT> = Vec::with_capacity(c.dst_data.len() + GUARD);
    dbuf.extend_from_slice(&c.dst_data);
    dbuf.extend(std::iter::repeat(GUARD_VAL).take(GUARD));

    let sbuf: Vec<WcharT> = match &c.src {
        Src::Own(v) => v.clone(),
        _ => Vec::new(),
    };

    let dptr: *mut WcharT = if c.dst_null {
        ptr::null_mut()
    } else {
        dbuf.as_mut_ptr()
    };
    let sptr: *const WcharT = match &c.src {
        Src::Null => ptr::null(),
        Src::Own(_) => sbuf.as_ptr(),
        Src::IntoDst(off) => {
            assert!(!c.dst_null, "IntoDst requires a non-null dst");
            assert!(*off <= dbuf.len(), "IntoDst offset out of allocation");
            unsafe { dbuf.as_ptr().add(*off) }
        }
    };

    let ret = unsafe { f(dptr, c.num_elem, sptr) };

    Outcome {
        ret,
        dst: dbuf,
        src: sbuf,
    }
}

/// Runs the case against both `.so`s and asserts byte-identical results.
#[track_caller]
pub fn assert_same(case: &Case, label: &str) {
    let i = impls();
    let a = exec(i.c, case);
    let b = exec(i.rust, case);
    if a != b {
        let mut msg = format!(
            "DIVERGENCE [{label}]\n  case: dst_null={} num_elem={} dst_len={} src={:?}\n",
            case.dst_null,
            case.num_elem,
            case.dst_data.len(),
            match &case.src {
                Src::Null => "NULL".to_string(),
                Src::Own(v) => format!("Own(len={}, {:?})", v.len(), trunc(v)),
                Src::IntoDst(o) => format!("IntoDst({o})"),
            }
        );
        msg += &format!("  dst_init: {:?}\n", trunc(&case.dst_data));
        if a.ret != b.ret {
            msg += &format!("  ret:  C={} RUST={}\n", a.ret, b.ret);
        }
        if a.dst != b.dst {
            msg += &format!("  dst:  C={:?}\n        RUST={:?}\n", trunc(&a.dst), trunc(&b.dst));
            if let Some(k) = a.dst.iter().zip(b.dst.iter()).position(|(x, y)| x != y) {
                msg += &format!(
                    "  first dst diff at index {k}: C={} RUST={}\n",
                    a.dst[k], b.dst[k]
                );
            }
        }
        if a.src != b.src {
            msg += &format!("  src:  C={:?}\n        RUST={:?}\n", trunc(&a.src), trunc(&b.src));
        }
        panic!("{msg}");
    }
}

/// Same as [`assert_same`] but also asserts the (already-agreed) return code.
#[track_caller]
pub fn assert_same_ret(case: &Case, expected: i32, label: &str) {
    assert_same(case, label);
    let i = impls();
    let a = exec(i.c, case);
    assert_eq!(
        a.ret, expected,
        "[{label}] C returned {} but the ERRORS.md/CONFIGS.md row expects {expected}",
        a.ret
    );
}

/// Runs both implementations and returns the (identical) outcome, asserting
/// agreement first.
#[track_caller]
pub fn both(case: &Case) -> Outcome {
    assert_same(case, "both()");
    exec(impls().c, case)
}

fn trunc(v: &[WcharT]) -> Vec<WcharT> {
    v.iter().copied().take(24).collect()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() - 1)]
    }
}

// ---------------------------------------------------------------------------
// wchar_t value classes (see CONFIGS.md axis A7)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ValClass {
    Ascii,
    NonBmp,
    Surrogate,
    AboveUnicodeMax,
    Negative,
    MixedRandom,
}

pub const ALL_CLASSES: [ValClass; 6] = [
    ValClass::Ascii,
    ValClass::NonBmp,
    ValClass::Surrogate,
    ValClass::AboveUnicodeMax,
    ValClass::Negative,
    ValClass::MixedRandom,
];

/// Produces a non-zero `wchar_t` in the requested class. Never returns 0, so
/// callers control termination explicitly.
pub fn gen_char(rng: &mut Rng, class: ValClass) -> WcharT {
    match class {
        ValClass::Ascii => rng.range(0x21, 0x7E) as i32,
        ValClass::NonBmp => rng.range(0x1_0000, 0x10_FFFF) as i32,
        ValClass::Surrogate => rng.range(0xD800, 0xDFFF) as i32,
        ValClass::AboveUnicodeMax => {
            if rng.range(0, 15) == 0 {
                i32::MAX
            } else {
                rng.range(0x11_0000, 0x7FFF_FFFF) as i32
            }
        }
        ValClass::Negative => {
            if rng.range(0, 15) == 0 {
                i32::MIN
            } else if rng.range(0, 7) == 0 {
                -1
            } else {
                -(rng.range(1, 0x7FFF_FFFF) as i64) as i32
            }
        }
        ValClass::MixedRandom => {
            let v = rng.next_u32() as i32;
            if v == 0 { 1 } else { v }
        }
    }
}

/// A NUL-terminated `src` of exactly `len` characters (allocation is `len + 1`).
pub fn gen_src(rng: &mut Rng, len: usize, class: ValClass) -> Vec<WcharT> {
    let mut v: Vec<WcharT> = (0..len).map(|_| gen_char(rng, class)).collect();
    v.push(0);
    v
}

/// A destination allocation of `phys` elements whose first NUL sits at index
/// `nul_at`. Pass `nul_at >= phys` for an allocation with no NUL at all.
pub fn gen_dst(rng: &mut Rng, phys: usize, nul_at: usize, class: ValClass) -> Vec<WcharT> {
    let mut v: Vec<WcharT> = Vec::with_capacity(phys);
    for i in 0..phys {
        if i == nul_at {
            v.push(0);
        } else if i > nul_at {
            // Garbage past the terminator; must be preserved or overwritten
            // identically by both implementations.
            v.push(gen_char(rng, class));
        } else {
            v.push(gen_char(rng, class));
        }
    }
    v
}
