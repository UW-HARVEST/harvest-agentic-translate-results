use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{CStr, c_char, c_int, c_uint, c_ulong, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

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

type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type LoadPngFn = unsafe extern "C" fn(*const u8, c_int) -> Image;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn setrlimit(resource: c_int, limits: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    current: u64,
    maximum: u64,
}

struct Api {
    _library: Library,
    inflate: InflateFn,
    load_png: LoadPngFn,
    error_reason: *mut *const c_char,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        unsafe {
            let os_library = libloading::os::unix::Library::open(
                Some(path),
                libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_LOCAL | 0x8,
            )
            .unwrap_or_else(|error| {
                panic!("failed to load {}: {error}", path.display());
            });
            let library = Library::from(os_library);
            let inflate = *library.get::<InflateFn>(b"cp_inflate").expect("cp_inflate");
            let load_png = *library
                .get::<LoadPngFn>(b"load_png_mem")
                .expect("load_png_mem");
            let error_reason = *library
                .get::<*mut *const c_char>(b"cp_error_reason")
                .expect("cp_error_reason");
            Self {
                _library: library,
                inflate,
                load_png,
                error_reason,
            }
        }
    }

    unsafe fn reason(&self) -> Option<String> {
        unsafe {
            let reason = *self.error_reason;
            (!reason.is_null()).then(|| CStr::from_ptr(reason).to_string_lossy().into_owned())
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libharvest-work-rhgXPt.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libload_png_mem_lib.so")
}

fn apis() -> (Api, Api) {
    assert!(
        c_library_path().is_file(),
        "build the C shared library before running tests"
    );
    assert!(
        rust_library_path().is_file(),
        "run cargo build --release before running tests"
    );
    unsafe {
        (
            Api::open(&c_library_path()),
            Api::open(&rust_library_path()),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InflateResult {
    code: c_int,
    output: Vec<u8>,
    reason: Option<String>,
}

unsafe fn call_inflate(
    api: &Api,
    encoded: &[u8],
    alignment: usize,
    output_len: usize,
    declared_output_len: c_int,
) -> InflateResult {
    let mut input = vec![0xa5; encoded.len() + alignment + 8];
    input[alignment..alignment + encoded.len()].copy_from_slice(encoded);
    let mut output = vec![0xcd; output_len.max(1)];
    unsafe {
        *api.error_reason = ptr::null();
        let code = (api.inflate)(
            input.as_mut_ptr().add(alignment).cast(),
            encoded.len() as c_int,
            output.as_mut_ptr().cast(),
            declared_output_len,
        );
        InflateResult {
            code,
            output,
            reason: api.reason(),
        }
    }
}

fn compare_inflate(
    c: &Api,
    rust: &Api,
    encoded: &[u8],
    alignment: usize,
    output_len: usize,
    declared_output_len: c_int,
) -> InflateResult {
    unsafe {
        let expected = call_inflate(c, encoded, alignment, output_len, declared_output_len);
        let actual = call_inflate(rust, encoded, alignment, output_len, declared_output_len);
        assert_eq!(
            actual, expected,
            "inflate mismatch: alignment={alignment}, input={encoded:02x?}"
        );
        actual
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ImageResult {
    w: c_int,
    h: c_int,
    pixels: Option<Vec<u8>>,
    reason: Option<String>,
}

unsafe fn call_load_png(api: &Api, png: &[u8]) -> ImageResult {
    unsafe {
        *api.error_reason = ptr::null();
        let image = (api.load_png)(png.as_ptr(), png.len() as c_int);
        let pixels = if image.pix.is_null() {
            None
        } else {
            let byte_len = (image.w as usize)
                .checked_mul(image.h as usize)
                .and_then(|count| count.checked_mul(size_of::<Pixel>()))
                .expect("valid image size");
            let bytes = std::slice::from_raw_parts(image.pix.cast::<u8>(), byte_len).to_vec();
            free(image.pix.cast());
            Some(bytes)
        };
        ImageResult {
            w: image.w,
            h: image.h,
            pixels,
            reason: api.reason(),
        }
    }
}

fn compare_png(c: &Api, rust: &Api, png: &[u8]) -> ImageResult {
    unsafe {
        let expected = call_load_png(c, png);
        let actual = call_load_png(rust, png);
        assert_eq!(actual, expected, "PNG mismatch for {} bytes", png.len());
        actual
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        start + self.next_u32() as usize % (end - start)
    }
}

#[repr(C)]
struct ZStream {
    next_in: *mut u8,
    avail_in: c_uint,
    total_in: c_ulong,
    next_out: *mut u8,
    avail_out: c_uint,
    total_out: c_ulong,
    msg: *mut c_char,
    state: *mut c_void,
    zalloc: Option<unsafe extern "C" fn(*mut c_void, c_uint, c_uint) -> *mut c_void>,
    zfree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
    data_type: c_int,
    adler: c_ulong,
    reserved: c_ulong,
}

impl Default for ZStream {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

fn zlib_compress(input: &[u8], raw: bool, level: c_int, strategy: c_int) -> Vec<u8> {
    type VersionFn = unsafe extern "C" fn() -> *const c_char;
    type InitFn = unsafe extern "C" fn(
        *mut ZStream,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        *const c_char,
        c_int,
    ) -> c_int;
    type DeflateFn = unsafe extern "C" fn(*mut ZStream, c_int) -> c_int;
    type EndFn = unsafe extern "C" fn(*mut ZStream) -> c_int;

    unsafe {
        let zlib = Library::new("libz.so.1").expect("libz.so.1");
        let version = *zlib.get::<VersionFn>(b"zlibVersion").unwrap();
        let init = *zlib.get::<InitFn>(b"deflateInit2_").unwrap();
        let deflate = *zlib.get::<DeflateFn>(b"deflate").unwrap();
        let end = *zlib.get::<EndFn>(b"deflateEnd").unwrap();

        let mut output = vec![0u8; input.len().saturating_mul(2).saturating_add(1024)];
        let mut stream = ZStream {
            next_in: input.as_ptr() as *mut u8,
            avail_in: input.len() as c_uint,
            next_out: output.as_mut_ptr(),
            avail_out: output.len() as c_uint,
            ..Default::default()
        };
        let window_bits = if raw { -15 } else { 15 };
        assert_eq!(
            init(
                &mut stream,
                level,
                8,
                window_bits,
                8,
                strategy,
                version(),
                size_of::<ZStream>() as c_int,
            ),
            0
        );
        assert_eq!(deflate(&mut stream, 4), 1);
        assert_eq!(end(&mut stream), 0);
        output.truncate(stream.total_out as usize);
        output
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_offset: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_offset: 0,
        }
    }

    fn bits(&mut self, count: u8) -> u32 {
        let mut result = 0;
        for index in 0..count {
            let byte = self.bytes[self.bit_offset / 8];
            result |= (((byte >> (self.bit_offset % 8)) & 1) as u32) << index;
            self.bit_offset += 1;
        }
        result
    }
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, u8, u8)> {
    let mut counts = [0u32; 16];
    for &length in lengths {
        counts[length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0;
    for bits in 1..=15 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    lengths
        .iter()
        .enumerate()
        .filter_map(|(symbol, &length)| {
            if length == 0 {
                None
            } else {
                let code = next[length as usize];
                next[length as usize] += 1;
                Some((reverse_bits(code, length), length, symbol as u8))
            }
        })
        .collect()
}

fn decode_symbol(reader: &mut BitReader<'_>, codes: &[(u32, u8, u8)]) -> u8 {
    let mut code = 0;
    for length in 1..=15 {
        code |= reader.bits(1) << (length - 1);
        if let Some((_, _, symbol)) = codes
            .iter()
            .find(|&&(candidate, bits, _)| bits == length && candidate == code)
        {
            return *symbol;
        }
    }
    panic!("invalid Huffman code in generated zlib stream");
}

fn dynamic_repeat_symbols(encoded: &[u8]) -> BTreeSet<u8> {
    let mut reader = BitReader::new(encoded);
    reader.bits(1);
    assert_eq!(reader.bits(2), 2);
    let literal_count = 257 + reader.bits(5) as usize;
    let distance_count = 1 + reader.bits(5) as usize;
    let code_count = 4 + reader.bits(4) as usize;
    let order = [
        16usize, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut lengths = [0u8; 19];
    for &symbol in &order[..code_count] {
        lengths[symbol] = reader.bits(3) as u8;
    }
    let codes = canonical_codes(&lengths);
    let mut repeats = BTreeSet::new();
    let mut decoded = 0;
    while decoded < literal_count + distance_count {
        let symbol = decode_symbol(&mut reader, &codes);
        match symbol {
            0..=15 => decoded += 1,
            16 => {
                repeats.insert(16);
                decoded += 3 + reader.bits(2) as usize;
            }
            17 => {
                repeats.insert(17);
                decoded += 3 + reader.bits(3) as usize;
            }
            18 => {
                repeats.insert(18);
                decoded += 11 + reader.bits(7) as usize;
            }
            _ => unreachable!(),
        }
    }
    repeats
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: vec![0],
            bit: 0,
        }
    }

    fn bits(&mut self, value: u32, count: u8) {
        for index in 0..count {
            if value & (1 << index) != 0 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 1 << self.bit;
            }
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.bytes.push(0);
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bit == 0 {
            self.bytes.pop();
        }
        self.bytes
    }
}

fn reverse_bits(value: u32, count: u8) -> u32 {
    value.reverse_bits() >> (32 - count)
}

fn fixed_symbol(writer: &mut BitWriter, symbol: u16) {
    let (code, count) = match symbol {
        0..=143 => (0x30 + symbol as u32, 8),
        144..=255 => (0x190 + (symbol - 144) as u32, 9),
        256..=279 => ((symbol - 256) as u32, 7),
        280..=287 => (0xc0 + (symbol - 280) as u32, 8),
        _ => panic!("invalid fixed symbol"),
    };
    writer.bits(reverse_bits(code, count), count);
}

fn fixed_match_stream(prefix: &[u8], distance_symbol: u8) -> Vec<u8> {
    let mut writer = BitWriter::new();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for &byte in prefix {
        fixed_symbol(&mut writer, byte as u16);
    }
    fixed_symbol(&mut writer, 257);
    writer.bits(reverse_bits(distance_symbol as u32, 5), 5);
    fixed_symbol(&mut writer, 256);
    writer.finish()
}

fn fixed_blocks(blocks: &[&[u8]]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    for (index, block) in blocks.iter().enumerate() {
        writer.bits((index + 1 == blocks.len()) as u32, 1);
        writer.bits(1, 2);
        for &byte in *block {
            fixed_symbol(&mut writer, byte as u16);
        }
        fixed_symbol(&mut writer, 256);
    }
    writer.finish()
}

fn stored_stream(blocks: &[&[u8]]) -> Vec<u8> {
    let mut result = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        assert!(block.len() <= u16::MAX as usize);
        result.push(if index + 1 == blocks.len() { 1 } else { 0 });
        let len = block.len() as u16;
        result.extend_from_slice(&len.to_le_bytes());
        result.extend_from_slice(&(!len).to_le_bytes());
        result.extend_from_slice(block);
    }
    result
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

fn filtered_rows(
    pixels: &[u8],
    width: usize,
    height: usize,
    bpp: usize,
    first_filter: u8,
    later_filter: u8,
) -> Vec<u8> {
    let row_len = width * bpp;
    assert_eq!(pixels.len(), row_len * height);
    let mut result = Vec::with_capacity((row_len + 1) * height);
    for y in 0..height {
        let filter = if y == 0 { first_filter } else { later_filter };
        result.push(filter);
        for x in 0..row_len {
            let raw = pixels[y * row_len + x];
            let left = if x >= bpp {
                pixels[y * row_len + x - bpp]
            } else {
                0
            };
            let up = if y > 0 {
                pixels[(y - 1) * row_len + x]
            } else {
                0
            };
            let upper_left = if y > 0 && x >= bpp {
                pixels[(y - 1) * row_len + x - bpp]
            } else {
                0
            };
            let prediction = match filter {
                0 => 0,
                1 => left,
                2 => up,
                3 => ((left as u16 + up as u16) / 2) as u8,
                4 => paeth(left, up, upper_left),
                _ => 0,
            };
            result.push(raw.wrapping_sub(prediction));
        }
    }
    result
}

fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() + 12);
    result.extend_from_slice(&(data.len() as u32).to_be_bytes());
    result.extend_from_slice(name);
    result.extend_from_slice(data);
    result.extend_from_slice(&[0; 4]);
    result
}

struct PngOptions<'a> {
    width: u32,
    height: u32,
    color_type: u8,
    scanlines: &'a [u8],
    palette: Option<&'a [u8]>,
    transparency: Option<&'a [u8]>,
    idat_splits: &'a [usize],
    ancillary: bool,
}

fn make_png(options: PngOptions<'_>) -> Vec<u8> {
    let compressed = zlib_compress(options.scanlines, false, 6, 0);
    let mut result = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&options.width.to_be_bytes());
    ihdr.extend_from_slice(&options.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, options.color_type, 0, 0, 0]);
    result.extend_from_slice(&chunk(b"IHDR", &ihdr));
    if options.ancillary {
        result.extend_from_slice(&chunk(b"ruSt", b"ignored"));
    }
    if let Some(palette) = options.palette {
        result.extend_from_slice(&chunk(b"PLTE", palette));
    }
    if options.ancillary {
        result.extend_from_slice(&chunk(b"ruS2", &[]));
    }
    if let Some(transparency) = options.transparency {
        result.extend_from_slice(&chunk(b"tRNS", transparency));
    }
    let mut offset = 0;
    for &split in options.idat_splits {
        let end = (offset + split).min(compressed.len());
        result.extend_from_slice(&chunk(b"IDAT", &compressed[offset..end]));
        offset = end;
    }
    if offset < compressed.len() || options.idat_splits.is_empty() {
        result.extend_from_slice(&chunk(b"IDAT", &compressed[offset..]));
    }
    result.extend_from_slice(&chunk(b"IEND", &[]));
    result
}

fn raw_png(ihdr: [u8; 13], chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut result = b"\x89PNG\r\n\x1a\n".to_vec();
    result.extend_from_slice(&chunk(b"IHDR", &ihdr));
    for &(name, data) in chunks {
        result.extend_from_slice(&chunk(name, data));
    }
    result.extend_from_slice(&chunk(b"IEND", &[]));
    result
}

fn ihdr(
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
) -> [u8; 13] {
    let mut result = [0u8; 13];
    result[..4].copy_from_slice(&width.to_be_bytes());
    result[4..8].copy_from_slice(&height.to_be_bytes());
    result[8..].copy_from_slice(&[depth, color, compression, filter, interlace]);
    result
}

fn assert_png_error(result: ImageResult, dimensions: (c_int, c_int), reason: &str) {
    assert_eq!((result.w, result.h), dimensions);
    assert_eq!(result.pixels, None);
    assert_eq!(result.reason.as_deref(), Some(reason));
}

#[test]
fn valid_inflate_stored_randomized() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let empty = stored_stream(&[&[]]);
    let result = compare_inflate(&c, &rust, &empty, 0, 1, 0);
    assert_eq!(result.code, 1);
    let boundary_values = [0, 1, 127, 128, 254, 255];
    let encoded = stored_stream(&[&boundary_values]);
    let result = compare_inflate(&c, &rust, &encoded, 0, boundary_values.len(), 6);
    assert_eq!(result.output, boundary_values);

    let mut rng = Rng::new(0x635f_7374_6f72_6564);
    for case in 0..96 {
        let len = rng.range(1, 384);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let stream = stored_stream(&[&payload]);
        for alignment in 0..4 {
            let extra = case % 9;
            let result = compare_inflate(
                &c,
                &rust,
                &stream,
                alignment,
                len + extra,
                (len + extra) as c_int,
            );
            assert_eq!(result.code, 1);
            assert_eq!(&result.output[..len], payload);
            assert!(result.output[len..].iter().all(|&byte| byte == 0xcd));
        }
    }
}

#[test]
fn valid_inflate_fixed_matches_and_mixed_blocks() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6669_7865_645f_6466);

    for case in 0..96 {
        let len = rng.range(1, 96);
        let payload: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let encoded = fixed_blocks(&[&payload]);
        assert_eq!((encoded[0] >> 1) & 3, 1);
        let result = compare_inflate(&c, &rust, &encoded, case % 4, len + 7, (len + 7) as c_int);
        assert_eq!(result.code, 1);
        assert_eq!(&result.output[..len], payload);
        assert!(result.output[len..].iter().all(|&byte| byte == 0xcd));
    }

    let distance_one = fixed_match_stream(b"A", 0);
    let result = compare_inflate(&c, &rust, &distance_one, 0, 4, 4);
    assert_eq!(result.code, 1);
    assert_eq!(result.output, b"AAAA");

    let distance_two = fixed_match_stream(b"AB", 1);
    let result = compare_inflate(&c, &rust, &distance_two, 1, 5, 5);
    assert_eq!(result.code, 1);
    assert_eq!(result.output, b"ABABA");

    for case in 0..64 {
        let prefix_len = rng.range(0, 80);
        let suffix_len = rng.range(0, 80);
        let prefix: Vec<u8> = (0..prefix_len).map(|_| rng.byte()).collect();
        let suffix: Vec<u8> = (0..suffix_len).map(|_| rng.byte()).collect();
        let encoded = fixed_blocks(&[&prefix, &suffix]);
        let mut expected = prefix;
        expected.extend_from_slice(&suffix);
        let result = compare_inflate(
            &c,
            &rust,
            &encoded,
            case % 4,
            expected.len(),
            expected.len() as c_int,
        );
        assert_eq!(result.code, 1);
        assert_eq!(result.output, expected);
    }
}

