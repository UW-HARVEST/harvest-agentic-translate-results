use flate2::Compression;
use flate2::write::DeflateEncoder;
use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::slice;
use std::sync::{Mutex, MutexGuard};

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

type LoadPng = unsafe extern "C" fn(*const u8, c_int) -> Image;
type Inflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
}

static FFI_LOCK: Mutex<()> = Mutex::new(());

fn ffi_lock() -> MutexGuard<'static, ()> {
    FFI_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Api {
    _library: Library,
    load_png: LoadPng,
    inflate: Inflate,
    error_reason: *mut *const c_char,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let load_png = unsafe { *library.get::<LoadPng>(b"load_png_mem\0").unwrap() };
        let inflate = unsafe { *library.get::<Inflate>(b"cp_inflate\0").unwrap() };
        let error_reason = unsafe {
            *library
                .get::<*mut *const c_char>(b"cp_error_reason\0")
                .unwrap()
        };
        Self {
            _library: library,
            load_png,
            inflate,
            error_reason,
        }
    }

    unsafe fn reason(&self) -> Option<String> {
        let reason = unsafe { *self.error_reason };
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
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    root().join("c_src/build/libtranslated_rust.so")
}

fn rust_library() -> PathBuf {
    std::env::var_os("RUST_SO_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("target/release/libload_png_mem_lib.so"))
}

unsafe fn apis() -> (Api, Api) {
    (unsafe { Api::open(&c_library()) }, unsafe {
        Api::open(&rust_library())
    })
}

#[derive(Debug, PartialEq, Eq)]
struct PngResult {
    w: i32,
    h: i32,
    pixels: Option<Vec<u8>>,
    reason: Option<String>,
}

unsafe fn call_png(api: &Api, png: *const u8, len: i32) -> PngResult {
    let image = unsafe { (api.load_png)(png, len) };
    let pixels = if image.pix.is_null() {
        None
    } else {
        let byte_len = (image.w as usize)
            .saturating_mul(image.h as usize)
            .saturating_mul(size_of::<Pixel>());
        let bytes = unsafe { slice::from_raw_parts(image.pix.cast::<u8>(), byte_len) }.to_vec();
        unsafe { free(image.pix.cast()) };
        Some(bytes)
    };
    PngResult {
        w: image.w,
        h: image.h,
        pixels,
        reason: if image.pix.is_null() {
            unsafe { api.reason() }
        } else {
            None
        },
    }
}

fn compare_png(png: &[u8]) -> PngResult {
    unsafe {
        let (c, rust) = apis();
        let c_result = call_png(&c, png.as_ptr(), png.len() as i32);
        let rust_result = call_png(&rust, png.as_ptr(), png.len() as i32);
        assert_eq!(rust_result, c_result);
        c_result
    }
}

fn compare_inflate_at_alignment(input: &[u8], output_capacity: usize, alignment: usize) -> Vec<u8> {
    assert!(alignment < 4);
    let mut storage = vec![0u8; input.len() + 7];
    let base = storage.as_ptr() as usize;
    let offset = (alignment + 4 - (base & 3)) & 3;
    storage[offset..offset + input.len()].copy_from_slice(input);
    let input_ptr = unsafe { storage.as_mut_ptr().add(offset) };
    assert_eq!((input_ptr as usize) & 3, alignment);

    unsafe {
        let (c, rust) = apis();
        let mut c_out = vec![0xa5; output_capacity];
        let mut rust_out = vec![0xa5; output_capacity];
        let c_return = (c.inflate)(
            input_ptr.cast(),
            input.len() as i32,
            c_out.as_mut_ptr().cast(),
            output_capacity as i32,
        );
        let c_reason = if c_return == 0 { c.reason() } else { None };
        let rust_return = (rust.inflate)(
            input_ptr.cast(),
            input.len() as i32,
            rust_out.as_mut_ptr().cast(),
            output_capacity as i32,
        );
        let rust_reason = if rust_return == 0 {
            rust.reason()
        } else {
            None
        };
        assert_eq!(rust_return, c_return);
        assert_eq!(rust_reason, c_reason);
        assert_eq!(rust_out, c_out);
        c_out
    }
}

fn compare_inflate(input: &[u8], output_capacity: usize) -> Vec<u8> {
    compare_inflate_at_alignment(input, output_capacity, 0)
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
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

    fn usize(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u32() as u8).collect()
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    pending: u8,
    bit_count: u8,
}

impl BitWriter {
    fn bits(&mut self, value: u32, count: u8) {
        for bit in 0..count {
            self.pending |= (((value >> bit) & 1) as u8) << self.bit_count;
            self.bit_count += 1;
            if self.bit_count == 8 {
                self.bytes.push(self.pending);
                self.pending = 0;
                self.bit_count = 0;
            }
        }
    }

    fn align(&mut self) {
        if self.bit_count != 0 {
            self.bytes.push(self.pending);
            self.pending = 0;
            self.bit_count = 0;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

fn reverse_bits(mut value: u32, count: u8) -> u32 {
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
        144..=255 => (0x190 + (symbol - 144) as u32, 9),
        256..=279 => ((symbol - 256) as u32, 7),
        280..=287 => (0xc0 + (symbol - 280) as u32, 8),
        _ => panic!("invalid fixed symbol {symbol}"),
    };
    writer.bits(reverse_bits(code, bits), bits);
}

fn fixed_block(writer: &mut BitWriter, final_block: bool, data: &[u8]) {
    writer.bits(final_block as u32, 1);
    writer.bits(1, 2);
    for &byte in data {
        fixed_symbol(writer, byte as u16);
    }
    fixed_symbol(writer, 256);
}

fn fixed_deflate(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    fixed_block(&mut writer, true, data);
    writer.finish()
}

fn fixed_distance_one() -> (Vec<u8>, Vec<u8>) {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    fixed_symbol(&mut writer, b'A' as u16);
    fixed_symbol(&mut writer, 257);
    writer.bits(0, 5);
    fixed_symbol(&mut writer, 256);
    (writer.finish(), b"AAAA".to_vec())
}

fn fixed_distance_three() -> (Vec<u8>, Vec<u8>) {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for byte in b"ABC" {
        fixed_symbol(&mut writer, *byte as u16);
    }
    fixed_symbol(&mut writer, 257);
    writer.bits(reverse_bits(2, 5), 5);
    fixed_symbol(&mut writer, 256);
    (writer.finish(), b"ABCABC".to_vec())
}

fn fixed_bad_distance() -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    fixed_symbol(&mut writer, 257);
    writer.bits(0, 5);
    fixed_symbol(&mut writer, 256);
    writer.finish()
}

fn fixed_string_overflow() -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    fixed_symbol(&mut writer, b'Z' as u16);
    fixed_symbol(&mut writer, 257);
    writer.bits(0, 5);
    fixed_symbol(&mut writer, 256);
    writer.finish()
}

fn stored_deflate(data: &[u8]) -> Vec<u8> {
    assert!(data.len() <= u16::MAX as usize);
    let len = data.len() as u16;
    let mut output = vec![1, len as u8, (len >> 8) as u8];
    let nlen = !len;
    output.extend_from_slice(&[nlen as u8, (nlen >> 8) as u8]);
    output.extend_from_slice(data);
    output
}

fn dynamic_deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data).unwrap();
    let output = encoder.finish().unwrap();
    assert_eq!((output[0] >> 1) & 3, 2, "fixture was not dynamic");
    output
}

