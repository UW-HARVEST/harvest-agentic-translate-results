use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uchar;

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/release/libunfilter_lib.so";

unsafe fn load_libs() -> (Library, Library) {
    let c = Library::new(C_LIB).expect("load C lib");
    let r = Library::new(RUST_LIB).expect("load Rust lib");
    (c, r)
}

type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut c_uchar) -> c_int;
type CpInflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

fn run_unfilter(lib: &Library, w: i32, h: i32, bpp: i32, data: &[u8]) -> (i32, Vec<u8>) {
    let mut buf = data.to_vec();
    unsafe {
        let f: Symbol<UnfilterFn> = lib.get(b"unfilter").unwrap();
        let r = f(w, h, bpp, buf.as_mut_ptr());
        (r, buf)
    }
}

fn run_inflate(lib: &Library, input: &[u8], out_size: usize) -> (i32, Vec<u8>) {
    let mut in_buf = input.to_vec();
    let mut out_buf = vec![0u8; out_size];
    unsafe {
        let f: Symbol<CpInflateFn> = lib.get(b"cp_inflate").unwrap();
        let r = f(
            in_buf.as_mut_ptr() as *mut c_void,
            in_buf.len() as c_int,
            out_buf.as_mut_ptr() as *mut c_void,
            out_buf.len() as c_int,
        );
        (r, out_buf)
    }
}