#[test]
fn valid_inflate_dynamic_randomized() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6479_6e61_6d69_635f);
    let alphabet = b"AAAAABBBBCCCDDE0123456789";
    let mut repeat_symbols = BTreeSet::new();

    for case in 0..96 {
        let len = rng.range(1024, 8192);
        let mut payload = Vec::with_capacity(len);
        for index in 0..len {
            let byte = if index % 97 < 72 {
                alphabet[rng.range(0, alphabet.len())]
            } else {
                rng.byte()
            };
            payload.push(byte);
        }
        let encoded = zlib_compress(&payload, true, 6, 0);
        assert_eq!(
            (encoded[0] >> 1) & 3,
            2,
            "default zlib stream did not exercise dynamic Huffman"
        );
        repeat_symbols.extend(dynamic_repeat_symbols(&encoded));
        let extra = case % 17;
        let result = compare_inflate(
            &c,
            &rust,
            &encoded,
            case % 4,
            len + extra,
            (len + extra) as c_int,
        );
        assert_eq!(result.code, 1);
        assert_eq!(&result.output[..len], payload);
        assert!(result.output[len..].iter().all(|&byte| byte == 0xcd));
    }

    for payload in [
        vec![b'A'; 16_384],
        (0..16_384)
            .map(|index| if index & 1 == 0 { b'A' } else { b'B' })
            .collect(),
    ] {
        let encoded = zlib_compress(&payload, true, 6, 0);
        assert_eq!((encoded[0] >> 1) & 3, 2);
        repeat_symbols.extend(dynamic_repeat_symbols(&encoded));
        let result = compare_inflate(
            &c,
            &rust,
            &encoded,
            0,
            payload.len(),
            payload.len() as c_int,
        );
        assert_eq!(result.output, payload);
    }

    let empty = zlib_compress(&[], true, 6, 0);
    let result = compare_inflate(&c, &rust, &empty, 3, 1, 0);
    assert_eq!(result.code, 1);
    assert_eq!(repeat_symbols, BTreeSet::from([16, 17, 18]));
}