#[derive(Clone, Copy)]
enum DeflateKind {
    Stored,
    Fixed,
    Dynamic,
}

fn zlib_stream(data: &[u8], kind: DeflateKind) -> Vec<u8> {
    let raw = match kind {
        DeflateKind::Stored => stored_deflate(data),
        DeflateKind::Fixed => fixed_deflate(data),
        DeflateKind::Dynamic => dynamic_deflate(data),
    };
    let mut output = vec![0x78, 0x01];
    output.extend_from_slice(&raw);
    output.extend_from_slice(&[0; 4]);
    output
}

fn png_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + 12);
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(name);
    output.extend_from_slice(data);
    output.extend_from_slice(&[0; 4]);
    output
}

fn filtered_scanlines(
    samples: &[u8],
    width: usize,
    height: usize,
    bpp: usize,
    filters: &[u8],
) -> Vec<u8> {
    assert_eq!(samples.len(), width * height * bpp);
    assert_eq!(filters.len(), height);
    let stride = width * bpp;
    let mut output = Vec::with_capacity((stride + 1) * height);
    for y in 0..height {
        let filter = filters[y];
        output.push(filter);
        for x in 0..stride {
            let raw = samples[y * stride + x];
            let left = if x >= bpp {
                samples[y * stride + x - bpp]
            } else {
                0
            };
            let up = if y > 0 {
                samples[(y - 1) * stride + x]
            } else {
                0
            };
            let upper_left = if y > 0 && x >= bpp {
                samples[(y - 1) * stride + x - bpp]
            } else {
                0
            };
            let predictor = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => paeth(left, up, upper_left),
                _ => 0,
            };
            output.push(raw.wrapping_sub(predictor));
        }
    }
    output
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

