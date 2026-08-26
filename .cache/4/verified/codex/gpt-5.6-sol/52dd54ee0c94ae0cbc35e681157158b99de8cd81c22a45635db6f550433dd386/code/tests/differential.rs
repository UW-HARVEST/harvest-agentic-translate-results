#![allow(unsafe_op_in_unsafe_fn)]

use flate2::Compression;
use flate2::write::DeflateEncoder;
use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

const C_SO: &str = "c_src/build/libtranslated_rust.so";
const RUST_SO_DEBUG: &str = "target/debug/libconvert_pix_lib.so";
const RUST_SO_RELEASE: &str = "target/release/libconvert_pix_lib.so";
static FFI_LOCK: Mutex<()> = Mutex::new(());

type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type ConvertFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut Pixel);

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug, Eq, PartialEq)]
struct InflateResult {
    status: c_int,
    output: Vec<u8>,
    error: Option<String>,
}

struct Pair {
    c: Library,
    rust: Library,
}

impl Pair {
    unsafe fn load() -> Self {
        Self {
            c: Library::new(crate_path(C_SO)).expect("load C shared library"),
            rust: Library::new(rust_so_path()).expect("load Rust shared library"),
        }
    }

    unsafe fn inflate_one(
        lib: &Library,
        stream: &[u8],
        input_mod4: usize,
        output_len: usize,
    ) -> InflateResult {
        let inflate = lib.get::<InflateFn>(b"cp_inflate").expect("cp_inflate");
        let error_symbol = lib
            .get::<*mut *const c_char>(b"cp_error_reason")
            .expect("cp_error_reason");
        let error_slot = *error_symbol;
        *error_slot = ptr::null();

        let mut storage = vec![0xa5; stream.len() + 8];
        let base_mod4 = storage.as_ptr() as usize & 3;
        let offset = (input_mod4 + 4 - base_mod4) & 3;
        storage[offset..offset + stream.len()].copy_from_slice(stream);
        let input = storage.as_mut_ptr().add(offset);
        let mut output = vec![0xcd; output_len];
        let status = inflate(
            input.cast(),
            stream.len() as c_int,
            output.as_mut_ptr().cast(),
            output_len as c_int,
        );
        let error_pointer = *error_slot;
        let error = if error_pointer.is_null() {
            None
        } else {
            Some(CStr::from_ptr(error_pointer).to_string_lossy().into_owned())
        };
        InflateResult {
            status,
            output,
            error,
        }
    }

    unsafe fn compare_inflate(
        &self,
        stream: &[u8],
        input_mod4: usize,
        output_len: usize,
    ) -> InflateResult {
        let c = Self::inflate_one(&self.c, stream, input_mod4, output_len);
        let rust = Self::inflate_one(&self.rust, stream, input_mod4, output_len);
        assert_eq!(rust, c, "stream={stream:02x?}, input_mod4={input_mod4}");
        c
    }

    unsafe fn convert_one(
        lib: &Library,
        bpp: c_int,
        width: c_int,
        height: c_int,
        source: *mut u8,
        destination: *mut Pixel,
    ) {
        let convert = lib.get::<ConvertFn>(b"convert_pix").expect("convert_pix");
        convert(bpp, width, height, source, destination);
    }
}

fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn rust_so_path() -> PathBuf {
    if let Ok(path) = std::env::var("RUST_SO") {
        return PathBuf::from(path);
    }
    let debug = crate_path(RUST_SO_DEBUG);
    if debug.exists() {
        debug
    } else {
        crate_path(RUST_SO_RELEASE)
    }
}