fn randomized_png_case(
    c: &Api,
    rust: &Api,
    rng: &mut Rng,
    color_type: u8,
    bpp: usize,
    first_filter: u8,
    later_filter: u8,
    height: usize,
    transparency: Option<&[u8]>,
    idat_splits: &[usize],
    ancillary: bool,
) {
    let width = rng.range(1, 19);
    let mut pixels: Vec<u8> = (0..width * height * bpp).map(|_| rng.byte()).collect();
    let mut palette = vec![0u8; 256 * 3];
    if color_type == 3 {
        for entry in &mut palette {
            *entry = rng.byte();
        }
        for index in &mut pixels {
            *index = rng.byte();
        }
    }
    let scanlines = filtered_rows(&pixels, width, height, bpp, first_filter, later_filter);
    let png = make_png(PngOptions {
        width: width as u32,
        height: height as u32,
        color_type,
        scanlines: &scanlines,
        palette: (color_type == 3).then_some(palette.as_slice()),
        transparency,
        idat_splits,
        ancillary,
    });
    let result = compare_png(c, rust, &png);
    assert_eq!((result.w, result.h), (width as c_int, height as c_int));
    assert_eq!(
        result.pixels.as_ref().map(Vec::len),
        Some(width * height * 4)
    );
    assert_eq!(result.reason, None);
}