struct PngOptions<'a> {
    width: u32,
    height: u32,
    color_type: u8,
    samples: &'a [u8],
    filters: &'a [u8],
    palette: Option<&'a [u8]>,
    transparency: Option<&'a [u8]>,
    deflate: DeflateKind,
    split_idat: bool,
    ancillary: bool,
}

fn make_png(options: PngOptions<'_>) -> Vec<u8> {
    let bpp = match options.color_type {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        value => panic!("unsupported test color type {value}"),
    };
    let raw = filtered_scanlines(
        options.samples,
        options.width as usize,
        options.height as usize,
        bpp,
        options.filters,
    );
    let zlib = zlib_stream(&raw, options.deflate);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&options.width.to_be_bytes());
    ihdr.extend_from_slice(&options.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, options.color_type, 0, 0, 0]);

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
    if let Some(palette) = options.palette {
        png.extend_from_slice(&png_chunk(b"PLTE", palette));
    }
    if let Some(transparency) = options.transparency {
        png.extend_from_slice(&png_chunk(b"tRNS", transparency));
    }
    if options.ancillary {
        png.extend_from_slice(&png_chunk(b"tEXt", b"k\0v"));
    }
    if options.split_idat {
        let split = zlib.len() / 2;
        png.extend_from_slice(&png_chunk(b"IDAT", &zlib[..split]));
        png.extend_from_slice(&png_chunk(b"IDAT", &zlib[split..]));
    } else {
        png.extend_from_slice(&png_chunk(b"IDAT", &zlib));
    }
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

fn basic_png(
    width: u32,
    height: u32,
    color_type: u8,
    samples: &[u8],
    filters: &[u8],
    deflate: DeflateKind,
) -> Vec<u8> {
    make_png(PngOptions {
        width,
        height,
        color_type,
        samples,
        filters,
        palette: None,
        transparency: None,
        deflate,
        split_idat: false,
        ancillary: false,
    })
}

unsafe fn data_symbol(api: &Api, name: &[u8], len: usize) -> Vec<u8> {
    let pointer = unsafe { *api._library.get::<*const u8>(name).unwrap() };
    unsafe { slice::from_raw_parts(pointer, len) }.to_vec()
}

#[test]
fn public_data_symbols_match() {
    let _guard = ffi_lock();
    unsafe {
        let (c, rust) = apis();
        for (name, len) in [
            (b"cp_fixed_table\0".as_slice(), 320),
            (b"cp_permutation_order\0".as_slice(), 19),
            (b"cp_len_extra_bits\0".as_slice(), 31),
            (b"cp_len_base\0".as_slice(), 31 * 4),
            (b"cp_dist_extra_bits\0".as_slice(), 32),
            (b"cp_dist_base\0".as_slice(), 32 * 4),
        ] {
            assert_eq!(
                data_symbol(&rust, name, len),
                data_symbol(&c, name, len),
                "{} differs",
                CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()
            );
        }
    }
}

