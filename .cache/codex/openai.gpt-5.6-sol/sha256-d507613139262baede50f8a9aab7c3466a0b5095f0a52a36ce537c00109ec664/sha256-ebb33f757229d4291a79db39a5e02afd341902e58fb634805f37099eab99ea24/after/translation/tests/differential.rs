use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

static DIFF_LOCK: Mutex<()> = Mutex::new(());

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Image {
    w: c_int,
    h: c_int,
    pix: *mut Pixel,
}

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn setrlimit(resource: c_int, limits: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

type Inflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type LoadPng = unsafe extern "C" fn(*const u8, c_int) -> Image;

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            lib: unsafe { Library::new(path) }
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn inflate(&self, input: &mut [u8], output: &mut [u8]) -> c_int {
        let function = unsafe { self.lib.get::<Inflate>(b"cp_inflate") }.unwrap();
        unsafe {
            function(
                input.as_mut_ptr().cast(),
                input.len() as c_int,
                output.as_mut_ptr().cast(),
                output.len() as c_int,
            )
        }
    }

    unsafe fn inflate_raw(
        &self,
        input: *mut c_void,
        input_len: c_int,
        output: *mut c_void,
        output_len: c_int,
    ) -> c_int {
        let function = unsafe { self.lib.get::<Inflate>(b"cp_inflate") }.unwrap();
        unsafe { function(input, input_len, output, output_len) }
    }

    unsafe fn load_png(&self, png: &[u8]) -> (Image, Vec<u8>) {
        let function = unsafe { self.lib.get::<LoadPng>(b"load_png_mem") }.unwrap();
        let image = unsafe { function(png.as_ptr(), png.len() as c_int) };
        let bytes = if image.pix.is_null() || image.w <= 0 || image.h <= 0 {
            Vec::new()
        } else {
            unsafe {
                std::slice::from_raw_parts(
                    image.pix.cast::<u8>(),
                    image.w as usize * image.h as usize * 4,
                )
                .to_vec()
            }
        };
        (image, bytes)
    }

    unsafe fn load_png_raw(&self, data: *const u8, len: c_int) -> Image {
        let function = unsafe { self.lib.get::<LoadPng>(b"load_png_mem") }.unwrap();
        unsafe { function(data, len) }
    }

    unsafe fn error_reason(&self) -> Option<String> {
        let slot = unsafe {
            self.lib
                .get::<*mut *const c_char>(b"cp_error_reason")
                .unwrap()
        };
        let reason = unsafe { **slot };
        if reason.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(reason) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }

    unsafe fn bytes<const N: usize>(&self, symbol: &[u8]) -> Vec<u8> {
        let address = unsafe { self.lib.get::<*const [u8; N]>(symbol) }.unwrap();
        unsafe { (**address).to_vec() }
    }

    unsafe fn u32s<const N: usize>(&self, symbol: &[u8]) -> Vec<u32> {
        let address = unsafe { self.lib.get::<*const [u32; N]>(symbol) }.unwrap();
        unsafe { (**address).to_vec() }
    }

    unsafe fn set_byte<const N: usize>(&self, symbol: &[u8], index: usize, value: u8) {
        let address = unsafe { self.lib.get::<*mut [u8; N]>(symbol) }.unwrap();
        unsafe { (**address)[index] = value };
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-O1k6j1.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libload_png_mem_lib.so")
}

fn apis() -> (Api, Api) {
    assert!(
        c_library_path().is_file(),
        "C shared library missing at {}",
        c_library_path().display()
    );
    assert!(
        rust_library_path().is_file(),
        "Rust shared library missing at {}",
        rust_library_path().display()
    );
    unsafe {
        (
            Api::open(&c_library_path()),
            Api::open(&rust_library_path()),
        )
    }
}

unsafe fn release_image(image: Image) {
    if !image.pix.is_null() {
        unsafe { free(image.pix.cast()) };
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

impl BitWriter {
    fn write(&mut self, mut value: u32, bits: u8) {
        for _ in 0..bits {
            if self.bit == 0 {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= ((value & 1) as u8) << self.bit;
            value >>= 1;
            self.bit = (self.bit + 1) & 7;
        }
    }

    fn align_byte(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
        }
    }

    fn extend_bytes(&mut self, bytes: &[u8]) {
        assert_eq!(self.bit, 0);
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn reverse_bits(mut value: u32, bits: u8) -> u32 {
    let mut result = 0;
    for _ in 0..bits {
        result = (result << 1) | (value & 1);
        value >>= 1;
    }
    result
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, u8)> {
    let mut counts = [0u32; 16];
    for &length in lengths {
        if length != 0 {
            counts[length as usize] += 1;
        }
    }
    let mut next = [0u32; 16];
    let mut code = 0;
    for bits in 1..=15 {
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
                (reverse_bits(code, length), length)
            }
        })
        .collect()
}

fn write_symbol(writer: &mut BitWriter, codes: &[(u32, u8)], symbol: usize) {
    let (code, length) = codes[symbol];
    assert_ne!(length, 0, "attempted to encode absent symbol {symbol}");
    writer.write(code, length);
}

fn fixed_lengths() -> (Vec<u8>, Vec<u8>) {
    let mut literal = vec![0u8; 288];
    literal[..144].fill(8);
    literal[144..256].fill(9);
    literal[256..280].fill(7);
    literal[280..].fill(8);
    (literal, vec![5; 32])
}

fn write_fixed_header(writer: &mut BitWriter, final_block: bool) {
    writer.write(final_block as u32, 1);
    writer.write(1, 2);
}

fn write_fixed_literals(writer: &mut BitWriter, final_block: bool, data: &[u8]) {
    let (literal_lengths, _) = fixed_lengths();
    let literal_codes = canonical_codes(&literal_lengths);
    write_fixed_header(writer, final_block);
    for &byte in data {
        write_symbol(writer, &literal_codes, byte as usize);
    }
    write_symbol(writer, &literal_codes, 256);
}

fn fixed_literal_stream(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    write_fixed_literals(&mut writer, true, data);
    writer.finish()
}

fn fixed_copy_stream(seed: &[u8], length_symbol: usize, distance_symbol: usize) -> Vec<u8> {
    let (literal_lengths, distance_lengths) = fixed_lengths();
    let literal_codes = canonical_codes(&literal_lengths);
    let distance_codes = canonical_codes(&distance_lengths);
    let mut writer = BitWriter::default();
    write_fixed_header(&mut writer, true);
    for &byte in seed {
        write_symbol(&mut writer, &literal_codes, byte as usize);
    }
    write_symbol(&mut writer, &literal_codes, length_symbol);
    write_symbol(&mut writer, &distance_codes, distance_symbol);
    write_symbol(&mut writer, &literal_codes, 256);
    writer.finish()
}

fn stored_stream(data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= u16::MAX as usize);
    let mut writer = BitWriter::default();
    writer.write(1, 1);
    writer.write(0, 2);
    writer.align_byte();
    let length = data.len() as u16;
    writer.extend_bytes(&length.to_le_bytes());
    writer.extend_bytes(&(!length).to_le_bytes());
    writer.extend_bytes(data);
    writer.finish()
}

fn write_dynamic_literal_block(writer: &mut BitWriter, final_block: bool, data: &[u8]) {
    writer.write(final_block as u32, 1);
    writer.write(2, 2);
    writer.write(0, 5);
    writer.write(0, 5);
    writer.write(14, 4);

    let permutation = [
        16usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_length_lengths = vec![0u8; 19];
    for symbol in [0usize, 1, 8, 9, 16, 17, 18] {
        code_length_lengths[symbol] = 3;
    }
    for &symbol in &permutation[..18] {
        writer.write(code_length_lengths[symbol] as u32, 3);
    }
    let code_length_codes = canonical_codes(&code_length_lengths);

    write_symbol(writer, &code_length_codes, 8);
    let mut repeated = 254usize;
    while repeated >= 3 {
        let count = repeated.min(6);
        if repeated - count < 3 && repeated - count != 0 {
            break;
        }
        write_symbol(writer, &code_length_codes, 16);
        writer.write((count - 3) as u32, 2);
        repeated -= count;
    }
    for _ in 0..repeated {
        write_symbol(writer, &code_length_codes, 8);
    }
    write_symbol(writer, &code_length_codes, 9);
    write_symbol(writer, &code_length_codes, 9);
    write_symbol(writer, &code_length_codes, 1);

    let mut literal_lengths = vec![8u8; 257];
    literal_lengths[255] = 9;
    literal_lengths[256] = 9;
    let literal_codes = canonical_codes(&literal_lengths);
    for &byte in data {
        write_symbol(writer, &literal_codes, byte as usize);
    }
    write_symbol(writer, &literal_codes, 256);
}

fn dynamic_literal_stream(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    write_dynamic_literal_block(&mut writer, true, data);
    writer.finish()
}

fn write_zero_run(writer: &mut BitWriter, codes: &[(u32, u8)], mut count: usize) {
    while count >= 11 {
        let run = count.min(138);
        write_symbol(writer, codes, 18);
        writer.write((run - 11) as u32, 7);
        count -= run;
    }
    while count >= 3 {
        let run = count.min(10);
        write_symbol(writer, codes, 17);
        writer.write((run - 3) as u32, 3);
        count -= run;
    }
    for _ in 0..count {
        write_symbol(writer, codes, 0);
    }
}

fn dynamic_repeat_stream(count: usize) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.write(1, 1);
    writer.write(2, 2);
    writer.write(0, 5);
    writer.write(0, 5);
    writer.write(14, 4);
    let permutation = [
        16usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_length_lengths = vec![0u8; 19];
    for symbol in [0usize, 1, 16, 17, 18] {
        code_length_lengths[symbol] = 3;
    }
    for &symbol in &permutation[..18] {
        writer.write(code_length_lengths[symbol] as u32, 3);
    }
    let code_length_codes = canonical_codes(&code_length_lengths);
    write_zero_run(&mut writer, &code_length_codes, 65);
    write_symbol(&mut writer, &code_length_codes, 1);
    write_zero_run(&mut writer, &code_length_codes, 190);
    write_symbol(&mut writer, &code_length_codes, 1);
    write_symbol(&mut writer, &code_length_codes, 1);

    let mut literal_lengths = vec![0u8; 257];
    literal_lengths[65] = 1;
    literal_lengths[256] = 1;
    let literal_codes = canonical_codes(&literal_lengths);
    for _ in 0..count {
        write_symbol(&mut writer, &literal_codes, 65);
    }
    write_symbol(&mut writer, &literal_codes, 256);
    writer.finish()
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn byte(&mut self) -> u8 {
        self.next_u32() as u8
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + self.next_u32() as usize % (end - start)
    }
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn filtered_scanlines(
    pixels: &[u8],
    width: usize,
    height: usize,
    bpp: usize,
    filter: u8,
) -> Vec<u8> {
    let stride = width * bpp;
    assert_eq!(pixels.len(), stride * height);
    let mut output = Vec::with_capacity((stride + 1) * height);
    for y in 0..height {
        output.push(filter);
        for x in 0..stride {
            let current = pixels[y * stride + x];
            let left = if x >= bpp {
                pixels[y * stride + x - bpp]
            } else {
                0
            };
            let up = if y > 0 {
                pixels[(y - 1) * stride + x]
            } else {
                0
            };
            let up_left = if y > 0 && x >= bpp {
                pixels[(y - 1) * stride + x - bpp]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => paeth(left, up, up_left),
                _ => 0,
            };
            output.push(current.wrapping_sub(predictor));
        }
    }
    output
}

fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() + 12);
    result.extend_from_slice(&(data.len() as u32).to_be_bytes());
    result.extend_from_slice(name);
    result.extend_from_slice(data);
    result.extend_from_slice(&[0; 4]);
    result
}

#[derive(Default)]
struct PngOptions {
    palette: Option<Vec<u8>>,
    transparency: Option<Vec<u8>>,
    idat_splits: Vec<usize>,
    before_palette: Vec<([u8; 4], Vec<u8>)>,
    before_idat: Vec<([u8; 4], Vec<u8>)>,
    compression: u8,
    filter_method: u8,
    interlace: u8,
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut result = vec![0x78, 0x01];
    result.extend_from_slice(&stored_stream(data));
    result.extend_from_slice(&[0, 0, 0, 0]);
    result
}

fn make_png(
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    filtered: &[u8],
    options: &PngOptions,
) -> Vec<u8> {
    let mut result = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(bit_depth);
    ihdr.push(color_type);
    ihdr.push(options.compression);
    ihdr.push(options.filter_method);
    ihdr.push(options.interlace);
    result.extend_from_slice(&chunk(b"IHDR", &ihdr));
    for (name, data) in &options.before_palette {
        result.extend_from_slice(&chunk(name, data));
    }
    if let Some(palette) = &options.palette {
        result.extend_from_slice(&chunk(b"PLTE", palette));
    }
    if let Some(transparency) = &options.transparency {
        result.extend_from_slice(&chunk(b"tRNS", transparency));
    }
    for (name, data) in &options.before_idat {
        result.extend_from_slice(&chunk(name, data));
    }
    let zlib = zlib_stored(filtered);
    if options.idat_splits.is_empty() {
        result.extend_from_slice(&chunk(b"IDAT", &zlib));
    } else {
        let mut start = 0;
        for &end in &options.idat_splits {
            let end = end.min(zlib.len()).max(start);
            result.extend_from_slice(&chunk(b"IDAT", &zlib[start..end]));
            start = end;
        }
        result.extend_from_slice(&chunk(b"IDAT", &zlib[start..]));
    }
    result.extend_from_slice(&chunk(b"IEND", &[]));
    result
}

fn make_png_with_idat(width: u32, height: u32, color_type: u8, idat: &[u8]) -> Vec<u8> {
    let mut result = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    result.extend_from_slice(&chunk(b"IHDR", &ihdr));
    result.extend_from_slice(&chunk(b"IDAT", idat));
    result.extend_from_slice(&chunk(b"IEND", &[]));
    result
}

fn compare_inflate(input: &[u8], output_len: usize, expected: Option<&[u8]>) {
    let _guard = DIFF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rust) = apis();
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let mut c_output = vec![0xa5; output_len];
    let mut rust_output = vec![0xa5; output_len];
    let c_result = unsafe { c.inflate(&mut c_input, &mut c_output) };
    let rust_result = unsafe { rust.inflate(&mut rust_input, &mut rust_output) };
    assert_eq!(rust_result, c_result, "return mismatch for {input:02x?}");
    assert_eq!(rust_output, c_output, "output mismatch for {input:02x?}");
    assert_eq!(c_result, 1, "C rejected a valid-path fixture");
    if let Some(expected) = expected {
        assert_eq!(&c_output[..expected.len()], expected);
    }
}

fn compare_png(png: &[u8]) -> (Image, Vec<u8>) {
    let _guard = DIFF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rust) = apis();
    let (c_image, c_bytes) = unsafe { c.load_png(png) };
    let (rust_image, rust_bytes) = unsafe { rust.load_png(png) };
    assert_eq!(rust_image.w, c_image.w, "width mismatch");
    assert_eq!(rust_image.h, c_image.h, "height mismatch");
    assert_eq!(
        rust_image.pix.is_null(),
        c_image.pix.is_null(),
        "success/null mismatch; C={:?}, Rust={:?}",
        unsafe { c.error_reason() },
        unsafe { rust.error_reason() }
    );
    assert_eq!(rust_bytes, c_bytes, "pixel mismatch");
    unsafe {
        release_image(rust_image);
    }
    (c_image, c_bytes)
}

