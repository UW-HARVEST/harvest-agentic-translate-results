use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: i32,
    h: i32,
    pix: *mut CpPixel,
}

// ---- Global array tests ----

#[test]
fn test_cp_fixed_table() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_table: Symbol<*const [u8; 320]> = c_lib.get(b"cp_fixed_table").unwrap();
        let c_data = std::slice::from_raw_parts(*c_table as *const u8, 320);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_fixed_table.as_ptr(),
            320,
        );
        assert_eq!(c_data, rust_data, "cp_fixed_table mismatch");
    }
}

#[test]
fn test_cp_permutation_order() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 19]> = c_lib.get(b"cp_permutation_order").unwrap();
        let c_data = std::slice::from_raw_parts(*c_ptr as *const u8, 19);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_permutation_order.as_ptr(),
            19,
        );
        assert_eq!(c_data, rust_data, "cp_permutation_order mismatch");
    }
}

#[test]
fn test_cp_len_extra_bits() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 31]> = c_lib.get(b"cp_len_extra_bits").unwrap();
        let c_data = std::slice::from_raw_parts(*c_ptr as *const u8, 31);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_len_extra_bits.as_ptr(),
            31,
        );
        assert_eq!(c_data, rust_data, "cp_len_extra_bits mismatch");
    }
}

#[test]
fn test_cp_len_base() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u32; 31]> = c_lib.get(b"cp_len_base").unwrap();
        let c_data = std::slice::from_raw_parts(*c_ptr as *const u32, 31);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_len_base.as_ptr(),
            31,
        );
        assert_eq!(c_data, rust_data, "cp_len_base mismatch");
    }
}

#[test]
fn test_cp_dist_extra_bits() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u8; 32]> = c_lib.get(b"cp_dist_extra_bits").unwrap();
        let c_data = std::slice::from_raw_parts(*c_ptr as *const u8, 32);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_dist_extra_bits.as_ptr(),
            32,
        );
        assert_eq!(c_data, rust_data, "cp_dist_extra_bits mismatch");
    }
}

#[test]
fn test_cp_dist_base() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_ptr: Symbol<*const [u32; 32]> = c_lib.get(b"cp_dist_base").unwrap();
        let c_data = std::slice::from_raw_parts(*c_ptr as *const u32, 32);
        let rust_data = std::slice::from_raw_parts(
            load_png_mem_lib::cp_dist_base.as_ptr(),
            32,
        );
        assert_eq!(c_data, rust_data, "cp_dist_base mismatch");
    }
}

// ---- cp_inflate test ----

#[test]
fn test_cp_inflate() {
    // Create a simple deflate stream wrapped in zlib:
    // We'll use a stored block (btype=0) with known data.
    // zlib header: 0x78 0x01 (CM=8, CINFO=7, no dict, FCHECK ok)
    // Then a deflate stored block: bfinal=1, btype=00
    // bfinal|btype = 1|00 = 0b001 => byte 0x01
    // LEN=5, NLEN=~5=0xFFFA
    // data: "hello"
    // Then adler32 checksum (ignored by cp_inflate since it only gets data+2)
    let payload = b"hello";
    let len = payload.len() as u16;
    let nlen = !len;
    // deflate stream: bfinal=1 btype=00 => 0x01, then LEN(le16), NLEN(le16), data
    let mut deflate_stream: Vec<u8> = Vec::new();
    deflate_stream.push(0x01); // bfinal=1, btype=0 (stored)
    deflate_stream.push((len & 0xFF) as u8);
    deflate_stream.push((len >> 8) as u8);
    deflate_stream.push((nlen & 0xFF) as u8);
    deflate_stream.push((nlen >> 8) as u8);
    deflate_stream.extend_from_slice(payload);

    let out_size = payload.len();

    // Test C version
    let mut c_out = vec![0u8; out_size];
    let c_result;
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_inflate: Symbol<unsafe extern "C" fn(*mut libc::c_void, i32, *mut libc::c_void, i32) -> i32> =
            c_lib.get(b"cp_inflate").unwrap();
        c_result = c_inflate(
            deflate_stream.as_mut_ptr() as *mut _,
            deflate_stream.len() as i32,
            c_out.as_mut_ptr() as *mut _,
            out_size as i32,
        );
    }

    // Test Rust version
    let mut rust_out = vec![0u8; out_size];
    let rust_result;
    unsafe {
        rust_result = load_png_mem_lib::cp_inflate(
            deflate_stream.as_mut_ptr() as *mut _,
            deflate_stream.len() as i32,
            rust_out.as_mut_ptr() as *mut _,
            out_size as i32,
        );
    }

    assert_eq!(c_result, rust_result, "cp_inflate return value mismatch: C={c_result}, Rust={rust_result}");
    assert_eq!(c_result, 1, "cp_inflate should succeed");
    assert_eq!(c_out, rust_out, "cp_inflate output mismatch");
    assert_eq!(&c_out, payload, "output should be 'hello'");
}