#[test]
fn inflate_stored_alignment_and_capacity_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0xd01_d04_d11_d12);
    for alignment in 0..4 {
        for case in 0..32 {
            let len = 1 + rng.usize(192);
            let data = rng.bytes(len);
            let stream = stored_deflate(&data);
            let extra = if case & 1 == 0 { 0 } else { 19 };
            let output = compare_inflate_at_alignment(&stream, data.len() + extra, alignment);
            assert!(output[data.len()..].iter().all(|&byte| byte == 0xa5));
        }
    }
}

#[test]
fn inflate_fixed_literal_multiple_and_distance_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0xd05_d06_d07_d08);
    for _ in 0..32 {
        let len = 1 + rng.usize(256);
        let data = rng.bytes(len);
        let stream = fixed_deflate(&data);
        let output = compare_inflate(&stream, data.len());
        assert_eq!(output, data);

        let split = 1 + rng.usize(data.len());
        let mut writer = BitWriter::default();
        fixed_block(&mut writer, false, &data[..split]);
        fixed_block(&mut writer, true, &data[split..]);
        let output = compare_inflate(&writer.finish(), data.len() + 7);
        assert_eq!(&output[..data.len()], data);
    }

    let (stream, expected) = fixed_distance_one();
    assert_eq!(compare_inflate(&stream, expected.len()), expected);
    let (stream, expected) = fixed_distance_three();
    assert_eq!(compare_inflate(&stream, expected.len()), expected);
}

#[test]
fn inflate_dynamic_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0xd09_d10_2026);
    for case in 0..20 {
        let pattern_len = 17 + rng.usize(31);
        let pattern = rng.bytes(pattern_len);
        let len = 4096 + rng.usize(4096);
        let mut data = Vec::with_capacity(len);
        for i in 0..len {
            let noise = if i % 97 == 0 { rng.next_u32() as u8 } else { 0 };
            data.push(pattern[i % pattern.len()] ^ noise);
        }
        let stream = dynamic_deflate(&data);
        let capacity = data.len() + (case % 3) * 13;
        let output = compare_inflate(&stream, capacity);
        assert_eq!(&output[..data.len()], data);
        assert!(output[data.len()..].iter().all(|&byte| byte == 0xa5));
    }
}

fn randomized_png_case(
    rng: &mut Rng,
    color_type: u8,
    width: usize,
    height: usize,
    first_filter: u8,
    later_filter: u8,
) {
    let bpp = match color_type {
        0 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => unreachable!(),
    };
    let samples = rng.bytes(width * height * bpp);
    let mut filters = vec![later_filter; height];
    filters[0] = first_filter;
    let png = basic_png(
        width as u32,
        height as u32,
        color_type,
        &samples,
        &filters,
        DeflateKind::Fixed,
    );
    let result = compare_png(&png);
    assert_eq!((result.w, result.h), (width as i32, height as i32));
    assert!(result.pixels.is_some());
}

#[test]
fn png_color_and_dimension_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0xc010_2026);
    for &(color_type, bpp) in &[(0, 1), (2, 3), (4, 2), (6, 4)] {
        for _ in 0..20 {
            let width = 1 + rng.usize(17);
            let height = 1 + rng.usize(7);
            let samples = rng.bytes(width * height * bpp);
            let filters = vec![0; height];
            let png = basic_png(
                width as u32,
                height as u32,
                color_type,
                &samples,
                &filters,
                DeflateKind::Fixed,
            );
            let result = compare_png(&png);
            assert_eq!((result.w, result.h), (width as i32, height as i32));
            assert_eq!(result.pixels.as_ref().unwrap().len(), width * height * 4);
        }
    }

    let png = basic_png(0, 3, 0, &[], &[0, 0, 0], DeflateKind::Fixed);
    let result = compare_png(&png);
    assert_eq!((result.w, result.h), (0, 3));
    assert_eq!(result.pixels.unwrap(), Vec::<u8>::new());
}

