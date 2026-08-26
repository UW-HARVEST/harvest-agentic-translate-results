use libloading::Library;
use std::collections::BTreeSet;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

use pinflate_lib as _;

type Pinflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

const SENTINEL: u8 = 0xa5;
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const LEN_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const DIST_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

struct Api {
    _library: Library,
    pinflate: Pinflate,
    error_slot: *mut *const c_char,
}

#[derive(Debug, PartialEq, Eq)]
struct CallResult {
    return_code: c_int,
    output: Vec<u8>,
    error: Option<Vec<u8>>,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = Library::new(path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let pinflate = *library
            .get::<Pinflate>(b"pinflate\0")
            .expect("missing pinflate");
        let error_slot = *library
            .get::<*mut *const c_char>(b"cp_error_reason\0")
            .expect("missing cp_error_reason");
        Self {
            _library: library,
            pinflate,
            error_slot,
        }
    }

    unsafe fn call(
        &self,
        input: &[u8],
        input_alignment: usize,
        output_capacity: usize,
    ) -> CallResult {
        assert!(input_alignment < 4);
        assert!(input.len() <= c_int::MAX as usize);
        assert!(output_capacity <= c_int::MAX as usize);

        let mut input_storage = vec![0u8; input.len() + 8];
        let base = input_storage.as_mut_ptr() as usize;
        let offset = (0..8)
            .find(|offset| (base + offset) & 3 == input_alignment)
            .unwrap();
        input_storage[offset..offset + input.len()].copy_from_slice(input);

        let allocation = output_capacity.max(1);
        let mut output = vec![SENTINEL; allocation];
        *self.error_slot = ptr::null();
        let return_code = (self.pinflate)(
            input_storage.as_mut_ptr().add(offset).cast(),
            input.len() as c_int,
            output.as_mut_ptr().cast(),
            output_capacity as c_int,
        );
        let error_pointer = *self.error_slot;
        let error = if error_pointer.is_null() {
            None
        } else {
            Some(CStr::from_ptr(error_pointer).to_bytes().to_vec())
        };
        CallResult {
            return_code,
            output,
            error,
        }
    }

