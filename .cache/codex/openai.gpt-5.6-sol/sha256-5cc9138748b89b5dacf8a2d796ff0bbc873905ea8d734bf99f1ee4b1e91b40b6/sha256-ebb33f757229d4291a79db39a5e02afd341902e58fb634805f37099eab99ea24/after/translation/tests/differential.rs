#![allow(unsafe_op_in_unsafe_fn)]

use flate2::Compression;
use flate2::write::DeflateEncoder;
use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const RTLD_DEEPBIND: c_int = 0x00008;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

type ConvertPix = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut Pixel);
type Inflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

struct Loaded {
    _library: Library,
    convert_pix: ConvertPix,
    inflate: Inflate,
    error_reason: *mut *const c_char,
    fixed_table: *mut [u8; 320],
    permutation_order: *mut [u8; 19],
    len_extra_bits: *mut [u8; 31],
    len_base: *mut [u32; 31],
    dist_extra_bits: *mut [u8; 32],
    dist_base: *mut [u32; 32],
}

impl Loaded {
    unsafe fn open(path: &Path) -> Self {
        let library = Library::open(Some(path), RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND)
            .unwrap_or_else(|e| {
                panic!("failed to load {}: {e}", path.display());
            });
        let convert_pix = *library.get::<ConvertPix>(b"convert_pix\0").unwrap();
        let inflate = *library.get::<Inflate>(b"cp_inflate\0").unwrap();
        let error_reason = library
            .get::<*mut c_void>(b"cp_error_reason\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let fixed_table = library
            .get::<*mut c_void>(b"cp_fixed_table\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let permutation_order = library
            .get::<*mut c_void>(b"cp_permutation_order\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let len_extra_bits = library
            .get::<*mut c_void>(b"cp_len_extra_bits\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let len_base = library
            .get::<*mut c_void>(b"cp_len_base\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let dist_extra_bits = library
            .get::<*mut c_void>(b"cp_dist_extra_bits\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        let dist_base = library
            .get::<*mut c_void>(b"cp_dist_base\0")
            .unwrap()
            .as_raw_ptr()
            .cast();
        Self {
            _library: library,
            convert_pix,
            inflate,
            error_reason,
            fixed_table,
            permutation_order,
            len_extra_bits,
            len_base,
            dist_extra_bits,
            dist_base,
        }
    }

    unsafe fn clear_error(&self) {
        *self.error_reason = std::ptr::null();
    }

    unsafe fn error(&self) -> Option<Vec<u8>> {
        let p = *self.error_reason;
        (!p.is_null()).then(|| CStr::from_ptr(p).to_bytes().to_vec())
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&build)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}",
        build.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libconvert_pix_lib.so")
}

fn libraries() -> (Loaded, Loaded) {
    let rust_path = rust_library_path();
    assert!(
        rust_path.exists(),
        "release Rust shared library missing: run cargo build --release"
    );
    unsafe {
        // RTLD_DEEPBIND prevents the two libraries' same-named data symbols from
        // interposing on one another when both are resident in this process.
        (Loaded::open(&c_library_path()), Loaded::open(&rust_path))
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x as u32
    }

    fn byte(&mut self) -> u8 {
        self.next_u32() as u8
    }

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    bit: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit: 0,
        }
    }

    fn bits(&mut self, value: u32, count: usize) {
        for i in 0..count {
            if self.bit / 8 == self.bytes.len() {
                self.bytes.push(0);
            }
            self.bytes[self.bit / 8] |= (((value >> i) & 1) as u8) << (self.bit & 7);
            self.bit += 1;
        }
    }

    fn align_byte(&mut self) {
        while self.bit & 7 != 0 {
            self.bits(0, 1);
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

fn reverse_bits(mut value: u32, count: usize) -> u32 {
    let mut reversed = 0;
    for _ in 0..count {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
    }
    reversed
}

fn fixed_symbol(writer: &mut BitWriter, symbol: u16) {
    let (code, bits) = match symbol {
        0..=143 => (0x30 + symbol as u32, 8),
        144..=255 => (0x190 + (symbol as u32 - 144), 9),
        256..=279 => (symbol as u32 - 256, 7),
        280..=287 => (0xc0 + (symbol as u32 - 280), 8),
        _ => panic!("invalid fixed symbol {symbol}"),
    };
    writer.bits(reverse_bits(code, bits), bits);
}

fn fixed_distance(writer: &mut BitWriter, symbol: u8) {
    writer.bits(reverse_bits(symbol as u32, 5), 5);
}

enum FixedToken {
    Literal(u8),
    Copy3Dist1,
    Copy3Dist4,
}

fn append_fixed_block(writer: &mut BitWriter, final_block: bool, tokens: &[FixedToken]) {
    writer.bits(final_block as u32, 1);
    writer.bits(1, 2);
    for token in tokens {
        match *token {
            FixedToken::Literal(value) => fixed_symbol(writer, value as u16),
            FixedToken::Copy3Dist1 => {
                fixed_symbol(writer, 257);
                fixed_distance(writer, 0);
            }
            FixedToken::Copy3Dist4 => {
                fixed_symbol(writer, 257);
                fixed_distance(writer, 3);
            }
        }
    }
    fixed_symbol(writer, 256);
}

fn fixed_literal_stream(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    let tokens: Vec<_> = data.iter().copied().map(FixedToken::Literal).collect();
    append_fixed_block(&mut writer, true, &tokens);
    writer.into_bytes()
}

fn stored_stream(data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= u16::MAX as usize);
    let mut writer = BitWriter::new();
    writer.bits(1, 1);
    writer.bits(0, 2);
    writer.align_byte();
    let len = data.len() as u16;
    for byte in len.to_le_bytes() {
        writer.bits(byte as u32, 8);
    }
    for byte in (!len).to_le_bytes() {
        writer.bits(byte as u32, 8);
    }
    for &byte in data {
        writer.bits(byte as u32, 8);
    }
    writer.into_bytes()
}

fn zlib_dynamic(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn first_block_type(stream: &[u8]) -> u8 {
    (stream[0] >> 1) & 3
}

fn aligned_input(bytes: &[u8], desired_mod4: usize) -> (Vec<u8>, usize) {
    let mut storage = vec![0u8; bytes.len() + 7];
    let base = storage.as_ptr() as usize;
    let offset = (desired_mod4 + 4 - (base & 3)) & 3;
    storage[offset..offset + bytes.len()].copy_from_slice(bytes);
    (storage, offset)
}

unsafe fn invoke_inflate(
    lib: &Loaded,
    stream: &[u8],
    alignment: usize,
    output_len: usize,
) -> (c_int, Vec<u8>, Option<Vec<u8>>) {
    let (mut storage, offset) = aligned_input(stream, alignment);
    let mut output = vec![0xa5; output_len];
    lib.clear_error();
    let result = (lib.inflate)(
        storage.as_mut_ptr().add(offset).cast(),
        stream.len() as c_int,
        output.as_mut_ptr().cast(),
        output_len as c_int,
    );
    (result, output, lib.error())
}

fn compare_success(
    c: &Loaded,
    rust: &Loaded,
    stream: &[u8],
    expected: &[u8],
    output_extra: usize,
    alignment: usize,
) {
    unsafe {
        let c_result = invoke_inflate(c, stream, alignment, expected.len() + output_extra);
        let rust_result = invoke_inflate(rust, stream, alignment, expected.len() + output_extra);
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result.0, 1);
        assert_eq!(&c_result.1[..expected.len()], expected);
        assert_eq!(c_result.1, rust_result.1);
        assert_eq!(c_result.2, rust_result.2);
    }
}

fn compare_error(
    c: &Loaded,
    rust: &Loaded,
    stream: &[u8],
    output_len: usize,
    expected_reason: &[u8],
) {
    unsafe {
        let c_result = invoke_inflate(c, stream, 0, output_len);
        let rust_result = invoke_inflate(rust, stream, 0, output_len);
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result.0, 0);
        assert_eq!(c_result.2.as_deref(), Some(expected_reason));
    }
}

#[test]
fn config_01_exported_data_matches_byte_for_byte() {
    let (c, rust) = libraries();
    unsafe {
        assert!((*c.error_reason).is_null());
        assert!((*rust.error_reason).is_null());
        assert_eq!(*c.fixed_table, *rust.fixed_table);
        assert_eq!(*c.permutation_order, *rust.permutation_order);
        assert_eq!(*c.len_extra_bits, *rust.len_extra_bits);
        assert_eq!(*c.len_base, *rust.len_base);
        assert_eq!(*c.dist_extra_bits, *rust.dist_extra_bits);
        assert_eq!(*c.dist_base, *rust.dist_base);
    }
}

fn expected_pixels(bpp: usize, w: usize, h: usize, src: &[u8]) -> Vec<Pixel> {
    let mut result = Vec::with_capacity(w * h);
    let mut offset = 0;
    for _ in 0..h {
        offset += 1;
        for _ in 0..w {
            let p = match bpp {
                1 => Pixel {
                    r: src[offset],
                    g: src[offset],
                    b: src[offset],
                    a: 0xff,
                },
                2 => Pixel {
                    r: src[offset],
                    g: src[offset],
                    b: src[offset],
                    a: src[offset + 1],
                },
                3 => Pixel {
                    r: src[offset],
                    g: src[offset + 1],
                    b: src[offset + 2],
                    a: 0xff,
                },
                4 => Pixel {
                    r: src[offset],
                    g: src[offset + 1],
                    b: src[offset + 2],
                    a: src[offset + 3],
                },
                _ => unreachable!(),
            };
            result.push(p);
            offset += bpp;
        }
    }
    result
}

#[test]
fn configs_02_through_06_convert_supported_formats_randomized() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x4350_4958_454c);
    for bpp in 1..=4usize {
        for case in 0..96 {
            let (w, h) = if case == 0 {
                (1, 1)
            } else {
                (1 + rng.usize(17), 1 + rng.usize(9))
            };
            let mut src = vec![0u8; h * (1 + w * bpp)];
            for byte in &mut src {
                *byte = rng.byte();
            }
            let sentinel = Pixel {
                r: 0xde,
                g: 0xad,
                b: 0xbe,
                a: 0xef,
            };
            let mut c_dst = vec![sentinel; w * h];
            let mut rust_dst = vec![sentinel; w * h];
            unsafe {
                (c.convert_pix)(
                    bpp as c_int,
                    w as c_int,
                    h as c_int,
                    src.as_mut_ptr(),
                    c_dst.as_mut_ptr(),
                );
                (rust.convert_pix)(
                    bpp as c_int,
                    w as c_int,
                    h as c_int,
                    src.as_mut_ptr(),
                    rust_dst.as_mut_ptr(),
                );
            }
            assert_eq!(c_dst, rust_dst, "bpp={bpp}, w={w}, h={h}");
            assert_eq!(c_dst, expected_pixels(bpp, w, h, &src));
        }
    }
}

#[test]
fn configs_07_through_10_convert_nonpositive_and_unsupported_no_write() {
    let (c, rust) = libraries();
    let sentinel = Pixel {
        r: 0x11,
        g: 0x22,
        b: 0x33,
        a: 0x44,
    };
    for &(bpp, w, h) in &[
        (1, 0, 5),
        (4, 5, 0),
        (3, -1, 4),
        (2, 4, -1),
        (1, -1, -1),
        (0, 0, 8),
        (5, 8, 0),
        (-1, 0, 0),
        (c_int::MAX, 0, 1),
    ] {
        let mut src = vec![0x77; 64];
        let mut c_dst = vec![sentinel; 16];
        let mut rust_dst = vec![sentinel; 16];
        unsafe {
            (c.convert_pix)(bpp, w, h, src.as_mut_ptr(), c_dst.as_mut_ptr());
            (rust.convert_pix)(bpp, w, h, src.as_mut_ptr(), rust_dst.as_mut_ptr());
        }
        assert_eq!(c_dst, rust_dst, "bpp={bpp}, w={w}, h={h}");
        assert!(c_dst.iter().all(|pixel| *pixel == sentinel));
    }

    for &bpp in &[0, 5] {
        let mut src = vec![0x77; 1 + 8 * 5];
        let mut c_dst = vec![sentinel; 8];
        let mut rust_dst = vec![sentinel; 8];
        unsafe {
            (c.convert_pix)(bpp, 8, 1, src.as_mut_ptr(), c_dst.as_mut_ptr());
            (rust.convert_pix)(bpp, 8, 1, src.as_mut_ptr(), rust_dst.as_mut_ptr());
        }
        assert_eq!(
            c_dst, rust_dst,
            "positive dimensions, unsupported bpp={bpp}"
        );
        assert!(c_dst.iter().all(|pixel| *pixel == sentinel));
    }

    let mut src = vec![0x77; 32];
    let centered = unsafe { src.as_mut_ptr().add(16) };
    let mut c_dst = vec![sentinel; 4];
    let mut rust_dst = vec![sentinel; 4];
    unsafe {
        (c.convert_pix)(-1, 4, 1, centered, c_dst.as_mut_ptr());
        (rust.convert_pix)(-1, 4, 1, centered, rust_dst.as_mut_ptr());
    }
    assert_eq!(c_dst, rust_dst);
    assert!(c_dst.iter().all(|pixel| *pixel == sentinel));
}

#[test]
fn configs_11_through_13_stored_blocks_randomized_and_aligned() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x5354_4f52_4544);
    for alignment in 0..4 {
        compare_success(&c, &rust, &stored_stream(&[]), &[], 8, alignment);
    }
    for _ in 0..128 {
        let len = 1 + rng.usize(1024);
        let mut data = vec![0; len];
        for byte in &mut data {
            *byte = rng.byte();
        }
        let stream = stored_stream(&data);
        compare_success(&c, &rust, &stream, &data, 0, rng.usize(4));
        compare_success(&c, &rust, &stream, &data, 17, rng.usize(4));
    }
}