fn compare_inflate_error(input: &[u8], output_len: usize, expected_reason: &str) {
    let _guard = DIFF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rust) = apis();
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let mut c_output = vec![0xa5; output_len];
    let mut rust_output = vec![0xa5; output_len];
    let c_result = unsafe { c.inflate(&mut c_input, &mut c_output) };
    let rust_result = unsafe { rust.inflate(&mut rust_input, &mut rust_output) };
    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);
    let c_reason = unsafe { c.error_reason() };
    let rust_reason = unsafe { rust.error_reason() };
    assert_eq!(c_reason.as_deref(), Some(expected_reason));
    assert_eq!(rust_reason, c_reason);
}

fn compare_png_error(png: &[u8], expected_reason: &str) {
    let _guard = DIFF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rust) = apis();
    let (c_image, c_bytes) = unsafe { c.load_png(png) };
    let (rust_image, rust_bytes) = unsafe { rust.load_png(png) };
    assert!(
        c_image.pix.is_null(),
        "C unexpectedly accepted error fixture"
    );
    assert!(
        rust_image.pix.is_null(),
        "Rust unexpectedly accepted error fixture"
    );
    assert_eq!((rust_image.w, rust_image.h), (c_image.w, c_image.h));
    assert_eq!(rust_bytes, c_bytes);
    let c_reason = unsafe { c.error_reason() };
    let rust_reason = unsafe { rust.error_reason() };
    assert_eq!(c_reason.as_deref(), Some(expected_reason));
    assert_eq!(rust_reason, c_reason);
}