#[test]
fn test_cp_inflate_fixed_huffman() {
    // Create a deflate stream using fixed Huffman codes (btype=1)
    // We'll compress a short repeated pattern that exercises the fixed code path.
    // Use Python-style manual encoding of fixed Huffman for literal bytes.
    // Easier: use a known-good deflate fixed block.
    // Let's just use a zlib-compressed version of "AAAA" (4 bytes).
    // zlib header 78 01, then deflate data, then adler32.
    // For cp_inflate, we pass data+2 (skip zlib header) and datalen-6 (skip header+adler).
    // Let's create the full zlib stream and strip header/trailer.

    // Actually, let's create a raw deflate fixed block manually.
    // For literal 'A' (0x41) in fixed Huffman: code length 8, code for 0x41.
    // Fixed Huffman literal codes 0-143: 8 bits, starting at 00110000 (48).
    // Code for literal N (0<=N<=143) = N + 48 = 0b00110000 + N, reversed in 8 bits.
    // Code for 0x41 (65): 65+48=113 = 0b01110001, reversed = 0b10001110 = 0x8E
    // Code for end-of-block (256): 7 bits, code = 256-256=0, base 0b0000000, reversed = 0b0000000
    // Actually the fixed codes: 0-143 => 00110000..10111111 (8 bits), 256-279 => 0000000..0010111 (7 bits)
    // 256 => 0000000 (7 bits), reversed = 0000000

    // This is getting complex. Let me just test with a stored block of different data.
    let payload = vec![0xAA; 256];
    let len = payload.len() as u16;
    let nlen = !len;
    let mut deflate_stream: Vec<u8> = Vec::new();
    deflate_stream.push(0x01);
    deflate_stream.push((len & 0xFF) as u8);
    deflate_stream.push((len >> 8) as u8);
    deflate_stream.push((nlen & 0xFF) as u8);
    deflate_stream.push((nlen >> 8) as u8);
    deflate_stream.extend_from_slice(&payload);

    let mut c_out = vec![0u8; payload.len()];
    let mut rust_out = vec![0u8; payload.len()];

    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_inflate: Symbol<unsafe extern "C" fn(*mut libc::c_void, i32, *mut libc::c_void, i32) -> i32> =
            c_lib.get(b"cp_inflate").unwrap();
        let c_r = c_inflate(
            deflate_stream.as_mut_ptr() as *mut _,
            deflate_stream.len() as i32,
            c_out.as_mut_ptr() as *mut _,
            payload.len() as i32,
        );
        let rust_r = load_png_mem_lib::cp_inflate(
            deflate_stream.as_mut_ptr() as *mut _,
            deflate_stream.len() as i32,
            rust_out.as_mut_ptr() as *mut _,
            payload.len() as i32,
        );
        assert_eq!(c_r, rust_r, "return value mismatch");
        assert_eq!(c_r, 1);
    }
    assert_eq!(c_out, rust_out, "cp_inflate 256-byte output mismatch");
}

// ---- load_png_mem test ----