#[test]
fn valid_png_color_filter_cross_product_randomized() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x504e_475f_4d41_5452);
    let color_types = [(0, 1), (2, 3), (3, 1), (4, 2), (6, 4)];

    for &(color_type, bpp) in &color_types {
        for filter in 0..=4 {
            for _ in 0..12 {
                randomized_png_case(
                    &c,
                    &rust,
                    &mut rng,
                    color_type,
                    bpp,
                    filter,
                    0,
                    1,
                    None,
                    &[],
                    false,
                );
                let height = rng.range(2, 9);
                randomized_png_case(
                    &c,
                    &rust,
                    &mut rng,
                    color_type,
                    bpp,
                    0,
                    filter,
                    height,
                    None,
                    &[],
                    false,
                );
            }
        }
    }
}

#[test]
fn valid_png_palette_chunks_and_dimensions_randomized() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x504c_5445_7452_4e53);
    let full_transparency: Vec<u8> = (0..=255).map(|value| value as u8).collect();
    let partial_transparency = [0, 1, 17, 127, 254];

    for case in 0..96 {
        let transparency = match case % 3 {
            0 => None,
            1 => Some(partial_transparency.as_slice()),
            _ => Some(full_transparency.as_slice()),
        };
        let (height, splits, ancillary) = match case % 4 {
            0 => (1, Vec::new(), false),
            1 => (rng.range(2, 7), vec![0, 1, 3], false),
            2 => (rng.range(2, 7), vec![2, 0, 5], true),
            _ => (rng.range(2, 7), vec![1], true),
        };
        randomized_png_case(
            &c,
            &rust,
            &mut rng,
            3,
            1,
            (case % 5) as u8,
            ((case / 5) % 5) as u8,
            height,
            transparency,
            &splits,
            ancillary,
        );
    }

    for &(width, height) in &[(1, 1), (1, 13), (17, 1), (17, 9)] {
        let pixels: Vec<u8> = (0..width * height * 4).map(|_| rng.byte()).collect();
        let scanlines = filtered_rows(&pixels, width, height, 4, 4, 4);
        let png = make_png(PngOptions {
            width: width as u32,
            height: height as u32,
            color_type: 6,
            scanlines: &scanlines,
            palette: None,
            transparency: None,
            idat_splits: &[0, 2, 1],
            ancillary: true,
        });
        let result = compare_png(&c, &rust, &png);
        assert_eq!((result.w, result.h), (width as c_int, height as c_int));
    }
}