#[test]
fn exported_data_symbols_match() {
    let _guard = DIFF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (c, rust) = apis();
    unsafe {
        assert_eq!(
            rust.bytes::<320>(b"cp_fixed_table"),
            c.bytes::<320>(b"cp_fixed_table")
        );
        assert_eq!(
            rust.bytes::<19>(b"cp_permutation_order"),
            c.bytes::<19>(b"cp_permutation_order")
        );
        assert_eq!(
            rust.bytes::<31>(b"cp_len_extra_bits"),
            c.bytes::<31>(b"cp_len_extra_bits")
        );
        assert_eq!(
            rust.u32s::<31>(b"cp_len_base"),
            c.u32s::<31>(b"cp_len_base")
        );
        assert_eq!(
            rust.bytes::<32>(b"cp_dist_extra_bits"),
            c.bytes::<32>(b"cp_dist_extra_bits")
        );
        assert_eq!(
            rust.u32s::<32>(b"cp_dist_base"),
            c.u32s::<32>(b"cp_dist_base")
        );
    }
}

#[test]
fn inflate_stored_randomized_and_aligned() {
    let mut random = Lcg::new(0x11c0ffee);
    for length in [0usize, 1, 2, 3, 7, 31, 255] {
        for _ in 0..12 {
            let data: Vec<u8> = (0..length).map(|_| random.byte()).collect();
            let stream = stored_stream(&data);
            for alignment in 0..4 {
                let mut aligned = vec![0u8; alignment];
                aligned.extend_from_slice(&stream);
                let slice = &aligned[alignment..];
                compare_inflate(slice, data.len(), None);
                compare_inflate(slice, data.len() + 17, None);
            }
        }
    }
}