#[test]
fn configs_14_through_16_fixed_huffman_shapes_randomized() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x4649_5845_44);
    for _ in 0..128 {
        let len = rng.usize(96);
        let mut literals = vec![0; len];
        for byte in &mut literals {
            *byte = rng.byte();
        }
        compare_success(
            &c,
            &rust,
            &fixed_literal_stream(&literals),
            &literals,
            rng.usize(12),
            rng.usize(4),
        );

        let repeated = rng.byte();
        let mut writer = BitWriter::new();
        let repeat_count = 1 + rng.usize(20);
        let mut tokens = vec![FixedToken::Literal(repeated)];
        for _ in 0..repeat_count {
            tokens.push(FixedToken::Copy3Dist1);
        }
        append_fixed_block(&mut writer, true, &tokens);
        let expected = vec![repeated; 1 + repeat_count * 3];
        compare_success(
            &c,
            &rust,
            &writer.into_bytes(),
            &expected,
            rng.usize(8),
            rng.usize(4),
        );

        let prefix = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
        let mut writer = BitWriter::new();
        let mut tokens: Vec<_> = prefix.iter().copied().map(FixedToken::Literal).collect();
        tokens.push(FixedToken::Copy3Dist4);
        append_fixed_block(&mut writer, true, &tokens);
        let expected = [prefix.as_slice(), &[prefix[0], prefix[1], prefix[2]]].concat();
        compare_success(
            &c,
            &rust,
            &writer.into_bytes(),
            &expected,
            rng.usize(8),
            rng.usize(4),
        );
    }
}