#[test]
fn error_inflate_explicit_rejections() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();

    let bad_complement = [1, 0, 0, 0, 0];
    let result = compare_inflate(&c, &rust, &bad_complement, 0, 1, 1);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Failed to find LEN and NLEN as complements within stored (uncompressed) stream.")
    );

    let trailing_stored_data = [1, 0, 0, 0xff, 0xff, 0];
    let result = compare_inflate(&c, &rust, &trailing_stored_data, 1, 1, 1);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Stored block extends beyond end of input stream.")
    );

    let literal = fixed_blocks(&[b"A"]);
    let result = compare_inflate(&c, &rust, &literal, 2, 1, 0);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Attempted to overwrite out buffer while outputting a symbol.")
    );

    let invalid_distance = fixed_match_stream(&[], 0);
    let result = compare_inflate(&c, &rust, &invalid_distance, 3, 3, 3);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Attempted to write before out buffer (invalid backwards distance).")
    );

    let overflowing_match = fixed_match_stream(b"A", 0);
    let result = compare_inflate(&c, &rust, &overflowing_match, 0, 2, 2);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Attempted to overwrite out buffer while outputting a string.")
    );

    let reserved_block = [7];
    let result = compare_inflate(&c, &rust, &reserved_block, 0, 1, 1);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Detected unknown block type within input stream.")
    );

    let result = compare_inflate(&c, &rust, &literal, 1, 1, -1);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.reason.as_deref(),
        Some("Attempted to overwrite out buffer while outputting a symbol.")
    );
}

