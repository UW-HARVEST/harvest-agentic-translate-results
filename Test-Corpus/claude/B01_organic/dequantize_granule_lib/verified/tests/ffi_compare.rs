// Integration test that loads both the C-built shared library and the
// Rust-built shared library via libloading, calls `dequantize_granule` on
// both with identical inputs, and asserts byte-for-byte identical outputs.

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone)]
struct BsT {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
#[derive(Clone)]
struct L12ScaleInfo {
    scf: [f32; 3 * 64],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

type DequantFn = unsafe extern "C" fn(
    grbuf: *mut f32,
    bs: *mut BsT,
    sci: *mut L12ScaleInfo,
    group_size: c_int,
) -> c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to translated_rust/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // The build script for tests doesn't necessarily produce a release lib;
    // we look in both common targets.
    let release = workspace_root().join("target/release/libdequantize_granule_lib.so");
    if release.exists() {
        return release;
    }
    let debug = workspace_root().join("target/debug/libdequantize_granule_lib.so");
    debug
}

unsafe fn load_dequant(lib: &Library) -> Symbol<DequantFn> {
    lib.get(b"dequantize_granule\0").expect("dequantize_granule export")
}

fn make_sci(total_bands: u8, bitalloc: &[u8]) -> L12ScaleInfo {
    let mut sci = L12ScaleInfo {
        scf: [0.0; 192],
        total_bands,
        stereo_bands: 0,
        bitalloc: [0; 64],
        scfcod: [0; 64],
    };
    for (i, &b) in bitalloc.iter().enumerate() {
        sci.bitalloc[i] = b;
    }
    sci
}

fn run_pair(
    c_fn: DequantFn,
    r_fn: DequantFn,
    bitstream: &[u8],
    bs_pos: c_int,
    bs_limit: c_int,
    sci: &L12ScaleInfo,
    group_size: c_int,
) {
    let grbuf_size = 32 * 18 * 4; // generous buffer
    let mut grbuf_c = vec![0.0f32; grbuf_size];
    let mut grbuf_r = vec![0.0f32; grbuf_size];

    let mut bs_c = BsT { buf: bitstream.as_ptr(), pos: bs_pos, limit: bs_limit };
    let mut bs_r = BsT { buf: bitstream.as_ptr(), pos: bs_pos, limit: bs_limit };

    let mut sci_c = sci.clone();
    let mut sci_r = sci.clone();

    let rc_c = unsafe { c_fn(grbuf_c.as_mut_ptr(), &mut bs_c, &mut sci_c, group_size) };
    let rc_r = unsafe { r_fn(grbuf_r.as_mut_ptr(), &mut bs_r, &mut sci_r, group_size) };

    assert_eq!(rc_c, rc_r, "return code mismatch");
    assert_eq!(bs_c.pos, bs_r.pos, "bs.pos mismatch");
    assert_eq!(bs_c.limit, bs_r.limit, "bs.limit mismatch");

    // Compare bit-pattern of each f32 (not float equality — NaNs and signed
    // zero are handled by comparing raw bits).
    for i in 0..grbuf_size {
        let a = grbuf_c[i].to_bits();
        let b = grbuf_r[i].to_bits();
        assert_eq!(
            a, b,
            "grbuf mismatch at index {}: c={:#x} r={:#x}",
            i, a, b
        );
    }
}

#[test]
fn test_simple_no_bitalloc() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    // total_bands=2 -> bands=4. All bitalloc=0 → no reads from stream.
    let sci = make_sci(2, &[0, 0, 0, 0]);
    let bitstream = vec![0u8; 1024];
    run_pair(c_fn, r_fn, &bitstream, 0, 8 * 1024, &sci, 12);
}

#[test]
fn test_ba_lt_17_various() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    // Use a deterministic-ish bitstream
    let mut bitstream = vec![0u8; 4096];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = ((i * 131 + 7) & 0xff) as u8;
    }

    // Try various ba values 1..=16 across bands
    let bitalloc_vals: Vec<u8> = (1..=16).cycle().take(8).collect();
    let sci = make_sci(4, &bitalloc_vals); // 8 bands
    run_pair(c_fn, r_fn, &bitstream, 0, 8 * (bitstream.len() as c_int), &sci, 12);
}

#[test]
fn test_ba_ge_17_various() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    let mut bitstream = vec![0u8; 4096];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = ((i * 211 + 23) & 0xff) as u8;
    }

    // Try ba values 17, 18, 19 which take the grouped path.
    let bitalloc_vals: Vec<u8> = vec![17, 18, 19, 17, 18, 19, 17, 18];
    let sci = make_sci(4, &bitalloc_vals);
    run_pair(c_fn, r_fn, &bitstream, 0, 8 * (bitstream.len() as c_int), &sci, 12);
}

#[test]
fn test_mixed_bitalloc() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    let mut bitstream = vec![0u8; 8192];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = ((i ^ (i >> 3) ^ 0x5a) & 0xff) as u8;
    }

    // Mix zero, small ba, and grouped ba
    let bitalloc_vals: Vec<u8> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        0, 5, 17, 0, 9, 18, 0, 0, 16, 17, 1, 2,
    ];
    let sci = make_sci(16, &bitalloc_vals); // 32 bands
    run_pair(c_fn, r_fn, &bitstream, 0, 8 * (bitstream.len() as c_int), &sci, 12);
}

#[test]
fn test_group_sizes() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    let mut bitstream = vec![0u8; 8192];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = ((i * 17 + 3) & 0xff) as u8;
    }

    let bitalloc_vals: Vec<u8> = vec![1, 2, 17, 0, 8, 18, 4, 16];
    let sci = make_sci(4, &bitalloc_vals);

    for &gs in &[1, 2, 3, 4, 6, 8, 12] {
        run_pair(c_fn, r_fn, &bitstream, 0, 8 * (bitstream.len() as c_int), &sci, gs);
    }
}

#[test]
fn test_starting_pos_offsets() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    let mut bitstream = vec![0u8; 4096];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = ((i * 23 + 11) & 0xff) as u8;
    }

    let bitalloc_vals: Vec<u8> = vec![3, 5, 7, 17, 18, 4];
    let sci = make_sci(3, &bitalloc_vals);

    for &start in &[0, 1, 3, 7, 8, 15, 16, 31] {
        run_pair(
            c_fn,
            r_fn,
            &bitstream,
            start,
            8 * (bitstream.len() as c_int),
            &sci,
            6,
        );
    }
}

#[test]
fn test_limit_exhaustion() {
    // Force `bs.pos > bs.limit` partway through to exercise the early-return
    // inside get_bits.
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };
    let c_fn = unsafe { *load_dequant(&c_lib) };
    let r_fn = unsafe { *load_dequant(&r_lib) };

    let mut bitstream = vec![0u8; 4096];
    for (i, b) in bitstream.iter_mut().enumerate() {
        *b = (i & 0xff) as u8;
    }

    let bitalloc_vals: Vec<u8> = vec![5, 7, 17, 9, 18, 3];
    let sci = make_sci(3, &bitalloc_vals);

    // A small limit will trip the get_bits early-return after some calls.
    for &lim in &[0, 1, 8, 16, 32, 64, 128, 256] {
        run_pair(c_fn, r_fn, &bitstream, 0, lim, &sci, 12);
    }
}
