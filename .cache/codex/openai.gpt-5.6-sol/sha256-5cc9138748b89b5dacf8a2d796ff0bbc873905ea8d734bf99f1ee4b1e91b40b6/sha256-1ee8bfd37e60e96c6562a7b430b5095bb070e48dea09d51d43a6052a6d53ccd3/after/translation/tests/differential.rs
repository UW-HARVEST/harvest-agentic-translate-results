#![allow(unsafe_op_in_unsafe_fn)]

use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type ConvertFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8, *mut Pixel);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
struct Pixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

struct Pair {
    c: Library,
    rust: Library,
}

#[derive(Debug, PartialEq, Eq)]
struct InflateResult {
    code: i32,
    output: Vec<u8>,
    reason: Option<String>,
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit: usize,
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl BitWriter {
    fn bits(&mut self, value: u32, count: usize) {
        for i in 0..count {
            if self.bit / 8 == self.bytes.len() {
                self.bytes.push(0);
            }
            self.bytes[self.bit / 8] |= (((value >> i) & 1) as u8) << (self.bit & 7);
            self.bit += 1;
        }
    }
}

impl BitReader<'_> {
    fn bits(&mut self, count: usize) -> u32 {
        let mut value = 0;
        for i in 0..count {
            value |= (((self.bytes[self.bit / 8] >> (self.bit & 7)) & 1) as u32) << i;
            self.bit += 1;
        }
        value
    }
}

