//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; every call
//! goes through the exported symbols, never through the Rust crate directly.

#![allow(dead_code)]

pub mod deflate;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// ABI types (mirror include/lib.h)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub type LoadPngMem = unsafe extern "C" fn(*const u8, c_int) -> CpImage;
pub type CpInflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub load_png_mem: LoadPngMem,
    pub cp_inflate: CpInflate,
    err: *mut *const c_char,
    pub tables: Vec<(&'static str, *const u8, usize)>,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("..").join("c_src").join("build");
    let mut found = None;
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found = Some(e.path());
            }
        }
    }
    found.unwrap_or_else(|| panic!("no C .so found in {:?}; build c_src first", build))
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let rel = manifest_dir()
        .join("target")
        .join("release")
        .join("libload_png_mem_lib.so");
    if rel.exists() {
        return rel;
    }
    let dbg = manifest_dir()
        .join("target")
        .join("debug")
        .join("libload_png_mem_lib.so");
    if dbg.exists() {
        return dbg;
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

const TABLE_SPECS: &[(&str, usize)] = &[
    ("cp_fixed_table", 288 + 32),
    ("cp_permutation_order", 19),
    ("cp_len_extra_bits", 31),
    ("cp_len_base", 31 * 4),
    ("cp_dist_extra_bits", 32),
    ("cp_dist_base", 32 * 4),
];

impl Lib {
    pub unsafe fn open(name: &'static str, path: &PathBuf) -> Lib {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path));
        let load_png_mem: libloading::Symbol<LoadPngMem> = lib.get(b"load_png_mem\0").unwrap();
        let load_png_mem = *load_png_mem;
        let cp_inflate: libloading::Symbol<CpInflate> = lib.get(b"cp_inflate\0").unwrap();
        let cp_inflate = *cp_inflate;
        let err: libloading::Symbol<*mut *const c_char> = lib.get(b"cp_error_reason\0").unwrap();
        let err = *err;
        let mut tables = Vec::new();
        for (sym, len) in TABLE_SPECS {
            let mut nul = sym.as_bytes().to_vec();
            nul.push(0);
            let s: libloading::Symbol<*const u8> = lib.get(&nul).unwrap();
            tables.push((*sym, *s, *len));
        }
        Lib {
            name,
            _lib: lib,
            load_png_mem,
            cp_inflate,
            err,
            tables,
        }
    }

    pub unsafe fn set_error_sentinel(&self) {
        *self.err = std::ptr::null();
    }

    pub unsafe fn error_reason(&self) -> Option<String> {
        let p = *self.err;
        if p.is_null() {
            None
        } else {
            Some(CStr::from_ptr(p).to_string_lossy().into_owned())
        }
    }
}

pub struct Pair {
    pub c: Lib,
    pub rs: Lib,
}

pub fn open_pair() -> Pair {
    unsafe {
        Pair {
            c: Lib::open("C", &c_so_path()),
            rs: Lib::open("Rust", &rust_so_path()),
        }
    }
}

// One shared pair per test binary.
use std::sync::OnceLock;
static PAIR: OnceLock<usize> = OnceLock::new();

pub fn pair() -> &'static Pair {
    let addr = PAIR.get_or_init(|| Box::into_raw(Box::new(open_pair())) as usize);
    unsafe { &*(*addr as *const Pair) }
}

/// `cp_error_reason` is a single process-global inside each `.so`, so no two
/// test threads may have a call in flight at the same time. Every entry point
/// in this module takes this lock for the duration of one library call; test
/// code must therefore never hold it across a call to these helpers.
static CALL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    match CALL_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Aligned, padded input buffers so that both libraries see byte-identical
// memory *and* an identical `ptr & 3`, which `cp_inflate` branches on.
// ---------------------------------------------------------------------------

