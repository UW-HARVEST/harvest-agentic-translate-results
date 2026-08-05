//! Phase B: full write->read roundtrip differential tests.
//!
//! A small C harness (`tests/harness.c`, compiled to `tests/libharness.so`)
//! dlopen()s the libpng-under-test and drives a complete encode+decode. We run
//! it once against the reference C `libpng.so` and once against the Rust
//! `liblibpng.so`, then assert BOTH the encoded byte stream AND the decoded
//! pixels are byte-identical. This exercises the whole read/write pipeline,
//! including the newly translated `png_handle_chunk`/`png_handle_unknown`.

mod common;
use common::{c_so_path, crate_root, rust_so_path, Rng};

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_int, c_uint};
use std::ptr;

#[allow(non_camel_case_types)]
type HarnessRoundtrip = unsafe extern "C" fn(
    lib_path: *const std::os::raw::c_char,
    width: c_uint,
    height: c_uint,
    bit_depth: c_int,
    color_type: c_int,
    interlace: c_int,
    filters: c_int,
    compression_level: c_int,
    use_phys: c_int,
    src_flat: *const u8,
    src_rowbytes: usize,
    palette_flat: *const u8,
    num_palette: c_int,
    enc_out: *mut *mut u8,
    enc_len: *mut usize,
    dec_rows_flat: *mut u8,
    dec_row_cap: usize,
    dec_w: *mut c_uint,
    dec_h: *mut c_uint,
    dec_bd: *mut c_int,
    dec_ct: *mut c_int,
    dec_il: *mut c_int,
    dec_rowbytes: *mut usize,
) -> c_int;

const GRAY: c_int = 0;
const PALETTE: c_int = 3;
const RGB: c_int = 2;
const RGB_ALPHA: c_int = 6;
const GRAY_ALPHA: c_int = 4;

fn channels(color_type: c_int) -> usize {
    match color_type {
        GRAY => 1,
        RGB => 3,
        PALETTE => 1,
        GRAY_ALPHA => 2,
        RGB_ALPHA => 4,
        _ => 1,
    }
}

fn rowbytes(width: u32, bit_depth: c_int, color_type: c_int) -> usize {
    let ch = channels(color_type);
    let bits = width as usize * ch * bit_depth as usize;
    (bits + 7) / 8
}