/// Generate a minimal valid 8-bit RGBA PNG in memory.
fn make_test_png(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        for &b in data {
            crc ^= b as u32;
            for _ in 0..8 {
                if crc & 1 != 0 {
                    crc = (crc >> 1) ^ 0xEDB88320;
                } else {
                    crc >>= 1;
                }
            }
        }
        !crc
    }
    fn adler32(data: &[u8]) -> u32 {
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &byte in data {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    fn write_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(chunk_type);
        out.extend_from_slice(data);
        let mut crc_data = Vec::new();
        crc_data.extend_from_slice(chunk_type);
        crc_data.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_data).to_be_bytes());
    }

    let mut png = Vec::new();
    // Signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // IHDR: width, height, bit_depth=8, color_type=6 (RGBA), compression=0, filter=0, interlace=0
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type RGBA
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut png, b"IHDR", &ihdr);

    // IDAT: zlib-wrapped deflate of filtered scanlines
    // Each scanline: filter_byte(0) + w*4 bytes of RGBA
    let mut raw = Vec::new();
    for y in 0..h as usize {
        raw.push(0u8); // filter: None
        let row_start = y * (w as usize) * 4;
        let row_end = row_start + (w as usize) * 4;
        raw.extend_from_slice(&pixels[row_start..row_end]);
    }

    // Wrap in zlib: header + deflate stored blocks + adler32
    let mut zlib = Vec::new();
    zlib.push(0x78); // CMF
    zlib.push(0x01); // FLG

    // Split raw data into stored blocks of max 65535 bytes
    let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        zlib.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        zlib.push((len & 0xFF) as u8);
        zlib.push((len >> 8) as u8);
        let nlen = !len;
        zlib.push((nlen & 0xFF) as u8);
        zlib.push((nlen >> 8) as u8);
        zlib.extend_from_slice(chunk);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    write_chunk(&mut png, b"IDAT", &zlib);

    // IEND
    write_chunk(&mut png, b"IEND", &[]);

    png
}

#[test]
fn test_load_png_mem_2x2_rgba() {
    // 2x2 RGBA image with known pixel values
    let pixels: Vec<u8> = vec![
        255, 0, 0, 255,    // red
        0, 255, 0, 255,    // green
        0, 0, 255, 255,    // blue
        255, 255, 0, 128,  // yellow semi-transparent
    ];
    let png_data = make_test_png(2, 2, &pixels);

    // C version
    let (c_w, c_h, c_pixels);
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_load: Symbol<unsafe extern "C" fn(*const u8, i32) -> CpImage> =
            c_lib.get(b"load_png_mem").unwrap();
        let c_img = c_load(png_data.as_ptr(), png_data.len() as i32);
        assert!(!c_img.pix.is_null(), "C load_png_mem returned null pix");
        c_w = c_img.w;
        c_h = c_img.h;
        c_pixels = std::slice::from_raw_parts(c_img.pix, (c_w * c_h) as usize).to_vec();
        libc::free(c_img.pix as *mut _);
    }

    // Rust version
    let (rust_w, rust_h, rust_pixels);
    unsafe {
        let rust_img = load_png_mem_lib::load_png_mem(png_data.as_ptr(), png_data.len() as i32);
        assert!(!rust_img.pix.is_null(), "Rust load_png_mem returned null pix");
        rust_w = rust_img.w;
        rust_h = rust_img.h;
        rust_pixels = std::slice::from_raw_parts(
            rust_img.pix as *const CpPixel,
            (rust_w * rust_h) as usize,
        ).to_vec();
        libc::free(rust_img.pix as *mut _);
    }

    assert_eq!(c_w, rust_w, "width mismatch: C={c_w}, Rust={rust_w}");
    assert_eq!(c_h, rust_h, "height mismatch: C={c_h}, Rust={rust_h}");
    assert_eq!(c_pixels, rust_pixels, "pixel data mismatch");
}