fn ffi_guard() -> MutexGuard<'static, ()> {
    FFI_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn range(&mut self, low: usize, high: usize) -> usize {
        low + self.next_u64() as usize % (high - low)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

impl BitWriter {
    fn bits(&mut self, value: u32, count: u8) {
        for index in 0..count {
            if self.bit == 0 {
                self.bytes.push(0);
            }
            if value >> index & 1 != 0 {
                *self.bytes.last_mut().unwrap() |= 1 << self.bit;
            }
            self.bit = (self.bit + 1) & 7;
        }
    }

    fn align(&mut self) {
        self.bit = 0;
    }

    fn aligned_bytes(&mut self, bytes: &[u8]) {
        assert_eq!(self.bit, 0);
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn reverse(mut value: u32, count: u8) -> u32 {
    let mut reversed = 0;
    for _ in 0..count {
        reversed = reversed << 1 | value & 1;
        value >>= 1;
    }
    reversed
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, u8)> {
    let mut counts = [0_u32; 16];
    for &length in lengths {
        counts[length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0_u32; 16];
    let mut code = 0;
    for bits in 1..16 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    lengths
        .iter()
        .map(|&length| {
            if length == 0 {
                (0, 0)
            } else {
                let code = next[length as usize];
                next[length as usize] += 1;
                (reverse(code, length), length)
            }
        })
        .collect()
}

fn write_symbol(writer: &mut BitWriter, codes: &[(u32, u8)], symbol: usize) {
    let (code, length) = codes[symbol];
    assert_ne!(length, 0);
    writer.bits(code, length);
}

#[derive(Clone, Copy)]
enum FixedToken {
    Literal(u8),
    Copy { length: usize, distance: usize },
}

const LENGTH_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn fixed_lengths() -> (Vec<u8>, Vec<u8>) {
    let mut literal = vec![0; 288];
    literal[..144].fill(8);
    literal[144..256].fill(9);
    literal[256..280].fill(7);
    literal[280..].fill(8);
    (literal, vec![5; 32])
}

fn write_fixed_block(writer: &mut BitWriter, final_block: bool, tokens: &[FixedToken]) {
    writer.bits(final_block as u32, 1);
    writer.bits(1, 2);
    let (literal_lengths, distance_lengths) = fixed_lengths();
    let literal_codes = canonical_codes(&literal_lengths);
    let distance_codes = canonical_codes(&distance_lengths);

    for token in tokens {
        match *token {
            FixedToken::Literal(byte) => write_symbol(writer, &literal_codes, byte as usize),
            FixedToken::Copy { length, distance } => {
                let length_index = LENGTH_BASE
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|&(index, base)| {
                        length >= *base && length <= *base + ((1_usize << LENGTH_EXTRA[index]) - 1)
                    })
                    .map(|(index, _)| index)
                    .expect("encodable length");
                write_symbol(writer, &literal_codes, 257 + length_index);
                writer.bits(
                    (length - LENGTH_BASE[length_index]) as u32,
                    LENGTH_EXTRA[length_index],
                );
                let distance_index = DISTANCE_BASE
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|&(index, base)| {
                        distance >= *base
                            && distance <= *base + ((1_usize << DISTANCE_EXTRA[index]) - 1)
                    })
                    .map(|(index, _)| index)
                    .expect("encodable distance");
                write_symbol(writer, &distance_codes, distance_index);
                writer.bits(
                    (distance - DISTANCE_BASE[distance_index]) as u32,
                    DISTANCE_EXTRA[distance_index],
                );
            }
        }
    }
    write_symbol(writer, &literal_codes, 256);
}

fn fixed_stream(tokens: &[FixedToken]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    write_fixed_block(&mut writer, true, tokens);
    writer.finish()
}

fn stored_stream(payload: &[u8], len_override: Option<u16>, nlen_override: Option<u16>) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(0, 2);
    writer.align();
    let length = len_override.unwrap_or(payload.len() as u16);
    let inverse = nlen_override.unwrap_or(!length);
    writer.aligned_bytes(&length.to_le_bytes());
    writer.aligned_bytes(&inverse.to_le_bytes());
    writer.aligned_bytes(payload);
    writer.finish()
}

fn flate2_stream(payload: &[u8], level: u32) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
    encoder.write_all(payload).unwrap();
    encoder.finish().unwrap()
}

fn block_type(stream: &[u8]) -> u8 {
    stream[0] >> 1 & 3
}

fn write_dynamic_header(
    writer: &mut BitWriter,
    literal_lengths: &[u8],
    distance_lengths: &[u8],
    code_length_lengths: &[u8; 19],
) -> Vec<(u32, u8)> {
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    writer.bits(1, 1);
    writer.bits(2, 2);
    writer.bits((literal_lengths.len() - 257) as u32, 5);
    writer.bits((distance_lengths.len() - 1) as u32, 5);
    writer.bits(14, 4);
    for &symbol in &ORDER[..18] {
        writer.bits(code_length_lengths[symbol] as u32, 3);
    }
    canonical_codes(code_length_lengths)
}