struct RunResult {
    ret: c_int,
    enc: Vec<u8>,
    dec: Vec<u8>,
    dec_rowbytes: usize,
    dec_w: u32,
    dec_h: u32,
    dec_bd: c_int,
    dec_ct: c_int,
    dec_il: c_int,
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_one(
    harness: &Symbol<HarnessRoundtrip>,
    lib_path: &str,
    width: u32,
    height: u32,
    bit_depth: c_int,
    color_type: c_int,
    interlace: c_int,
    filters: c_int,
    compression_level: c_int,
    use_phys: c_int,
    src: &[u8],
    src_rowbytes: usize,
    palette: &[u8],
    num_palette: c_int,
) -> RunResult {
    let path = CString::new(lib_path).unwrap();
    let mut enc_out: *mut u8 = ptr::null_mut();
    let mut enc_len: usize = 0;
    // generous decode buffer: height * rowbytes rounded up, plus slack
    let per_row = src_rowbytes + 16;
    let dec_cap = per_row * height as usize;
    let mut dec = vec![0u8; dec_cap.max(16)];
    let mut dw = 0u32;
    let mut dh = 0u32;
    let mut dbd = 0;
    let mut dct = 0;
    let mut dil = 0;
    let mut drb = 0usize;

    let ret = harness(
        path.as_ptr(),
        width,
        height,
        bit_depth,
        color_type,
        interlace,
        filters,
        compression_level,
        use_phys,
        src.as_ptr(),
        src_rowbytes,
        if palette.is_empty() {
            ptr::null()
        } else {
            palette.as_ptr()
        },
        num_palette,
        &mut enc_out,
        &mut enc_len,
        dec.as_mut_ptr(),
        dec.len(),
        &mut dw,
        &mut dh,
        &mut dbd,
        &mut dct,
        &mut dil,
        &mut drb,
    );

    let enc = if !enc_out.is_null() && enc_len > 0 {
        let s = std::slice::from_raw_parts(enc_out, enc_len).to_vec();
        libc_free(enc_out);
        s
    } else {
        Vec::new()
    };

    RunResult {
        ret,
        enc,
        dec,
        dec_rowbytes: drb,
        dec_w: dw,
        dec_h: dh,
        dec_bd: dbd,
        dec_ct: dct,
        dec_il: dil,
    }
}

unsafe fn libc_free(p: *mut u8) {
    extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    free(p as *mut _);
}

fn load_harness() -> Library {
    let p = crate_root().join("tests/libharness.so");
    unsafe { Library::new(&p).unwrap_or_else(|e| panic!("load harness {:?}: {e}", p)) }
}

#[allow(clippy::too_many_arguments)]
fn differential_case(
    label: &str,
    width: u32,
    height: u32,
    bit_depth: c_int,
    color_type: c_int,
    interlace: c_int,
    filters: c_int,
    compression_level: c_int,
    use_phys: c_int,
    seed: u64,
) {
    let harness_lib = load_harness();
    let harness: Symbol<HarnessRoundtrip> =
        unsafe { harness_lib.get(b"harness_roundtrip").unwrap() };

    let rb = rowbytes(width, bit_depth, color_type);
    let mut rng = Rng::new(seed);
    let mut src = vec![0u8; rb * height as usize];
    for b in src.iter_mut() {
        *b = rng.next_u8();
    }
    // For sub-byte depths, unused trailing bits in a row must be consistent;
    // libpng ignores them on write but let's leave random (both libs treat
    // identically). For palette, indices must be < num_palette.
    let (palette, num_palette) = if color_type == PALETTE {
        let np = match bit_depth {
            1 => 2,
            2 => 4,
            4 => 16,
            _ => 256,
        };
        let mut pal = vec![0u8; np * 3];
        for b in pal.iter_mut() {
            *b = rng.next_u8();
        }
        // clamp indices in src to < np (only matters at bit_depth 8 with np<256;
        // for np==256 every byte is a valid index).
        if bit_depth == 8 && np < 256 {
            for b in src.iter_mut() {
                *b %= np as u8;
            }
        }
        (pal, np as c_int)
    } else {
        (Vec::new(), 0)
    };

    let c_path = c_so_path();
    let r_path = rust_so_path();

    let cres = unsafe {
        run_one(
            &harness,
            c_path.to_str().unwrap(),
            width,
            height,
            bit_depth,
            color_type,
            interlace,
            filters,
            compression_level,
            use_phys,
            &src,
            rb,
            &palette,
            num_palette,
        )
    };
    let rres = unsafe {
        run_one(
            &harness,
            r_path.to_str().unwrap(),
            width,
            height,
            bit_depth,
            color_type,
            interlace,
            filters,
            compression_level,
            use_phys,
            &src,
            rb,
            &palette,
            num_palette,
        )
    };

    assert_eq!(cres.ret, rres.ret, "[{label}] return codes differ (C={}, Rust={})", cres.ret, rres.ret);
    assert_eq!(cres.ret, 0, "[{label}] C harness returned error {}", cres.ret);

    // encoded bytes must be byte-identical
    assert_eq!(
        cres.enc.len(),
        rres.enc.len(),
        "[{label}] encoded length differs C={} Rust={}",
        cres.enc.len(),
        rres.enc.len()
    );
    assert!(cres.enc == rres.enc, "[{label}] encoded PNG bytes differ");

    // decoded metadata
    assert_eq!(cres.dec_w, rres.dec_w, "[{label}] decoded width");
    assert_eq!(cres.dec_h, rres.dec_h, "[{label}] decoded height");
    assert_eq!(cres.dec_bd, rres.dec_bd, "[{label}] decoded bit depth");
    assert_eq!(cres.dec_ct, rres.dec_ct, "[{label}] decoded color type");
    assert_eq!(cres.dec_il, rres.dec_il, "[{label}] decoded interlace");
    assert_eq!(cres.dec_rowbytes, rres.dec_rowbytes, "[{label}] decoded rowbytes");

    // decoded pixels: compare the valid region per row
    let drb = cres.dec_rowbytes;
    let per_row_c = (cres.dec.len()) / (height as usize).max(1);
    let per_row_r = (rres.dec.len()) / (height as usize).max(1);
    // Only compare decoded-vs-source when each row occupies whole bytes; for
    // sub-byte depths the trailing partial byte's unused bits are zeroed by
    // libpng on decode, so source != decode there (both libs identically). The
    // C-vs-Rust comparison below is the differential assertion that matters.
    let whole_byte_rows = (width as usize * channels(color_type) * bit_depth as usize) % 8 == 0;
    for row in 0..height as usize {
        let cs = &cres.dec[row * per_row_c..row * per_row_c + drb];
        let rs = &rres.dec[row * per_row_r..row * per_row_r + drb];
        assert!(cs == rs, "[{label}] decoded row {row} differs (C vs Rust)");
        if whole_byte_rows {
            let ss = &src[row * rb..row * rb + rb.min(drb)];
            assert_eq!(&cs[..ss.len()], ss, "[{label}] C decode != source row {row}");
        }
    }
}

// CONFIGS row 11: GRAY, various bit depths
#[test]
fn gray_depths() {
    for &bd in &[1i32, 2, 4, 8, 16] {
        differential_case(
            &format!("gray bd{bd}"),
            17,
            9,
            bd,
            GRAY,
            0,
            -1,
            6,
            0,
            100 + bd as u64,
        );
    }
}

// CONFIGS row 12: RGB
#[test]
fn rgb_depths() {
    for &bd in &[8i32, 16] {
        differential_case(&format!("rgb bd{bd}"), 13, 11, bd, RGB, 0, -1, 6, 0, 200 + bd as u64);
    }
}

// CONFIGS row 13: PALETTE
#[test]
fn palette_depths() {
    for &bd in &[1i32, 2, 4, 8] {
        differential_case(
            &format!("palette bd{bd}"),
            20,
            7,
            bd,
            PALETTE,
            0,
            -1,
            6,
            0,
            300 + bd as u64,
        );
    }
}

// CONFIGS row 14: GRAY_ALPHA
#[test]
fn gray_alpha() {
    for &bd in &[8i32, 16] {
        differential_case(&format!("ga bd{bd}"), 10, 10, bd, GRAY_ALPHA, 0, -1, 6, 0, 400 + bd as u64);
    }
}

// CONFIGS row 15: RGB_ALPHA
#[test]
fn rgb_alpha() {
    for &bd in &[8i32, 16] {
        differential_case(&format!("rgba bd{bd}"), 12, 8, bd, RGB_ALPHA, 0, -1, 6, 0, 500 + bd as u64);
    }
}

// CONFIGS row 16: interlace ADAM7
#[test]
fn interlaced() {
    differential_case("rgb8 adam7", 15, 13, 8, RGB, 1, -1, 6, 0, 600);
    differential_case("gray8 adam7", 15, 13, 8, GRAY, 1, -1, 6, 0, 601);
    differential_case("rgba8 adam7", 9, 9, 8, RGB_ALPHA, 1, -1, 6, 0, 602);
}

// CONFIGS row 19: varying widths/heights
#[test]
fn varying_sizes() {
    let mut rng = Rng::new(700);
    for _ in 0..12 {
        let w = rng.range(1, 40);
        let h = rng.range(1, 30);
        differential_case(&format!("rgb8 {w}x{h}"), w, h, 8, RGB, 0, -1, 6, 0, 700 + (w * 100 + h) as u64);
    }
}

// CONFIGS row 20: filters and compression levels
#[test]
fn filters_and_levels() {
    // PNG_ALL_FILTERS = 0xF8 ; individual: NONE=0x08, SUB=0x10, UP=0x20, AVG=0x40, PAETH=0x80
    for &f in &[0x08i32, 0x10, 0x20, 0x40, 0x80, 0xF8] {
        differential_case(&format!("rgb8 filter{f:#x}"), 16, 12, 8, RGB, 0, f, 6, 0, 800 + f as u64);
    }
    for &lvl in &[0i32, 1, 3, 6, 9] {
        differential_case(&format!("rgb8 lvl{lvl}"), 16, 12, 8, RGB, 0, -1, lvl, 0, 900 + lvl as u64);
    }
}

// CONFIGS row 17: ancillary chunk (pHYs) present
#[test]
fn with_phys_chunk() {
    differential_case("rgb8 phys", 14, 10, 8, RGB, 0, -1, 6, 1, 1000);
    differential_case("gray8 phys", 14, 10, 8, GRAY, 0, -1, 6, 1, 1001);
}