#[test]
fn inflate_fixed_literals_randomized() {
    let mut random = Lcg::new(0xf17ed123);
    for _ in 0..80 {
        let length = random.range(0, 300);
        let data: Vec<u8> = (0..length).map(|_| random.byte()).collect();
        compare_inflate(&fixed_literal_stream(&data), length + 8, Some(&data));
    }
}

#[test]
fn inflate_fixed_length_distance_modes() {
    for repeated in 1..80 {
        let stream = fixed_copy_stream(&[b'Z'], 257, 0);
        compare_inflate(&stream, 4, Some(b"ZZZZ"));
        if repeated % 3 == 0 {
            let stream = fixed_copy_stream(b"abc", 260, 2);
            compare_inflate(&stream, 9, Some(b"abcabcabc"));
        }
    }
}

#[test]
fn inflate_dynamic_literals_and_repeat_codes() {
    let mut random = Lcg::new(0xd1a0_2026);
    for _ in 0..64 {
        let length = random.range(0, 220);
        let data: Vec<u8> = (0..length).map(|_| random.byte()).collect();
        compare_inflate(&dynamic_literal_stream(&data), length + 5, Some(&data));
    }
    for count in [0usize, 1, 2, 3, 17, 127] {
        let expected = vec![b'A'; count];
        compare_inflate(&dynamic_repeat_stream(count), count + 3, Some(&expected));
    }
}