fn dynamic_input(rng: &mut Rng, len: usize) -> Vec<u8> {
    let alphabet = b"aaaaaaaabbbbccddeefghijklmnopqrstuvwxyz0123456789";
    (0..len)
        .map(|i| {
            if i >= 64 && i % 7 != 0 {
                // Repeated windows induce both short and longer distances.
                alphabet[(i + (i / 31)) % alphabet.len()]
            } else {
                alphabet[rng.usize(alphabet.len())]
            }
        })
        .collect()
}

#[test]
fn configs_17_18_20_21_22_dynamic_huffman_randomized() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x4459_4e41_4d49_43);
    for _ in 0..64 {
        let len = 4096 + rng.usize(4096);
        let data = dynamic_input(&mut rng, len);
        let stream = zlib_dynamic(&data);
        assert_eq!(
            first_block_type(&stream),
            2,
            "fixture must exercise the dynamic-Huffman branch"
        );
        for alignment in 0..4 {
            compare_success(&c, &rust, &stream, &data, alignment * 3, alignment);
        }
    }
}

#[test]
fn config_19_multiple_fixed_blocks() {
    let (c, rust) = libraries();
    let mut rng = Rng::new(0x4d55_4c54_4942_4c4b);
    for _ in 0..128 {
        let mut first = vec![0; 1 + rng.usize(80)];
        let mut second = vec![0; 1 + rng.usize(80)];
        for byte in first.iter_mut().chain(second.iter_mut()) {
            *byte = rng.byte();
        }
        let mut writer = BitWriter::new();
        let first_tokens: Vec<_> = first.iter().copied().map(FixedToken::Literal).collect();
        let second_tokens: Vec<_> = second.iter().copied().map(FixedToken::Literal).collect();
        append_fixed_block(&mut writer, false, &first_tokens);
        append_fixed_block(&mut writer, true, &second_tokens);
        let expected = [first, second].concat();
        compare_success(
            &c,
            &rust,
            &writer.into_bytes(),
            &expected,
            rng.usize(16),
            rng.usize(4),
        );
    }
}

