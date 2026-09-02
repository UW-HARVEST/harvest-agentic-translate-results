//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through `dlsym`, so the `#[no_mangle]` export wrappers are exercised
//! exactly as an external C caller would exercise them.

#![allow(dead_code, non_camel_case_types, non_snake_case)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};

pub mod deflate;
pub mod rng;

// ---------------------------------------------------------------------------
// ABI mirror of include/lib.h
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl std::fmt::Debug for CpPixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{},{})", self.r, self.g, self.b, self.a)
    }
}

pub type ConvertPixFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut CpPixel);
pub type CpInflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

/// One loaded implementation (C or Rust), accessed only through `dlsym`.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub convert_pix: ConvertPixFn,
    pub cp_inflate: CpInflateFn,
    pub cp_error_reason: *mut *const c_char,
    pub cp_fixed_table: *mut u8,
    pub cp_permutation_order: *mut u8,
    pub cp_len_extra_bits: *mut u8,
    pub cp_len_base: *mut u32,
    pub cp_dist_extra_bits: *mut u8,
    pub cp_dist_base: *mut u32,
}

pub const N_FIXED_TABLE: usize = 288 + 32;
pub const N_PERMUTATION: usize = 19;
pub const N_LEN: usize = 29 + 2;
pub const N_DIST: usize = 30 + 2;

impl Impl {
    pub fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {name} at {}: {e}", path.display()));
        unsafe {
            let convert_pix: ConvertPixFn = *lib
                .get(b"convert_pix\0")
                .unwrap_or_else(|e| panic!("{name}: convert_pix: {e}"));
            let cp_inflate: CpInflateFn = *lib
                .get(b"cp_inflate\0")
                .unwrap_or_else(|e| panic!("{name}: cp_inflate: {e}"));
            let cp_error_reason: *mut *const c_char = *lib
                .get(b"cp_error_reason\0")
                .unwrap_or_else(|e| panic!("{name}: cp_error_reason: {e}"));
            let cp_fixed_table: *mut u8 = *lib
                .get(b"cp_fixed_table\0")
                .unwrap_or_else(|e| panic!("{name}: cp_fixed_table: {e}"));
            let cp_permutation_order: *mut u8 = *lib
                .get(b"cp_permutation_order\0")
                .unwrap_or_else(|e| panic!("{name}: cp_permutation_order: {e}"));
            let cp_len_extra_bits: *mut u8 = *lib
                .get(b"cp_len_extra_bits\0")
                .unwrap_or_else(|e| panic!("{name}: cp_len_extra_bits: {e}"));
            let cp_len_base: *mut u32 = *lib
                .get(b"cp_len_base\0")
                .unwrap_or_else(|e| panic!("{name}: cp_len_base: {e}"));
            let cp_dist_extra_bits: *mut u8 = *lib
                .get(b"cp_dist_extra_bits\0")
                .unwrap_or_else(|e| panic!("{name}: cp_dist_extra_bits: {e}"));
            let cp_dist_base: *mut u32 = *lib
                .get(b"cp_dist_base\0")
                .unwrap_or_else(|e| panic!("{name}: cp_dist_base: {e}"));
            Impl {
                name,
                _lib: lib,
                convert_pix,
                cp_inflate,
                cp_error_reason,
                cp_fixed_table,
                cp_permutation_order,
                cp_len_extra_bits,
                cp_len_base,
                cp_dist_extra_bits,
                cp_dist_base,
            }
        }
    }

    pub fn clear_error(&self) {
        unsafe { std::ptr::write(self.cp_error_reason, std::ptr::null()) }
    }

    pub fn error(&self) -> Option<String> {
        unsafe {
            let p = std::ptr::read(self.cp_error_reason);
            if p.is_null() {
                None
            } else {
                Some(CStr::from_ptr(p).to_string_lossy().into_owned())
            }
        }
    }

    // --- snapshot / restore of the exported writable tables -----------------

    pub fn fixed_table(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.cp_fixed_table, N_FIXED_TABLE).to_vec() }
    }
    pub fn permutation_order(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.cp_permutation_order, N_PERMUTATION).to_vec() }
    }
    pub fn len_extra_bits(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.cp_len_extra_bits, N_LEN).to_vec() }
    }
    pub fn len_base(&self) -> Vec<u32> {
        unsafe { std::slice::from_raw_parts(self.cp_len_base, N_LEN).to_vec() }
    }
    pub fn dist_extra_bits(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.cp_dist_extra_bits, N_DIST).to_vec() }
    }
    pub fn dist_base(&self) -> Vec<u32> {
        unsafe { std::slice::from_raw_parts(self.cp_dist_base, N_DIST).to_vec() }
    }

    pub fn set_fixed_table(&self, v: &[u8]) {
        assert_eq!(v.len(), N_FIXED_TABLE);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_fixed_table, v.len()) }
    }
    pub fn set_permutation_order(&self, v: &[u8]) {
        assert_eq!(v.len(), N_PERMUTATION);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_permutation_order, v.len()) }
    }
    pub fn set_len_extra_bits(&self, v: &[u8]) {
        assert_eq!(v.len(), N_LEN);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_len_extra_bits, v.len()) }
    }
    pub fn set_len_base(&self, v: &[u32]) {
        assert_eq!(v.len(), N_LEN);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_len_base, v.len()) }
    }
    pub fn set_dist_extra_bits(&self, v: &[u8]) {
        assert_eq!(v.len(), N_DIST);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_dist_extra_bits, v.len()) }
    }
    pub fn set_dist_base(&self, v: &[u32]) {
        assert_eq!(v.len(), N_DIST);
        unsafe { std::ptr::copy_nonoverlapping(v.as_ptr(), self.cp_dist_base, v.len()) }
    }
}