fn lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_so() -> PathBuf {
    let root = manifest_dir();
    let candidates = [
        root.join("target/release/libconvert_pix_lib.so"),
        root.join("target/debug/libconvert_pix_lib.so"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .expect("build the Rust cdylib before running integration tests")
}

fn pair() -> Pair {
    let c = manifest_dir().join("../c_src/build/libharvest-work-lPsfCn.so");
    assert!(
        c.exists(),
        "C shared library does not exist at {}",
        c.display()
    );
    let rust = rust_so();
    unsafe {
        Pair {
            c: Library::new(c).unwrap(),
            rust: Library::new(rust).unwrap(),
        }
    }
}

unsafe fn error_reason(lib: &Library) -> Option<String> {
    let slot = lib.get::<*mut *const c_char>(b"cp_error_reason\0").unwrap();
    let ptr = **slot;
    (!ptr.is_null()).then(|| CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

unsafe fn clear_error(lib: &Library) {
    let slot = lib.get::<*mut *const c_char>(b"cp_error_reason\0").unwrap();
    **slot = std::ptr::null();
}

fn aligned_input(bytes: &[u8], alignment: usize) -> (Vec<u8>, usize) {
    let mut storage = vec![0u8; bytes.len() + 8];
    let base = storage.as_ptr() as usize;
    let offset = (alignment + 4 - (base & 3)) & 3;
    storage[offset..offset + bytes.len()].copy_from_slice(bytes);
    (storage, offset)
}

unsafe fn inflate(
    lib: &Library,
    bytes: &[u8],
    alignment: usize,
    out_bytes: usize,
) -> InflateResult {
    clear_error(lib);
    let function = lib.get::<InflateFn>(b"cp_inflate\0").unwrap();
    let (mut input, offset) = aligned_input(bytes, alignment);
    let physical_len = out_bytes.max(1);
    let mut output = vec![0xa5; physical_len];
    let code = function(
        input.as_mut_ptr().add(offset).cast(),
        bytes.len() as c_int,
        output.as_mut_ptr().cast(),
        out_bytes as c_int,
    );
    InflateResult {
        code,
        output,
        reason: error_reason(lib),
    }
}

fn compare_inflate(pair: &Pair, bytes: &[u8], alignment: usize, out_bytes: usize) -> InflateResult {
    unsafe {
        let c = inflate(&pair.c, bytes, alignment, out_bytes);
        let rust = inflate(&pair.rust, bytes, alignment, out_bytes);
        assert_eq!(rust, c, "stream={bytes:02x?}, alignment={alignment}");
        c
    }
}

fn reverse(value: u32, bits: usize) -> u32 {
    value.reverse_bits() >> (32 - bits)
}

fn fixed_symbol(writer: &mut BitWriter, symbol: u16) {
    let (code, bits) = match symbol {
        0..=143 => (0x30 + symbol as u32, 8),
        144..=255 => (0x190 + symbol as u32 - 144, 9),
        256..=279 => (symbol as u32 - 256, 7),
        280..=287 => (0xc0 + symbol as u32 - 280, 8),
        _ => panic!("invalid fixed symbol"),
    };
    writer.bits(reverse(code, bits), bits);
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn fixed_match(writer: &mut BitWriter, length: usize, distance: usize) {
    let length_code = (0..29)
        .find(|&i| {
            let max = LENGTH_BASE[i] as usize + ((1usize << LENGTH_EXTRA[i]) - 1);
            length >= LENGTH_BASE[i] as usize && length <= max
        })
        .unwrap();
    fixed_symbol(writer, 257 + length_code as u16);
    writer.bits(
        (length - LENGTH_BASE[length_code] as usize) as u32,
        LENGTH_EXTRA[length_code] as usize,
    );

    let distance_code = (0..30)
        .find(|&i| {
            let max = DIST_BASE[i] as usize + ((1usize << DIST_EXTRA[i]) - 1);
            distance >= DIST_BASE[i] as usize && distance <= max
        })
        .unwrap();
    writer.bits(reverse(distance_code as u32, 5), 5);
    writer.bits(
        (distance - DIST_BASE[distance_code] as usize) as u32,
        DIST_EXTRA[distance_code] as usize,
    );
}

fn fixed_literals(bytes: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    fixed_block(&mut writer, true, bytes);
    writer.bytes
}

fn fixed_block(writer: &mut BitWriter, final_block: bool, bytes: &[u8]) {
    writer.bits(final_block as u32, 1);
    writer.bits(1, 2);
    for &byte in bytes {
        fixed_symbol(writer, byte as u16);
    }
    fixed_symbol(writer, 256);
}

fn fixed_copy(prefix: &[u8], length: usize, distance: usize) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for &byte in prefix {
        fixed_symbol(&mut writer, byte as u16);
    }
    fixed_match(&mut writer, length, distance);
    fixed_symbol(&mut writer, 256);
    writer.bytes
}

fn stored(bytes: &[u8]) -> Vec<u8> {
    assert!(bytes.len() <= u16::MAX as usize);
    let len = bytes.len() as u16;
    let mut result = vec![
        1,
        len as u8,
        (len >> 8) as u8,
        !len as u8,
        (!len >> 8) as u8,
    ];
    result.extend_from_slice(bytes);
    result
}

fn next_random(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 24) as u8
}

fn random_bytes(state: &mut u64, len: usize) -> Vec<u8> {
    (0..len).map(|_| next_random(state)).collect()
}

fn zlib_deflate(source: &[u8], level: c_int) -> Vec<u8> {
    type CompressBound = unsafe extern "C" fn(usize) -> usize;
    type Compress2 = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, c_int) -> c_int;
    unsafe {
        let zlib = Library::new("libz.so.1").unwrap();
        let bound = zlib.get::<CompressBound>(b"compressBound\0").unwrap()(source.len());
        let mut wrapped = vec![0u8; bound];
        let mut len = wrapped.len();
        let code = zlib.get::<Compress2>(b"compress2\0").unwrap()(
            wrapped.as_mut_ptr(),
            &mut len,
            source.as_ptr(),
            source.len(),
            level,
        );
        assert_eq!(code, 0);
        wrapped.truncate(len);
        assert!(wrapped.len() >= 6);
        wrapped[2..wrapped.len() - 4].to_vec()
    }
}

fn dynamic_source(state: &mut u64, iteration: usize) -> Vec<u8> {
    let mut source = Vec::with_capacity(4096);
    let alphabet = b"etaoin shrdlu ETAOIN 0123456789\n";
    for i in 0..(2048 + iteration * 17) {
        let random = next_random(state) as usize;
        source.push(alphabet[(random + i / 11) % alphabet.len()]);
        if i % 97 == 0 {
            source.extend_from_slice(b"repeated-dictionary-phrase:");
        }
    }
    source
}

fn dynamic_header_uses_repeat(stream: &[u8]) -> bool {
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut reader = BitReader {
        bytes: stream,
        bit: 0,
    };
    reader.bits(1);
    assert_eq!(reader.bits(2), 2);
    let symbol_count = 257 + reader.bits(5) as usize + 1 + reader.bits(5) as usize;
    let code_length_count = 4 + reader.bits(4) as usize;
    let mut lengths = [0u8; 19];
    for &symbol in &ORDER[..code_length_count] {
        lengths[symbol] = reader.bits(3) as u8;
    }

    let mut counts = [0u16; 16];
    for &length in &lengths {
        counts[length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u16; 16];
    for length in 1..16 {
        next[length] = (next[length - 1] + counts[length - 1]) << 1;
    }
    let mut codes = [(0u16, 0u8); 19];
    for (symbol, &length) in lengths.iter().enumerate() {
        if length != 0 {
            codes[symbol] = (next[length as usize], length);
            next[length as usize] += 1;
        }
    }

    let mut produced = 0;
    let mut repeated = false;
    while produced < symbol_count {
        let mut incoming = 0u16;
        let mut symbol = None;
        for length in 1..16 {
            incoming |= (reader.bits(1) as u16) << (length - 1);
            symbol = codes.iter().position(|&(code, bits)| {
                bits as usize == length && reverse(code as u32, length) as u16 == incoming
            });
            if symbol.is_some() {
                break;
            }
        }
        match symbol.expect("invalid dynamic code-length symbol") {
            0..=15 => produced += 1,
            16 => {
                repeated = true;
                produced += 3 + reader.bits(2) as usize;
            }
            17 => {
                repeated = true;
                produced += 3 + reader.bits(3) as usize;
            }
            18 => {
                repeated = true;
                produced += 11 + reader.bits(7) as usize;
            }
            _ => unreachable!(),
        }
    }
    repeated
}

fn convert(lib: &Library, bpp: i32, w: i32, h: i32, source: &mut [u8]) -> Vec<Pixel> {
    let pixels = (w.max(0) as usize).saturating_mul(h.max(0) as usize);
    let mut output = vec![
        Pixel {
            r: 0xa5,
            g: 0xa5,
            b: 0xa5,
            a: 0xa5,
        };
        pixels.max(1)
    ];
    unsafe {
        lib.get::<ConvertFn>(b"convert_pix\0").unwrap()(
            bpp,
            w,
            h,
            source.as_mut_ptr(),
            output.as_mut_ptr(),
        );
    }
    output
}

#[test]
fn convert_pix_all_configurations() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0x5eed_1234_9876_abcd;

    for bpp in 1..=4 {
        for iteration in 0..64 {
            let w = 1 + iteration % 13;
            let h = 1 + iteration % 7;
            let mut source = random_bytes(&mut state, h * (1 + w * bpp as usize));
            let mut c_source = source.clone();
            let c = convert(&pair.c, bpp, w as i32, h as i32, &mut c_source);
            let rust = convert(&pair.rust, bpp, w as i32, h as i32, &mut source);
            assert_eq!(rust, c, "bpp={bpp}, w={w}, h={h}");
        }
    }

    for &(bpp, w, h) in &[
        (1, 5, 0),
        (4, 5, -1),
        (3, 0, 4),
        (2, -1, 4),
        (0, 5, 3),
        (5, 5, 3),
    ] {
        let len = (h.max(0) as usize) * (1 + (w.max(0) as usize) * bpp.max(0) as usize);
        let mut source = random_bytes(&mut state, len.max(8));
        let mut c_source = source.clone();
        assert_eq!(
            convert(&pair.rust, bpp, w, h, &mut source),
            convert(&pair.c, bpp, w, h, &mut c_source),
            "bpp={bpp}, w={w}, h={h}"
        );
    }

    unsafe {
        let c = pair.c.get::<ConvertFn>(b"convert_pix\0").unwrap();
        let rust = pair.rust.get::<ConvertFn>(b"convert_pix\0").unwrap();
        c(1, 1, 0, std::ptr::null_mut(), std::ptr::null_mut());
        rust(1, 1, 0, std::ptr::null_mut(), std::ptr::null_mut());
    }
}

#[test]
fn inflate_stored_randomized() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0x1020_3040_5060_7080;
    for iteration in 0..96 {
        let expected = random_bytes(&mut state, iteration * 7 % 503);
        let stream = stored(&expected);
        let result = compare_inflate(&pair, &stream, iteration & 3, expected.len());
        assert_eq!(result.code, 1);
    }
}

#[test]
fn inflate_fixed_literals_randomized() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0xa55a_9876_0123_4567;
    for iteration in 0..96 {
        let expected = random_bytes(&mut state, iteration * 11 % 701);
        let stream = fixed_literals(&expected);
        let result = compare_inflate(
            &pair,
            &stream,
            iteration & 3,
            expected.len() + (iteration & 1) * 17,
        );
        assert_eq!(result.code, 1);
        assert_eq!(&result.output[..expected.len()], &expected);
    }
}