#[test]
fn inflate_multiple_blocks() {
    let mut random = Lcg::new(0xb10c_5eed);
    for _ in 0..40 {
        let left: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let right: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let mut writer = BitWriter::default();
        write_fixed_literals(&mut writer, false, &left);
        write_fixed_literals(&mut writer, true, &right);
        let mut expected = left;
        expected.extend_from_slice(&right);
        compare_inflate(&writer.finish(), expected.len() + 9, Some(&expected));

        let dynamic_left: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let fixed_right: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let mut writer = BitWriter::default();
        write_dynamic_literal_block(&mut writer, false, &dynamic_left);
        write_fixed_literals(&mut writer, true, &fixed_right);
        let mut expected = dynamic_left;
        expected.extend_from_slice(&fixed_right);
        compare_inflate(&writer.finish(), expected.len() + 9, Some(&expected));

        let fixed_left: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let dynamic_right: Vec<u8> = (0..random.range(0, 80)).map(|_| random.byte()).collect();
        let mut writer = BitWriter::default();
        write_fixed_literals(&mut writer, false, &fixed_left);
        write_dynamic_literal_block(&mut writer, true, &dynamic_right);
        let mut expected = fixed_left;
        expected.extend_from_slice(&dynamic_right);
        compare_inflate(&writer.finish(), expected.len() + 9, Some(&expected));
    }
}

#[test]
fn png_all_color_and_filter_combinations_randomized() {
    let mut random = Lcg::new(0x504e_4726);
    for (color_type, bpp) in [(0u8, 1usize), (2, 3), (3, 1), (4, 2), (6, 4)] {
        for filter in 0u8..=4 {
            for iteration in 0..16 {
                let (width, height) = match iteration % 4 {
                    0 => (1, 1),
                    1 => (1, random.range(2, 8)),
                    2 => (random.range(2, 9), 1),
                    _ => (random.range(2, 9), random.range(2, 8)),
                };
                let mut pixels: Vec<u8> =
                    (0..width * height * bpp).map(|_| random.byte()).collect();
                let mut options = PngOptions::default();
                if color_type == 3 {
                    for index in &mut pixels {
                        *index %= 16;
                    }
                    options.palette = Some(
                        (0..16 * 3)
                            .map(|index| (index as u8).wrapping_mul(17))
                            .collect(),
                    );
                }
                let filtered = filtered_scanlines(&pixels, width, height, bpp, filter);
                let png = make_png(
                    width as u32,
                    height as u32,
                    8,
                    color_type,
                    &filtered,
                    &options,
                );
                let (c_image, _) = compare_png(&png);
                assert_eq!(c_image.w, width as c_int);
                assert_eq!(c_image.h, height as c_int);
                unsafe { release_image(c_image) };
            }
        }
    }
}

#[test]
fn png_palette_transparency_shapes() {
    let palette: Vec<u8> = (0..24).map(|value| value * 9).collect();
    let pixels = [0u8, 1, 2, 3, 4, 5, 6, 7];
    let filtered = filtered_scanlines(&pixels, 4, 2, 1, 4);
    for transparency in [
        None,
        Some(vec![1, 2, 3]),
        Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
    ] {
        let options = PngOptions {
            palette: Some(palette.clone()),
            transparency,
            ..PngOptions::default()
        };
        let png = make_png(4, 2, 8, 3, &filtered, &options);
        let (c_image, _) = compare_png(&png);
        unsafe { release_image(c_image) };
    }
}

#[test]
fn png_chunk_layout_variants() {
    let mut random = Lcg::new(0xc4a6_5eed);
    for _ in 0..32 {
        let width = random.range(1, 10);
        let height = random.range(1, 8);
        let pixels: Vec<u8> = (0..width * height * 4).map(|_| random.byte()).collect();
        let filtered = filtered_scanlines(&pixels, width, height, 4, random.range(0, 5) as u8);
        let zlib_len = zlib_stored(&filtered).len();
        let split1 = random.range(1, zlib_len);
        let split2 = random.range(split1, zlib_len);
        let options = PngOptions {
            palette: Some(vec![1, 2, 3, 4, 5, 6]),
            transparency: Some(vec![4, 5]),
            idat_splits: vec![split1, split2],
            before_palette: vec![(*b"aaAA", vec![9, 8, 7])],
            before_idat: vec![(*b"bbBB", vec![6, 5, 4, 3])],
            ..PngOptions::default()
        };
        let png = make_png(width as u32, height as u32, 8, 6, &filtered, &options);
        let (c_image, _) = compare_png(&png);
        unsafe { release_image(c_image) };

        let zlib = zlib_stored(&filtered);
        for splits in [
            vec![1],
            vec![2],
            vec![2, 3],
            vec![zlib.len().saturating_sub(4)],
            vec![2, zlib.len().saturating_sub(4)],
        ] {
            let options = PngOptions {
                idat_splits: splits,
                ..PngOptions::default()
            };
            let png = make_png(width as u32, height as u32, 8, 6, &filtered, &options);
            let (c_image, _) = compare_png(&png);
            unsafe { release_image(c_image) };
        }
    }
}

