use flate2::Compression;
use flate2::write::DeflateEncoder;
use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard};

type Inflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type Unfilter = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

const C_LIBRARY: &str = "c_src/build/libtranslated_rust.so";
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Api {
    library: Library,
    inflate: Inflate,
    unfilter: Unfilter,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let inflate = unsafe { *library.get::<Inflate>(b"cp_inflate\0").unwrap() };
        let unfilter = unsafe { *library.get::<Unfilter>(b"unfilter\0").unwrap() };
        Self {
            library,
            inflate,
            unfilter,
        }
    }

    unsafe fn data(&self, name: &[u8], len: usize) -> Vec<u8> {
        let symbol = unsafe { self.library.get::<*const u8>(name).unwrap() };
        unsafe { std::slice::from_raw_parts(*symbol, len) }.to_vec()
    }

    unsafe fn error_reason(&self) -> Option<String> {
        let symbol = unsafe {
            self.library
                .get::<*const *const c_char>(b"cp_error_reason\0")
                .unwrap()
        };
        let reason = unsafe { **symbol };
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

    unsafe fn set_table_byte(&self, name: &[u8], index: usize, value: u8) {
        let symbol = unsafe { self.library.get::<*mut u8>(name).unwrap() };
        unsafe { *(*symbol).add(index) = value };
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join(C_LIBRARY)
}

fn rust_library_path() -> PathBuf {
    let target = manifest_dir().join("target");
    let release = target.join("release/libunfilter_lib.so");
    let source = manifest_dir().join("src/lib.rs");
    let needs_build = !release.exists()
        || source.metadata().unwrap().modified().unwrap()
            > release.metadata().unwrap().modified().unwrap();
    if needs_build {
        let status = Command::new(env!("CARGO"))
            .args(["build", "--release", "--no-default-features"])
            .current_dir(manifest_dir())
            .status()
            .expect("failed to launch cargo build for the Rust cdylib");
        assert!(status.success(), "cargo build for the Rust cdylib failed");
    }
    assert!(
        release.exists(),
        "cargo build did not create {}",
        release.display()
    );
    release
}

fn apis() -> (Api, Api) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.exists(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.exists(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
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

    fn usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            self.next_u32() as usize % upper
        }
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next_u32() as u8;
        }
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
            if self.bit / 8 == self.bytes.len() {
                self.bytes.push(0);
            }
            self.bytes[self.bit / 8] |= (((value >> index) & 1) as u8) << (self.bit & 7);
            self.bit += 1;
        }
    }

    fn align_byte(&mut self) {
        self.bit = (self.bit + 7) & !7;
        self.bytes.resize(self.bit / 8, 0);
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

fn reverse_code(code: u32, len: usize) -> u32 {
    code.reverse_bits() >> (32 - len)
}

fn fixed_code(symbol: u16) -> (u32, usize) {
    match symbol {
        0..=143 => (0x30 + u32::from(symbol), 8),
        144..=255 => (0x190 + u32::from(symbol - 144), 9),
        256..=279 => (u32::from(symbol - 256), 7),
        280..=287 => (0xc0 + u32::from(symbol - 280), 8),
        _ => panic!("invalid fixed symbol {symbol}"),
    }
}

fn write_fixed_symbol(writer: &mut BitWriter, symbol: u16) {
    let (code, len) = fixed_code(symbol);
    writer.write(reverse_code(code, len), len);
}

fn append_fixed_literals(writer: &mut BitWriter, final_block: bool, data: &[u8]) {
    writer.write(u32::from(final_block), 1);
    writer.write(1, 2);
    for &byte in data {
        write_fixed_symbol(writer, u16::from(byte));
    }
    write_fixed_symbol(writer, 256);
}

fn fixed_literals(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    append_fixed_literals(&mut writer, true, data);
    writer.into_bytes()
}

const LENGTH_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [usize; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [usize; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

fn write_fixed_match(writer: &mut BitWriter, length: usize, distance: usize) {
    let length_index = LENGTH_BASE
        .iter()
        .enumerate()
        .find(|(index, base)| {
            let max = **base + ((1usize << LENGTH_EXTRA[*index]) - 1);
            length >= **base && length <= max
        })
        .map(|(index, _)| index)
        .unwrap();
    write_fixed_symbol(writer, 257 + length_index as u16);
    writer.write(
        (length - LENGTH_BASE[length_index]) as u32,
        LENGTH_EXTRA[length_index],
    );

    let distance_index = DIST_BASE
        .iter()
        .enumerate()
        .find(|(index, base)| {
            let max = **base + ((1usize << DIST_EXTRA[*index]) - 1);
            distance >= **base && distance <= max
        })
        .map(|(index, _)| index)
        .unwrap();
    writer.write(reverse_code(distance_index as u32, 5), 5);
    writer.write(
        (distance - DIST_BASE[distance_index]) as u32,
        DIST_EXTRA[distance_index],
    );
}

fn fixed_match(prefix: &[u8], length: usize, distance: usize) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.write(1, 1);
    writer.write(1, 2);
    for &byte in prefix {
        write_fixed_symbol(&mut writer, u16::from(byte));
    }
    write_fixed_match(&mut writer, length, distance);
    write_fixed_symbol(&mut writer, 256);
    writer.into_bytes()
}

fn stored(data: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.write(1, 1);
    writer.write(0, 2);
    writer.align_byte();
    let len = u16::try_from(data.len()).unwrap();
    writer.bytes.extend_from_slice(&len.to_le_bytes());
    writer.bytes.extend_from_slice(&(!len).to_le_bytes());
    writer.bytes.extend_from_slice(data);
    writer.into_bytes()
}

fn dynamic(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
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
            value |= u32::from((self.bytes[self.bit / 8] >> (self.bit & 7)) & 1) << index;
            self.bit += 1;
        }
        Some(value)
    }
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, usize, u16)> {
    let mut counts = [0_u32; 16];
    for &len in lengths {
        counts[len as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0_u32; 16];
    let mut code = 0;
    for bits in 1..=15 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    let mut result = Vec::new();
    for (symbol, &len) in lengths.iter().enumerate() {
        if len != 0 {
            let code = next[len as usize];
            next[len as usize] += 1;
            result.push((
                reverse_code(code, len as usize),
                len as usize,
                symbol as u16,
            ));
        }
    }
    result
}

fn decode_symbol(reader: &mut BitReader<'_>, codes: &[(u32, usize, u16)]) -> Option<u16> {
    let mut value = 0;
    for len in 1..=15 {
        value |= reader.read(1)? << (len - 1);
        if let Some((_, _, symbol)) = codes
            .iter()
            .find(|(code, code_len, _)| *code_len == len && *code == value)
        {
            return Some(*symbol);
        }
    }
    None
}

#[derive(Debug)]
struct DynamicHeader {
    repeats: [bool; 3],
    literal_lengths: Vec<u8>,
    distance_lengths: Vec<u8>,
    has_literal: bool,
    has_match: bool,
}

fn dynamic_header(stream: &[u8]) -> Option<DynamicHeader> {
    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut reader = BitReader::new(stream);
    reader.read(1)?;
    if reader.read(2)? != 2 {
        return None;
    }
    let literal_count = reader.read(5)? as usize + 257;
    let distance_count = reader.read(5)? as usize + 1;
    let code_length_count = reader.read(4)? as usize + 4;
    let mut code_lengths = [0_u8; 19];
    for &symbol in &ORDER[..code_length_count] {
        code_lengths[symbol] = reader.read(3)? as u8;
    }
    let codes = canonical_codes(&code_lengths);
    let total = literal_count + distance_count;
    let mut lengths = Vec::with_capacity(total);
    let mut repeats = [false; 3];
    while lengths.len() < total {
        match decode_symbol(&mut reader, &codes)? {
            symbol @ 0..=15 => lengths.push(symbol as u8),
            16 => {
                repeats[0] = true;
                let previous = *lengths.last()?;
                let count = reader.read(2)? as usize + 3;
                lengths.extend(std::iter::repeat_n(previous, count));
            }
            17 => {
                repeats[1] = true;
                let count = reader.read(3)? as usize + 3;
                lengths.extend(std::iter::repeat_n(0, count));
            }
            18 => {
                repeats[2] = true;
                let count = reader.read(7)? as usize + 11;
                lengths.extend(std::iter::repeat_n(0, count));
            }
            _ => return None,
        }
    }
    if lengths.len() != total {
        return None;
    }
    let literal_lengths = lengths[..literal_count].to_vec();
    let distance_lengths = lengths[literal_count..].to_vec();
    let literal_codes = canonical_codes(&literal_lengths);
    let distance_codes = canonical_codes(&distance_lengths);
    let mut has_literal = false;
    let mut has_match = false;
    loop {
        let symbol = decode_symbol(&mut reader, &literal_codes)?;
        match symbol {
            0..=255 => has_literal = true,
            256 => break,
            257..=285 => {
                has_match = true;
                let length_index = symbol as usize - 257;
                reader.read(LENGTH_EXTRA[length_index])?;
                let distance_symbol = decode_symbol(&mut reader, &distance_codes)? as usize;
                if distance_symbol >= DIST_EXTRA.len() {
                    return None;
                }
                reader.read(DIST_EXTRA[distance_symbol])?;
            }
            _ => return None,
        }
    }
    Some(DynamicHeader {
        repeats,
        literal_lengths,
        distance_lengths,
        has_literal,
        has_match,
    })
}

fn call_inflate(
    api: &Api,
    stream: &[u8],
    output_capacity: usize,
    address_modulo_four: usize,
) -> (c_int, Vec<u8>, Option<String>) {
    let mut input = vec![0_u8; stream.len() + 8];
    let base = input.as_ptr() as usize;
    let offset = (0..4)
        .find(|offset| (base + offset) & 3 == address_modulo_four)
        .unwrap();
    input[offset..offset + stream.len()].copy_from_slice(stream);
    let mut output = vec![0xa5_u8; output_capacity];
    let status = unsafe {
        (api.inflate)(
            input.as_mut_ptr().add(offset).cast(),
            stream.len() as c_int,
            output.as_mut_ptr().cast(),
            output_capacity as c_int,
        )
    };
    let reason = unsafe { api.error_reason() };
    (status, output, reason)
}

fn compare_inflate(
    c: &Api,
    rust: &Api,
    stream: &[u8],
    output_capacity: usize,
    address_modulo_four: usize,
) -> (c_int, Vec<u8>, Option<String>) {
    let c_result = call_inflate(c, stream, output_capacity, address_modulo_four);
    let rust_result = call_inflate(rust, stream, output_capacity, address_modulo_four);
    assert_eq!(rust_result, c_result, "stream={stream:02x?}");
    c_result
}

fn compare_unfilter(c: &Api, rust: &Api, w: i32, h: i32, bpp: i32, input: &[u8]) {
    let mut c_bytes = input.to_vec();
    let mut rust_bytes = input.to_vec();
    let c_status = unsafe { (c.unfilter)(w, h, bpp, c_bytes.as_mut_ptr()) };
    let rust_status = unsafe { (rust.unfilter)(w, h, bpp, rust_bytes.as_mut_ptr()) };
    assert_eq!(
        rust_status, c_status,
        "status mismatch w={w} h={h} bpp={bpp}"
    );
    assert_eq!(rust_bytes, c_bytes, "buffer mismatch w={w} h={h} bpp={bpp}");
}

fn raw_rows(w: usize, h: usize, bpp: usize, filters: &[u8], rng: &mut Rng) -> Vec<u8> {
    assert_eq!(filters.len(), h);
    let row_len = w * bpp;
    let trailing_boundary_space = usize::from(w == 0) * bpp;
    let mut bytes = vec![0_u8; h * (row_len + 1) + trailing_boundary_space];
    for y in 0..h {
        bytes[y * (row_len + 1)] = filters[y];
        rng.fill(&mut bytes[y * (row_len + 1) + 1..(y + 1) * (row_len + 1)]);
    }
    bytes
}

#[test]
fn exported_data_matches() {
    let _guard = test_lock();
    let (c, rust) = apis();
    let symbols: &[(&[u8], usize)] = &[
        (b"cp_fixed_table\0", 320),
        (b"cp_permutation_order\0", 19),
        (b"cp_len_extra_bits\0", 31),
        (b"cp_len_base\0", 31 * 4),
        (b"cp_dist_extra_bits\0", 32),
        (b"cp_dist_base\0", 32 * 4),
    ];
    for &(name, len) in symbols {
        let c_bytes = unsafe { c.data(name, len) };
        let rust_bytes = unsafe { rust.data(name, len) };
        assert_eq!(rust_bytes, c_bytes, "data symbol {name:?}");
    }
    assert_eq!(unsafe { c.error_reason() }, None);
    assert_eq!(unsafe { rust.error_reason() }, None);
}

#[test]
fn unfilter_valid_configuration_matrix() {
    let _guard = test_lock();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x6f0d_93a4_b871_2c55);

    for h in [0, -1, -17] {
        for _ in 0..32 {
            let mut bytes = vec![0_u8; 32];
            rng.fill(&mut bytes);
            compare_unfilter(&c, &rust, 7, h, 4, &bytes);
        }
    }

    for filter in 0..=4 {
        for bpp in [1, 2, 4] {
            for w in [0, 1, 2, 9] {
                for _ in 0..32 {
                    let bytes = raw_rows(w, 1, bpp, &[filter], &mut rng);
                    compare_unfilter(&c, &rust, w as i32, 1, bpp as i32, &bytes);
                }
            }
        }
    }

    for first_filter in 0..=4 {
        for later_filter in 0..=4 {
            for bpp in [1, 2, 4] {
                for w in [0, 1, 2, 9] {
                    for _ in 0..16 {
                        let bytes = raw_rows(
                            w,
                            3,
                            bpp,
                            &[first_filter, later_filter, later_filter],
                            &mut rng,
                        );
                        compare_unfilter(&c, &rust, w as i32, 3, bpp as i32, &bytes);
                    }
                }
            }
        }
    }
}

