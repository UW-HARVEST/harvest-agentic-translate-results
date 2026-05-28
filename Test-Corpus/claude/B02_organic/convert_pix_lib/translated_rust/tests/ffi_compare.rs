// Integration tests that load the C .so and the Rust .so via libloading,
// invoke their exported FFI symbols, and compare the results byte-for-byte.

use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

type ConvertPixFn =
    unsafe extern "C" fn(bpp: i32, w: i32, h: i32, src: *mut u8, dst: *mut CpPixel);
type CpInflateFn = unsafe extern "C" fn(
    in_ptr: *mut std::ffi::c_void,
    in_bytes: i32,
    out_ptr: *mut std::ffi::c_void,
    out_bytes: i32,
) -> i32;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("release");
    p.push("libconvert_pix_lib.so");
    p
}

fn load_c() -> Library {
    unsafe { Library::new(c_so_path()).expect("load C .so") }
}
fn load_rust() -> Library {
    unsafe { Library::new(rust_so_path()).expect("load Rust .so") }
}

unsafe fn read_static<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let sym: Symbol<*const T> = lib.get(name).expect("symbol");
    *(*sym)
}

#[test]
fn static_tables_match() {
    unsafe {
        let c = load_c();
        let r = load_rust();

        let c_fixed: [u8; 320] = read_static(&c, b"cp_fixed_table\0");
        let r_fixed: [u8; 320] = read_static(&r, b"cp_fixed_table\0");
        assert_eq!(&c_fixed[..], &r_fixed[..], "cp_fixed_table mismatch");

        let c_perm: [u8; 19] = read_static(&c, b"cp_permutation_order\0");
        let r_perm: [u8; 19] = read_static(&r, b"cp_permutation_order\0");
        assert_eq!(&c_perm[..], &r_perm[..], "cp_permutation_order mismatch");

        let c_lex: [u8; 31] = read_static(&c, b"cp_len_extra_bits\0");
        let r_lex: [u8; 31] = read_static(&r, b"cp_len_extra_bits\0");
        assert_eq!(&c_lex[..], &r_lex[..], "cp_len_extra_bits mismatch");

        let c_lb: [u32; 31] = read_static(&c, b"cp_len_base\0");
        let r_lb: [u32; 31] = read_static(&r, b"cp_len_base\0");
        assert_eq!(&c_lb[..], &r_lb[..], "cp_len_base mismatch");

        let c_dex: [u8; 32] = read_static(&c, b"cp_dist_extra_bits\0");
        let r_dex: [u8; 32] = read_static(&r, b"cp_dist_extra_bits\0");
        assert_eq!(&c_dex[..], &r_dex[..], "cp_dist_extra_bits mismatch");

        let c_db: [u32; 32] = read_static(&c, b"cp_dist_base\0");
        let r_db: [u32; 32] = read_static(&r, b"cp_dist_base\0");
        assert_eq!(&c_db[..], &r_db[..], "cp_dist_base mismatch");
    }
}

// Helper: call convert_pix on both libs and assert outputs match.
fn convert_pix_compare(bpp: i32, w: i32, h: i32, raw: &[u8]) {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_fn: Symbol<ConvertPixFn> = c.get(b"convert_pix\0").unwrap();
        let r_fn: Symbol<ConvertPixFn> = r.get(b"convert_pix\0").unwrap();

        let mut src_c = raw.to_vec();
        let mut src_r = raw.to_vec();

        let n = (w * h) as usize;
        let mut dst_c: Vec<CpPixel> = vec![CpPixel { r: 0, g: 0, b: 0, a: 0 }; n];
        let mut dst_r: Vec<CpPixel> = vec![CpPixel { r: 0, g: 0, b: 0, a: 0 }; n];

        c_fn(bpp, w, h, src_c.as_mut_ptr(), dst_c.as_mut_ptr());
        r_fn(bpp, w, h, src_r.as_mut_ptr(), dst_r.as_mut_ptr());

        assert_eq!(dst_c, dst_r, "convert_pix bpp={} w={} h={}", bpp, w, h);
    }
}

#[test]
fn convert_pix_bpp1_simple() {
    // Each row has 1 leading filter byte then w pixels of 1 byte each.
    let w = 4;
    let h = 3;
    let bpp = 1;
    let mut raw: Vec<u8> = Vec::new();
    for y in 0..h {
        raw.push(0x10 + y as u8); // filter byte (skipped)
        for x in 0..w {
            raw.push((y * w + x) as u8);
        }
    }
    convert_pix_compare(bpp, w as i32, h as i32, &raw);
}

#[test]
fn convert_pix_bpp2() {
    let w = 5;
    let h = 4;
    let bpp = 2;
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0); // filter
        for x in 0..w {
            raw.push((y * 11 + x) as u8);
            raw.push((x * 7 + y) as u8);
        }
    }
    convert_pix_compare(bpp, w as i32, h as i32, &raw);
}

#[test]
fn convert_pix_bpp3() {
    let w = 8;
    let h = 6;
    let bpp = 3;
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0); // filter
        for x in 0..w {
            raw.push((x as u8).wrapping_mul(17));
            raw.push((y as u8).wrapping_mul(31));
            raw.push((x + y) as u8);
        }
    }
    convert_pix_compare(bpp, w as i32, h as i32, &raw);
}