#[test]
fn inflate_explicit_error_returns() {
    compare_inflate_error(
        &[0x01, 0x01, 0x00, 0x00, 0x00, b'X'],
        1,
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
    );
    compare_inflate_error(
        &[0x01, 0x00, 0x00, 0xff, 0xff, 0x00],
        1,
        "Stored block extends beyond end of input stream.",
    );
    compare_inflate_error(
        &fixed_literal_stream(b"X"),
        0,
        "Attempted to overwrite out buffer while outputting a symbol.",
    );
    compare_inflate_error(
        &fixed_copy_stream(&[], 257, 0),
        3,
        "Attempted to write before out buffer (invalid backwards distance).",
    );
    compare_inflate_error(
        &fixed_copy_stream(b"Z", 257, 0),
        3,
        "Attempted to overwrite out buffer while outputting a string.",
    );
    compare_inflate_error(
        &[0x07],
        0,
        "Detected unknown block type within input stream.",
    );
}

#[test]
fn png_header_and_ihdr_errors() {
    compare_png_error(
        b"not png!",
        "incorrect file signature (is this a png file?)",
    );

    let mut missing_ihdr = b"\x89PNG\r\n\x1a\n".to_vec();
    missing_ihdr.extend_from_slice(&chunk(b"IEND", &[]));
    compare_png_error(&missing_ihdr, "unable to find IHDR chunk");

    let valid = make_png(1, 1, 8, 6, &[0, 1, 2, 3, 4], &PngOptions::default());
    for (offset, value, reason) in [
        (24usize, 16u8, "only bit-depth of 8 is supported"),
        (25, 1, "unknown color type"),
        (26, 1, "only standard compression DEFLATE is supported"),
        (27, 1, "only standard adaptive filtering is supported"),
        (28, 1, "interlacing is not supported"),
    ] {
        let mut png = valid.clone();
        png[offset] = value;
        compare_png_error(&png, reason);
    }

    let mut zero_width = valid.clone();
    zero_width[16..20].copy_from_slice(&u32::MAX.to_be_bytes());
    compare_png_error(
        &zero_width,
        "invalid IHDR chunk found, image width was less than 1",
    );

    let mut zero_height = valid.clone();
    zero_height[20..24].copy_from_slice(&0u32.to_be_bytes());
    compare_png_error(
        &zero_height,
        "invalid IHDR chunk found, image height was less than 1",
    );

    let mut too_large = valid;
    too_large[16..20].copy_from_slice(&30_000u32.to_be_bytes());
    too_large[20..24].copy_from_slice(&30_000u32.to_be_bytes());
    compare_png_error(&too_large, "image too large");
}

#[test]
fn png_zlib_filter_and_palette_errors() {
    let no_idat = {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&1u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        png.extend_from_slice(&chunk(b"IHDR", &ihdr));
        png.extend_from_slice(&chunk(b"IEND", &[]));
        png
    };
    compare_png_error(&no_idat, "corrupt zlib structure in DEFLATE stream");
    compare_png_error(
        &make_png_with_idat(1, 1, 6, &[0; 5]),
        "corrupt zlib structure in DEFLATE stream",
    );
    compare_png_error(
        &make_png_with_idat(1, 1, 6, &[0x79, 0, 0, 0, 0, 0]),
        "only zlib compression method (RFC 1950) is supported",
    );
    compare_png_error(
        &make_png_with_idat(1, 1, 6, &[0x88, 0, 0, 0, 0, 0]),
        "innapropriate window size detected",
    );
    compare_png_error(
        &make_png_with_idat(1, 1, 6, &[0x78, 0x20, 0, 0, 0, 0]),
        "preset dictionary is present and not supported",
    );
    compare_png_error(
        &make_png_with_idat(1, 1, 6, &[0x78, 0, 0x07, 0, 0, 0, 0]),
        "DEFLATE algorithm failed",
    );

    let first_bad_filter = make_png(1, 1, 8, 6, &[5, 1, 2, 3, 4], &PngOptions::default());
    compare_png_error(&first_bad_filter, "invalid filter byte found");
    let later_bad_filter = make_png(
        1,
        2,
        8,
        6,
        &[0, 1, 2, 3, 4, 5, 5, 6, 7, 8],
        &PngOptions::default(),
    );
    compare_png_error(&later_bad_filter, "invalid filter byte found");

    let indexed_without_palette = make_png(1, 1, 8, 3, &[0, 0], &PngOptions::default());
    compare_png_error(
        &indexed_without_palette,
        "color type of indexed requires a PLTE chunk",
    );
}

