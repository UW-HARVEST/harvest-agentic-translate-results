use libloading::{Library, Symbol};
use std::os::raw::c_int;

// Mirror the C structs exactly
#[repr(C)]
struct bs_t {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
struct L12_scale_info {
    scf: [f32; 192],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

type DequantizeGranuleFn =
    unsafe extern "C" fn(*mut f32, *mut bs_t, *mut L12_scale_info, c_int) -> c_int;

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libtranslated_rust.so")
}

fn make_sci(total_bands: u8, bitalloc: &[u8]) -> L12_scale_info {
    let mut sci = L12_scale_info {
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

/// Run dequantize_granule through both C and Rust, compare outputs.
fn run_compare(
    buf: &[u8],
    total_bands: u8,
    bitalloc: &[u8],
    group_size: i32,
) {
    let limit = (buf.len() * 8) as c_int;

    // --- C version ---
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let c_fn: Symbol<DequantizeGranuleFn> =
        unsafe { c_lib.get(b"dequantize_granule").unwrap() };

    let mut c_grbuf = vec![0.0f32; 2304]; // 576*4 max
    let mut c_bs = bs_t { buf: buf.as_ptr(), pos: 0, limit };
    let mut c_sci = make_sci(total_bands, bitalloc);
    let c_ret = unsafe { c_fn(c_grbuf.as_mut_ptr(), &mut c_bs, &mut c_sci, group_size) };
    let c_pos = c_bs.pos;

    // --- Rust version ---
    let mut r_grbuf = vec![0.0f32; 2304];
    let mut r_bs = dequantize_granule_lib::bs_t {
        buf: buf.as_ptr(),
        pos: 0,
        limit,
    };
    let mut r_sci = dequantize_granule_lib::L12_scale_info {
        scf: [0.0; 192],
        total_bands,
        stereo_bands: 0,
        bitalloc: [0; 64],
        scfcod: [0; 64],
    };
    for (i, &b) in bitalloc.iter().enumerate() {
        r_sci.bitalloc[i] = b;
    }
    let r_ret = unsafe {
        dequantize_granule_lib::dequantize_granule(
            r_grbuf.as_mut_ptr(),
            &mut r_bs,
            &mut r_sci,
            group_size,
        )
    };
    let r_pos = r_bs.pos;

    // Compare
    assert_eq!(c_ret, r_ret, "return value mismatch");
    assert_eq!(c_pos, r_pos, "bs.pos mismatch");

    // Compare float buffers byte-for-byte
    let c_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(c_grbuf.as_ptr() as *const u8, c_grbuf.len() * 4)
    };
    let r_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(r_grbuf.as_ptr() as *const u8, r_grbuf.len() * 4)
    };
    if c_bytes != r_bytes {
        // Find first mismatch for debugging
        for i in 0..c_grbuf.len() {
            if c_grbuf[i].to_bits() != r_grbuf[i].to_bits() {
                panic!(
                    "grbuf mismatch at index {}: C={} Rust={} (C bits={:#010x} Rust bits={:#010x})",
                    i, c_grbuf[i], r_grbuf[i], c_grbuf[i].to_bits(), r_grbuf[i].to_bits()
                );
            }
        }
    }
}

#[test]
fn test_all_zero_bitalloc() {
    // All bitalloc=0 means no bits read, output stays zero
    let buf = vec![0xABu8; 64];
    run_compare(&buf, 2, &[0; 4], 1);
}

#[test]
fn test_ba_small_values() {
    // ba=1..16 path: read ba bits, subtract half
    // total_bands=1 means 2 entries in bitalloc used per iteration
    // 4 iterations * 2 entries = 8 calls with ba=4
    // Each call reads 4 bits * group_size times
    // group_size=1: 8 reads of 4 bits = 32 bits = 4 bytes
    let buf = vec![0xA5u8; 64];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 4;
    bitalloc[1] = 4;
    run_compare(&buf, 1, &bitalloc, 1);
}

#[test]
fn test_ba_large_values() {
    // ba=17 path: grouped decoding with mod
    // ba=17 → mod = (2<<0)+1 = 3, bits = 3+2-(3>>3) = 5-0 = 5
    let buf = vec![0x55u8; 128];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 17;
    bitalloc[1] = 17;
    run_compare(&buf, 1, &bitalloc, 1);
}

#[test]
fn test_mixed_ba() {
    // Mix of ba=0, ba<17, ba>=17
    let buf: Vec<u8> = (0..256).map(|i| (i & 0xFF) as u8).collect();
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 0;
    bitalloc[1] = 8;
    bitalloc[2] = 17;
    bitalloc[3] = 0;
    run_compare(&buf, 2, &bitalloc, 1);
}

#[test]
fn test_group_size_3() {
    // group_size=3 exercises the inner k loop more
    let buf: Vec<u8> = (0..512).map(|i| ((i * 37) & 0xFF) as u8).collect();
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 5;
    bitalloc[1] = 10;
    run_compare(&buf, 1, &bitalloc, 3);
}

#[test]
fn test_multiple_bands() {
    // total_bands=4 → 8 bitalloc entries per j iteration
    let buf: Vec<u8> = (0..1024).map(|i| ((i * 53 + 17) & 0xFF) as u8).collect();
    let mut bitalloc = [0u8; 64];
    for i in 0..8 {
        bitalloc[i] = ((i % 3) + 1) as u8; // ba = 1, 2, 3, 1, 2, 3, 1, 2
    }
    run_compare(&buf, 4, &bitalloc, 1);
}

#[test]
fn test_ba_16_boundary() {
    // ba=16 is the max for the "small" path
    let buf = vec![0xFFu8; 256];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 16;
    bitalloc[1] = 16;
    run_compare(&buf, 1, &bitalloc, 1);
}

#[test]
fn test_ba_18() {
    // ba=18 → mod = (2<<1)+1 = 5, bits = 5+2-(5>>3) = 7-0 = 7
    let buf: Vec<u8> = (0..256).map(|i| ((i * 97) & 0xFF) as u8).collect();
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 18;
    bitalloc[1] = 18;
    run_compare(&buf, 1, &bitalloc, 3);
}
