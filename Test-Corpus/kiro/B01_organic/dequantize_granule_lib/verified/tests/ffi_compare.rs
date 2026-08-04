use libloading::{Library, Symbol};
use std::ffi::c_int;

#[repr(C)]
struct BsT {
    buf: *const u8,
    pos: c_int,
    limit: c_int,
}

#[repr(C)]
struct L12ScaleInfo {
    scf: [f32; 192],
    total_bands: u8,
    stereo_bands: u8,
    bitalloc: [u8; 64],
    scfcod: [u8; 64],
}

type DequantizeGranuleFn =
    unsafe extern "C" fn(*mut f32, *mut BsT, *mut L12ScaleInfo, c_int) -> c_int;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdequantize_granule_lib.so");
    unsafe { Library::new(path).expect("failed to load Rust .so") }
}

fn load_fn(lib: &Library) -> Symbol<DequantizeGranuleFn> {
    unsafe { lib.get(b"dequantize_granule").expect("symbol not found") }
}

fn run_both(
    buf: &[u8],
    total_bands: u8,
    bitalloc: &[u8; 64],
    group_size: c_int,
) {
    // Need enough output space: 4 groups, each group has interleaved channels
    // Max offset: group_size*3 + (2*total_bands-1) * max(576, 18-576) ... 
    // Safe upper bound: 576*4 = 2304 floats
    let mut grbuf_c = vec![0.0f32; 2304];
    let mut grbuf_r = vec![0.0f32; 2304];

    let mut bs_c = BsT { buf: buf.as_ptr(), pos: 0, limit: (buf.len() * 8) as c_int };
    let mut bs_r = BsT { buf: buf.as_ptr(), pos: 0, limit: (buf.len() * 8) as c_int };

    let mut sci_c = L12ScaleInfo {
        scf: [0.0; 192],
        total_bands: total_bands,
        stereo_bands: 0,
        bitalloc: *bitalloc,
        scfcod: [0; 64],
    };
    let mut sci_r = sci_c.clone_into_new();

    let c = c_lib();
    let r = rust_lib();
    let c_fn = load_fn(&c);
    let r_fn = load_fn(&r);

    let ret_c = unsafe { c_fn(grbuf_c.as_mut_ptr(), &mut bs_c, &mut sci_c, group_size) };
    let ret_r = unsafe { r_fn(grbuf_r.as_mut_ptr(), &mut bs_r, &mut sci_r, group_size) };

    assert_eq!(ret_c, ret_r, "return values differ");
    assert_eq!(bs_c.pos, bs_r.pos, "bs.pos differs after call");

    // Compare output buffers byte-for-byte
    let c_bytes = unsafe {
        std::slice::from_raw_parts(grbuf_c.as_ptr() as *const u8, grbuf_c.len() * 4)
    };
    let r_bytes = unsafe {
        std::slice::from_raw_parts(grbuf_r.as_ptr() as *const u8, grbuf_r.len() * 4)
    };
    if c_bytes != r_bytes {
        // Find first mismatch as float index
        for i in 0..grbuf_c.len() {
            if grbuf_c[i].to_bits() != grbuf_r[i].to_bits() {
                panic!(
                    "mismatch at float index {}: C={} Rust={} (bits: C={:#010x} R={:#010x})",
                    i, grbuf_c[i], grbuf_r[i], grbuf_c[i].to_bits(), grbuf_r[i].to_bits()
                );
            }
        }
    }
}

impl L12ScaleInfo {
    fn clone_into_new(&self) -> Self {
        L12ScaleInfo {
            scf: self.scf,
            total_bands: self.total_bands,
            stereo_bands: self.stereo_bands,
            bitalloc: self.bitalloc,
            scfcod: self.scfcod,
        }
    }
}

#[test]
fn test_zero_bands() {
    let buf = [0u8; 64];
    let bitalloc = [0u8; 64];
    run_both(&buf, 0, &bitalloc, 3);
}

#[test]
fn test_ba_below_17() {
    // total_bands=1 means 2 bands iterated (2*total_bands), group_size=3
    // ba=4 for first two bands: reads 4 bits per sample, half = (1<<3)-1 = 7
    let buf = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A,
               0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55,
               0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
               0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 4;
    bitalloc[1] = 8;
    run_both(&buf, 1, &bitalloc, 3);
}

#[test]
fn test_ba_17_and_above() {
    // ba=17: mod = (2<<0)+1 = 3, bits = 3+2-(3>>3) = 5
    // ba=18: mod = (2<<1)+1 = 5, bits = 5+2-(5>>3) = 7
    let buf = [0xFF; 64];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 17;
    bitalloc[1] = 18;
    run_both(&buf, 1, &bitalloc, 3);
}

#[test]
fn test_mixed_ba_values() {
    let buf: Vec<u8> = (0..128).collect();
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 1;   // ba=1: 1 bit, half=0
    bitalloc[1] = 16;  // ba=16: 16 bits, half=32767
    bitalloc[2] = 17;  // mod path
    bitalloc[3] = 0;   // skip
    run_both(&buf, 2, &bitalloc, 3);
}

#[test]
fn test_group_size_1() {
    let buf = [0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA,
               0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 5;
    bitalloc[1] = 10;
    run_both(&buf, 1, &bitalloc, 1);
}

#[test]
fn test_many_bands() {
    // total_bands=4 means 8 bands iterated
    let buf: Vec<u8> = (0u8..=255).cycle().take(256).collect();
    let mut bitalloc = [0u8; 64];
    for i in 0..8 {
        bitalloc[i] = ((i % 16) + 1) as u8;
    }
    run_both(&buf, 4, &bitalloc, 3);
}

#[test]
fn test_limit_exceeded() {
    // Very small buffer - get_bits should return 0 when pos > limit
    let buf = [0xFF; 2];
    let mut bitalloc = [0u8; 64];
    bitalloc[0] = 16; // tries to read 16 bits but limit is only 16 bits total
    bitalloc[1] = 8;
    run_both(&buf, 1, &bitalloc, 3);
}