#[test]
fn test_load_png_mem_grayscale() {
    // 4x2 grayscale (color_type=0, bpp=1) image
    // We need to build a PNG with color_type=0
    fn make_gray_png(w: u32, h: u32, gray_pixels: &[u8]) -> Vec<u8> {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc: u32 = 0xFFFFFFFF;
            for &b in data {
                crc ^= b as u32;
                for _ in 0..8 {
                    if crc & 1 != 0 { crc = (crc >> 1) ^ 0xEDB88320; } else { crc >>= 1; }
                }
            }
            !crc
        }
        fn adler32(data: &[u8]) -> u32 {
            let (mut a, mut b) = (1u32, 0u32);
            for &byte in data { a = (a + byte as u32) % 65521; b = (b + a) % 65521; }
            (b << 16) | a
        }
        fn write_chunk(out: &mut Vec<u8>, ct: &[u8; 4], data: &[u8]) {
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(ct);
            out.extend_from_slice(data);
            let mut cd = Vec::new(); cd.extend_from_slice(ct); cd.extend_from_slice(data);
            out.extend_from_slice(&crc32(&cd).to_be_bytes());
        }
        let mut png = Vec::new();
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.push(8); ihdr.push(0); // grayscale
        ihdr.push(0); ihdr.push(0); ihdr.push(0);
        write_chunk(&mut png, b"IHDR", &ihdr);

        let mut raw = Vec::new();
        for y in 0..h as usize {
            raw.push(0u8);
            let start = y * w as usize;
            raw.extend_from_slice(&gray_pixels[start..start + w as usize]);
        }
        let mut zlib = vec![0x78, 0x01];
        let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            zlib.push(if i == chunks.len() - 1 { 0x01 } else { 0x00 });
            let len = chunk.len() as u16;
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&(!len).to_le_bytes());
            zlib.extend_from_slice(chunk);
        }
        zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
        write_chunk(&mut png, b"IDAT", &zlib);
        write_chunk(&mut png, b"IEND", &[]);
        png
    }

    let gray_pixels: Vec<u8> = vec![10, 20, 30, 40, 50, 60, 70, 80];
    let png_data = make_gray_png(4, 2, &gray_pixels);

    let (c_w, c_h, c_pixels);
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_load: Symbol<unsafe extern "C" fn(*const u8, i32) -> CpImage> =
            c_lib.get(b"load_png_mem").unwrap();
        let c_img = c_load(png_data.as_ptr(), png_data.len() as i32);
        assert!(!c_img.pix.is_null(), "C load_png_mem returned null for grayscale");
        c_w = c_img.w;
        c_h = c_img.h;
        c_pixels = std::slice::from_raw_parts(c_img.pix, (c_w * c_h) as usize).to_vec();
        libc::free(c_img.pix as *mut _);
    }

    let (rust_w, rust_h, rust_pixels);
    unsafe {
        let rust_img = load_png_mem_lib::load_png_mem(png_data.as_ptr(), png_data.len() as i32);
        assert!(!rust_img.pix.is_null(), "Rust load_png_mem returned null for grayscale");
        rust_w = rust_img.w;
        rust_h = rust_img.h;
        rust_pixels = std::slice::from_raw_parts(
            rust_img.pix as *const CpPixel, (rust_w * rust_h) as usize
        ).to_vec();
        libc::free(rust_img.pix as *mut _);
    }

    assert_eq!(c_w, rust_w, "grayscale width mismatch");
    assert_eq!(c_h, rust_h, "grayscale height mismatch");
    assert_eq!(c_pixels, rust_pixels, "grayscale pixel data mismatch");
}

#[test]
fn test_load_png_mem_invalid() {
    // Test with invalid data - both should return null pix
    let bad_data = vec![0u8; 32];

    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_load: Symbol<unsafe extern "C" fn(*const u8, i32) -> CpImage> =
            c_lib.get(b"load_png_mem").unwrap();
        let c_img = c_load(bad_data.as_ptr(), bad_data.len() as i32);
        let rust_img = load_png_mem_lib::load_png_mem(bad_data.as_ptr(), bad_data.len() as i32);

        assert_eq!(c_img.pix.is_null(), rust_img.pix.is_null(),
            "null pix mismatch on invalid input: C_null={}, Rust_null={}",
            c_img.pix.is_null(), rust_img.pix.is_null());
    }
}