fn dynamic_single_symbol(count: usize, zero_symbol: usize) -> Vec<u8> {
    assert!(zero_symbol == 17 || zero_symbol == 18);
    let mut writer = BitWriter::default();
    let mut code_lengths = [0_u8; 19];
    code_lengths[1] = 1;
    code_lengths[zero_symbol] = 1;
    let code_length_codes = write_dynamic_header(&mut writer, &vec![0; 257], &[1], &code_lengths);

    let zeros = |writer: &mut BitWriter, mut amount: usize| {
        if zero_symbol == 18 {
            while amount != 0 {
                let run = amount.min(138);
                assert!(run >= 11);
                write_symbol(writer, &code_length_codes, 18);
                writer.bits((run - 11) as u32, 7);
                amount -= run;
            }
        } else {
            while amount != 0 {
                let run = amount.min(10);
                assert!(run >= 3);
                write_symbol(writer, &code_length_codes, 17);
                writer.bits((run - 3) as u32, 3);
                amount -= run;
            }
        }
    };
    zeros(&mut writer, 65);
    write_symbol(&mut writer, &code_length_codes, 1);
    zeros(&mut writer, 190);
    write_symbol(&mut writer, &code_length_codes, 1);
    write_symbol(&mut writer, &code_length_codes, 1);

    let mut literal_lengths = vec![0; 257];
    literal_lengths[65] = 1;
    literal_lengths[256] = 1;
    let literal_codes = canonical_codes(&literal_lengths);
    for _ in 0..count {
        write_symbol(&mut writer, &literal_codes, 65);
    }
    write_symbol(&mut writer, &literal_codes, 256);
    writer.finish()
}

fn dynamic_repeat_16(payload: &[u8]) -> Vec<u8> {
    assert!(payload.iter().all(|byte| (65..=71).contains(byte)));
    let mut writer = BitWriter::default();
    let mut code_lengths = [0_u8; 19];
    for symbol in [1, 3, 16, 18] {
        code_lengths[symbol] = 2;
    }
    let code_length_codes = write_dynamic_header(&mut writer, &vec![0; 257], &[1], &code_lengths);
    write_symbol(&mut writer, &code_length_codes, 18);
    writer.bits(54, 7);
    write_symbol(&mut writer, &code_length_codes, 3);
    write_symbol(&mut writer, &code_length_codes, 16);
    writer.bits(3, 2);
    write_symbol(&mut writer, &code_length_codes, 18);
    writer.bits(127, 7);
    write_symbol(&mut writer, &code_length_codes, 18);
    writer.bits(35, 7);
    write_symbol(&mut writer, &code_length_codes, 3);
    write_symbol(&mut writer, &code_length_codes, 1);

    let mut literal_lengths = vec![0; 257];
    literal_lengths[65..72].fill(3);
    literal_lengths[256] = 3;
    let literal_codes = canonical_codes(&literal_lengths);
    for &byte in payload {
        write_symbol(&mut writer, &literal_codes, byte as usize);
    }
    write_symbol(&mut writer, &literal_codes, 256);
    writer.finish()
}