#[test]
fn convert_pix_bpp4() {
    let w = 6;
    let h = 5;
    let bpp = 4;
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0); // filter
        for x in 0..w {
            raw.push((x as u8).wrapping_mul(11));
            raw.push((y as u8).wrapping_mul(13));
            raw.push((x + y) as u8);
            raw.push(((x ^ y) as u8).wrapping_mul(5));
        }
    }
    convert_pix_compare(bpp, w as i32, h as i32, &raw);
}

#[test]
fn convert_pix_zero_dim() {
    // Zero rows, zero columns: should be a no-op.
    convert_pix_compare(4, 0, 0, &[0]);
}

#[test]
fn convert_pix_single_row() {
    let w = 16;
    let h = 1;
    let bpp = 4;
    let mut raw = vec![0u8];
    for x in 0..w {
        raw.push((x as u8).wrapping_mul(7));
        raw.push((x as u8).wrapping_mul(11));
        raw.push((x as u8).wrapping_mul(13));
        raw.push((x as u8).wrapping_mul(17));
    }
    convert_pix_compare(bpp, w as i32, h as i32, &raw);
}

// Deflate (raw, no zlib wrapper) inputs for cp_inflate.
fn deflate_raw(data: &[u8], level: u32) -> Vec<u8> {
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn cp_inflate_compare(input: &[u8], out_len: usize) {
    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_fn: Symbol<CpInflateFn> = c.get(b"cp_inflate\0").unwrap();
        let r_fn: Symbol<CpInflateFn> = r.get(b"cp_inflate\0").unwrap();

        // Allocate input buffers as Vec<u8> so they have similar alignments
        // (the function deals with internal alignment quirks anyway).
        let mut in_c = input.to_vec();
        let mut in_r = input.to_vec();
        let mut out_c = vec![0u8; out_len];
        let mut out_r = vec![0u8; out_len];

        let rc_c = c_fn(
            in_c.as_mut_ptr() as *mut _,
            in_c.len() as i32,
            out_c.as_mut_ptr() as *mut _,
            out_len as i32,
        );
        let rc_r = r_fn(
            in_r.as_mut_ptr() as *mut _,
            in_r.len() as i32,
            out_r.as_mut_ptr() as *mut _,
            out_len as i32,
        );
        assert_eq!(rc_c, rc_r, "cp_inflate return value differs");
        if rc_c == 1 {
            assert_eq!(out_c, out_r, "cp_inflate output differs");
        }
    }
}

#[test]
fn cp_inflate_dynamic_text() {
    // Long-ish, repeated-ish text -> dynamic huffman block.
    let text: Vec<u8> = (0..2000)
        .map(|i| ((i * 37 + (i % 13)) & 0xFF) as u8)
        .collect();
    let compressed = deflate_raw(&text, 9);
    cp_inflate_compare(&compressed, text.len());
}

#[test]
fn cp_inflate_short_fixed() {
    let text = b"Hello, world! Hello, world!".to_vec();
    let compressed = deflate_raw(&text, 6);
    cp_inflate_compare(&compressed, text.len());
}

#[test]
fn cp_inflate_stored_block() {
    // level 0 -> stored (uncompressed) blocks.
    let text: Vec<u8> = (0..500).map(|i| (i as u8).wrapping_mul(7)).collect();
    let compressed = deflate_raw(&text, 0);
    cp_inflate_compare(&compressed, text.len());
}

#[test]
fn cp_inflate_repeated_pattern() {
    // Highly repetitive data -> exercises long backreferences.
    let mut data = Vec::new();
    for _ in 0..100 {
        data.extend_from_slice(b"abcdefghij");
    }
    let compressed = deflate_raw(&data, 6);
    cp_inflate_compare(&compressed, data.len());
}

#[test]
fn cp_inflate_distance_one() {
    // Long run of a single byte exercises the backwards_distance == 1 path.
    let data = vec![0xABu8; 1024];
    let compressed = deflate_raw(&data, 9);
    cp_inflate_compare(&compressed, data.len());
}

#[test]
fn cp_inflate_unaligned_input() {
    // Force the input buffer pointer to be unaligned, so the
    // first_bytes / last_bytes branches are exercised.
    let text = b"DEFLATE alignment exercising payload payload payload payload payload".to_vec();
    let compressed = deflate_raw(&text, 6);

    unsafe {
        let c = load_c();
        let r = load_rust();
        let c_fn: Symbol<CpInflateFn> = c.get(b"cp_inflate\0").unwrap();
        let r_fn: Symbol<CpInflateFn> = r.get(b"cp_inflate\0").unwrap();

        for offset in 0..4 {
            let mut buf_c = vec![0u8; compressed.len() + 8];
            let mut buf_r = vec![0u8; compressed.len() + 8];
            buf_c[offset..offset + compressed.len()].copy_from_slice(&compressed);
            buf_r[offset..offset + compressed.len()].copy_from_slice(&compressed);

            let mut out_c = vec![0u8; text.len()];
            let mut out_r = vec![0u8; text.len()];

            let rc_c = c_fn(
                buf_c.as_mut_ptr().add(offset) as *mut _,
                compressed.len() as i32,
                out_c.as_mut_ptr() as *mut _,
                text.len() as i32,
            );
            let rc_r = r_fn(
                buf_r.as_mut_ptr().add(offset) as *mut _,
                compressed.len() as i32,
                out_r.as_mut_ptr() as *mut _,
                text.len() as i32,
            );
            assert_eq!(rc_c, rc_r, "rc mismatch at offset {}", offset);
            if rc_c == 1 {
                assert_eq!(out_c, out_r, "output mismatch at offset {}", offset);
                assert_eq!(&out_c[..], &text[..], "output not equal to original");
            }
        }
    }
}
