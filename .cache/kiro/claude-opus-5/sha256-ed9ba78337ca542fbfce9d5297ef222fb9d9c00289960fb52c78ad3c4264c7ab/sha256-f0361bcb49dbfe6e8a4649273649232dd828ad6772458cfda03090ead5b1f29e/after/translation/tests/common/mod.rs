//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects (the C one built by CMake and the Rust `cdylib`)
//! with `libloading` and calls `wcscat` through the FFI boundary only — the Rust
//! implementation is never called directly, so the `#[no_mangle]` export wrapper
//! is under test as well.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `wchar_t` as the platform C compiler sees it (4-byte signed `int` on
/// Linux/glibc). The harness asserts the assumption below.
pub type Wchar = i32;

pub type WcscatFn = unsafe extern "C" fn(*mut Wchar, usize, *const Wchar) -> std::ffi::c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("WCSCAT_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "WCSCAT_RUST_SO={} does not exist", p.display());
        return p;
    }
    let root = workspace_root().join("translation").join("target");
    // Prefer the profile the tests were built with, then fall back.
    let candidates = [
        root.join("release").join("libwcscat_lib.so"),
        root.join("debug").join("libwcscat_lib.so"),
    ];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "no Rust cdylib found; build it with `cargo build --release` (looked for {:?})",
        candidates
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// The libraries are only ever used through raw function pointers to `extern "C"`
// functions that touch no shared state, so sharing them across threads is fine.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        // Note: `Library::new` uses RTLD_LOCAL, so the C `wcscat` (which shadows
        // the libc symbol of the same name) does not leak into the global scope
        // and cannot interpose on the Rust library's lookups.
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", rust_path.display()));
        Libs { c, rust }
    })
}

/// `wcscat` as exported by the **C** shared object.
pub fn c_wcscat() -> WcscatFn {
    let l = libs();
    let sym: Symbol<WcscatFn> = unsafe { l.c.get(b"wcscat\0") }.expect("C .so exports wcscat");
    *sym
}

/// `wcscat` as exported by the **Rust** shared object.
pub fn rust_wcscat() -> WcscatFn {
    let l = libs();
    let sym: Symbol<WcscatFn> =
        unsafe { l.rust.get(b"wcscat\0") }.expect("Rust .so exports wcscat");
    *sym
}

pub fn assert_platform_assumptions() {
    assert_eq!(
        std::mem::size_of::<Wchar>(),
        4,
        "harness assumes 4-byte wchar_t"
    );
}

/// Guard element written past the region the callee is allowed to touch.
pub const GUARD: Wchar = 0x5A5A_5A5A_u32 as Wchar;
/// Number of guard elements appended after `numElem`.
pub const GUARD_LEN: usize = 8;

/// One differential call outcome: return value + the complete buffer image
/// (including the guard region) after the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ret: std::ffi::c_int,
    pub buf: Vec<Wchar>,
}

/// Runs `f` (a `wcscat` from one of the two `.so`s) on a *fresh private copy* of
/// `dst_image`, with `GUARD_LEN` guard elements appended, and returns the
/// outcome. `dst_image` must be exactly the elements the callee may see, i.e.
/// `num_elem` is passed independently so oversized/undersized lengths can be
/// tested.
pub fn run(
    f: WcscatFn,
    dst_image: &[Wchar],
    num_elem: usize,
    src: Option<&[Wchar]>,
    null_dst: bool,
) -> Outcome {
    if let Some(s) = src {
        assert!(
            s.last() == Some(&0),
            "`src` slices passed to run() must include their NUL terminator"
        );
    }
    let mut buf: Vec<Wchar> = Vec::with_capacity(dst_image.len() + GUARD_LEN);
    buf.extend_from_slice(dst_image);
    buf.extend(std::iter::repeat(GUARD).take(GUARD_LEN));

    // `src` is copied into its own allocation so an accidental write through the
    // `const` pointer would show up as a difference too.
    let mut src_buf: Vec<Wchar> = src.map(|s| s.to_vec()).unwrap_or_default();

    let dst_ptr = if null_dst {
        std::ptr::null_mut()
    } else {
        buf.as_mut_ptr()
    };
    let src_ptr = match src {
        None => std::ptr::null(),
        Some(_) => src_buf.as_mut_ptr() as *const Wchar,
    };

    let ret = unsafe { f(dst_ptr, num_elem, src_ptr) };

    // Fold the (possibly mutated) src image into the compared state as well.
    let mut image = buf;
    image.push(Wchar::MIN); // separator so the two regions can't alias visually
    image.append(&mut src_buf);
    Outcome { ret, buf: image }
}