fn dynamic_copy(distance: usize, copies: usize) -> (Vec<u8>, Vec<u8>) {
    assert!(distance == 1 || distance == 2);
    let mut writer = BitWriter::default();
    let mut code_lengths = [0_u8; 19];
    code_lengths[18] = 1;
    code_lengths[1] = 2;
    code_lengths[2] = 2;
    let code_length_codes =
        write_dynamic_header(&mut writer, &vec![0; 258], &[1, 1], &code_lengths);
    let zeros = |writer: &mut BitWriter, amount: usize| {
        let mut left = amount;
        while left != 0 {
            let run = left.min(138);
            write_symbol(writer, &code_length_codes, 18);
            writer.bits((run - 11) as u32, 7);
            left -= run;
        }
    };
    zeros(&mut writer, 65);
    write_symbol(&mut writer, &code_length_codes, 2);
    write_symbol(&mut writer, &code_length_codes, 2);
    zeros(&mut writer, 189);
    write_symbol(&mut writer, &code_length_codes, 2);
    write_symbol(&mut writer, &code_length_codes, 2);
    write_symbol(&mut writer, &code_length_codes, 1);
    write_symbol(&mut writer, &code_length_codes, 1);

    let mut literal_lengths = vec![0; 258];
    for symbol in [65, 66, 256, 257] {
        literal_lengths[symbol] = 2;
    }
    let literal_codes = canonical_codes(&literal_lengths);
    let distance_codes = canonical_codes(&[1, 1]);
    let mut expected = Vec::new();
    write_symbol(&mut writer, &literal_codes, 65);
    expected.push(b'A');
    if distance == 2 {
        write_symbol(&mut writer, &literal_codes, 66);
        expected.push(b'B');
    }
    for _ in 0..copies {
        write_symbol(&mut writer, &literal_codes, 257);
        write_symbol(&mut writer, &distance_codes, distance - 1);
        for _ in 0..3 {
            let byte = expected[expected.len() - distance];
            expected.push(byte);
        }
    }
    write_symbol(&mut writer, &literal_codes, 256);
    (writer.finish(), expected)
}

unsafe fn global_bytes(lib: &Library, name: &[u8], len: usize) -> Vec<u8> {
    let symbol = lib.get::<*const u8>(name).expect("exported data symbol");
    std::slice::from_raw_parts(*symbol, len).to_vec()
}

#[test]
fn exported_data_symbols_match() {
    let _guard = ffi_guard();
    for library in ["c", "rust"] {
        assert!(
            child_status("initial_error", library).success(),
            "{library} cp_error_reason was not initially null"
        );
    }
    unsafe {
        let pair = Pair::load();
        for (name, len) in [
            (&b"cp_fixed_table"[..], 320),
            (&b"cp_permutation_order"[..], 19),
            (&b"cp_len_extra_bits"[..], 31),
            (&b"cp_len_base"[..], 31 * 4),
            (&b"cp_dist_extra_bits"[..], 32),
            (&b"cp_dist_base"[..], 32 * 4),
        ] {
            assert_eq!(
                global_bytes(&pair.rust, name, len),
                global_bytes(&pair.c, name, len),
                "{}",
                String::from_utf8_lossy(name)
            );
        }
        for lib in [&pair.c, &pair.rust] {
            let symbol = lib
                .get::<*mut *const c_char>(b"cp_error_reason")
                .expect("cp_error_reason");
            **symbol = ptr::null();
            assert!((**symbol).is_null());
        }
    }
}

unsafe fn compare_convert(
    pair: &Pair,
    bpp: c_int,
    width: c_int,
    height: c_int,
    source: &[u8],
    source_offset: usize,
) -> Vec<Pixel> {
    let pixel_count = if width > 0 && height > 0 {
        width as usize * height as usize
    } else {
        16
    };
    let canary = Pixel {
        r: 0x11,
        g: 0x22,
        b: 0x33,
        a: 0x44,
    };
    let mut c_source = source.to_vec();
    let mut rust_source = source.to_vec();
    let mut c_output = vec![canary; pixel_count + 8];
    let mut rust_output = c_output.clone();
    Pair::convert_one(
        &pair.c,
        bpp,
        width,
        height,
        c_source.as_mut_ptr().add(source_offset),
        c_output.as_mut_ptr(),
    );
    Pair::convert_one(
        &pair.rust,
        bpp,
        width,
        height,
        rust_source.as_mut_ptr().add(source_offset),
        rust_output.as_mut_ptr(),
    );
    assert_eq!(rust_source, c_source);
    assert_eq!(rust_output, c_output);
    c_output
}