#[test]
fn arithmetic_guards_follow_prior_public_checks() {
    // The two cp_out_size rejection branches are source-level guards that cannot
    // be reached after the preceding accepted-dimension predicate:
    // 0 < (w * h * 4) < INT_MAX and bpp is one of 1,2,3,4.
    for bpp in 1i64..=4 {
        for (w, h) in [(1i64, 1i64), (2, 1), (1, 2), (1024, 1024), (536_870_910, 1)] {
            let rgba = w * h * 4;
            if rgba < c_int::MAX as i64 {
                assert!(rgba >= 1);
                assert!(w * h * bpp >= 1);
                assert!(w * h * bpp < c_int::MAX as i64);
            }
        }
    }
}

#[test]
fn internal_assertion_surface_is_accounted_for() {
    let c = include_str!("../../c_src/src/lib.c");
    let rust = include_str!("../src/lib.rs");
    assert_eq!(c.matches("assert(").count(), 10);
    for c_assertion in [
        "assert(!(s->bits_left & 7));",
        "assert(s->word_index <= s->word_count);",
        "assert(s->count >= num_bits_to_read);",
        "assert(num_bits_to_read <= 32);",
        "assert(num_bits_to_read >= 0);",
        "assert(s->bits_left > 0);",
        "assert(s->count <= 64);",
        "assert(!cp_would_overflow(s, num_bits_to_read));",
        "assert(len < 16);",
        "assert((search >> len) == (key >> len));",
    ] {
        assert!(c.contains(c_assertion), "missing C assertion {c_assertion}");
    }
    for rust_assertion in [
        "assert!((*s).bits_left & 7 == 0);",
        "assert!((*s).word_index <= (*s).word_count);",
        "assert!((*s).count >= num_bits_to_read);",
        "assert!(num_bits_to_read <= 32);",
        "assert!(num_bits_to_read >= 0);",
        "assert!((*s).bits_left > 0);",
        "assert!((*s).count <= 64);",
        "assert!(!would_overflow(s, num_bits_to_read));",
        "assert!(tree_len < 16);",
        "assert!((search >> length_shift) == (key >> length_shift));",
    ] {
        assert!(
            rust.contains(rust_assertion),
            "missing Rust assertion {rust_assertion}"
        );
    }

    // These four assertions are invariant guards rather than independently
    // selectable public-input branches.
    assert!(c.contains("cp_read_bits(s, s->count & 7);"));
    assert!(c.contains("if (s->word_index < s->word_count)"));
    assert!(c.contains("uint8_t cp_len_extra_bits"));
    assert!(c.contains("if (s->count < num_bits_to_read)"));
}

fn run_child(library: &str, case: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ffi_boundary_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .output()
        .unwrap()
}

fn child_marker(output: &Output) -> Option<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            line.split_once("CHILD_RESULT:")
                .map(|(_, marker)| marker.to_owned())
        })
}

#[test]
fn ffi_boundary_process_results_match() {
    for case in [
        "inflate_empty",
        "inflate_fixed_truncated",
        "inflate_null_input",
        "inflate_null_output",
        "load_null",
        "load_zero_length",
        "load_negative_length",
        "fixed_length_16",
        "read_more_than_32",
        "decode_prefix_mismatch",
        "inflate_negative_length",
        "inflate_oversized_length",
        "inflate_negative_output_length",
        "inflate_oversized_output_length",
        "load_oversized_length",
        "initial_error_reason",
    ] {
        let c = run_child("c", case);
        let rust = run_child("rust", case);
        assert_eq!(
            rust.status.signal(),
            c.status.signal(),
            "signal mismatch for {case}\nC stderr:\n{}\nRust stderr:\n{}",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr)
        );
        assert_eq!(
            rust.status.code(),
            c.status.code(),
            "exit-code mismatch for {case}"
        );
        assert_eq!(
            child_marker(&rust),
            child_marker(&c),
            "normal-result mismatch for {case}"
        );
    }
}

#[test]
fn allocation_failure_results_match() {
    let c = run_child("c", "malloc_failure");
    let rust = run_child("rust", "malloc_failure");
    assert!(
        c.status.success(),
        "C child failed: {}",
        String::from_utf8_lossy(&c.stderr)
    );
    assert!(
        rust.status.success(),
        "Rust child failed: {}",
        String::from_utf8_lossy(&rust.stderr)
    );
    assert_eq!(child_marker(&rust), child_marker(&c));
    assert_eq!(
        child_marker(&c).as_deref(),
        Some("1:10000:10000:unable to allocate raw image space")
    );
}