#[test]
fn inflate_stored_randomized() {
    let _guard = test_lock();
    let (c, rust) = apis();
    let mut rng = Rng::new(0xd818_b681_f763_7a21);
    for iteration in 0..192 {
        let len = match iteration {
            0 => 0,
            1 => 1,
            _ => rng.usize(2048),
        };
        let mut payload = vec![0_u8; len];
        rng.fill(&mut payload);
        let stream = stored(&payload);
        for extra in [0, rng.usize(33)] {
            let (status, output, _) =
                compare_inflate(&c, &rust, &stream, payload.len() + extra, iteration & 3);
            assert_eq!(status, 1);
            assert_eq!(output.len(), payload.len() + extra);
        }
    }
}

#[test]
fn inflate_fixed_literals_and_matches_randomized() {
    let _guard = test_lock();
    let (c, rust) = apis();
    let mut rng = Rng::new(0xc107_53b2_99d4_e68f);

    for iteration in 0..256 {
        let len = match iteration {
            0 => 0,
            1 => 1,
            _ => rng.usize(768),
        };
        let mut payload = vec![0_u8; len];
        rng.fill(&mut payload);
        let stream = fixed_literals(&payload);
        let (status, output, _) = compare_inflate(&c, &rust, &stream, payload.len(), iteration & 3);
        assert_eq!(status, 1);
        assert_eq!(output, payload);
    }

    for length in [3, 4, 7, 10, 18, 34, 66, 130, 258] {
        let stream = fixed_match(b"z", length, 1);
        let expected = vec![b'z'; length + 1];
        let (status, output, _) = compare_inflate(&c, &rust, &stream, expected.len(), length & 3);
        assert_eq!(status, 1);
        assert_eq!(output, expected);
    }

    for &(prefix, length, distance) in &[
        (&b"abc"[..], 3, 3),
        (&b"abcdef"[..], 12, 6),
        (&b"0123456789"[..], 30, 10),
    ] {
        let stream = fixed_match(prefix, length, distance);
        let mut expected = prefix.to_vec();
        for index in 0..length {
            expected.push(expected[expected.len() - distance]);
            assert_eq!(expected.len(), prefix.len() + index + 1);
        }
        let (status, output, _) = compare_inflate(&c, &rust, &stream, expected.len(), distance & 3);
        assert_eq!(status, 1);
        assert_eq!(output, expected);
    }
}

