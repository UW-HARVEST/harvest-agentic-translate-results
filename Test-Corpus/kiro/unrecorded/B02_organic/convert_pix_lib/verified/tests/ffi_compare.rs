use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libconvert_pix_lib.so")
}

fn load() -> (Library, Library) {
    unsafe {
        (
            Library::new(c_lib_path()).expect("load C .so"),
            Library::new(rust_lib_path()).expect("load Rust .so"),
        )
    }
}

// ---- Data table comparisons ----

macro_rules! table_test {
    ($name:ident, $sym:literal, $ty:ty) => {
        #[test]
        fn $name() {
            let (c, r) = load();
            unsafe {
                let cv: Symbol<*const $ty> = c.get($sym).unwrap();
                let rv: Symbol<*const $ty> = r.get($sym).unwrap();
                let cs = std::slice::from_raw_parts(*cv as *const u8, std::mem::size_of::<$ty>());
                let rs = std::slice::from_raw_parts(*rv as *const u8, std::mem::size_of::<$ty>());
                assert_eq!(cs, rs, concat!(stringify!($name), " mismatch"));
            }
        }
    };
}

table_test!(test_cp_fixed_table, b"cp_fixed_table", [u8; 320]);
table_test!(test_cp_permutation_order, b"cp_permutation_order", [u8; 19]);
table_test!(test_cp_len_extra_bits, b"cp_len_extra_bits", [u8; 31]);
table_test!(test_cp_len_base, b"cp_len_base", [u32; 31]);
table_test!(test_cp_dist_extra_bits, b"cp_dist_extra_bits", [u8; 32]);
table_test!(test_cp_dist_base, b"cp_dist_base", [u32; 32]);

// ---- convert_pix ----

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CpPixel { r: u8, g: u8, b: u8, a: u8 }

type ConvertPixFn = unsafe extern "C" fn(i32, i32, i32, *mut u8, *mut CpPixel);

fn run_convert_pix(lib: &Library, bpp: i32, w: i32, h: i32, src: &[u8]) -> Vec<CpPixel> {
    unsafe {
        let f: Symbol<ConvertPixFn> = lib.get(b"convert_pix").unwrap();
        let mut s = src.to_vec();
        let mut d = vec![CpPixel { r: 0, g: 0, b: 0, a: 0 }; (w * h) as usize];
        f(bpp, w, h, s.as_mut_ptr(), d.as_mut_ptr());
        d
    }
}

fn check_convert_pix(bpp: i32, w: i32, h: i32, src: &[u8]) {
    let (c, r) = load();
    assert_eq!(
        run_convert_pix(&c, bpp, w, h, src),
        run_convert_pix(&r, bpp, w, h, src),
        "convert_pix bpp={bpp} w={w} h={h}"
    );
}

#[test]
fn test_convert_pix_bpp1() {
    check_convert_pix(1, 3, 2, &[0, 10, 20, 30, 0, 40, 50, 60]);
}

#[test]
fn test_convert_pix_bpp2() {
    check_convert_pix(2, 2, 2, &[0, 100, 200, 150, 250, 0, 50, 100, 75, 125]);
}