#[test]
fn png_indexed_palette_and_transparency_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0x1de0_ed20_26);
    for case in 0..24 {
        let width = 3 + rng.usize(29);
        let height = 1 + rng.usize(5);
        let palette_entries = 4 + rng.usize(20);
        let palette = rng.bytes(palette_entries * 3);
        let indices: Vec<u8> = (0..width * height)
            .map(|_| rng.usize(palette_entries) as u8)
            .collect();
        let filters = vec![case as u8 % 5; height];
        let transparency_storage = if case % 3 == 0 {
            None
        } else if case % 3 == 1 {
            Some(rng.bytes(palette_entries / 2))
        } else {
            Some(rng.bytes(palette_entries))
        };
        let png = make_png(PngOptions {
            width: width as u32,
            height: height as u32,
            color_type: 3,
            samples: &indices,
            filters: &filters,
            palette: Some(&palette),
            transparency: transparency_storage.as_deref(),
            deflate: DeflateKind::Fixed,
            split_idat: false,
            ancillary: false,
        });
        let result = compare_png(&png);
        assert_eq!((result.w, result.h), (width as i32, height as i32));
        assert!(result.pixels.is_some());
    }
}

#[test]
fn png_filter_cross_product_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0xf01_f10_2026);
    for first_filter in 0..=4 {
        for &(color_type, bpp) in &[(0, 1), (4, 2), (2, 3), (6, 4)] {
            for case in 0..16 {
                let width = match case % 4 {
                    0 => 1,
                    1 => bpp,
                    2 => bpp + 1,
                    _ => 2 + rng.usize(23),
                };
                randomized_png_case(&mut rng, color_type, width, 1, first_filter, 0);
            }
        }
    }

    for later_filter in 0..=4 {
        for &(color_type, bpp) in &[(0, 1), (4, 2), (2, 3), (6, 4)] {
            for case in 0..16 {
                let width = match case % 3 {
                    0 => 1,
                    1 => bpp + 1,
                    _ => 2 + rng.usize(23),
                };
                let height = 2 + rng.usize(5);
                randomized_png_case(&mut rng, color_type, width, height, 0, later_filter);
            }
        }
    }
}

#[test]
fn png_chunk_and_deflate_matrix() {
    let _guard = ffi_lock();
    let mut rng = Rng::new(0x1da7_2026);
    for case in 0..20 {
        let kind = match case % 3 {
            0 => DeflateKind::Stored,
            1 => DeflateKind::Fixed,
            _ => DeflateKind::Dynamic,
        };
        let (width, height) = if matches!(kind, DeflateKind::Dynamic) {
            (192, 12)
        } else {
            (2 + rng.usize(31), 1 + rng.usize(8))
        };
        let mut samples = rng.bytes(width * height * 4);
        if matches!(kind, DeflateKind::Dynamic) {
            for (index, byte) in samples.iter_mut().enumerate() {
                *byte &= 0x0f;
                *byte ^= (index % 11) as u8;
            }
        }
        let filters = vec![case as u8 % 5; height];
        let png = make_png(PngOptions {
            width: width as u32,
            height: height as u32,
            color_type: 6,
            samples: &samples,
            filters: &filters,
            palette: None,
            transparency: None,
            deflate: kind,
            split_idat: case & 1 != 0,
            ancillary: case % 4 == 0,
        });
        let result = compare_png(&png);
        assert_eq!((result.w, result.h), (width as i32, height as i32));
        assert!(result.pixels.is_some());
    }
}

fn compare_inflate_error(input: &[u8], output_capacity: usize, expected: &str) {
    unsafe {
        let (c, rust) = apis();
        let mut c_out = vec![0xa5; output_capacity];
        let mut rust_out = vec![0xa5; output_capacity];
        let c_return = (c.inflate)(
            input.as_ptr().cast_mut().cast(),
            input.len() as i32,
            c_out.as_mut_ptr().cast(),
            output_capacity as i32,
        );
        let c_reason = c.reason();
        let rust_return = (rust.inflate)(
            input.as_ptr().cast_mut().cast(),
            input.len() as i32,
            rust_out.as_mut_ptr().cast(),
            output_capacity as i32,
        );
        let rust_reason = rust.reason();
        assert_eq!(c_return, 0);
        assert_eq!(rust_return, c_return);
        assert_eq!(c_reason.as_deref(), Some(expected));
        assert_eq!(rust_reason, c_reason);
        assert_eq!(rust_out, c_out);
    }
}