#[test]
fn inflate_dynamic_randomized_and_header_forms() {
    let _guard = test_lock();
    let (c, rust) = apis();
    let mut rng = Rng::new(0x9254_07ad_3e61_f8c2);
    let mut observed_repeats = [false; 3];
    let mut observed_literal = false;
    let mut observed_match = false;
    let mut dynamic_streams = 0;

    for iteration in 0..800 {
        let len = 300 + rng.usize(5000);
        let alphabet = 2 + iteration % 91;
        let mut payload = vec![0_u8; len];
        for (index, byte) in payload.iter_mut().enumerate() {
            let random = rng.next_u32() as usize;
            *byte = ((random + index * (iteration + 1)) % alphabet) as u8;
        }
        let stream = dynamic(&payload);
        let Some(header) = dynamic_header(&stream) else {
            continue;
        };
        dynamic_streams += 1;
        for (seen, current) in observed_repeats.iter_mut().zip(header.repeats) {
            *seen |= current;
        }
        observed_literal |= header.has_literal;
        observed_match |= header.has_match;
        assert!(!header.literal_lengths.is_empty());
        assert!(!header.distance_lengths.is_empty());

        let extra = iteration % 17;
        let (status, output, _) =
            compare_inflate(&c, &rust, &stream, payload.len() + extra, iteration & 3);
        assert_eq!(status, 1);
        assert_eq!(&output[..payload.len()], payload);

        if observed_repeats == [true; 3]
            && observed_literal
            && observed_match
            && dynamic_streams >= 64
        {
            break;
        }
    }

    assert!(
        dynamic_streams >= 64,
        "only found {dynamic_streams} dynamic streams"
    );
    assert_eq!(observed_repeats, [true; 3]);
    assert!(observed_literal);
    assert!(observed_match);
}

