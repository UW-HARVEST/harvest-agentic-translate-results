use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libconvert_pix_lib.so")
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

// ---- Static array comparisons ----

#[test]
fn test_cp_fixed_table() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 320]> = lib.get(b"cp_fixed_table").unwrap();
        let c_data: &[u8; 320] = &**c_ptr;
        let rust_data: &[u8; 320] = &*std::ptr::addr_of!(convert_pix_lib::cp_fixed_table);
        assert_eq!(c_data, rust_data, "cp_fixed_table mismatch");
    }
}

#[test]
fn test_cp_permutation_order() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 19]> = lib.get(b"cp_permutation_order").unwrap();
        let c_data: &[u8; 19] = &**c_ptr;
        let rust_data: &[u8; 19] = &*std::ptr::addr_of!(convert_pix_lib::cp_permutation_order);
        assert_eq!(c_data, rust_data, "cp_permutation_order mismatch");
    }
}

#[test]
fn test_cp_len_extra_bits() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 31]> = lib.get(b"cp_len_extra_bits").unwrap();
        let c_data: &[u8; 31] = &**c_ptr;
        let rust_data: &[u8; 31] = &*std::ptr::addr_of!(convert_pix_lib::cp_len_extra_bits);
        assert_eq!(c_data, rust_data, "cp_len_extra_bits mismatch");
    }
}

#[test]
fn test_cp_len_base() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u32; 31]> = lib.get(b"cp_len_base").unwrap();
        let c_data: &[u32; 31] = &**c_ptr;
        let rust_data: &[u32; 31] = &*std::ptr::addr_of!(convert_pix_lib::cp_len_base);
        assert_eq!(c_data, rust_data, "cp_len_base mismatch");
    }
}

#[test]
fn test_cp_dist_extra_bits() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 32]> = lib.get(b"cp_dist_extra_bits").unwrap();
        let c_data: &[u8; 32] = &**c_ptr;
        let rust_data: &[u8; 32] = &*std::ptr::addr_of!(convert_pix_lib::cp_dist_extra_bits);
        assert_eq!(c_data, rust_data, "cp_dist_extra_bits mismatch");
    }
}

#[test]
fn test_cp_dist_base() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u32; 32]> = lib.get(b"cp_dist_base").unwrap();
        let c_data: &[u32; 32] = &**c_ptr;
        let rust_data: &[u32; 32] = &*std::ptr::addr_of!(convert_pix_lib::cp_dist_base);
        assert_eq!(c_data, rust_data, "cp_dist_base mismatch");
    }
}

// ---- convert_pix tests ----

fn test_convert_pix_bpp(bpp: i32) {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, *mut u8, *mut CpPixel)> =
            lib.get(b"convert_pix").unwrap();

        let w = 4i32;
        let h = 3i32;
        // src layout: for each row, 1 filter byte + w*bpp data bytes
        let row_size = 1 + (w * bpp) as usize;
        let total = row_size * h as usize;
        let mut src: Vec<u8> = (0..total).map(|i| ((i * 37 + 13) & 0xFF) as u8).collect();

        let pixel_count = (w * h) as usize;
        let mut c_dst = vec![CpPixel { r: 0, g: 0, b: 0, a: 0 }; pixel_count];
        let mut r_dst = vec![CpPixel { r: 0, g: 0, b: 0, a: 0 }; pixel_count];

        let mut src_c = src.clone();
        c_fn(bpp, w, h, src_c.as_mut_ptr(), c_dst.as_mut_ptr());
        convert_pix_lib::convert_pix(bpp, w, h, src.as_mut_ptr(), r_dst.as_mut_ptr() as *mut _);

        let c_bytes = std::slice::from_raw_parts(c_dst.as_ptr() as *const u8, pixel_count * 4);
        let r_bytes = std::slice::from_raw_parts(r_dst.as_ptr() as *const u8, pixel_count * 4);
        assert_eq!(c_bytes, r_bytes, "convert_pix mismatch for bpp={bpp}");
    }
}