    unsafe fn data(&self, name: &[u8], len: usize) -> Vec<u8> {
        let pointer = *self
            ._library
            .get::<*const u8>(name)
            .unwrap_or_else(|_| panic!("missing data symbol {:?}", name));
        std::slice::from_raw_parts(pointer, len).to_vec()
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let deps = executable.parent().expect("test deps directory");
    let profile = deps.parent().expect("target profile directory");
    let profile_library = profile.join("libpinflate_lib.so");
    if profile_library.exists() {
        profile_library
    } else {
        deps.join("libpinflate_lib.so")
    }
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn compare_call(
    c: &Api,
    rust: &Api,
    stream: &[u8],
    alignment: usize,
    output_capacity: usize,
) -> CallResult {
    let c_result = unsafe { c.call(stream, alignment, output_capacity) };
    let rust_result = unsafe { rust.call(stream, alignment, output_capacity) };
    assert_eq!(
        c_result,
        rust_result,
        "C/Rust mismatch: input_len={}, alignment={}, output_capacity={}",
        stream.len(),
        alignment,
        output_capacity
    );
    c_result
}

fn compare_valid(
    c: &Api,
    rust: &Api,
    stream: &[u8],
    expected: &[u8],
    alignment: usize,
    spare_capacity: usize,
) {
    let result = compare_call(c, rust, stream, alignment, expected.len() + spare_capacity);
    assert_eq!(
        result.return_code, 1,
        "unexpected C rejection: {:?}",
        result.error
    );
    assert_eq!(&result.output[..expected.len()], expected);
    assert!(result.output[expected.len()..]
        .iter()
        .all(|byte| *byte == SENTINEL));
    assert_eq!(result.error, None);
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

    fn usize(&mut self, upper: usize) -> usize {
        assert!(upper > 0);
        (self.next_u64() as usize) % upper
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit: usize,
}

impl BitWriter {
    fn write(&mut self, value: u32, count: usize) {
        for index in 0..count {
            if self.bit & 7 == 0 {
                self.bytes.push(0);
            }
            self.bytes[self.bit >> 3] |= (((value >> index) & 1) as u8) << (self.bit & 7);
            self.bit += 1;
        }
    }

    fn align_byte(&mut self) {
        while self.bit & 7 != 0 {
            self.write(0, 1);
        }
    }

    fn append_bytes_as_bits(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write(*byte as u32, 8);
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

fn write_fixed_symbol(writer: &mut BitWriter, symbol: usize) {
    let (code, length) = match symbol {
        0..=143 => (0x30 + symbol as u32, 8),
        144..=255 => (0x190 + (symbol - 144) as u32, 9),
        256..=279 => ((symbol - 256) as u32, 7),
        280..=287 => (0xc0 + (symbol - 280) as u32, 8),
        _ => panic!("invalid fixed symbol {symbol}"),
    };
    writer.write(reverse_bits(code, length), length);
}

fn write_length(writer: &mut BitWriter, length: usize) {
    let index = LEN_BASE
        .iter()
        .enumerate()
        .rev()
        .find(|(_, base)| **base <= length)
        .map(|(index, _)| index)
        .expect("length below three");
    let extra_count = LEN_EXTRA[index] as usize;
    assert!(length < LEN_BASE[index] + (1usize << extra_count) || index == 28);
    write_fixed_symbol(writer, 257 + index);
    writer.write((length - LEN_BASE[index]) as u32, extra_count);
}

fn write_distance(writer: &mut BitWriter, distance: usize) {
    let index = DIST_BASE
        .iter()
        .enumerate()
        .rev()
        .find(|(_, base)| **base <= distance)
        .map(|(index, _)| index)
        .expect("zero distance");
    let extra_count = DIST_EXTRA[index] as usize;
    assert!(distance < DIST_BASE[index] + (1usize << extra_count));
    writer.write(reverse_bits(index as u32, 5), 5);
    writer.write((distance - DIST_BASE[index]) as u32, extra_count);
}

enum FixedToken {
    Literal(u8),
    Copy { length: usize, distance: usize },
}

fn write_fixed_block(writer: &mut BitWriter, final_block: bool, tokens: &[FixedToken]) {
    writer.write(final_block as u32, 1);
    writer.write(1, 2);
    for token in tokens {
        match token {
            FixedToken::Literal(byte) => write_fixed_symbol(writer, *byte as usize),
            FixedToken::Copy { length, distance } => {
                write_length(writer, *length);
                write_distance(writer, *distance);
            }
        }
    }
    write_fixed_symbol(writer, 256);
}

fn fixed_literals(payload: &[u8]) -> Vec<u8> {
    let tokens: Vec<_> = payload.iter().copied().map(FixedToken::Literal).collect();
    let mut writer = BitWriter::default();
    write_fixed_block(&mut writer, true, &tokens);
    writer.into_bytes()
}

fn fixed_tokens(tokens: &[FixedToken]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    write_fixed_block(&mut writer, true, tokens);
    writer.into_bytes()
}

fn stored(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() <= u16::MAX as usize);
    let len = payload.len() as u16;
    let mut writer = BitWriter::default();
    writer.write(1, 1);
    writer.write(0, 2);
    writer.align_byte();
    writer.write(len as u32, 16);
    writer.write((!len) as u32, 16);
    writer.append_bytes_as_bits(payload);
    writer.into_bytes()
}

fn first_bytes_for_alignment(alignment: usize) -> usize {
    (4 - alignment) & 3
}

fn with_partial_word(stream: &[u8], alignment: usize, target_tail: usize) -> Vec<u8> {
    let first_bytes = first_bytes_for_alignment(alignment);
    for padding in 0..8 {
        if stream.len() + padding >= first_bytes
            && (stream.len() + padding - first_bytes) & 3 == target_tail
        {
            let mut padded = stream.to_vec();
            padded.extend((0..padding).map(|index| 0xd0 | index as u8));
            return padded;
        }
    }
    unreachable!()
}

#[repr(C)]
struct ZStream {
    next_in: *mut u8,
    avail_in: u32,
    total_in: CULong,
    next_out: *mut u8,
    avail_out: u32,
    total_out: CULong,
    msg: *mut c_char,
    state: *mut c_void,
    zalloc: Option<unsafe extern "C" fn(*mut c_void, u32, u32) -> *mut c_void>,
    zfree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
    data_type: c_int,
    adler: CULong,
    reserved: CULong,
}

#[cfg(target_pointer_width = "64")]
type CULong = u64;
#[cfg(target_pointer_width = "32")]
type CULong = u32;

type DeflateInit2 = unsafe extern "C" fn(
    *mut ZStream,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type Deflate = unsafe extern "C" fn(*mut ZStream, c_int) -> c_int;
type DeflateEnd = unsafe extern "C" fn(*mut ZStream) -> c_int;
type ZlibVersion = unsafe extern "C" fn() -> *const c_char;

struct Zlib {
    _library: Library,
    init2: DeflateInit2,
    deflate: Deflate,
    end: DeflateEnd,
    version: ZlibVersion,
}

impl Zlib {
    unsafe fn load() -> Self {
        let library = Library::new("libz.so.1").expect("load libz.so.1");
        let init2 = *library
            .get::<DeflateInit2>(b"deflateInit2_\0")
            .expect("deflateInit2_");
        let deflate = *library.get::<Deflate>(b"deflate\0").expect("deflate");
        let end = *library
            .get::<DeflateEnd>(b"deflateEnd\0")
            .expect("deflateEnd");
        let version = *library
            .get::<ZlibVersion>(b"zlibVersion\0")
            .expect("zlibVersion");
        Self {
            _library: library,
            init2,
            deflate,
            end,
            version,
        }
    }

    fn raw_deflate(&self, input: &[u8], level: c_int, strategy: c_int) -> Vec<u8> {
        assert!(input.len() <= u32::MAX as usize);
        let mut output = vec![0u8; input.len().saturating_mul(2).saturating_add(4096)];
        let mut stream: ZStream = unsafe { std::mem::zeroed() };
        stream.next_in = input.as_ptr() as *mut u8;
        stream.avail_in = input.len() as u32;
        stream.next_out = output.as_mut_ptr();
        stream.avail_out = output.len() as u32;

        let initialized = unsafe {
            (self.init2)(
                &mut stream,
                level,
                8,
                -15,
                8,
                strategy,
                (self.version)(),
                std::mem::size_of::<ZStream>() as c_int,
            )
        };
        assert_eq!(initialized, 0, "deflateInit2_ failed");
        let result = unsafe { (self.deflate)(&mut stream, 4) };
        let ended = unsafe { (self.end)(&mut stream) };
        assert_eq!(result, 1, "deflate did not reach stream end");
        assert_eq!(ended, 0, "deflateEnd failed");
        assert_eq!(stream.avail_in, 0);
        output.truncate(stream.total_out as usize);
        output
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, bit: 0 }
    }

    fn read(&mut self, count: usize) -> Option<u32> {
        if self.bit.checked_add(count)? > self.bytes.len() * 8 {
            return None;
        }
        let mut value = 0;
        for index in 0..count {
            value |= (((self.bytes[self.bit >> 3] >> (self.bit & 7)) & 1) as u32) << index;
            self.bit += 1;
        }
        Some(value)
    }
}

#[derive(Debug)]
struct DecodeEntry {
    reversed_code: u32,
    length: usize,
    symbol: usize,
}

fn canonical_entries(lengths: &[u8]) -> Vec<DecodeEntry> {
    let mut counts = [0u32; 16];
    for length in lengths {
        counts[*length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0;
    for bits in 1..=15 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    let mut entries = Vec::new();
    for (symbol, length) in lengths.iter().copied().enumerate() {
        if length != 0 {
            let length = length as usize;
            entries.push(DecodeEntry {
                reversed_code: reverse_bits(next[length], length),
                length,
                symbol,
            });
            next[length] += 1;
        }
    }
    entries
}

fn decode_symbol(reader: &mut BitReader<'_>, entries: &[DecodeEntry]) -> Option<usize> {
    let mut code = 0;
    for length in 1..=15 {
        code |= reader.read(1)? << (length - 1);
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.length == length && entry.reversed_code == code)
        {
            return Some(entry.symbol);
        }
    }
    None
}

#[derive(Debug)]
struct DynamicInfo {
    repeat_symbols: BTreeSet<usize>,
    max_tree_length: u8,
    has_zero_tree_length: bool,
}

fn dynamic_header_info(stream: &[u8]) -> Option<DynamicInfo> {
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut reader = BitReader::new(stream);
    reader.read(1)?;
    if reader.read(2)? != 2 {
        return None;
    }
    let literal_count = 257 + reader.read(5)? as usize;
    let distance_count = 1 + reader.read(5)? as usize;
    let code_length_count = 4 + reader.read(4)? as usize;
    let mut code_lengths = [0u8; 19];
    for index in 0..code_length_count {
        code_lengths[ORDER[index]] = reader.read(3)? as u8;
    }
    let decoder = canonical_entries(&code_lengths);
    let mut lengths = Vec::with_capacity(literal_count + distance_count);
    let mut repeats = BTreeSet::new();
    while lengths.len() < literal_count + distance_count {
        let symbol = decode_symbol(&mut reader, &decoder)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            16 => {
                repeats.insert(16);
                let previous = *lengths.last()?;
                let count = 3 + reader.read(2)? as usize;
                lengths.extend(std::iter::repeat_n(previous, count));
            }
            17 => {
                repeats.insert(17);
                let count = 3 + reader.read(3)? as usize;
                lengths.extend(std::iter::repeat_n(0, count));
            }
            18 => {
                repeats.insert(18);
                let count = 11 + reader.read(7)? as usize;
                lengths.extend(std::iter::repeat_n(0, count));
            }
            _ => return None,
        }
    }
    Some(DynamicInfo {
        repeat_symbols: repeats,
        max_tree_length: lengths.iter().copied().max().unwrap_or(0),
        has_zero_tree_length: lengths.contains(&0),
    })
}

fn dynamic_payload(rng: &mut Rng, index: usize) -> Vec<u8> {
    let len = 500 + rng.usize(9500);
    match index % 5 {
        0 => rng.bytes(len),
        1 => {
            let alphabet = 2 + rng.usize(18);
            (0..len).map(|_| rng.usize(alphabet) as u8).collect()
        }
        2 => {
            let pattern_len = 1 + rng.usize(99);
            let pattern = rng.bytes(pattern_len);
            pattern.iter().copied().cycle().take(len).collect()
        }
        3 => (0..len)
            .map(|position| ((position * position + index) % 251) as u8)
            .collect(),
        _ => {
            let mut pattern = b"abracadabra".to_vec();
            pattern.extend(rng.bytes(10));
            pattern.iter().copied().cycle().take(len).collect()
        }
    }
}

fn assert_error(c: &Api, rust: &Api, stream: &[u8], output_capacity: usize, expected: &[u8]) {
    for alignment in 0..4 {
        let result = compare_call(c, rust, stream, alignment, output_capacity);
        assert_eq!(result.return_code, 0);
        assert_eq!(result.error.as_deref(), Some(expected));
    }
}

#[test]
fn complete_differential_surface() {
    if std::env::var_os("PINFLATE_CHILD_CASE").is_some() {
        return;
    }

    let (c, rust) = load_apis();
    let symbols = [
        (b"cp_fixed_table\0".as_slice(), 320),
        (b"cp_permutation_order\0".as_slice(), 19),
        (b"cp_len_extra_bits\0".as_slice(), 31),
        (b"cp_len_base\0".as_slice(), 124),
        (b"cp_dist_extra_bits\0".as_slice(), 32),
        (b"cp_dist_base\0".as_slice(), 128),
    ];
    for (name, len) in symbols {
        assert_eq!(
            unsafe { c.data(name, len) },
            unsafe { rust.data(name, len) },
            "data symbol differs: {:?}",
            name
        );
    }

    let c_source = std::fs::read_to_string(manifest_dir().join("c_src/src/lib.c")).unwrap();
    let rust_source = std::fs::read_to_string(manifest_dir().join("src/lib.rs")).unwrap();
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
            c_source.contains(assertion),
            "missing C assertion {assertion}"
        );
    }
    for assertion in [
        "assert_eq!((*s).bits_left & 7, 0)",
        "assert!((*s).word_index <= (*s).word_count)",
        "assert!((*s).count >= num_bits_to_read)",
        "assert!(num_bits_to_read <= 32)",
        "assert!(num_bits_to_read >= 0)",
        "assert!((*s).bits_left > 0)",
        "assert!((*s).count <= 64)",
        "assert!(!would_overflow(s, num_bits_to_read))",
        "assert!(len < 16)",
        "assert_eq!(search >> len, key >> len)",
    ] {
        assert!(
            rust_source.contains(assertion),
            "missing Rust assertion {assertion}"
        );
    }

    let mut rng = Rng::new(0x5eed_cafe_1234_9876);

    // Stored blocks: empty, exact/spare capacity, and every alignment/tail pair.
    for alignment in 0..4 {
        compare_valid(&c, &rust, &stored(&[]), &[], alignment, 0);
    }
    for alignment in 0..4 {
        for target_tail in 0..4 {
            for round in 0..8 {
                let first_bytes = first_bytes_for_alignment(alignment);
                let mut len = 1 + rng.usize(256);
                while (5 + len - first_bytes) & 3 != target_tail {
                    len += 1;
                }
                let payload = rng.bytes(len);
                let stream = stored(&payload);
                assert_eq!((stream.len() - first_bytes) & 3, target_tail);
                compare_valid(&c, &rust, &stream, &payload, alignment, 0);
                compare_valid(&c, &rust, &stream, &payload, alignment, 1 + round);
            }
        }
    }

    // Fixed Huffman: EOB-only and randomized literal-only streams.
    let empty_fixed = fixed_literals(&[]);
    for alignment in 0..4 {
        for target_tail in 0..4 {
            let stream = with_partial_word(&empty_fixed, alignment, target_tail);
            compare_valid(&c, &rust, &stream, &[], alignment, 0);
        }
    }
    for alignment in 0..4 {
        for target_tail in 0..4 {
            for round in 0..8 {
                let payload_len = 1 + rng.usize(192);
                let payload = rng.bytes(payload_len);
                let stream = with_partial_word(&fixed_literals(&payload), alignment, target_tail);
                compare_valid(&c, &rust, &stream, &payload, alignment, round & 7);
            }
        }
    }

    // Fixed back-references cover both copy branches and nonzero extras.
    for round in 0..48 {
        let alignment = round & 3;
        let target_tail = (round >> 2) & 3;
        let byte = rng.next_u64() as u8;
        let length = 3 + rng.usize(8);
        let distance_one = fixed_tokens(&[
            FixedToken::Literal(byte),
            FixedToken::Copy {
                length,
                distance: 1,
            },
        ]);
        let expected = vec![byte; length + 1];
        let stream = with_partial_word(&distance_one, alignment, target_tail);
        compare_valid(&c, &rust, &stream, &expected, alignment, round & 1);

        let prefix = rng.bytes(4);
        let mut tokens: Vec<_> = prefix.iter().copied().map(FixedToken::Literal).collect();
        tokens.push(FixedToken::Copy {
            length: 4,
            distance: 4,
        });
        let mut expected = prefix.clone();
        expected.extend_from_slice(&prefix);
        let stream = with_partial_word(&fixed_tokens(&tokens), alignment, target_tail);
        compare_valid(&c, &rust, &stream, &expected, alignment, round & 3);

        let extra_length = 11 + rng.usize(8);
        let stream = fixed_tokens(&[
            FixedToken::Literal(byte),
            FixedToken::Copy {
                length: extra_length,
                distance: 1,
            },
        ]);
        compare_valid(
            &c,
            &rust,
            &with_partial_word(&stream, alignment, target_tail),
            &vec![byte; extra_length + 1],
            alignment,
            0,
        );

        let prefix = rng.bytes(5);
        let mut tokens: Vec<_> = prefix.iter().copied().map(FixedToken::Literal).collect();
        tokens.push(FixedToken::Copy {
            length: 3,
            distance: 5,
        });
        let mut expected = prefix.clone();
        expected.extend_from_slice(&prefix[..3]);
        compare_valid(
            &c,
            &rust,
            &with_partial_word(&fixed_tokens(&tokens), alignment, target_tail),
            &expected,
            alignment,
            0,
        );
    }

    // Back-to-back fixed blocks retain bit alignment across BFINAL=0.
    for round in 0..32 {
        let first_len = 1 + rng.usize(40);
        let second_len = 1 + rng.usize(40);
        let first = rng.bytes(first_len);
        let second = rng.bytes(second_len);
        let first_tokens: Vec<_> = first.iter().copied().map(FixedToken::Literal).collect();
        let second_tokens: Vec<_> = second.iter().copied().map(FixedToken::Literal).collect();
        let mut writer = BitWriter::default();
        write_fixed_block(&mut writer, false, &first_tokens);
        write_fixed_block(&mut writer, true, &second_tokens);
        let alignment = round & 3;
        let target_tail = (round >> 2) & 3;
        let stream = with_partial_word(&writer.into_bytes(), alignment, target_tail);
        let mut expected = first;
        expected.extend(second);
        compare_valid(&c, &rust, &stream, &expected, alignment, round & 3);
    }

    let zlib = unsafe { Zlib::load() };
    let mut dynamic_cases = Vec::new();
    for index in 0..800 {
        let payload = dynamic_payload(&mut rng, index);
        let strategy = if index % 7 == 0 { 2 } else { 0 };
        let stream = zlib.raw_deflate(&payload, 6, strategy);
        if let Some(info) = dynamic_header_info(&stream) {
            dynamic_cases.push((payload, stream, info));
        }
        if dynamic_cases.len() >= 240 {
            break;
        }
    }
    assert!(
        dynamic_cases.len() >= 120,
        "too few dynamic streams: {}",
        dynamic_cases.len()
    );

    let mut repeat_counts = [0usize; 3];
    let mut long_code_count = 0;
    let mut zero_length_count = 0;
    for (index, (payload, stream, info)) in dynamic_cases.iter().enumerate() {
        for symbol in &info.repeat_symbols {
            if (16..=18).contains(symbol) {
                repeat_counts[symbol - 16] += 1;
            }
        }
        long_code_count += usize::from(info.max_tree_length > 9);
        zero_length_count += usize::from(info.has_zero_tree_length);
        let alignment = index & 3;
        let target_tail = (index >> 2) & 3;
        let stream = with_partial_word(stream, alignment, target_tail);
        compare_valid(&c, &rust, &stream, payload, alignment, index & 7);
    }
    assert!(
        repeat_counts.iter().all(|count| *count >= 12),
        "{repeat_counts:?}"
    );
    assert!(
        long_code_count >= 12,
        "only {long_code_count} long-code cases"
    );
    assert!(
        zero_length_count >= 12,
        "only {zero_length_count} zero-length cases"
    );

    // A fixed non-final block followed immediately by a final dynamic block.
    for (index, (payload, dynamic_stream, _)) in dynamic_cases.iter().take(32).enumerate() {
        let prefix_len = 1 + rng.usize(20);
        let prefix = rng.bytes(prefix_len);
        let prefix_tokens: Vec<_> = prefix.iter().copied().map(FixedToken::Literal).collect();
        let mut writer = BitWriter::default();
        write_fixed_block(&mut writer, false, &prefix_tokens);
        writer.append_bytes_as_bits(dynamic_stream);
        let alignment = index & 3;
        let target_tail = (index >> 2) & 3;
        let stream = with_partial_word(&writer.into_bytes(), alignment, target_tail);
        let mut expected = prefix;
        expected.extend_from_slice(payload);
        compare_valid(&c, &rust, &stream, &expected, alignment, index & 3);
    }

    // Large streams cross input words and include long-distance opportunities.
    for round in 0..8 {
        let prefix: Vec<_> = (0..32_768).map(|_| rng.usize(32) as u8).collect();
        let mut payload = prefix.clone();
        payload.extend_from_slice(&prefix);
        payload.extend((0..8192).map(|_| rng.usize(7) as u8));
        let stream = zlib.raw_deflate(&payload, 9, 0);
        compare_valid(&c, &rust, &stream, &payload, round & 3, round);
    }
    for alignment in 0..4 {
        let prefix: Vec<_> = (0..32_768).map(|_| rng.next_u64() as u8).collect();
        let mut tokens: Vec<_> = prefix.iter().copied().map(FixedToken::Literal).collect();
        tokens.push(FixedToken::Copy {
            length: 258,
            distance: 32_768,
        });
        let mut expected = prefix.clone();
        expected.extend_from_slice(&prefix[..258]);
        compare_valid(&c, &rust, &fixed_tokens(&tokens), &expected, alignment, 0);
    }

    const COMPLEMENT: &[u8] =
        b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
    const STORED_END: &[u8] = b"Stored block extends beyond end of input stream.";
    const SYMBOL_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.";
    const DISTANCE: &[u8] = b"Attempted to write before out buffer (invalid backwards distance).";
    const STRING_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a string.";
    const BLOCK_TYPE: &[u8] = b"Detected unknown block type within input stream.";

    assert_error(
        &c,
        &rust,
        &[0x01, 0x01, 0x00, 0x00, 0x00, b'x'],
        8,
        COMPLEMENT,
    );
    let mut stored_with_extra = stored(&[]);
    stored_with_extra.push(0);
    assert_error(&c, &rust, &stored_with_extra, 0, STORED_END);
    assert_error(&c, &rust, &fixed_literals(b"x"), 0, SYMBOL_OUTPUT);
    assert_error(
        &c,
        &rust,
        &fixed_tokens(&[FixedToken::Copy {
            length: 3,
            distance: 1,
        }]),
        8,
        DISTANCE,
    );
    assert_error(
        &c,
        &rust,
        &fixed_tokens(&[
            FixedToken::Literal(b'x'),
            FixedToken::Copy {
                length: 3,
                distance: 1,
            },
        ]),
        2,
        STRING_OUTPUT,
    );
    assert_error(&c, &rust, &[0x07], 8, BLOCK_TYPE);

    for case in [
        "zero_input",
        "truncated_fixed",
        "invalid_huffman",
        "null_input",
        "null_output",
        "negative_input",
        "negative_output",
        "oversized_input",
    ] {
        let c_outcome = run_child(&c_library_path(), case);
        let rust_outcome = run_child(&rust_library_path(), case);
        assert_eq!(
            c_outcome, rust_outcome,
            "boundary behavior class differs for {case}"
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ChildOutcome {
    Returned(String),
    Terminated,
}

fn classify_child(status: ExitStatus, stdout: &[u8]) -> ChildOutcome {
    if status.success() {
        let output = String::from_utf8_lossy(stdout);
        let start = output
            .find("PINFLATE_CHILD_RESULT=")
            .unwrap_or_else(|| panic!("child succeeded without marker: {output}"));
        let marker = output[start..].lines().next().unwrap();
        ChildOutcome::Returned(marker.to_owned())
    } else {
        ChildOutcome::Terminated
    }
}

fn run_child(library: &Path, case: &str) -> ChildOutcome {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "ffi_boundary_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PINFLATE_CHILD_CASE", case)
        .env("PINFLATE_CHILD_LIBRARY", library)
        .output()
        .unwrap_or_else(|error| panic!("failed to run child for {case}: {error}"));
    classify_child(output.status, &output.stdout)
}

#[test]
fn ffi_boundary_child() {
    let Some(case) = std::env::var_os("PINFLATE_CHILD_CASE") else {
        return;
    };
    let path = PathBuf::from(std::env::var_os("PINFLATE_CHILD_LIBRARY").unwrap());
    let api = unsafe { Api::load(&path) };
    let mut input = match case.to_str().unwrap() {
        "zero_input" => vec![0],
        "truncated_fixed" => vec![0xc2, 0x68, 0xd8],
        "invalid_huffman" => hex_bytes("d4ebf0089295ebab5c2d6eb1c6f97c153219"),
        "null_input" => vec![0; 4],
        "null_output" | "negative_output" => fixed_literals(b"x"),
        "negative_input" | "oversized_input" => vec![0x07; 8],
        _ => panic!("unknown child case {:?}", case),
    };
    let mut output = vec![SENTINEL; 16];
    unsafe {
        *api.error_slot = ptr::null();
        let input_pointer = if case == "null_input" {
            ptr::null_mut()
        } else {
            input.as_mut_ptr().cast()
        };
        let output_pointer = if case == "null_output" {
            ptr::null_mut()
        } else {
            output.as_mut_ptr().cast()
        };
        let input_length = match case.to_str().unwrap() {
            "zero_input" => 0,
            "negative_input" => -1,
            "oversized_input" => c_int::MAX / 8 + 1,
            _ => input.len() as c_int,
        };
        let output_length = if case == "negative_output" { -1 } else { 16 };
        let result = (api.pinflate)(input_pointer, input_length, output_pointer, output_length);
        let reason = if (*api.error_slot).is_null() {
            "none".to_owned()
        } else {
            String::from_utf8_lossy(CStr::from_ptr(*api.error_slot).to_bytes()).into_owned()
        };
        println!("PINFLATE_CHILD_RESULT={result}:{reason}");
    }
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert_eq!(value.len() & 1, 0);
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}