#[test]
fn inflate_multiblock_alignment_and_tail_matrix() {
    let _guard = test_lock();
    let (c, rust) = apis();

    let mut writer = BitWriter::default();
    append_fixed_literals(&mut writer, false, b"first:");
    append_fixed_literals(&mut writer, true, b"second");
    let stream = writer.into_bytes();
    let (status, output, _) = compare_inflate(&c, &rust, &stream, 12, 0);
    assert_eq!(status, 1);
    assert_eq!(output, b"first:second");

    let mut covered = [[false; 4]; 4];
    for address_modulo in 0..4 {
        let first_bytes = (4 - address_modulo) & 3;
        for len in 4..512 {
            let payload = vec![b'a' + (len % 23) as u8; len];
            let candidate = fixed_literals(&payload);
            let tail = (candidate.len() - first_bytes) & 3;
            if !covered[address_modulo][tail] {
                let (status, output, _) =
                    compare_inflate(&c, &rust, &candidate, payload.len(), address_modulo);
                assert_eq!(status, 1);
                assert_eq!(output, payload);
                covered[address_modulo][tail] = true;
            }
            if covered[address_modulo] == [true; 4] {
                break;
            }
        }
    }
    assert_eq!(covered, [[true; 4]; 4]);
}

#[test]
fn graceful_error_results_match() {
    let _guard = test_lock();
    let (c, rust) = apis();

    let cases: Vec<(Vec<u8>, usize, &str)> = vec![
        (
            vec![1, 1, 0, 0, 0],
            1,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        ),
        (
            vec![1, 0, 0, 0xff, 0xff, 0x42],
            1,
            "Stored block extends beyond end of input stream.",
        ),
        (
            fixed_literals(b"x"),
            0,
            "Attempted to overwrite out buffer while outputting a symbol.",
        ),
        (
            fixed_match(b"", 3, 1),
            3,
            "Attempted to write before out buffer (invalid backwards distance).",
        ),
        (
            fixed_match(b"a", 3, 1),
            1,
            "Attempted to overwrite out buffer while outputting a string.",
        ),
        (
            vec![7],
            0,
            "Detected unknown block type within input stream.",
        ),
    ];
    for (stream, capacity, reason) in cases {
        let (status, _, actual_reason) = compare_inflate(&c, &rust, &stream, capacity, 0);
        assert_eq!(status, 0);
        assert_eq!(actual_reason.as_deref(), Some(reason));
    }

    let mut rng = Rng::new(0xf8e7_64b2_190c_a35d);
    for bad_filter in [5, 6, 127, 255] {
        for _ in 0..64 {
            let first = raw_rows(7, 1, 4, &[bad_filter], &mut rng);
            let mut c_first = first.clone();
            let mut rust_first = first;
            assert_eq!(unsafe { (c.unfilter)(7, 1, 4, c_first.as_mut_ptr()) }, 0);
            assert_eq!(
                unsafe { (rust.unfilter)(7, 1, 4, rust_first.as_mut_ptr()) },
                0
            );
            assert_eq!(rust_first, c_first);

            let later = raw_rows(7, 2, 4, &[0, bad_filter], &mut rng);
            let mut c_later = later.clone();
            let mut rust_later = later;
            assert_eq!(unsafe { (c.unfilter)(7, 2, 4, c_later.as_mut_ptr()) }, 0);
            assert_eq!(
                unsafe { (rust.unfilter)(7, 2, 4, rust_later.as_mut_ptr()) },
                0
            );
            assert_eq!(rust_later, c_later);
        }
    }

    let literal = fixed_literals(b"x");
    let c_null = unsafe {
        (c.inflate)(
            literal.as_ptr().cast_mut().cast(),
            literal.len() as c_int,
            std::ptr::null_mut(),
            0,
        )
    };
    let rust_null = unsafe {
        (rust.inflate)(
            literal.as_ptr().cast_mut().cast(),
            literal.len() as c_int,
            std::ptr::null_mut(),
            0,
        )
    };
    assert_eq!(rust_null, c_null);
    assert_eq!(c_null, 0);

    let c_empty = unsafe { (c.unfilter)(0, 0, 0, std::ptr::null_mut()) };
    let rust_empty = unsafe { (rust.unfilter)(0, 0, 0, std::ptr::null_mut()) };
    assert_eq!(rust_empty, c_empty);
    assert_eq!(c_empty, 1);
}