#[test]
fn error_png_signature_header_and_size_rejections() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();

    assert_png_error(
        compare_png(&c, &rust, &[0; 8]),
        (0, 0),
        "incorrect file signature (is this a png file?)",
    );

    let mut missing_ihdr = b"\x89PNG\r\n\x1a\n".to_vec();
    missing_ihdr.extend_from_slice(&chunk(b"JHDR", &[0; 13]));
    assert_png_error(
        compare_png(&c, &rust, &missing_ihdr),
        (0, 0),
        "unable to find IHDR chunk",
    );

    let bad_depth = raw_png(ihdr(1, 1, 16, 6, 0, 0, 0), &[]);
    assert_png_error(
        compare_png(&c, &rust, &bad_depth),
        (0, 0),
        "only bit-depth of 8 is supported",
    );

    for color in [1, 5, 7, 255] {
        let png = raw_png(ihdr(1, 1, 8, color, 0, 0, 0), &[]);
        assert_png_error(compare_png(&c, &rust, &png), (0, 0), "unknown color type");
    }

    let bad_width = raw_png(ihdr(u32::MAX, 1, 8, 6, 0, 0, 0), &[]);
    assert_png_error(
        compare_png(&c, &rust, &bad_width),
        (0, 0),
        "invalid IHDR chunk found, image width was less than 1",
    );

    let bad_height = raw_png(ihdr(1, 0, 8, 6, 0, 0, 0), &[]);
    assert_png_error(
        compare_png(&c, &rust, &bad_height),
        (0, 0),
        "invalid IHDR chunk found, image height was less than 1",
    );

    for &(width, height) in &[(536_870_911, 1), (32_768, 32_768), (1, 536_870_911)] {
        let png = raw_png(ihdr(width, height, 8, 6, 0, 0, 0), &[]);
        assert_png_error(compare_png(&c, &rust, &png), (0, 0), "image too large");
    }
}