#[test]
fn convert_pix_all_formats_and_shapes() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let mut rng = Rng::new(0x0ddc_0ffe_e15e_beef);

        for bpp in 1..=4 {
            for &(width, height) in &[(1, 1), (2, 3), (7, 5)] {
                for _ in 0..32 {
                    let source = rng.bytes(height + width * height * bpp);
                    let output = compare_convert(
                        &pair,
                        bpp as c_int,
                        width as c_int,
                        height as c_int,
                        &source,
                        0,
                    );
                    for y in 0..height {
                        for x in 0..width {
                            let source_index = y * (1 + width * bpp) + 1 + x * bpp;
                            let expected = match bpp {
                                1 => Pixel {
                                    r: source[source_index],
                                    g: source[source_index],
                                    b: source[source_index],
                                    a: 0xff,
                                },
                                2 => Pixel {
                                    r: source[source_index],
                                    g: source[source_index],
                                    b: source[source_index],
                                    a: source[source_index + 1],
                                },
                                3 => Pixel {
                                    r: source[source_index],
                                    g: source[source_index + 1],
                                    b: source[source_index + 2],
                                    a: 0xff,
                                },
                                4 => Pixel {
                                    r: source[source_index],
                                    g: source[source_index + 1],
                                    b: source[source_index + 2],
                                    a: source[source_index + 3],
                                },
                                _ => unreachable!(),
                            };
                            assert_eq!(output[y * width + x], expected);
                        }
                    }
                }
            }
        }

        let source = rng.bytes(128);
        for &(width, height) in &[(0, 8), (-3, 8), (8, 0), (8, -3)] {
            compare_convert(&pair, 4, width, height, &source, 32);
        }

        let convert_c = pair.c.get::<ConvertFn>(b"convert_pix").unwrap();
        let convert_rust = pair.rust.get::<ConvertFn>(b"convert_pix").unwrap();
        convert_c(4, 1, 0, ptr::null_mut(), ptr::null_mut());
        convert_rust(4, 1, 0, ptr::null_mut(), ptr::null_mut());
        let mut row_bytes = [0_u8; 8];
        convert_c(4, 0, 3, row_bytes.as_mut_ptr(), ptr::null_mut());
        convert_rust(4, 0, 3, row_bytes.as_mut_ptr(), ptr::null_mut());
    }
}

#[test]
fn stored_blocks_randomized() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let mut rng = Rng::new(0x51_70_12_ed);
        for &length in &[0, 1, 2, 7, 31, 255, 1024] {
            for _ in 0..24 {
                let expected = rng.bytes(length);
                let stream = stored_stream(&expected, None, None);
                let exact = pair.compare_inflate(&stream, rng.range(0, 4), expected.len());
                assert_eq!(exact.status, 1);

                let spare = rng.range(1, 33);
                let larger = pair.compare_inflate(&stream, rng.range(0, 4), expected.len() + spare);
                assert_eq!(larger.status, 1);
                assert!(
                    larger.output[expected.len()..]
                        .iter()
                        .all(|&byte| byte == 0xcd)
                );
            }
        }
    }
}

#[test]
fn fixed_blocks_randomized_literals_and_copies() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let mut rng = Rng::new(0xf1_ed_5eed);

        for &length in &[0, 1, 2, 15, 128, 1024] {
            for _ in 0..32 {
                let expected = rng.bytes(length);
                let tokens: Vec<_> = expected.iter().copied().map(FixedToken::Literal).collect();
                let result =
                    pair.compare_inflate(&fixed_stream(&tokens), rng.range(0, 4), length + 7);
                assert_eq!(result.status, 1);
                assert_eq!(&result.output[..length], expected);
            }
        }

        for copies in 1..32 {
            let mut tokens = vec![FixedToken::Literal(rng.byte())];
            tokens.extend((0..copies).map(|_| FixedToken::Copy {
                length: rng.range(3, 11),
                distance: 1,
            }));
            let mut expected = Vec::new();
            for token in &tokens {
                match *token {
                    FixedToken::Literal(byte) => expected.push(byte),
                    FixedToken::Copy { length, distance } => {
                        for _ in 0..length {
                            expected.push(expected[expected.len() - distance]);
                        }
                    }
                }
            }
            let result = pair.compare_inflate(&fixed_stream(&tokens), copies & 3, expected.len());
            assert_eq!(result.status, 1);
            assert_eq!(result.output, expected);
        }

        for distance in 2..=16 {
            let mut expected: Vec<_> = (0..distance).map(|_| rng.byte()).collect();
            let mut tokens: Vec<_> = expected.iter().copied().map(FixedToken::Literal).collect();
            for _ in 0..12 {
                let length = rng.range(3, 20);
                tokens.push(FixedToken::Copy { length, distance });
                for _ in 0..length {
                    expected.push(expected[expected.len() - distance]);
                }
            }
            let result =
                pair.compare_inflate(&fixed_stream(&tokens), distance & 3, expected.len() + 5);
            assert_eq!(result.status, 1);
            assert_eq!(&result.output[..expected.len()], expected);
        }
    }
}