#[test]
fn inflate_fixed_copy_branches_randomized() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0x0ddc_0ffe_e123_4567;
    for iteration in 0..64 {
        let byte = next_random(&mut state);
        let length = 3 + iteration % 31;
        let stream = fixed_copy(&[byte], length, 1);
        let expected = vec![byte; length + 1];
        let result = compare_inflate(&pair, &stream, iteration & 3, expected.len());
        assert_eq!(result.code, 1);
        assert_eq!(result.output, expected);

        let distance = 2 + iteration % 7;
        let prefix = random_bytes(&mut state, distance);
        let length = 3 + iteration % 43;
        let stream = fixed_copy(&prefix, length, distance);
        let mut expected = prefix;
        for i in 0..length {
            let byte = expected[expected.len() - distance];
            expected.push(byte);
            assert_eq!(expected[distance + i], byte);
        }
        let result = compare_inflate(&pair, &stream, (iteration + 1) & 3, expected.len());
        assert_eq!(result.code, 1);
        assert_eq!(result.output, expected);
    }
}

#[test]
fn inflate_dynamic_randomized() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0xd1a0_1c00_5eed_0001;
    for iteration in 0..48 {
        let expected = dynamic_source(&mut state, iteration);
        let stream = zlib_deflate(&expected, 6);
        assert_eq!(
            (stream[0] >> 1) & 3,
            2,
            "zlib did not select a dynamic block"
        );
        assert!(
            stream.len() < expected.len(),
            "stream should exercise matches"
        );
        assert!(
            dynamic_header_uses_repeat(&stream),
            "dynamic header did not use a repeat symbol"
        );
        let result = compare_inflate(
            &pair,
            &stream,
            iteration & 3,
            expected.len() + iteration % 19,
        );
        assert_eq!(result.code, 1);
        assert_eq!(&result.output[..expected.len()], &expected);
    }
}