#[test]
fn config_23_success_preserves_previous_error_reason() {
    let (c, rust) = libraries();
    let invalid = [0b0000_0111u8];
    compare_error(
        &c,
        &rust,
        &invalid,
        8,
        b"Detected unknown block type within input stream.",
    );
    let stream = stored_stream(b"ok");
    unsafe {
        let c_before = c.error();
        let rust_before = rust.error();
        assert_eq!(c_before, rust_before);
        let c_result = invoke_inflate(&c, &stream, 0, 2);
        let rust_result = invoke_inflate(&rust, &stream, 0, 2);
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result.0, 1);
        // invoke_inflate clears errors, so set and test preservation explicitly.
        *c.error_reason = b"sticky\0".as_ptr().cast();
        *rust.error_reason = b"sticky\0".as_ptr().cast();
        let (mut c_input, c_offset) = aligned_input(&stream, 0);
        let (mut r_input, r_offset) = aligned_input(&stream, 0);
        let mut c_output = [0u8; 2];
        let mut r_output = [0u8; 2];
        assert_eq!(
            (c.inflate)(
                c_input.as_mut_ptr().add(c_offset).cast(),
                stream.len() as c_int,
                c_output.as_mut_ptr().cast(),
                2,
            ),
            1
        );
        assert_eq!(
            (rust.inflate)(
                r_input.as_mut_ptr().add(r_offset).cast(),
                stream.len() as c_int,
                r_output.as_mut_ptr().cast(),
                2,
            ),
            1
        );
        assert_eq!(c.error(), rust.error());
        assert_eq!(c.error().as_deref(), Some(b"sticky".as_slice()));
    }
}