#[test]
fn ffi_boundary_child() {
    let Ok(case) = std::env::var("DIFF_CHILD_CASE") else {
        return;
    };
    let library = std::env::var("DIFF_CHILD_LIBRARY").unwrap();
    let path = if library == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::open(&path) };
    match case.as_str() {
        "inflate_empty" => {
            let mut byte = 0u8;
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    (&mut byte as *mut u8).cast(),
                    0,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}:{:?}", unsafe { api.error_reason() });
        }
        "inflate_fixed_truncated" => {
            let mut input = [0x03u8];
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}:{:?}", unsafe { api.error_reason() });
        }
        "inflate_null_input" => {
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    std::ptr::null_mut(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}");
        }
        "inflate_null_output" => {
            let mut input = fixed_literal_stream(b"X");
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    std::ptr::null_mut(),
                    8,
                )
            };
            println!("CHILD_RESULT:{result}");
        }
        "load_null" => {
            let image = unsafe { api.load_png_raw(std::ptr::null(), 8) };
            println!(
                "CHILD_RESULT:{}:{}:{}",
                image.pix.is_null() as u8,
                image.w,
                image.h
            );
            unsafe { release_image(image) };
        }
        "load_zero_length" => {
            let data = b"\x89PNG\r\n\x1a\n";
            let image = unsafe { api.load_png_raw(data.as_ptr(), 0) };
            println!(
                "CHILD_RESULT:{}:{}:{}:{:?}",
                image.pix.is_null() as u8,
                image.w,
                image.h,
                unsafe { api.error_reason() }
            );
            unsafe { release_image(image) };
        }
        "load_negative_length" => {
            let data = [0u8; 32];
            let image = unsafe { api.load_png_raw(data.as_ptr(), -1) };
            println!(
                "CHILD_RESULT:{}:{}:{}:{:?}",
                image.pix.is_null() as u8,
                image.w,
                image.h,
                unsafe { api.error_reason() }
            );
            unsafe { release_image(image) };
        }
        "fixed_length_16" => {
            unsafe { api.set_byte::<320>(b"cp_fixed_table", 0, 16) };
            let mut input = fixed_literal_stream(b"X");
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}");
        }
        "read_more_than_32" => {
            unsafe { api.set_byte::<31>(b"cp_len_extra_bits", 0, 33) };
            let mut input = fixed_copy_stream(b"Z", 257, 0);
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}");
        }
        "decode_prefix_mismatch" => {
            for index in 0..288 {
                unsafe { api.set_byte::<320>(b"cp_fixed_table", index, 0) };
            }
            unsafe { api.set_byte::<320>(b"cp_fixed_table", 256, 1) };
            let mut input = [0x0bu8, 0, 0, 0];
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                )
            };
            println!("CHILD_RESULT:{result}");
        }
        "inflate_negative_length" => {
            let mut input = [0u8; 8];
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(input.as_mut_ptr().cast(), -1, output.as_mut_ptr().cast(), 8)
            };
            println!("CHILD_RESULT:{result}:{:?}", unsafe { api.error_reason() });
        }
        "inflate_oversized_length" => {
            let mut input = [0x07u8, 0, 0, 0, 0, 0, 0, 0];
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    c_int::MAX,
                    output.as_mut_ptr().cast(),
                    8,
                )
            };
            println!("CHILD_RESULT:{result}:{:?}", unsafe { api.error_reason() });
        }
        "inflate_negative_output_length" => {
            let mut input = fixed_literal_stream(b"X");
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    -1,
                )
            };
            println!("CHILD_RESULT:{result}:{:?}", unsafe { api.error_reason() });
        }
        "inflate_oversized_output_length" => {
            let mut input = fixed_literal_stream(b"X");
            let mut output = [0u8; 8];
            let result = unsafe {
                api.inflate_raw(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    c_int::MAX,
                )
            };
            println!(
                "CHILD_RESULT:{result}:{:?}:{}",
                unsafe { api.error_reason() },
                output[0]
            );
        }
        "load_oversized_length" => {
            let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
            data.extend_from_slice(&chunk(b"IEND", &[]));
            let image = unsafe { api.load_png_raw(data.as_ptr(), c_int::MAX) };
            println!(
                "CHILD_RESULT:{}:{}:{}:{:?}",
                image.pix.is_null() as u8,
                image.w,
                image.h,
                unsafe { api.error_reason() }
            );
            unsafe { release_image(image) };
        }
        "malloc_failure" => {
            let png = make_png(10_000, 10_000, 8, 6, &[], &PngOptions::default());
            let limits = RLimit {
                current: 256 * 1024 * 1024,
                maximum: 256 * 1024 * 1024,
            };
            assert_eq!(unsafe { setrlimit(9, &limits) }, 0);
            let image = unsafe { api.load_png_raw(png.as_ptr(), png.len() as c_int) };
            println!(
                "CHILD_RESULT:{}:{}:{}:{}",
                image.pix.is_null() as u8,
                image.w,
                image.h,
                unsafe { api.error_reason() }.unwrap_or_default()
            );
            unsafe { release_image(image) };
        }
        "initial_error_reason" => {
            println!("CHILD_RESULT:{:?}", unsafe { api.error_reason() });
        }
        _ => panic!("unknown child case {case}"),
    }
}