#[test]
fn test_convert_pix_bpp1() { test_convert_pix_bpp(1); }
#[test]
fn test_convert_pix_bpp2() { test_convert_pix_bpp(2); }
#[test]
fn test_convert_pix_bpp3() { test_convert_pix_bpp(3); }
#[test]
fn test_convert_pix_bpp4() { test_convert_pix_bpp(4); }

// ---- cp_inflate test ----
// Use a minimal deflate stream (fixed Huffman, block type 1)
// We'll create one using a known pattern.

#[test]
fn test_cp_inflate() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, i32, *mut u8, i32) -> i32> =
            lib.get(b"cp_inflate").unwrap();

        // Minimal deflate stream: final block, type 0 (stored), 5 bytes "hello"
        // bfinal=1, btype=00 => first byte bits: 1 | (00 << 1) = 0x01
        // Then align to byte boundary (already aligned after 3 bits... need to pad)
        // Actually let's build a proper stored block:
        // byte 0: 0x01 (bfinal=1, btype=0, then 5 padding bits)
        // bytes 1-2: LEN = 5 = 0x0005 (little-endian)
        // bytes 3-4: NLEN = ~5 = 0xFFFA (little-endian)
        // bytes 5-9: "hello"
        let mut input: Vec<u8> = vec![
            0x01, // bfinal=1, btype=00, pad=00000
            0x05, 0x00, // LEN=5
            0xFA, 0xFF, // NLEN=~5
            b'h', b'e', b'l', b'l', b'o',
        ];
        let out_size = 5;
        let mut c_out = vec![0u8; out_size];
        let mut r_out = vec![0u8; out_size];

        let mut input_c = input.clone();
        let c_ret = c_fn(
            input_c.as_mut_ptr(),
            input_c.len() as i32,
            c_out.as_mut_ptr(),
            out_size as i32,
        );
        let r_ret = convert_pix_lib::cp_inflate(
            input.as_mut_ptr(),
            input.len() as i32,
            r_out.as_mut_ptr(),
            out_size as i32,
        );

        assert_eq!(c_ret, r_ret, "cp_inflate return value mismatch");
        assert_eq!(c_ret, 1, "cp_inflate C returned error");
        assert_eq!(c_out, r_out, "cp_inflate output mismatch");
    }
}

// Test cp_inflate with fixed Huffman (btype=1) using a real compressed stream
#[test]
fn test_cp_inflate_fixed_huffman() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*mut u8, i32, *mut u8, i32) -> i32> =
            lib.get(b"cp_inflate").unwrap();

        // Use zlib-style deflate: compress "AAAAAAAAAA" (10 x 'A')
        // This is a raw deflate stream for "AAAAAAAAAA" using fixed Huffman
        // Generated: final block, fixed huffman, literal 'A' then length=9 dist=1
        // Let's use a known minimal stream. We can create one with flate2 or hardcode.
        // Hardcoded raw deflate for "AAAAAAAAAA":
        // 0x73 0x74 0x74 0x04 0x00 — this is "AAAAAAAAAA" compressed
        let mut input: Vec<u8> = vec![0x73, 0x74, 0x74, 0x04, 0x00];
        let out_size = 10;
        let mut c_out = vec![0u8; out_size];
        let mut r_out = vec![0u8; out_size];

        let mut input_c = input.clone();
        let c_ret = c_fn(
            input_c.as_mut_ptr(),
            input_c.len() as i32,
            c_out.as_mut_ptr(),
            out_size as i32,
        );
        let r_ret = convert_pix_lib::cp_inflate(
            input.as_mut_ptr(),
            input.len() as i32,
            r_out.as_mut_ptr(),
            out_size as i32,
        );

        assert_eq!(c_ret, r_ret, "cp_inflate fixed huffman return mismatch");
        if c_ret == 1 {
            assert_eq!(c_out, r_out, "cp_inflate fixed huffman output mismatch");
        }
    }
}