#[test]
fn errors_11_12_stored_rejections() {
    let (c, rust) = libraries();
    let mut complement = stored_stream(b"x");
    complement[3] ^= 1;
    compare_error(
        &c,
        &rust,
        &complement,
        8,
        b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
    );

    let mut extends = stored_stream(&[]);
    extends.push(0xaa);
    compare_error(
        &c,
        &rust,
        &extends,
        8,
        b"Stored block extends beyond end of input stream.",
    );
}

#[test]
fn errors_13_through_16_block_rejections_and_exact_reasons() {
    let (c, rust) = libraries();
    compare_error(
        &c,
        &rust,
        &fixed_literal_stream(b"x"),
        0,
        b"Attempted to overwrite out buffer while outputting a symbol.",
    );

    let mut writer = BitWriter::new();
    append_fixed_block(&mut writer, true, &[FixedToken::Copy3Dist1]);
    compare_error(
        &c,
        &rust,
        &writer.into_bytes(),
        8,
        b"Attempted to write before out buffer (invalid backwards distance).",
    );

    let mut writer = BitWriter::new();
    append_fixed_block(
        &mut writer,
        true,
        &[FixedToken::Literal(b'x'), FixedToken::Copy3Dist1],
    );
    compare_error(
        &c,
        &rust,
        &writer.into_bytes(),
        1,
        b"Attempted to overwrite out buffer while outputting a string.",
    );

    compare_error(
        &c,
        &rust,
        &[0b0000_0111],
        8,
        b"Detected unknown block type within input stream.",
    );
}

#[test]
fn generic_null_and_zero_non_dereferencing_boundaries() {
    let (c, rust) = libraries();
    unsafe {
        (c.convert_pix)(1, 0, 1, std::ptr::null_mut(), std::ptr::null_mut());
        (rust.convert_pix)(1, 0, 1, std::ptr::null_mut(), std::ptr::null_mut());
        (c.convert_pix)(1, 1, 0, std::ptr::null_mut(), std::ptr::null_mut());
        (rust.convert_pix)(1, 1, 0, std::ptr::null_mut(), std::ptr::null_mut());

        let stream = stored_stream(&[]);
        let (mut c_input, c_offset) = aligned_input(&stream, 0);
        let (mut r_input, r_offset) = aligned_input(&stream, 0);
        c.clear_error();
        rust.clear_error();
        let c_result = (c.inflate)(
            c_input.as_mut_ptr().add(c_offset).cast(),
            stream.len() as c_int,
            std::ptr::null_mut(),
            0,
        );
        let rust_result = (rust.inflate)(
            r_input.as_mut_ptr().add(r_offset).cast(),
            stream.len() as c_int,
            std::ptr::null_mut(),
            0,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, 1);
        assert_eq!(c.error(), rust.error());
    }
}