fn assert_png_error(png: &[u8], expected: &str, dimensions: (i32, i32)) {
    let result = compare_png(png);
    assert_eq!((result.w, result.h), dimensions);
    assert!(result.pixels.is_none());
    assert_eq!(result.reason.as_deref(), Some(expected));
}

fn compare_png_with_length(bytes: &[u8], length: i32) -> PngResult {
    unsafe {
        let (c, rust) = apis();
        let c_result = call_png(&c, bytes.as_ptr(), length);
        let rust_result = call_png(&rust, bytes.as_ptr(), length);
        assert_eq!(rust_result, c_result);
        c_result
    }
}

fn png_header_only(width: u32, height: u32, color_type: u8) -> Vec<u8> {
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

fn png_with_zlib(
    width: u32,
    height: u32,
    color_type: u8,
    zlib: &[u8],
    palette: Option<&[u8]>,
) -> Vec<u8> {
    let mut png = png_header_only(width, height, color_type);
    png.truncate(8 + 12 + 13);
    if let Some(palette) = palette {
        png.extend_from_slice(&png_chunk(b"PLTE", palette));
    }
    png.extend_from_slice(&png_chunk(b"IDAT", zlib));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

fn idat_data_range(png: &[u8]) -> std::ops::Range<usize> {
    let mut offset = 8;
    while offset + 12 <= png.len() {
        let len = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
        if &png[offset + 4..offset + 8] == b"IDAT" {
            return offset + 8..offset + 8 + len;
        }
        offset += len + 12;
    }
    panic!("IDAT not found");
}

#[test]
fn inflate_explicit_error_matrix() {
    let _guard = ffi_lock();
    compare_inflate_error(
        &[1, 1, 0, 0, 0, b'X'],
        1,
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
    );
    compare_inflate_error(
        &[1, 1, 0, 0xfe, 0xff, b'X', b'Y', b'Z'],
        1,
        "Stored block extends beyond end of input stream.",
    );
    compare_inflate_error(
        &fixed_deflate(b"Q"),
        0,
        "Attempted to overwrite out buffer while outputting a symbol.",
    );
    compare_inflate_error(
        &fixed_bad_distance(),
        3,
        "Attempted to write before out buffer (invalid backwards distance).",
    );
    compare_inflate_error(
        &fixed_string_overflow(),
        2,
        "Attempted to overwrite out buffer while outputting a string.",
    );
    compare_inflate_error(
        &[0x07],
        0,
        "Detected unknown block type within input stream.",
    );
}

#[test]
fn png_signature_and_ihdr_error_matrix() {
    let _guard = ffi_lock();
    let valid = basic_png(1, 1, 0, &[17], &[0], DeflateKind::Fixed);

    let mut bad = valid.clone();
    bad[0] ^= 0xff;
    assert_png_error(
        &bad,
        "incorrect file signature (is this a png file?)",
        (0, 0),
    );

    bad = valid.clone();
    bad[12..16].copy_from_slice(b"NOPE");
    assert_png_error(&bad, "unable to find IHDR chunk", (0, 0));
    bad = valid.clone();
    bad[8..12].copy_from_slice(&12u32.to_be_bytes());
    assert_png_error(&bad, "unable to find IHDR chunk", (0, 0));
    assert_png_error(&valid[..20], "unable to find IHDR chunk", (0, 0));

    bad = valid.clone();
    bad[24] = 16;
    assert_png_error(&bad, "only bit-depth of 8 is supported", (0, 0));
    for color_type in [1, 5, 7, 255] {
        bad = valid.clone();
        bad[25] = color_type;
        assert_png_error(&bad, "unknown color type", (0, 0));
    }

    for width in [0xffff_ffffu32, 0x7fff_ffff] {
        bad = valid.clone();
        bad[16..20].copy_from_slice(&width.to_be_bytes());
        assert_png_error(
            &bad,
            "invalid IHDR chunk found, image width was less than 1",
            (0, 0),
        );
    }
    for height in [0u32, 0x8000_0000, 0xffff_ffff] {
        bad = valid.clone();
        bad[20..24].copy_from_slice(&height.to_be_bytes());
        assert_png_error(
            &bad,
            "invalid IHDR chunk found, image height was less than 1",
            (0, 0),
        );
    }

    bad = valid.clone();
    bad[16..20].copy_from_slice(&30_000u32.to_be_bytes());
    bad[20..24].copy_from_slice(&30_000u32.to_be_bytes());
    assert_png_error(&bad, "image too large", (0, 0));
}

#[test]
fn png_methods_and_zlib_header_error_matrix() {
    let _guard = ffi_lock();
    let valid = basic_png(1, 1, 0, &[23], &[0], DeflateKind::Fixed);
    for (offset, value, expected) in [
        (26, 1, "only standard compression DEFLATE is supported"),
        (27, 1, "only standard adaptive filtering is supported"),
        (28, 1, "interlacing is not supported"),
    ] {
        let mut bad = valid.clone();
        bad[offset] = value;
        assert_png_error(&bad, expected, (1, 1));
    }

    assert_png_error(
        &png_header_only(1, 1, 0),
        "corrupt zlib structure in DEFLATE stream",
        (1, 1),
    );
    assert_png_error(
        &png_with_zlib(1, 1, 0, &[0; 5], None),
        "corrupt zlib structure in DEFLATE stream",
        (1, 1),
    );

    let range = idat_data_range(&valid);
    let mut bad = valid.clone();
    bad[range.start] = 0x77;
    assert_png_error(
        &bad,
        "only zlib compression method (RFC 1950) is supported",
        (1, 1),
    );
    bad = valid.clone();
    bad[range.start] = 0x88;
    assert_png_error(&bad, "innapropriate window size detected", (1, 1));
    bad = valid.clone();
    bad[range.start + 1] |= 0x20;
    assert_png_error(
        &bad,
        "preset dictionary is present and not supported",
        (1, 1),
    );
}

#[test]
fn png_decode_filter_and_palette_error_matrix() {
    let _guard = ffi_lock();
    let invalid_first = basic_png(1, 1, 0, &[42], &[5], DeflateKind::Fixed);
    assert_png_error(&invalid_first, "invalid filter byte found", (1, 1));

    let invalid_later = basic_png(2, 2, 0, &[1, 2, 3, 4], &[0, 255], DeflateKind::Fixed);
    assert_png_error(&invalid_later, "invalid filter byte found", (2, 2));

    let unknown_block_zlib = [0x78, 0x01, 0x07, 0, 0, 0, 0];
    let bad_deflate = png_with_zlib(1, 1, 0, &unknown_block_zlib, None);
    assert_png_error(&bad_deflate, "DEFLATE algorithm failed", (1, 1));

    let indexed_without_palette = basic_png(2, 1, 3, &[0, 0], &[0], DeflateKind::Fixed);
    assert_png_error(
        &indexed_without_palette,
        "color type of indexed requires a PLTE chunk",
        (2, 1),
    );
}

#[test]
fn generic_nonfaulting_length_boundaries_match() {
    let _guard = ffi_lock();
    let bytes = *b"not png!";
    for length in [0, -1, i32::MAX] {
        let result = compare_png_with_length(&bytes, length);
        assert_eq!((result.w, result.h), (0, 0));
        assert!(result.pixels.is_none());
        assert_eq!(
            result.reason.as_deref(),
            Some("incorrect file signature (is this a png file?)")
        );
    }
}

#[test]
fn unreachable_rejections_and_internal_invariants_are_mapped() {
    let _guard = ffi_lock();
    for bpp in 1i64..=4 {
        for &(w, h) in &[(1i64, 1i64), (2, 17), (1024, 1024), (536_870_910, 1)] {
            if w * h * 4 < i32::MAX as i64 {
                assert!(w * h * 4 >= 1);
                assert!(w * h * bpp >= 1);
            }
        }
    }

    let c_source = include_str!("../c_src/src/lib.c");
    let rust_source = include_str!("../src/lib.rs");
    assert_eq!(c_source.matches("assert(").count(), 10);
    assert_eq!(rust_source.matches("assert!(").count(), 10);
    for rust_guard in [
        "bits_left & 7 == 0",
        "word_index <= (*s).word_count",
        "count >= num_bits_to_read",
        "num_bits_to_read <= 32",
        "num_bits_to_read >= 0",
        "bits_left > 0",
        "count <= 64",
        "!would_overflow(s, num_bits_to_read)",
        "len < 16",
        "(search >> len) == (key >> len)",
    ] {
        assert!(rust_source.contains(rust_guard), "missing {rust_guard}");
    }
}

#[cfg(unix)]
fn subprocess_case(library: &Path, case: &str) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("ffi_subprocess_case")
        .arg("--nocapture")
        .env("FFI_CHILD_LIBRARY", library)
        .env("FFI_CHILD_CASE", case)
        .status()
        .unwrap()
}

#[cfg(unix)]
#[test]
fn ffi_subprocess_case() {
    let _guard = ffi_lock();
    let Some(case) = std::env::var_os("FFI_CHILD_CASE") else {
        return;
    };
    let library = PathBuf::from(std::env::var_os("FFI_CHILD_LIBRARY").unwrap());
    let case = case.to_string_lossy();
    unsafe {
        let api = Api::open(&library);
        match case.as_ref() {
            "null_png" => {
                (api.load_png)(std::ptr::null(), 8);
            }
            "null_inflate" => {
                let mut output = [0u8; 8];
                (api.inflate)(std::ptr::null_mut(), 8, output.as_mut_ptr().cast(), 8);
            }
            "zero_inflate" => {
                let mut input = [0u8; 8];
                let mut output = [0u8; 8];
                (api.inflate)(input.as_mut_ptr().cast(), 0, output.as_mut_ptr().cast(), 8);
            }
            "huge_inflate" => {
                let mut input = [0u8; 8];
                let mut output = [0u8; 8];
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    i32::MAX,
                    output.as_mut_ptr().cast(),
                    8,
                );
            }
            "truncated_fixed" => {
                let mut input = [0x03u8];
                let mut output = [0u8; 8];
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as i32,
                    output.as_mut_ptr().cast(),
                    output.len() as i32,
                );
            }
            "allocation_failure" => {
                #[repr(C)]
                struct RLimit {
                    current: u64,
                    maximum: u64,
                }
                unsafe extern "C" {
                    fn setrlimit(resource: c_int, limit: *const RLimit) -> c_int;
                }
                const RLIMIT_AS: c_int = 9;
                let limit = RLimit {
                    current: 128 * 1024 * 1024,
                    maximum: 128 * 1024 * 1024,
                };
                assert_eq!(setrlimit(RLIMIT_AS, &limit), 0);
                let png = png_header_only(10_000, 10_000, 0);
                let image = (api.load_png)(png.as_ptr(), png.len() as i32);
                assert!(image.pix.is_null());
                assert_eq!(
                    CStr::from_ptr(*api.error_reason).to_bytes(),
                    b"unable to allocate raw image space"
                );
            }
            value => panic!("unknown child case {value}"),
        }
    }
}

#[cfg(unix)]
#[test]
fn fault_assertion_and_allocation_boundaries_match() {
    let _guard = ffi_lock();
    use std::os::unix::process::ExitStatusExt;

    for case in [
        "null_png",
        "null_inflate",
        "zero_inflate",
        "huge_inflate",
        "truncated_fixed",
    ] {
        let c_status = subprocess_case(&c_library(), case);
        let rust_status = subprocess_case(&rust_library(), case);
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "signal differs for {case}"
        );
        assert!(c_status.signal().is_some(), "{case} did not fault/abort");
    }

    let c_status = subprocess_case(&c_library(), "allocation_failure");
    let rust_status = subprocess_case(&rust_library(), "allocation_failure");
    assert!(c_status.success(), "C allocation-failure child failed");
    assert!(
        rust_status.success(),
        "Rust allocation-failure child failed"
    );
}