#[test]
fn dynamic_blocks_cover_code_length_and_copy_branches() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        for zero_symbol in [17, 18] {
            for count in [0, 1, 2, 7, 63, 511] {
                let stream = dynamic_single_symbol(count, zero_symbol);
                assert_eq!(block_type(&stream), 2);
                let result = pair.compare_inflate(&stream, count & 3, count + 11);
                assert_eq!(result.status, 1);
                assert_eq!(&result.output[..count], vec![b'A'; count]);
            }
        }

        let mut rng = Rng::new(0xd1_a0_16);
        for length in [1, 2, 9, 64, 513] {
            let payload: Vec<_> = (0..length).map(|_| b'A' + rng.range(0, 7) as u8).collect();
            let stream = dynamic_repeat_16(&payload);
            let result = pair.compare_inflate(&stream, length & 3, length);
            assert_eq!(result.status, 1);
            assert_eq!(result.output, payload);
        }

        for distance in [1, 2] {
            for copies in 1..32 {
                let (stream, expected) = dynamic_copy(distance, copies);
                let result = pair.compare_inflate(&stream, copies & 3, expected.len() + copies % 5);
                assert_eq!(result.status, 1);
                assert_eq!(&result.output[..expected.len()], expected);
            }
        }
    }
}

#[test]
fn compressor_generated_dynamic_corpus() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let mut rng = Rng::new(0xc0_5e_ed);
        let mut dynamic_count = 0;
        for case in 0..256 {
            let motif_len = rng.range(3, 48);
            let motif = rng.bytes(motif_len);
            let mut payload = Vec::new();
            for index in 0..rng.range(32, 256) {
                payload.push(motif[(index + case) % motif.len()]);
                if rng.next_u64() & 31 == 0 {
                    payload.push(rng.byte());
                }
            }
            let stream = flate2_stream(&payload, 6);
            if block_type(&stream) != 2 {
                continue;
            }
            let result = pair.compare_inflate(&stream, case & 3, payload.len() + 13);
            assert_eq!(result.status, 1);
            assert_eq!(&result.output[..payload.len()], payload);
            dynamic_count += 1;
            if dynamic_count == 32 {
                break;
            }
        }
        assert_eq!(
            dynamic_count, 32,
            "failed to generate enough dynamic blocks"
        );
    }
}

#[test]
fn multiblock_and_alignment_cross_product() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let first = [FixedToken::Literal(b'A'), FixedToken::Literal(b'B')];
        let second = [
            FixedToken::Literal(b'C'),
            FixedToken::Copy {
                length: 9,
                distance: 3,
            },
        ];
        let mut writer = BitWriter::default();
        write_fixed_block(&mut writer, false, &first);
        write_fixed_block(&mut writer, true, &second);
        let stream = writer.finish();
        let expected = b"ABCABCABCABC";
        let result = pair.compare_inflate(&stream, 0, expected.len());
        assert_eq!(result.status, 1);
        assert_eq!(result.output, expected);

        let base = fixed_stream(&[
            FixedToken::Literal(b'x'),
            FixedToken::Literal(b'y'),
            FixedToken::Literal(b'z'),
        ]);
        for input_mod4 in 0..4 {
            let first_bytes = (4 - input_mod4) & 3;
            for last_bytes in 0..4 {
                let mut aligned = base.clone();
                let current = (aligned.len() - first_bytes) & 3;
                let padding = (last_bytes + 4 - current) & 3;
                aligned.extend((0..padding).map(|index| 0x80 | index as u8));
                assert_eq!((aligned.len() - first_bytes) & 3, last_bytes);
                let result = pair.compare_inflate(&aligned, input_mod4, 9);
                assert_eq!(result.status, 1);
                assert_eq!(&result.output[..3], b"xyz");
            }
        }
    }
}