/// The C and Rust implementations, both dlopen'ed.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    // The runner selects which C build to compare against:
    //   * c_src/build          - built exactly as the task specifies (no NDEBUG,
    //                            so `assert()` is live)  -> Rust default features
    //   * ../c_ndebug_build    - same sources with -DNDEBUG -> Rust
    //                            --no-default-features
    if let Ok(p) = std::env::var("CP_C_SO") {
        let path = PathBuf::from(&p);
        assert!(path.exists(), "CP_C_SO={p} does not exist");
        return path;
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {} - build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

/// Path of the Rust `.so` under test (exposed for the symbol-parity test).
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

/// Path of the C `.so` under test (exposed for the symbol-parity test).
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

fn find_rust_so() -> PathBuf {
    // Prefer the release artifact (the real shipping .so); fall back to debug.
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libconvert_pix_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "no Rust .so found under {} — build it with `cargo build --release`",
        base.display()
    )
}

/// Load only one side. Child processes in the abort-differential tests use this
/// so they pay for a single `dlopen`.
pub fn load_one(which: &str) -> Impl {
    match which {
        "c" => Impl::load("C", &find_c_so()),
        "rust" => Impl::load("Rust", &find_rust_so()),
        other => panic!("unknown implementation {other:?}"),
    }
}

pub fn load_pair() -> Pair {
    Pair {
        c: Impl::load("C", &find_c_so()),
        rust: Impl::load("Rust", &find_rust_so()),
    }
}

// ---------------------------------------------------------------------------
// Aligned buffers (the `in` pointer alignment is a real behavioural axis of
// cp_inflate: `first_bytes` is computed from the pointer value).
// ---------------------------------------------------------------------------

/// A heap buffer whose start address is congruent to `align_off` modulo 4.
///
/// The data is surrounded by generous zero margins. That matters because the C
/// reads outside the declared input in two places:
///
///  * `cp_ptr()` can point a few bytes *before* the data (it subtracts the
///    number of buffered bytes from the next unread word), and
///  * `cp_stored()` `memcpy`s `LEN` bytes with no bound check (row U3), so a
///    truncated or over-declared stored block reads past the end.
///
/// With margins those reads land in deterministic zeros in both processes, so
/// the behaviour stays comparable instead of depending on unrelated heap.
pub struct AlignedBuf {
    backing: Vec<u8>,
    off: usize,
    len: usize,
}

const FRONT_MARGIN: usize = 64;
const BACK_MARGIN: usize = 0x1_0000 + 64;