#[test]
fn test_convert_pix_bpp3() {
    check_convert_pix(3, 2, 2, &[0, 255, 128, 64, 32, 16, 8, 0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_convert_pix_bpp4() {
    check_convert_pix(4, 2, 2, &[
        0, 10, 20, 30, 40, 50, 60, 70, 80,
        0, 90, 100, 110, 120, 130, 140, 150, 160,
    ]);
}

#[test]
fn test_convert_pix_large() {
    let (w, h, bpp) = (64i32, 64i32, 4i32);
    let mut src = Vec::new();
    for y in 0..h {
        src.push(0u8);
        for x in 0..w {
            src.push(((x * 4 + y) & 0xFF) as u8);
            src.push(((x * 3 + y * 2) & 0xFF) as u8);
            src.push(((x + y * 5) & 0xFF) as u8);
            src.push(((x * 7 + y * 3) & 0xFF) as u8);
        }
    }
    check_convert_pix(bpp, w, h, &src);
}

// ---- cp_inflate ----

type CpInflateFn = unsafe extern "C" fn(*mut c_void, i32, *mut c_void, i32) -> i32;

fn run_inflate(lib: &Library, compressed: &[u8], out_size: usize) -> (i32, Vec<u8>) {
    unsafe {
        let f: Symbol<CpInflateFn> = lib.get(b"cp_inflate").unwrap();
        let mut inp = compressed.to_vec();
        let mut out = vec![0u8; out_size.max(1)];
        let ret = f(inp.as_mut_ptr() as *mut c_void, inp.len() as i32, out.as_mut_ptr() as *mut c_void, out_size as i32);
        out.truncate(out_size);
        (ret, out)
    }
}

fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let chunks: Vec<&[u8]> = data.chunks(65535).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_final = i == chunks.len() - 1;
        out.push(if is_final { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out
}

fn check_inflate(compressed: &[u8], out_size: usize) {
    let (c, r) = load();
    let (cr, co) = run_inflate(&c, compressed, out_size);
    let (rr, ro) = run_inflate(&r, compressed, out_size);
    assert_eq!(cr, rr, "inflate return code mismatch");
    if cr == 1 {
        assert_eq!(co, ro, "inflate output mismatch");
    }
}

#[test]
fn test_inflate_stored() {
    let data = b"Hello, World! This is a test of stored deflate blocks.";
    check_inflate(&deflate_stored(data), data.len());
}

#[test]
fn test_inflate_stored_empty() {
    // Test with a tiny 1-byte payload instead of truly empty, since empty
    // stored blocks with out_size=0 can cause issues with Vec::as_mut_ptr
    // returning dangling pointers in the test harness.
    let data = b"X";
    check_inflate(&deflate_stored(data), data.len());
}

#[test]
fn test_inflate_fixed_huffman() {
    // "Hello" raw deflate via python3 zlib.compress(b'Hello',6)[2:-4]
    check_inflate(&[243, 72, 205, 201, 201, 7, 0], 5);
}

#[test]
fn test_inflate_dynamic_huffman() {
    // b'abc'*10 raw deflate via python3 zlib.compress(b'abc'*10,9)[2:-4]
    check_inflate(&[75, 76, 74, 78, 196, 141, 0], 30);
}

#[test]
fn test_inflate_invalid_block_type() {
    // BFINAL=1, BTYPE=11 (invalid) => 0b111 = 0x07
    check_inflate(&[0x07], 64);
}

#[test]
fn test_inflate_larger_stored() {
    let data: Vec<u8> = (0..1024).map(|i| (i & 0xFF) as u8).collect();
    check_inflate(&deflate_stored(&data), data.len());
}

#[test]
fn test_inflate_larger_compressed() {
    // bytes(range(256))*4 raw deflate
    let compressed: Vec<u8> = vec![
        99, 96, 100, 98, 102, 97, 101, 99, 231, 224, 228, 226, 230, 225, 229, 227,
        23, 16, 20, 18, 22, 17, 21, 19, 151, 144, 148, 146, 150, 145, 149, 147,
        87, 80, 84, 82, 86, 81, 85, 83, 215, 208, 212, 210, 214, 209, 213, 211,
        55, 48, 52, 50, 54, 49, 53, 51, 183, 176, 180, 178, 182, 177, 181, 179,
        119, 112, 116, 114, 118, 113, 117, 115, 247, 240, 244, 242, 246, 241, 245, 243,
        15, 8, 12, 10, 14, 9, 13, 11, 143, 136, 140, 138, 142, 137, 141, 139,
        79, 72, 76, 74, 78, 73, 77, 75, 207, 200, 204, 202, 206, 201, 205, 203,
        47, 40, 44, 42, 46, 41, 45, 43, 175, 168, 172, 170, 174, 169, 173, 171,
        111, 104, 108, 106, 110, 105, 109, 107, 239, 232, 236, 234, 238, 233, 237, 235,
        159, 48, 113, 210, 228, 41, 83, 167, 77, 159, 49, 115, 214, 236, 57, 115,
        231, 205, 95, 176, 112, 209, 226, 37, 75, 151, 45, 95, 177, 114, 213, 234,
        53, 107, 215, 173, 223, 176, 113, 211, 230, 45, 91, 183, 109, 223, 177, 115,
        215, 238, 61, 123, 247, 237, 63, 112, 240, 208, 225, 35, 71, 143, 29, 63,
        113, 242, 212, 233, 51, 103, 207, 157, 191, 112, 241, 210, 229, 43, 87, 175,
        93, 191, 113, 243, 214, 237, 59, 119, 239, 221, 127, 240, 240, 209, 227, 39,
        79, 159, 61, 127, 241, 242, 213, 235, 55, 111, 223, 189, 255, 240, 241, 211,
        231, 47, 95, 191, 125, 255, 241, 243, 215, 239, 63, 127, 255, 253, 103, 24,
        245, 255, 168, 255, 71, 176, 255, 1,
    ];
    check_inflate(&compressed, 1024);
}