#[test]
fn error_png_format_zlib_filter_and_palette_rejections() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    let dimensions = (1, 1);

    for (field, reason) in [
        ([1, 0, 0], "only standard compression DEFLATE is supported"),
        ([0, 1, 0], "only standard adaptive filtering is supported"),
        ([0, 0, 1], "interlacing is not supported"),
    ] {
        let png = raw_png(ihdr(1, 1, 8, 6, field[0], field[1], field[2]), &[]);
        assert_png_error(compare_png(&c, &rust, &png), dimensions, reason);
    }

    let no_idat = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[]);
    assert_png_error(
        compare_png(&c, &rust, &no_idat),
        dimensions,
        "corrupt zlib structure in DEFLATE stream",
    );
    for len in 0..6 {
        let bytes = vec![0u8; len];
        let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &bytes)]);
        assert_png_error(
            compare_png(&c, &rust, &png),
            dimensions,
            "corrupt zlib structure in DEFLATE stream",
        );
    }

    let mut bad_zlib = [0u8; 7];
    bad_zlib[0] = 0x71;
    let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &bad_zlib)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "only zlib compression method (RFC 1950) is supported",
    );

    bad_zlib[0] = 0x88;
    let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &bad_zlib)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "innapropriate window size detected",
    );

    bad_zlib[0] = 0x78;
    bad_zlib[1] = 0x20;
    let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &bad_zlib)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "preset dictionary is present and not supported",
    );

    let invalid_deflate = [0x78, 0, 7, 0, 0, 0, 0];
    let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &invalid_deflate)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "DEFLATE algorithm failed",
    );

    let invalid_first = zlib_compress(&[5, 1, 2, 3, 4], false, 6, 0);
    let png = raw_png(ihdr(1, 1, 8, 6, 0, 0, 0), &[(b"IDAT", &invalid_first)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "invalid filter byte found",
    );

    let invalid_later = zlib_compress(&[0, 1, 2, 3, 4, 5, 5, 6, 7, 8], false, 6, 0);
    let png = raw_png(ihdr(1, 2, 8, 6, 0, 0, 0), &[(b"IDAT", &invalid_later)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        (1, 2),
        "invalid filter byte found",
    );

    let indexed_data = zlib_compress(&[0, 0], false, 6, 0);
    let png = raw_png(ihdr(1, 1, 8, 3, 0, 0, 0), &[(b"IDAT", &indexed_data)]);
    assert_png_error(
        compare_png(&c, &rust, &png),
        dimensions,
        "color type of indexed requires a PLTE chunk",
    );
}

#[test]
fn isolated_ffi_child() {
    let Ok(case) = std::env::var("DIFF_CHILD_CASE") else {
        return;
    };
    let library = std::env::var("DIFF_CHILD_LIBRARY").expect("DIFF_CHILD_LIBRARY");
    let path = if library == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    let api = unsafe { Api::open(&path) };

    unsafe {
        *api.error_reason = ptr::null();
        match case.as_str() {
            "load_null" => {
                let image = (api.load_png)(ptr::null(), 0);
                println!("CHILD:{}:{}:{}", image.w, image.h, image.pix.is_null());
            }
            "load_zero_length" | "load_negative_length" | "load_oversized_length" => {
                let mut bytes = vec![0u8; 64];
                bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
                let declared = match case.as_str() {
                    "load_zero_length" => 0,
                    "load_negative_length" => -1,
                    _ => 4096,
                };
                let image = (api.load_png)(bytes.as_ptr(), declared);
                println!(
                    "CHILD:{}:{}:{}:{}",
                    image.w,
                    image.h,
                    image.pix.is_null(),
                    api.reason().unwrap_or_default()
                );
            }
            "inflate_null_input" => {
                let mut output = [0u8; 8];
                let code = (api.inflate)(ptr::null_mut(), 1, output.as_mut_ptr().cast(), 8);
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "inflate_null_output" => {
                let mut input = fixed_blocks(&[b"A"]);
                let code = (api.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    ptr::null_mut(),
                    1,
                );
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "inflate_zero_input" | "inflate_negative_input" => {
                let mut input = [0u8; 8];
                let mut output = [0u8; 8];
                let declared = if case == "inflate_zero_input" { 0 } else { -1 };
                let code = (api.inflate)(
                    input.as_mut_ptr().cast(),
                    declared,
                    output.as_mut_ptr().cast(),
                    8,
                );
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "inflate_truncated_fixed" => {
                let mut input = [3u8];
                let mut output = [0u8; 8];
                let code =
                    (api.inflate)(input.as_mut_ptr().cast(), 1, output.as_mut_ptr().cast(), 8);
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "inflate_truncated_stored" => {
                let mut input = [1u8];
                let mut output = [0u8; 8];
                let code =
                    (api.inflate)(input.as_mut_ptr().cast(), 1, output.as_mut_ptr().cast(), 8);
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "inflate_malformed_dynamic" => {
                let mut input = [5u8, 0, 0, 0, 0, 0, 0, 0];
                let mut output = [0u8; 8];
                let code = (api.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    8,
                );
                println!("CHILD:{code}:{}", api.reason().unwrap_or_default());
            }
            "allocation_failure" => {
                let png = raw_png(ihdr(250_000_000, 1, 8, 6, 0, 0, 0), &[]);
                let limits = RLimit {
                    current: 128 * 1024 * 1024,
                    maximum: 128 * 1024 * 1024,
                };
                assert_eq!(setrlimit(9, &limits), 0);
                let image = (api.load_png)(png.as_ptr(), png.len() as c_int);
                println!(
                    "CHILD:{}:{}:{}:{}",
                    image.w,
                    image.h,
                    image.pix.is_null(),
                    api.reason().unwrap_or_default()
                );
            }
            _ => panic!("unknown child case: {case}"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ChildOutcome {
    code: Option<i32>,
    signal: Option<i32>,
    marker: Option<String>,
}

fn child_outcome(library: &str, case: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "isolated_ffi_child", "--nocapture"])
        .env("DIFF_CHILD_LIBRARY", library)
        .env("DIFF_CHILD_CASE", case)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let marker = stdout
        .lines()
        .find_map(|line| line.find("CHILD:").map(|offset| line[offset..].to_owned()));
    ChildOutcome {
        code: output.status.code(),
        signal: output.status.signal(),
        marker,
    }
}

#[test]
fn error_process_boundaries_and_internal_assertions() {
    let _guard = TEST_LOCK.lock().unwrap();
    for case in [
        "load_null",
        "load_zero_length",
        "load_negative_length",
        "load_oversized_length",
        "inflate_null_input",
        "inflate_null_output",
        "inflate_zero_input",
        "inflate_negative_input",
        "inflate_truncated_fixed",
        "inflate_truncated_stored",
        "inflate_malformed_dynamic",
        "allocation_failure",
    ] {
        let expected = child_outcome("c", case);
        let actual = child_outcome("rust", case);
        assert_eq!(actual, expected, "isolated boundary mismatch for {case}");
    }
}

#[test]
fn exported_data_tables_match_byte_for_byte() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (c, rust) = apis();
    for (name, byte_len) in [
        (b"cp_fixed_table".as_slice(), 320usize),
        (b"cp_permutation_order".as_slice(), 19),
        (b"cp_len_extra_bits".as_slice(), 31),
        (b"cp_len_base".as_slice(), 31 * 4),
        (b"cp_dist_extra_bits".as_slice(), 32),
        (b"cp_dist_base".as_slice(), 32 * 4),
    ] {
        unsafe {
            let c_ptr = *c._library.get::<*const u8>(name).unwrap();
            let rust_ptr = *rust._library.get::<*const u8>(name).unwrap();
            assert_eq!(
                std::slice::from_raw_parts(rust_ptr, byte_len),
                std::slice::from_raw_parts(c_ptr, byte_len),
                "exported data differs for {}",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn internal_assertion_surface_is_translated() {
    let c_source = include_str!("../../c_src/src/lib.c");
    let rust_source = include_str!("../src/lib.rs");
    assert_eq!(c_source.matches("assert(").count(), 10);
    assert_eq!(rust_source.matches("assert!(").count(), 10);

    for condition in [
        "bits_left & 7 == 0",
        "word_index <= (*s).word_count",
        "count >= num_bits_to_read",
        "num_bits_to_read <= 32",
        "num_bits_to_read >= 0",
        "bits_left > 0",
        "count <= 64",
        "!would_overflow(s, num_bits_to_read)",
        "tree_len < 16",
        "(search >> len) == (key >> len)",
    ] {
        assert!(
            rust_source.contains(condition),
            "missing translated assertion condition: {condition}"
        );
    }
}