impl AlignedBuf {
    pub fn new(data: &[u8], align_off: usize) -> AlignedBuf {
        assert!(align_off < 4);
        let mut backing = vec![0u8; FRONT_MARGIN + data.len() + BACK_MARGIN];
        let base = backing.as_ptr() as usize;
        let mut off = FRONT_MARGIN;
        while (base + off) % 4 != align_off % 4 {
            off += 1;
        }
        backing[off..off + data.len()].copy_from_slice(data);
        AlignedBuf {
            backing,
            off,
            len: data.len(),
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.backing.as_mut_ptr().add(self.off) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn residue(&self) -> usize {
        (unsafe { self.backing.as_ptr().add(self.off) } as usize) % 4
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Outcome of one `cp_inflate` call.
#[derive(PartialEq, Eq)]
pub struct InflateOutcome {
    pub ret: c_int,
    pub err: Option<String>,
    pub out: Vec<u8>,
}

impl std::fmt::Debug for InflateOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ret={} err={:?} out({})={}",
            self.ret,
            self.err,
            self.out.len(),
            hex_head(&self.out, 48)
        )
    }
}

pub fn hex_head(b: &[u8], n: usize) -> String {
    let mut s = String::new();
    for x in b.iter().take(n) {
        s.push_str(&format!("{x:02x}"));
    }
    if b.len() > n {
        s.push_str("..");
    }
    s
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

/// FNV-1a 64, used to keep child-process result lines short while still being an
/// exact comparison of the whole output buffer.
pub fn digest(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &x in b {
        h ^= x as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{}:{:016x}:{}", b.len(), h, hex_head(b, 24))
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "bad hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("bad hex"))
        .collect()
}

/// `cp_error_reason` is a process-wide mutable global inside each `.so`, and
/// `dlopen`ing the same library twice in one process shares it. Cargo runs the
/// `#[test]` functions of one binary on parallel threads, so every
/// clear-call-read sequence must be serialized or the tests clobber each
/// other's error string.
pub static CALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn lock() -> std::sync::MutexGuard<'static, ()> {
    match CALL_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Run `cp_inflate` on one implementation.
///
/// `in_align` fixes the residue of the *input* pointer mod 4 so that both
/// implementations see the same `first_bytes` split. `in_bytes_override` lets a
/// test lie about the input length (used by the error-path tests).
pub fn run_inflate(
    im: &Impl,
    input: &[u8],
    in_align: usize,
    out_bytes: usize,
    in_bytes_override: Option<c_int>,
) -> InflateOutcome {
    let mut inbuf = AlignedBuf::new(input, in_align);
    let mut out = vec![0u8; out_bytes];
    let in_bytes = in_bytes_override.unwrap_or(input.len() as c_int);
    let _g = lock();
    im.clear_error();
    let ret = unsafe {
        (im.cp_inflate)(
            inbuf.as_mut_ptr() as *mut c_void,
            in_bytes,
            out.as_mut_ptr() as *mut c_void,
            out_bytes as c_int,
        )
    };
    let err = im.error();
    drop(_g);
    InflateOutcome { ret, err, out }
}

/// Differentially run `cp_inflate` on both implementations and assert equality.
pub fn diff_inflate(p: &Pair, ctx: &str, input: &[u8], in_align: usize, out_bytes: usize) {
    let c = run_inflate(&p.c, input, in_align, out_bytes, None);
    let r = run_inflate(&p.rust, input, in_align, out_bytes, None);
    assert_eq!(
        c.ret, r.ret,
        "[{ctx}] return code mismatch\n  C   : {c:?}\n  Rust: {r:?}\n  in({}) align={in_align} out_bytes={out_bytes}: {}",
        input.len(),
        hex_head(input, 64)
    );
    assert_eq!(
        c.err, r.err,
        "[{ctx}] cp_error_reason mismatch\n  C   : {c:?}\n  Rust: {r:?}\n  in({}) align={in_align}: {}",
        input.len(),
        hex_head(input, 64)
    );
    if c.out != r.out {
        let at = c
            .out
            .iter()
            .zip(r.out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        panic!(
            "[{ctx}] out buffer mismatch at byte {at}\n  C   : {c:?}\n  Rust: {r:?}\n  in({}) align={in_align} out_bytes={out_bytes}: {}",
            input.len(),
            hex_head(input, 64)
        );
    }
}

/// Differentially run `cp_inflate` with an explicit `in_bytes` value.
pub fn diff_inflate_raw(
    p: &Pair,
    ctx: &str,
    input: &[u8],
    in_align: usize,
    out_bytes: usize,
    in_bytes: c_int,
) {
    let c = run_inflate(&p.c, input, in_align, out_bytes, Some(in_bytes));
    let r = run_inflate(&p.rust, input, in_align, out_bytes, Some(in_bytes));
    assert_eq!(c.ret, r.ret, "[{ctx}] ret mismatch\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.err, r.err, "[{ctx}] err mismatch\n C: {c:?}\n R: {r:?}");
    assert_eq!(c.out, r.out, "[{ctx}] out mismatch\n C: {c:?}\n R: {r:?}");
}

/// Differentially run `convert_pix` on both implementations.
///
/// `src_len` / `dst_len` are given explicitly so tests can deliberately supply
/// buffers of any size (including zero-length, for the no-write configurations).
pub fn diff_convert_pix(
    p: &Pair,
    ctx: &str,
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: &[u8],
    dst_len: usize,
) {
    let sentinel = CpPixel {
        r: 0x11,
        g: 0x22,
        b: 0x33,
        a: 0x44,
    };
    let mut src_c = src.to_vec();
    let mut src_r = src.to_vec();
    let mut dst_c = vec![sentinel; dst_len];
    let mut dst_r = vec![sentinel; dst_len];
    {
        let _g = lock();
        unsafe {
            (p.c.convert_pix)(bpp, w, h, src_c.as_mut_ptr(), dst_c.as_mut_ptr());
            (p.rust.convert_pix)(bpp, w, h, src_r.as_mut_ptr(), dst_r.as_mut_ptr());
        }
    }
    assert_eq!(
        src_c, src_r,
        "[{ctx}] convert_pix must not modify src (bpp={bpp} w={w} h={h})"
    );
    if dst_c != dst_r {
        let at = dst_c
            .iter()
            .zip(dst_r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(0);
        panic!(
            "[{ctx}] convert_pix dst mismatch at pixel {at} (bpp={bpp} w={w} h={h})\n  C   : {:?}\n  Rust: {:?}",
            &dst_c[at..(at + 4).min(dst_c.len())],
            &dst_r[at..(at + 4).min(dst_r.len())]
        );
    }
}

/// `convert_pix` with NULL `src`/`dst` (only legal when nothing is dereferenced).
pub fn diff_convert_pix_null(p: &Pair, ctx: &str, bpp: c_int, w: c_int, h: c_int) {
    unsafe {
        (p.c.convert_pix)(bpp, w, h, std::ptr::null_mut(), std::ptr::null_mut());
        (p.rust.convert_pix)(bpp, w, h, std::ptr::null_mut(), std::ptr::null_mut());
    }
    let _ = ctx;
}