#[test]
fn inflate_multiple_blocks_and_tail_shapes() {
    let _guard = lock();
    let pair = pair();
    let mut state = 0x1234_5678_9abc_def0;
    for iteration in 0..64 {
        let first = random_bytes(&mut state, 1 + iteration % 37);
        let second = random_bytes(&mut state, 1 + iteration % 41);
        let mut writer = BitWriter::default();
        fixed_block(&mut writer, false, &first);
        fixed_block(&mut writer, true, &second);
        let mut expected = first;
        expected.extend_from_slice(&second);
        let result = compare_inflate(&pair, &writer.bytes, iteration & 3, expected.len() + 9);
        assert_eq!(result.code, 1);
        assert_eq!(&result.output[..expected.len()], &expected);
    }

    let mut seen = [false; 4];
    for len in 0..128 {
        let expected = random_bytes(&mut state, len);
        let stream = fixed_literals(&expected);
        seen[stream.len() & 3] = true;
        let result = compare_inflate(&pair, &stream, len & 3, expected.len());
        assert_eq!(result.code, 1);
    }
    assert!(seen.into_iter().all(|value| value));
}

#[test]
fn exported_data_matches() {
    let _guard = lock();
    let pair = pair();
    unsafe fn compare<const N: usize>(c: &Library, rust: &Library, name: &[u8]) {
        let c_data = c.get::<*const [u8; N]>(name).unwrap();
        let rust_data = rust.get::<*const [u8; N]>(name).unwrap();
        assert_eq!(&**rust_data, &**c_data);
    }
    unsafe {
        compare::<320>(&pair.c, &pair.rust, b"cp_fixed_table\0");
        compare::<19>(&pair.c, &pair.rust, b"cp_permutation_order\0");
        compare::<31>(&pair.c, &pair.rust, b"cp_len_extra_bits\0");
        compare::<124>(&pair.c, &pair.rust, b"cp_len_base\0");
        compare::<32>(&pair.c, &pair.rust, b"cp_dist_extra_bits\0");
        compare::<128>(&pair.c, &pair.rust, b"cp_dist_base\0");
        assert_eq!(error_reason(&pair.c), None);
        assert_eq!(error_reason(&pair.rust), None);
    }
}