fn assert_error(result: &InflateResult, expected: &str) {
    assert_eq!(result.status, 0);
    assert_eq!(result.error.as_deref(), Some(expected));
}

#[test]
fn explicit_inflate_error_returns_match() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();

        let complement = stored_stream(&[], Some(0), Some(0));
        assert_error(
            &pair.compare_inflate(&complement, 0, 8),
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );

        let stored_end = stored_stream(&[0x77], Some(0), None);
        assert_error(
            &pair.compare_inflate(&stored_end, 0, 8),
            "Stored block extends beyond end of input stream.",
        );

        let literals = fixed_stream(&[
            FixedToken::Literal(b'a'),
            FixedToken::Literal(b'b'),
            FixedToken::Literal(b'c'),
            FixedToken::Literal(b'd'),
        ]);
        assert_error(
            &pair.compare_inflate(&literals, 0, 0),
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
        assert_error(
            &pair.compare_inflate(&literals, 1, 3),
            "Attempted to overwrite out buffer while outputting a symbol.",
        );

        let bad_distance = fixed_stream(&[FixedToken::Copy {
            length: 3,
            distance: 1,
        }]);
        assert_error(
            &pair.compare_inflate(&bad_distance, 2, 8),
            "Attempted to write before out buffer (invalid backwards distance).",
        );

        let string_overflow = fixed_stream(&[
            FixedToken::Literal(b'x'),
            FixedToken::Copy {
                length: 3,
                distance: 1,
            },
        ]);
        assert_error(
            &pair.compare_inflate(&string_overflow, 3, 3),
            "Attempted to overwrite out buffer while outputting a string.",
        );

        assert_error(
            &pair.compare_inflate(&[0x07], 0, 8),
            "Detected unknown block type within input stream.",
        );
    }
}

unsafe fn signed_output_call(lib: &Library, stream: &[u8], out_bytes: c_int) -> InflateResult {
    let inflate = lib.get::<InflateFn>(b"cp_inflate").unwrap();
    let error_symbol = lib.get::<*mut *const c_char>(b"cp_error_reason").unwrap();
    let error_slot = *error_symbol;
    *error_slot = ptr::null();
    let mut input = stream.to_vec();
    let mut output = vec![0xcd; 16];
    let status = inflate(
        input.as_mut_ptr().cast(),
        input.len() as c_int,
        output.as_mut_ptr().cast(),
        out_bytes,
    );
    let error_pointer = *error_slot;
    InflateResult {
        status,
        output,
        error: if error_pointer.is_null() {
            None
        } else {
            Some(CStr::from_ptr(error_pointer).to_string_lossy().into_owned())
        },
    }
}

#[test]
fn negative_output_length_matches() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let stream = fixed_stream(&[FixedToken::Literal(b'x')]);
        let c = signed_output_call(&pair.c, &stream, -1);
        let rust = signed_output_call(&pair.rust, &stream, -1);
        assert_eq!(rust, c);
        assert_error(
            &c,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
}

#[test]
fn unsupported_convert_formats_leave_destination_unchanged() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        let mut rng = Rng::new(0xba_d0_b0_0b);
        for &bpp in &[0, 5, 6, -1, -4] {
            for width in 1..=7 {
                let source = rng.bytes(512);
                let output = compare_convert(&pair, bpp, width, 3, &source, 192);
                assert!(output.iter().all(|pixel| {
                    *pixel
                        == Pixel {
                            r: 0x11,
                            g: 0x22,
                            b: 0x33,
                            a: 0x44,
                        }
                }));
            }
        }
    }
}

#[test]
fn internal_only_helpers_are_not_ffi_symbols() {
    let _guard = ffi_guard();
    unsafe {
        let pair = Pair::load();
        for name in [
            &b"cp_unfilter"[..],
            &b"cp_chunk"[..],
            &b"cp_find"[..],
            &b"cp_ptr"[..],
            &b"cp_peak_bits"[..],
            &b"cp_consume_bits"[..],
            &b"cp_read_bits"[..],
            &b"cp_build"[..],
            &b"cp_decode"[..],
        ] {
            assert!(pair.c.get::<*const c_void>(name).is_err());
            assert!(pair.rust.get::<*const c_void>(name).is_err());
        }
    }
}