#[test]
fn unfilter_filter_none_simple() {
    unsafe {
        let (c, r) = load_libs();
        // h=1, w=4, bpp=3, filter byte = 0 then RGB row
        let data = vec![0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (rc_c, out_c) = run_unfilter(&c, 4, 1, 3, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 1, 3, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

#[test]
fn unfilter_filter_sub() {
    unsafe {
        let (c, r) = load_libs();
        let data = vec![1u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (rc_c, out_c) = run_unfilter(&c, 4, 1, 3, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 1, 3, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

#[test]
fn unfilter_filter_average() {
    unsafe {
        let (c, r) = load_libs();
        let data = vec![3u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (rc_c, out_c) = run_unfilter(&c, 4, 1, 3, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 1, 3, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

#[test]
fn unfilter_filter_paeth() {
    unsafe {
        let (c, r) = load_libs();
        let data = vec![4u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (rc_c, out_c) = run_unfilter(&c, 4, 1, 3, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 1, 3, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

#[test]
fn unfilter_multi_row_mixed_filters() {
    unsafe {
        let (c, r) = load_libs();
        // h=3, w=4, bpp=4 (RGBA)
        let mut data = Vec::new();
        // row 0 - filter 0 (none)
        data.push(0);
        for v in 0u8..16 {
            data.push(v.wrapping_mul(7));
        }
        // row 1 - filter 1 (sub)
        data.push(1);
        for v in 0u8..16 {
            data.push(v.wrapping_mul(11));
        }
        // row 2 - filter 4 (paeth)
        data.push(4);
        for v in 0u8..16 {
            data.push(v.wrapping_mul(13));
        }
        let (rc_c, out_c) = run_unfilter(&c, 4, 3, 4, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 3, 4, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

#[test]
fn unfilter_all_filter_types_multi_row() {
    unsafe {
        let (c, r) = load_libs();
        // h=5, w=3, bpp=3
        for first_filter in 0..=4u8 {
            let mut data = Vec::new();
            data.push(first_filter);
            for v in 0u8..9 {
                data.push(v.wrapping_mul(17).wrapping_add(3));
            }
            for next_filter in 0..=4u8 {
                data.push(next_filter);
                for v in 0u8..9 {
                    data.push(v.wrapping_mul(19).wrapping_add(5));
                }
            }
            // Now data has h=6 rows. Adjust:
            let h = 6;
            let (rc_c, out_c) = run_unfilter(&c, 3, h, 3, &data);
            let (rc_r, out_r) = run_unfilter(&r, 3, h, 3, &data);
            assert_eq!(rc_c, rc_r, "first_filter={}", first_filter);
            assert_eq!(out_c, out_r, "first_filter={}", first_filter);
        }
    }
}

#[test]
fn unfilter_invalid_filter_returns_zero() {
    unsafe {
        let (c, r) = load_libs();
        let data = vec![5u8, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let (rc_c, _) = run_unfilter(&c, 4, 1, 3, &data);
        let (rc_r, _) = run_unfilter(&r, 4, 1, 3, &data);
        assert_eq!(rc_c, rc_r);
    }
}

#[test]
fn unfilter_h_zero() {
    unsafe {
        let (c, r) = load_libs();
        let data = vec![0u8; 16];
        let (rc_c, out_c) = run_unfilter(&c, 4, 0, 3, &data);
        let (rc_r, out_r) = run_unfilter(&r, 4, 0, 3, &data);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

// ----------------------------------------------------------------------
// cp_inflate tests
// ----------------------------------------------------------------------

// Build a deflate stream that's a single fixed-Huffman block containing
// only literals 'A'..'D' followed by end-of-block.
// Use a known-good zlib-deflate-stripped byte sequence. We hand-craft a
// stored block so we don't depend on a deflate encoder.
fn make_stored_block(payload: &[u8]) -> Vec<u8> {
    // A single stored DEFLATE block:
    //   header byte: bfinal=1, btype=00 -> 0b001 = 0x01 ... but bits are LSB-first.
    //   The first byte of a stored block (after the 3 header bits) is padded to byte boundary,
    //   then LEN (2 bytes LE), NLEN (2 bytes LE), then payload.
    // Header: bit0=bfinal=1, bits1-2=btype=00 -> first byte = 0b00000001 = 0x01
    let mut out = Vec::new();
    out.push(0x01);
    let len = payload.len() as u16;
    let nlen = !len;
    out.push((len & 0xFF) as u8);
    out.push((len >> 8) as u8);
    out.push((nlen & 0xFF) as u8);
    out.push((nlen >> 8) as u8);
    out.extend_from_slice(payload);
    out
}

#[test]
fn cp_inflate_stored_block() {
    unsafe {
        let (c, r) = load_libs();
        let payload = b"Hello, world!";
        let in_data = make_stored_block(payload);
        let (rc_c, out_c) = run_inflate(&c, &in_data, payload.len());
        let (rc_r, out_r) = run_inflate(&r, &in_data, payload.len());
        assert_eq!(rc_c, rc_r);
        assert_eq!(&out_c[..payload.len()], &out_r[..payload.len()]);
        assert_eq!(&out_c[..payload.len()], payload);
    }
}

#[test]
fn cp_inflate_stored_empty() {
    unsafe {
        let (c, r) = load_libs();
        let payload: &[u8] = b"";
        let in_data = make_stored_block(payload);
        let (rc_c, out_c) = run_inflate(&c, &in_data, 1);
        let (rc_r, out_r) = run_inflate(&r, &in_data, 1);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

// Fixed-Huffman block containing just an end-of-block marker (symbol 256).
// In fixed Huffman, symbol 256 has length 7 and code 0000000.
// Bits: BFINAL=1, BTYPE=01, then EOB code 0000000 (7 bits)
// LSB-first within byte: byte0 = 0b0_00000_011 = first 3 bits are header (1,0,1 -> low=1,low+1=0,low+2=1?)
// Actually header is bfinal then btype low bit then btype high bit.
// bfinal=1 (1 bit), btype=01 -> btype bit0=1, btype bit1=0
// So header bits in stream order: 1,1,0
// Then EOB code: in fixed huffman, lit lengths 0..143 are length 8 with code 00110000..,
// 144..255 are length 9, 256..279 are length 7 with code 0000000..0010111,
// 280..287 are length 8 with code 11000000..11000111.
// EOB = symbol 256 -> code 0000000 (7 bits). Codes are written MSB-first into stream
// per RFC 1951 (huffman codes are packed starting with MSB of code first).
// So the 7-bit code 0000000 emits 0,0,0,0,0,0,0.
// Total: 1,1,0, 0,0,0,0,0,0,0 = 10 bits -> two bytes:
// byte0 bits: 1,1,0,0,0,0,0,0 (LSB first) = 0b00000011 = 0x03
// byte1 bits: 0,0 (lsb), padding zeros -> 0x00
fn make_fixed_eob_only() -> Vec<u8> {
    vec![0x03, 0x00]
}

#[test]
fn cp_inflate_dynamic_huffman_real_data() {
    // Raw deflate stream produced by Python's zlib for "Hello, World! Hello, World! Hello, World!"
    let in_data: Vec<u8> = vec![
        0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0xd7, 0x51, 0x08, 0xcf, 0x2f, 0xca, 0x49, 0x51, 0x54, 0xf0,
        0xc0, 0xcd, 0x03, 0x00,
    ];
    let expected = b"Hello, World! Hello, World! Hello, World!";
    unsafe {
        let (c, r) = load_libs();
        let (rc_c, out_c) = run_inflate(&c, &in_data, expected.len());
        let (rc_r, out_r) = run_inflate(&r, &in_data, expected.len());
        assert_eq!(rc_c, rc_r);
        assert_eq!(&out_c[..expected.len()], &out_r[..expected.len()]);
        assert_eq!(&out_c[..expected.len()], expected);
    }
}

#[test]
fn cp_inflate_fixed_eob_only() {
    unsafe {
        let (c, r) = load_libs();
        let in_data = make_fixed_eob_only();
        let (rc_c, out_c) = run_inflate(&c, &in_data, 16);
        let (rc_r, out_r) = run_inflate(&r, &in_data, 16);
        assert_eq!(rc_c, rc_r);
        assert_eq!(out_c, out_r);
    }
}

// Test that exported symbols match across the libs
#[test]
fn exported_symbols_present() {
    unsafe {
        let (c, r) = load_libs();
        // Check that all expected symbols load from both
        let symbols: &[&[u8]] = &[
            b"unfilter\0",
            b"cp_inflate\0",
            b"cp_error_reason\0",
            b"cp_fixed_table\0",
            b"cp_permutation_order\0",
            b"cp_len_extra_bits\0",
            b"cp_len_base\0",
            b"cp_dist_extra_bits\0",
            b"cp_dist_base\0",
        ];
        for s in symbols {
            let _: Symbol<*mut c_void> = c.get(s).expect("missing C symbol");
            let _: Symbol<*mut c_void> = r.get(s).expect("missing Rust symbol");
        }
    }
}

#[test]
fn fixed_table_data_matches() {
    unsafe {
        let (c, r) = load_libs();
        let cs: Symbol<*mut c_char> = c.get(b"cp_fixed_table\0").unwrap();
        let rs: Symbol<*mut c_char> = r.get(b"cp_fixed_table\0").unwrap();
        let c_ptr = (*cs) as *const u8;
        let r_ptr = (*rs) as *const u8;
        let c_slice = std::slice::from_raw_parts(c_ptr, 288 + 32);
        let r_slice = std::slice::from_raw_parts(r_ptr, 288 + 32);
        assert_eq!(c_slice, r_slice);
    }
}

#[test]
fn permutation_order_data_matches() {
    unsafe {
        let (c, r) = load_libs();
        let cs: Symbol<*mut u8> = c.get(b"cp_permutation_order\0").unwrap();
        let rs: Symbol<*mut u8> = r.get(b"cp_permutation_order\0").unwrap();
        let c_slice = std::slice::from_raw_parts(*cs as *const u8, 19);
        let r_slice = std::slice::from_raw_parts(*rs as *const u8, 19);
        assert_eq!(c_slice, r_slice);
    }
}

#[test]
fn len_base_data_matches() {
    unsafe {
        let (c, r) = load_libs();
        let cs: Symbol<*mut u32> = c.get(b"cp_len_base\0").unwrap();
        let rs: Symbol<*mut u32> = r.get(b"cp_len_base\0").unwrap();
        let c_slice = std::slice::from_raw_parts(*cs as *const u32, 31);
        let r_slice = std::slice::from_raw_parts(*rs as *const u32, 31);
        assert_eq!(c_slice, r_slice);
    }
}

#[test]
fn dist_base_data_matches() {
    unsafe {
        let (c, r) = load_libs();
        let cs: Symbol<*mut u32> = c.get(b"cp_dist_base\0").unwrap();
        let rs: Symbol<*mut u32> = r.get(b"cp_dist_base\0").unwrap();
        let c_slice = std::slice::from_raw_parts(*cs as *const u32, 32);
        let r_slice = std::slice::from_raw_parts(*rs as *const u32, 32);
        assert_eq!(c_slice, r_slice);
    }
}