#[test]
fn explicit_error_returns_and_reasons_match() {
    let _guard = lock();
    let pair = pair();
    let cases = [
        (
            vec![1, 1, 0, 0, 0, b'x'],
            8,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        ),
        (
            vec![1, 0, 0, 0xff, 0xff, 0],
            8,
            "Stored block extends beyond end of input stream.",
        ),
        (
            fixed_literals(b"x"),
            0,
            "Attempted to overwrite out buffer while outputting a symbol.",
        ),
        (
            fixed_copy(&[], 3, 1),
            8,
            "Attempted to write before out buffer (invalid backwards distance).",
        ),
        (
            fixed_copy(b"x", 10, 1),
            5,
            "Attempted to overwrite out buffer while outputting a string.",
        ),
        (
            vec![7],
            8,
            "Detected unknown block type within input stream.",
        ),
    ];
    for (stream, out_len, reason) in cases {
        for alignment in 0..4 {
            let result = compare_inflate(&pair, &stream, alignment, out_len);
            assert_eq!(result.code, 0);
            assert_eq!(result.reason.as_deref(), Some(reason));
        }
    }
}

#[test]
fn fatal_error_boundaries_match() {
    if let (Ok(which), Ok(case)) = (
        std::env::var("CP_ABORT_LIBRARY"),
        std::env::var("CP_ABORT_CASE"),
    ) {
        let pair = pair();
        let lib = if which == "c" { &pair.c } else { &pair.rust };
        let mut input = match case.as_str() {
            "zero-input" | "null-input" => vec![0],
            "truncated-fixed" => vec![3],
            "invalid-dynamic" => vec![5, 0, 0, 0],
            "null-output" => fixed_literals(b"x"),
            _ => panic!("unknown fatal case"),
        };
        let input_pointer = if case == "null-input" {
            std::ptr::null_mut()
        } else {
            input.as_mut_ptr()
        };
        let input_length = if case == "zero-input" {
            0
        } else {
            input.len() as c_int
        };
        let mut output = vec![0u8; 8];
        let output_pointer = if case == "null-output" {
            std::ptr::null_mut()
        } else {
            output.as_mut_ptr()
        };
        unsafe {
            let function = lib.get::<InflateFn>(b"cp_inflate\0").unwrap();
            function(
                input_pointer.cast(),
                input_length,
                output_pointer.cast(),
                output.len() as c_int,
            );
        }
        panic!("{case} cp_inflate unexpectedly returned");
    }

    let _guard = lock();
    let executable = std::env::current_exe().unwrap();
    let run = |which: &str, case: &str| {
        std::process::Command::new(&executable)
            .arg("--exact")
            .arg("fatal_error_boundaries_match")
            .arg("--nocapture")
            .env("CP_ABORT_LIBRARY", which)
            .env("CP_ABORT_CASE", case)
            .status()
            .unwrap()
    };
    for case in [
        "zero-input",
        "truncated-fixed",
        "invalid-dynamic",
        "null-input",
        "null-output",
    ] {
        let c = run("c", case);
        let rust = run("rust", case);
        assert!(!c.success(), "C unexpectedly survived {case}");
        assert!(!rust.success(), "Rust unexpectedly survived {case}");
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(rust.signal(), c.signal(), "different signal for {case}");
        }
    }
}

#[test]
fn shared_object_paths_are_real_files() {
    let _guard = lock();
    assert!(Path::new(&rust_so()).is_file());
    assert!(
        manifest_dir()
            .join("../c_src/build/libharvest-work-lPsfCn.so")
            .is_file()
    );
}