#[test]
fn source_level_internal_rejections_are_preserved() {
    let _guard = ffi_guard();
    let c = include_str!("../c_src/src/lib.c");
    let rust = include_str!("../src/lib.rs");
    for assertion in [
        "assert(!(s->bits_left & 7))",
        "assert(s->word_index <= s->word_count)",
        "assert(s->count >= num_bits_to_read)",
        "assert(num_bits_to_read <= 32)",
        "assert(num_bits_to_read >= 0)",
        "assert(s->bits_left > 0)",
        "assert(s->count <= 64)",
        "assert(!cp_would_overflow(s, num_bits_to_read))",
        "assert(len < 16)",
        "assert((search >> len) == (key >> len))",
    ] {
        assert!(
            c.contains(assertion),
            "missing C assertion inventory: {assertion}"
        );
    }
    for assertion in [
        "assert!(s.bits_left & 7 == 0)",
        "assert!(s.word_index <= s.word_count)",
        "assert!(s.count >= num_bits_to_read)",
        "assert!(num_bits_to_read <= 32)",
        "assert!(num_bits_to_read >= 0)",
        "assert!(s.bits_left > 0)",
        "assert!(s.count <= 64)",
        "assert!(!cp_would_overflow(s, num_bits_to_read))",
        "assert!(bit_len < 16)",
        "assert!(search >> len == key >> len)",
    ] {
        assert!(
            rust.contains(assertion),
            "missing Rust assertion translation: {assertion}"
        );
    }
    assert_eq!(c.matches("default:\n      return 0;").count(), 2);
    assert_eq!(rust.matches("_ => return 0,").count(), 2);
    assert_eq!(c.matches("return 0;").count(), 7);
    assert!(rust.matches("ptr::null()").count() >= 2);
}

#[test]
fn abort_probe_child() {
    let _guard = ffi_guard();
    let Ok(library_kind) = std::env::var("DIFF_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("DIFF_CHILD_CASE").unwrap();
    let path = if library_kind == "c" {
        crate_path(C_SO)
    } else {
        rust_so_path()
    };
    unsafe {
        let library = Library::new(path).unwrap();
        let inflate = library.get::<InflateFn>(b"cp_inflate").unwrap();
        let mut storage = vec![0_u8; 64];
        let aligned_offset = (4 - (storage.as_ptr() as usize & 3)) & 3;
        let aligned = storage.as_mut_ptr().add(aligned_offset + 16);
        let mut output = [0_u8; 16];
        match case.as_str() {
            "initial_error" => {
                let error_symbol = library
                    .get::<*const *const c_char>(b"cp_error_reason")
                    .unwrap();
                assert!((**error_symbol).is_null());
            }
            "zero_input" => {
                inflate(aligned.cast(), 0, output.as_mut_ptr().cast(), 16);
            }
            "negative_input" => {
                inflate(aligned.cast(), -1, output.as_mut_ptr().cast(), 16);
            }
            "null_input" => {
                inflate(ptr::null_mut(), 1, output.as_mut_ptr().cast(), 16);
            }
            "null_output" => {
                let stream = fixed_stream(&[FixedToken::Literal(b'x')]);
                inflate(
                    stream.as_ptr().cast_mut().cast(),
                    stream.len() as c_int,
                    ptr::null_mut(),
                    1,
                );
            }
            _ => panic!("unknown child case"),
        };
    }
}

fn child_status(case: &str, library: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "abort_probe_child", "--nocapture"])
        .env("DIFF_CHILD_CASE", case)
        .env("DIFF_CHILD_LIBRARY", library)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn child_signal(case: &str, library: &str) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    child_status(case, library).signal()
}

#[test]
#[cfg(unix)]
fn abort_and_null_pointer_behavior_matches() {
    let _guard = ffi_guard();
    for case in ["zero_input", "negative_input", "null_input", "null_output"] {
        let c = child_signal(case, "c");
        let rust = child_signal(case, "rust");
        assert!(c.is_some(), "{case}: C unexpectedly returned normally");
        assert_eq!(rust, c, "{case}: termination signal differs");
    }
}