/// Padding placed after every buffer handed to the libraries.
///
/// The C performs unbounded out-of-bounds accesses on malformed input (a stored
/// block with `LEN == 0xFFFF` makes `cp_stored` `memcpy` 64 KiB regardless of
/// `out_bytes`). Without slack those writes corrupt the process heap, and the
/// *consequence* then depends on the allocation layout, which differs between
/// the two children for reasons that have nothing to do with the translation.
/// With 128 KiB of slack the access stays inside our own allocation, and the
/// padding is part of the compared region, so an overflow shows up as a data
/// difference instead of as a random crash.
pub const PAD: usize = 1 << 17;

/// Heap buffer whose payload starts at `align_off` bytes past a 16-aligned
/// base, followed by `PAD` bytes of deterministic filler.
pub struct AlignedBuf {
    base: *mut u8,
    pub ptr: *mut u8,
    pub len: usize,
}

impl AlignedBuf {
    pub fn new(data: &[u8], align_off: usize) -> AlignedBuf {
        let total = align_off + data.len() + PAD;
        unsafe {
            let base = libc::malloc(total.max(1)) as *mut u8;
            assert!(!base.is_null());
            assert_eq!(base as usize % 16, 0, "malloc should be 16-aligned");
            // deterministic filler everywhere, then the payload
            std::ptr::write_bytes(base, 0, total);
            let ptr = base.add(align_off);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
            AlignedBuf {
                base,
                ptr,
                len: data.len(),
            }
        }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe { libc::free(self.base as *mut c_void) }
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct PngResult {
    pub w: c_int,
    pub h: c_int,
    pub null: bool,
    pub pixels: Vec<CpPixel>,
    pub error: Option<String>,
}

unsafe fn run_png(lib: &Lib, png: &[u8], length: c_int, align_off: usize) -> PngResult {
    let buf = AlignedBuf::new(png, align_off);
    lib.set_error_sentinel();
    let img = (lib.load_png_mem)(buf.ptr, length);
    let error = lib.error_reason();
    let mut pixels = Vec::new();
    if !img.pix.is_null() {
        let n = (img.w as i64) * (img.h as i64);
        if n > 0 && n < 64 * 1024 * 1024 {
            pixels = std::slice::from_raw_parts(img.pix, n as usize).to_vec();
        }
        libc::free(img.pix as *mut c_void);
    }
    PngResult {
        w: img.w,
        h: img.h,
        null: img.pix.is_null(),
        pixels,
        error: if img.pix.is_null() { error } else { None },
    }
}

/// Runs `load_png_mem` on the C `.so` only (used to prove a test vector really
/// reaches the intended branch before the differential comparison).
pub fn c_png(png: &[u8], length: c_int) -> PngResult {
    let _g = lock();
    unsafe { run_png(&pair().c, png, length, 0) }
}

pub fn c_inflate(input: &[u8], in_bytes: c_int, out_bytes: c_int, out_cap: usize) -> InflateResult {
    let _g = lock();
    unsafe { run_inflate(&pair().c, input, in_bytes, 0, out_bytes, out_cap) }
}

/// `cp_inflate` with `out == NULL`, on one library. Takes the shared lock.
pub fn inflate_null_out(which_c: bool, input: &[u8], in_bytes: c_int, out_bytes: c_int)
    -> (c_int, Option<String>)
{
    let p = pair();
    let lib = if which_c { &p.c } else { &p.rs };
    let _g = lock();
    unsafe {
        let inbuf = AlignedBuf::new(input, 0);
        lib.set_error_sentinel();
        let r = (lib.cp_inflate)(
            inbuf.ptr as *mut c_void,
            in_bytes,
            std::ptr::null_mut(),
            out_bytes,
        );
        (r, lib.error_reason())
    }
}

pub fn c_inflate_align(
    input: &[u8],
    in_bytes: c_int,
    align: usize,
    out_bytes: c_int,
    out_cap: usize,
) -> InflateResult {
    let _g = lock();
    unsafe { run_inflate(&pair().c, input, in_bytes, align, out_bytes, out_cap) }
}

/// Runs `load_png_mem` on both libraries and asserts byte-identical results.
pub fn diff_png(label: &str, png: &[u8]) {
    diff_png_len(label, png, png.len() as c_int)
}

pub fn diff_png_len(label: &str, png: &[u8], length: c_int) {
    let p = pair();
    let _g = lock();
    unsafe {
        let a = run_png(&p.c, png, length, 0);
        let b = run_png(&p.rs, png, length, 0);
        assert_eq!(a.null, b.null, "[{label}] pix null-ness differs: C={a:?} Rust={b:?}");
        assert_eq!(a.w, b.w, "[{label}] w differs: C={} Rust={}", a.w, b.w);
        assert_eq!(a.h, b.h, "[{label}] h differs: C={} Rust={}", a.h, b.h);
        assert_eq!(
            a.error, b.error,
            "[{label}] cp_error_reason differs: C={:?} Rust={:?}",
            a.error, b.error
        );
        if a.pixels != b.pixels {
            let idx = a
                .pixels
                .iter()
                .zip(b.pixels.iter())
                .position(|(x, y)| x != y);
            panic!(
                "[{label}] pixel mismatch (len C={} Rust={}) first diff at {:?}: C={:?} Rust={:?}",
                a.pixels.len(),
                b.pixels.len(),
                idx,
                idx.map(|i| a.pixels[i]),
                idx.map(|i| b.pixels[i]),
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct InflateResult {
    pub ret: c_int,
    pub out: Vec<u8>,
    pub error: Option<String>,
}

unsafe fn run_inflate(
    lib: &Lib,
    input: &[u8],
    in_bytes: c_int,
    in_align: usize,
    out_bytes: c_int,
    out_cap: usize,
) -> InflateResult {
    let inbuf = AlignedBuf::new(input, in_align);
    let outbuf = AlignedBuf::new(&vec![0u8; out_cap], 0);
    lib.set_error_sentinel();
    let ret = (lib.cp_inflate)(
        inbuf.ptr as *mut c_void,
        in_bytes,
        outbuf.ptr as *mut c_void,
        out_bytes,
    );
    // Compare the padding too, so writes past `out_bytes` are caught.
    let out = std::slice::from_raw_parts(outbuf.ptr, out_cap + PAD).to_vec();
    InflateResult {
        ret,
        out,
        error: if ret == 0 { lib.error_reason() } else { None },
    }
}

/// Runs `cp_inflate` on both libraries and asserts identical return value,
/// output bytes and error string.
pub fn diff_inflate(label: &str, input: &[u8], out_cap: usize) {
    diff_inflate_full(label, input, input.len() as c_int, 0, out_cap as c_int, out_cap)
}

pub fn diff_inflate_align(label: &str, input: &[u8], align: usize, out_cap: usize) {
    diff_inflate_full(
        label,
        input,
        input.len() as c_int,
        align,
        out_cap as c_int,
        out_cap,
    )
}

pub fn diff_inflate_full(
    label: &str,
    input: &[u8],
    in_bytes: c_int,
    in_align: usize,
    out_bytes: c_int,
    out_cap: usize,
) {
    let p = pair();
    let _g = lock();
    unsafe {
        let a = run_inflate(&p.c, input, in_bytes, in_align, out_bytes, out_cap);
        let b = run_inflate(&p.rs, input, in_bytes, in_align, out_bytes, out_cap);
        assert_eq!(a.ret, b.ret, "[{label}] cp_inflate return differs: C={} Rust={}", a.ret, b.ret);
        assert_eq!(
            a.error, b.error,
            "[{label}] cp_error_reason differs: C={:?} Rust={:?}",
            a.error, b.error
        );
        if a.out != b.out {
            let idx = a.out.iter().zip(b.out.iter()).position(|(x, y)| x != y);
            panic!(
                "[{label}] output mismatch, first diff at {:?}: C={:?} Rust={:?}",
                idx,
                idx.map(|i| a.out[i]),
                idx.map(|i| b.out[i])
            );
        }
    }
}