/// Core differential assertion: identical inputs must give identical return
/// value AND identical post-call memory, byte for byte.
#[track_caller]
pub fn assert_same(
    label: &str,
    dst_image: &[Wchar],
    num_elem: usize,
    src: Option<&[Wchar]>,
    null_dst: bool,
) {
    let c = run(c_wcscat(), dst_image, num_elem, src, null_dst);
    let r = run(rust_wcscat(), dst_image, num_elem, src, null_dst);
    if c != r {
        let mut first_diff = None;
        for (i, (a, b)) in c.buf.iter().zip(r.buf.iter()).enumerate() {
            if a != b {
                first_diff = Some((i, *a, *b));
                break;
            }
        }
        panic!(
            "DIVERGENCE [{label}]\n  dst_image = {dst_image:?}\n  num_elem  = {num_elem}\n  \
             src       = {src:?}\n  null_dst  = {null_dst}\n  \
             C   ret = {} buf = {:?}\n  Rust ret = {} buf = {:?}\n  first differing element: {:?}",
            c.ret, c.buf, r.ret, r.buf, first_diff
        );
    }
    // Guard region must be untouched by both (no write past `num_elem`).
    let guard_start = dst_image.len();
    if !null_dst {
        for (i, g) in c.buf[guard_start..guard_start + GUARD_LEN].iter().enumerate() {
            assert_eq!(
                *g, GUARD,
                "[{label}] C wrote past the buffer at guard index {i}"
            );
        }
    }
}

/// Deterministic xorshift64* PRNG — reproducible across runs and platforms.
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
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    /// A non-zero `wchar_t`, drawn from the full 32-bit range.
    pub fn nonzero_wchar(&mut self) -> Wchar {
        loop {
            let v = self.next_u64() as u32 as Wchar;
            if v != 0 {
                return v;
            }
        }
    }
    /// A non-zero printable-ASCII-ish `wchar_t` (keeps failures readable).
    pub fn ascii_wchar(&mut self) -> Wchar {
        (0x21 + self.below(0x5E)) as Wchar
    }
}

/// Interesting extreme `wchar_t` values (nothing here is zero).
pub const EXTREMES: &[Wchar] = &[
    Wchar::MIN,
    Wchar::MIN + 1,
    -2,
    -1,
    1,
    0x7F,
    0x80,
    0xD800, // lone surrogate
    0xDFFF,
    0xFFFF,
    0x1_0000,
    0x10_FFFF,
    0x11_0000, // past the Unicode range
    Wchar::MAX,
];

/// Builds a `dst` image of `len` elements: a NUL-terminated prefix of
/// `prefix_len` non-zero elements (terminator at index `prefix_len`), and
/// pseudo-random non-zero residue after the terminator. If `prefix_len == len`
/// the image is left *unterminated*.
pub fn make_dst(rng: &mut Rng, len: usize, prefix_len: usize, ascii: bool) -> Vec<Wchar> {
    assert!(prefix_len <= len);
    let mut v = Vec::with_capacity(len);
    for _ in 0..prefix_len {
        v.push(if ascii { rng.ascii_wchar() } else { rng.nonzero_wchar() });
    }
    if prefix_len < len {
        v.push(0);
        while v.len() < len {
            v.push(if ascii { rng.ascii_wchar() } else { rng.nonzero_wchar() });
        }
    }
    debug_assert_eq!(v.len(), len);
    v
}

/// Builds a NUL-terminated `src` of `len` visible elements (so `len + 1` total).
pub fn make_src(rng: &mut Rng, len: usize, ascii: bool) -> Vec<Wchar> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push(if ascii { rng.ascii_wchar() } else { rng.nonzero_wchar() });
    }
    v.push(0);
    v
}