#[test]
fn generic_negative_output_length_matches_exact_rejection() {
    let (c, rust) = libraries();
    let stream = fixed_literal_stream(b"x");
    unsafe {
        let (mut c_input, c_offset) = aligned_input(&stream, 0);
        let (mut r_input, r_offset) = aligned_input(&stream, 0);
        let mut c_output = [0xa5u8; 8];
        let mut r_output = [0xa5u8; 8];
        let c_out = c_output.as_mut_ptr().add(4);
        let r_out = r_output.as_mut_ptr().add(4);
        c.clear_error();
        rust.clear_error();
        let c_result = (c.inflate)(
            c_input.as_mut_ptr().add(c_offset).cast(),
            stream.len() as c_int,
            c_out.cast(),
            -1,
        );
        let rust_result = (rust.inflate)(
            r_input.as_mut_ptr().add(r_offset).cast(),
            stream.len() as c_int,
            r_out.cast(),
            -1,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(c_result, 0);
        assert_eq!(c_output, r_output);
        assert_eq!(c.error(), rust.error());
        assert_eq!(
            c.error().as_deref(),
            Some(b"Attempted to overwrite out buffer while outputting a symbol.".as_slice())
        );
    }
}

fn run_probe(kind: &str, case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_fatal_probe_child")
        .arg("--nocapture")
        .env("CP_PROBE_KIND", kind)
        .env("CP_PROBE_CASE", case)
        .status()
        .unwrap()
}

#[test]
fn generic_fatal_boundaries_match_process_status() {
    use std::os::unix::process::ExitStatusExt;

    for case in [
        "empty_input",
        "null_convert_src",
        "null_convert_dst",
        "negative_input_length",
        "oversized_input_length",
        "malformed_huffman",
    ] {
        let c_status = run_probe("c", case);
        let rust_status = run_probe("rust", case);
        assert!(!c_status.success(), "C unexpectedly survived {case}");
        assert!(!rust_status.success(), "Rust unexpectedly survived {case}");
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different fatal signal for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
    }
}

#[test]
fn ffi_fatal_probe_child() {
    let Ok(kind) = std::env::var("CP_PROBE_KIND") else {
        return;
    };
    let case = std::env::var("CP_PROBE_CASE").unwrap();
    let path = if kind == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let lib = unsafe { Loaded::open(&path) };
    unsafe {
        match case.as_str() {
            "empty_input" => {
                let mut output = [0u8; 8];
                (lib.inflate)(
                    std::ptr::null_mut(),
                    0,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "null_convert_src" => {
                let mut output = [Pixel::default(); 1];
                (lib.convert_pix)(1, 1, 1, std::ptr::null_mut(), output.as_mut_ptr());
            }
            "null_convert_dst" => {
                let mut input = [0u8; 2];
                (lib.convert_pix)(1, 1, 1, input.as_mut_ptr(), std::ptr::null_mut());
            }
            "negative_input_length" => {
                let mut input = [0u8; 8];
                let mut output = [0u8; 8];
                (lib.inflate)(
                    input.as_mut_ptr().cast(),
                    -1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "oversized_input_length" => {
                let mut input = [0u8; 8];
                let mut output = [0u8; 8];
                (lib.inflate)(
                    input.as_mut_ptr().cast(),
                    c_int::MAX,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "malformed_huffman" => {
                let mut input = [5u8, 0, 0, 0];
                let mut output = [0u8; 16];
                (lib.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            _ => panic!("unknown probe case {case}"),
        }
    }
    panic!("fatal probe unexpectedly returned");
}

#[test]
fn internal_only_rejections_are_not_exported_by_either_library() {
    let c = unsafe {
        Library::open(
            Some(c_library_path()),
            RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND,
        )
        .unwrap()
    };
    let rust = unsafe {
        Library::open(
            Some(rust_library_path()),
            RTLD_NOW | RTLD_LOCAL | RTLD_DEEPBIND,
        )
        .unwrap()
    };
    for symbol in [
        b"cp_ptr\0".as_slice(),
        b"cp_peak_bits\0",
        b"cp_consume_bits\0",
        b"cp_read_bits\0",
        b"cp_build\0",
        b"cp_decode\0",
        b"cp_unfilter\0",
    ] {
        unsafe {
            assert!(c.get::<*mut c_void>(symbol).is_err());
            assert!(rust.get::<*mut c_void>(symbol).is_err());
        }
    }
}