fn probe_status(library: &Path, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_probe", "--nocapture"])
        .env("DIFFERENTIAL_CRASH_LIBRARY", library)
        .env("DIFFERENTIAL_CRASH_CASE", case)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|error| panic!("failed to run crash probe {case}: {error}"))
}

#[test]
fn crash_probe() {
    let Ok(case) = std::env::var("DIFFERENTIAL_CRASH_CASE") else {
        return;
    };
    let library = PathBuf::from(std::env::var_os("DIFFERENTIAL_CRASH_LIBRARY").unwrap());
    let api = unsafe { Api::load(&library) };
    match case.as_str() {
        "zero_input" => {
            let mut byte = 0_u8;
            let mut output = 0_u8;
            unsafe {
                (api.inflate)(
                    (&mut byte as *mut u8).cast(),
                    0,
                    (&mut output as *mut u8).cast(),
                    1,
                );
            }
        }
        "negative_input_length" => {
            let mut byte = 0_u8;
            let mut output = 0_u8;
            unsafe {
                (api.inflate)(
                    (&mut byte as *mut u8).cast(),
                    -1,
                    (&mut output as *mut u8).cast(),
                    1,
                );
            }
        }
        "null_input" => unsafe {
            (api.inflate)(std::ptr::null_mut(), 8, std::ptr::null_mut(), 0);
        },
        "null_unfilter" => unsafe {
            (api.unfilter)(1, 1, 1, std::ptr::null_mut());
        },
        "oversized_unfilter" => {
            let mut bytes = vec![1_u8; 4096];
            unsafe {
                (api.unfilter)(c_int::MAX, 1, 1, bytes.as_mut_ptr());
            }
        }
        "fixed_length_16" => {
            unsafe { api.set_table_byte(b"cp_fixed_table\0", 0, 16) };
            let mut stream = fixed_literals(b"a");
            let mut output = [0_u8; 1];
            unsafe {
                (api.inflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
        }
        "read_more_than_32" => {
            unsafe { api.set_table_byte(b"cp_len_extra_bits\0", 0, 33) };
            let mut stream = fixed_match(b"", 3, 1);
            let mut output = [0_u8; 3];
            unsafe {
                (api.inflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
        }
        "read_overflow" => {
            unsafe { api.set_table_byte(b"cp_len_extra_bits\0", 0, 13) };
            let mut stream = fixed_match(b"", 3, 1);
            stream.truncate(2);
            let mut output = [0_u8; 3];
            unsafe {
                (api.inflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
        }
        "truncated_fixed" => {
            let mut input = [0x0b_u8];
            let mut output = [0_u8; 8];
            unsafe {
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
        }
        "empty_dynamic_tree" => {
            let mut input = [0x05_u8, 0, 0, 0, 0, 0, 0, 0];
            let mut output = [0_u8; 8];
            unsafe {
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
        }
        unknown => panic!("unknown crash probe {unknown}"),
    }
}

#[test]
#[cfg(unix)]
fn process_boundary_failures_match() {
    let _guard = test_lock();
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    let cases = [
        ("zero_input", libc_signal::SIGABRT),
        ("negative_input_length", libc_signal::SIGABRT),
        ("null_input", libc_signal::SIGSEGV),
        ("null_unfilter", libc_signal::SIGSEGV),
        ("oversized_unfilter", libc_signal::SIGSEGV),
        ("fixed_length_16", libc_signal::SIGABRT),
        ("read_more_than_32", libc_signal::SIGABRT),
        ("read_overflow", libc_signal::SIGABRT),
        ("truncated_fixed", libc_signal::SIGABRT),
        ("empty_dynamic_tree", libc_signal::SIGABRT),
    ];
    for (case, expected_signal) in cases {
        let c_status = probe_status(&c_path, case);
        let rust_status = probe_status(&rust_path, case);
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "different termination for {case}: C={c_status:?}, Rust={rust_status:?}"
        );
        assert_eq!(
            c_status.signal(),
            Some(expected_signal),
            "unexpected C termination for {case}: {c_status:?}"
        );
    }
}

mod libc_signal {
    pub const SIGABRT: i32 = 6;
    pub const SIGSEGV: i32 = 11;
}

#[test]
fn c_assertion_inventory_is_complete() {
    let source = include_str!("../c_src/src/lib.c");
    let predicates = [
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
    ];
    assert_eq!(source.matches("assert(").count(), predicates.len());
    for predicate in predicates {
        assert!(source.contains(predicate), "missing assertion: {predicate}");
    }
}
