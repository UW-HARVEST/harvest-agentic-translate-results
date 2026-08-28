use flate2::Compression;
use flate2::write::DeflateEncoder;
use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::Mutex;

type InflateFn = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type UnfilterFn = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

static ABI_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    inflate: InflateFn,
    unfilter: UnfilterFn,
    error_reason: *mut *const c_char,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        unsafe {
            let library = Library::new(path).unwrap_or_else(|error| {
                panic!("failed to load {}: {error}", path.display());
            });
            let inflate = *library.get::<InflateFn>(b"cp_inflate").unwrap();
            let unfilter = *library.get::<UnfilterFn>(b"unfilter").unwrap();
            let error_reason = *library
                .get::<*mut *const c_char>(b"cp_error_reason")
                .unwrap();
            Self {
                _library: library,
                inflate,
                unfilter,
                error_reason,
            }
        }
    }

    unsafe fn error(&self) -> Option<Vec<u8>> {
        unsafe {
            let reason = *self.error_reason;
            (!reason.is_null()).then(|| CStr::from_ptr(reason).to_bytes().to_vec())
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut libraries: Vec<_> = std::fs::read_dir(&build)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();
    assert_eq!(
        libraries.len(),
        1,
        "expected one C shared object in {}",
        build.display()
    );
    libraries.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libunfilter_lib.so")
}

unsafe fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
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

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn bits(&mut self, mut value: u32, count: u8) {
        for _ in 0..count {
            self.current |= ((value & 1) as u8) << self.used;
            self.used += 1;
            value >>= 1;
            if self.used == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
    }

    fn align(&mut self) {
        if self.used != 0 {
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn bytes(&mut self, bytes: &[u8]) {
        assert_eq!(self.used, 0);
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

fn reverse(mut value: u32, count: u8) -> u32 {
    let mut result = 0;
    for _ in 0..count {
        result = (result << 1) | (value & 1);
        value >>= 1;
    }
    result
}

fn canonical_codes(lengths: &[u8]) -> Vec<(u32, u8)> {
    let mut counts = [0u32; 16];
    for &length in lengths {
        counts[length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0u32; 16];
    for length in 1..=15 {
        next[length] = (next[length - 1] + counts[length - 1]) << 1;
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

fn symbol(writer: &mut BitWriter, codes: &[(u32, u8)], value: usize) {
    let (code, length) = codes[value];
    assert_ne!(length, 0, "symbol {value} has no code");
    writer.bits(code, length);
}

fn stored_blocks(blocks: &[&[u8]]) -> Vec<u8> {
    let mut writer = BitWriter::default();
    for (index, payload) in blocks.iter().enumerate() {
        writer.bits(u32::from(index + 1 == blocks.len()), 1);
        writer.bits(0, 2);
        writer.align();
        let length = u16::try_from(payload.len()).unwrap();
        writer.bytes(&length.to_le_bytes());
        writer.bytes(&(!length).to_le_bytes());
        writer.bytes(payload);
    }
    writer.finish()
}

fn fixed_lengths() -> Vec<u8> {
    let mut lengths = vec![0; 288];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..].fill(8);
    lengths
}

fn fixed_stream(literals: &[u8], backref_distance_symbol: Option<usize>) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(1, 2);
    let literal_codes = canonical_codes(&fixed_lengths());
    let distance_codes = canonical_codes(&vec![5; 32]);
    for &literal in literals {
        symbol(&mut writer, &literal_codes, literal as usize);
    }
    if let Some(distance_symbol) = backref_distance_symbol {
        symbol(&mut writer, &literal_codes, 257);
        symbol(&mut writer, &distance_codes, distance_symbol);
    }
    symbol(&mut writer, &literal_codes, 256);
    writer.finish()
}

#[derive(Clone, Copy)]
enum ZeroRepeat {
    Seventeen,
    Eighteen,
}

fn write_code_lengths(
    writer: &mut BitWriter,
    lengths: &[u8],
    codes: &[(u32, u8)],
    zero_repeat: ZeroRepeat,
    repeat_nonzero: bool,
) {
    let mut index = 0;
    while index < lengths.len() {
        let value = lengths[index];
        let mut run = 1;
        while index + run < lengths.len() && lengths[index + run] == value {
            run += 1;
        }
        if value == 0 {
            let mut remaining = run;
            while remaining != 0 {
                match zero_repeat {
                    ZeroRepeat::Eighteen if remaining >= 11 => {
                        let count = remaining.min(138);
                        symbol(writer, codes, 18);
                        writer.bits((count - 11) as u32, 7);
                        remaining -= count;
                    }
                    _ if remaining >= 3 => {
                        let count = remaining.min(10);
                        symbol(writer, codes, 17);
                        writer.bits((count - 3) as u32, 3);
                        remaining -= count;
                    }
                    _ => {
                        symbol(writer, codes, 0);
                        remaining -= 1;
                    }
                }
            }
        } else {
            symbol(writer, codes, value as usize);
            let mut remaining = run - 1;
            while repeat_nonzero && remaining >= 3 {
                let count = remaining.min(6);
                symbol(writer, codes, 16);
                writer.bits((count - 3) as u32, 2);
                remaining -= count;
            }
            for _ in 0..remaining {
                symbol(writer, codes, value as usize);
            }
        }
        index += run;
    }
}

#[derive(Clone, Copy)]
enum DynamicToken {
    Literal(u8),
    Length3Distance(usize),
}

fn dynamic_stream(
    literal_lengths: &[(usize, u8)],
    distance_count: usize,
    distance_lengths: &[(usize, u8)],
    tokens: &[DynamicToken],
    zero_repeat: ZeroRepeat,
    repeat_nonzero: bool,
) -> (Vec<u8>, Vec<u8>) {
    let highest_literal = literal_lengths
        .iter()
        .map(|&(symbol, _)| symbol)
        .chain(std::iter::once(256))
        .max()
        .unwrap();
    let literal_count = highest_literal.max(256) + 1;
    assert!((257..=286).contains(&literal_count));
    assert!((1..=32).contains(&distance_count));

    let mut literal_tree = vec![0; literal_count];
    for &(value, length) in literal_lengths {
        literal_tree[value] = length;
    }
    let mut distance_tree = vec![0; distance_count];
    for &(value, length) in distance_lengths {
        distance_tree[value] = length;
    }

    let mut writer = BitWriter::default();
    writer.bits(1, 1);
    writer.bits(2, 2);
    writer.bits((literal_count - 257) as u32, 5);
    writer.bits((distance_count - 1) as u32, 5);

    const ORDER: [usize; 19] = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let code_length_count = 18;
    writer.bits((code_length_count - 4) as u32, 4);
    let mut code_length_tree = vec![0; 19];
    for value in [0, 1, 2, 3, 16, 17, 18] {
        code_length_tree[value] = 3;
    }
    for &value in &ORDER[..code_length_count] {
        writer.bits(code_length_tree[value] as u32, 3);
    }
    let code_length_codes = canonical_codes(&code_length_tree);
    let all_lengths: Vec<_> = literal_tree
        .iter()
        .chain(distance_tree.iter())
        .copied()
        .collect();
    write_code_lengths(
        &mut writer,
        &all_lengths,
        &code_length_codes,
        zero_repeat,
        repeat_nonzero,
    );

    let literal_codes = canonical_codes(&literal_tree);
    let distance_codes = canonical_codes(&distance_tree);
    let mut expected = Vec::new();
    for &token in tokens {
        match token {
            DynamicToken::Literal(value) => {
                symbol(&mut writer, &literal_codes, value as usize);
                expected.push(value);
            }
            DynamicToken::Length3Distance(distance_symbol) => {
                symbol(&mut writer, &literal_codes, 257);
                symbol(&mut writer, &distance_codes, distance_symbol);
                let distance = [1, 2, 3, 4][distance_symbol];
                for _ in 0..3 {
                    expected.push(expected[expected.len() - distance]);
                }
            }
        }
    }
    symbol(&mut writer, &literal_codes, 256);
    (writer.finish(), expected)
}

fn compress(data: &[u8], compression: Compression) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), compression);
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

unsafe fn compare_inflate(
    c: &Api,
    rust: &Api,
    input: &[u8],
    output_capacity: usize,
) -> (c_int, Vec<u8>) {
    let mut c_input = input.to_vec();
    let mut rust_input = input.to_vec();
    let mut c_output = vec![0xa5; output_capacity.max(1)];
    let mut rust_output = vec![0xa5; output_capacity.max(1)];
    unsafe {
        *c.error_reason = std::ptr::null();
        *rust.error_reason = std::ptr::null();
        let c_result = (c.inflate)(
            c_input.as_mut_ptr().cast(),
            c_int::try_from(input.len()).unwrap(),
            c_output.as_mut_ptr().cast(),
            c_int::try_from(output_capacity).unwrap(),
        );
        let rust_result = (rust.inflate)(
            rust_input.as_mut_ptr().cast(),
            c_int::try_from(input.len()).unwrap(),
            rust_output.as_mut_ptr().cast(),
            c_int::try_from(output_capacity).unwrap(),
        );
        assert_eq!(
            rust_result, c_result,
            "return mismatch for input {input:02x?}"
        );
        assert_eq!(
            rust_output, c_output,
            "output mismatch for input {input:02x?}"
        );
        assert_eq!(
            rust.error(),
            c.error(),
            "error reason mismatch for input {input:02x?}"
        );
        (c_result, c_output)
    }
}

fn positioned_input(
    stream: &[u8],
    first_bytes: usize,
    last_bytes: usize,
) -> (Vec<u8>, usize, usize) {
    assert!(first_bytes < 4);
    assert!(last_bytes < 4);
    let mut storage = vec![0u8; stream.len() + 16];
    let offset = (0..4)
        .find(|&offset| {
            let address = storage.as_ptr() as usize + offset;
            ((address + 3) & !3) - address == first_bytes
        })
        .unwrap();
    let trailing = (last_bytes + 4 - ((stream.len() - first_bytes) & 3)) & 3;
    let input_length = stream.len() + trailing;
    storage[offset..offset + stream.len()].copy_from_slice(stream);
    for index in stream.len()..input_length {
        storage[offset + index] = 0xa5;
    }
    (storage, offset, input_length)
}

unsafe fn compare_positioned_inflate(
    c: &Api,
    rust: &Api,
    stream: &[u8],
    first_bytes: usize,
    last_bytes: usize,
    output_capacity: usize,
) {
    let (mut c_storage, c_offset, c_length) = positioned_input(stream, first_bytes, last_bytes);
    let (mut rust_storage, rust_offset, rust_length) =
        positioned_input(stream, first_bytes, last_bytes);
    assert_eq!(c_length, rust_length);
    let mut c_output = vec![0xa5; output_capacity.max(1)];
    let mut rust_output = vec![0xa5; output_capacity.max(1)];
    unsafe {
        let c_result = (c.inflate)(
            c_storage.as_mut_ptr().add(c_offset).cast(),
            c_length as c_int,
            c_output.as_mut_ptr().cast(),
            output_capacity as c_int,
        );
        let rust_result = (rust.inflate)(
            rust_storage.as_mut_ptr().add(rust_offset).cast(),
            rust_length as c_int,
            rust_output.as_mut_ptr().cast(),
            output_capacity as c_int,
        );
        assert_eq!(rust_result, c_result);
        assert_eq!(rust_output, c_output);
    }
}

unsafe fn compare_unfilter(c: &Api, rust: &Api, w: c_int, h: c_int, bpp: c_int, raw: &[u8]) {
    let mut c_raw = raw.to_vec();
    let mut rust_raw = raw.to_vec();
    unsafe {
        let c_result = (c.unfilter)(w, h, bpp, c_raw.as_mut_ptr());
        let rust_result = (rust.unfilter)(w, h, bpp, rust_raw.as_mut_ptr());
        assert_eq!(
            rust_result, c_result,
            "return mismatch: w={w}, h={h}, bpp={bpp}"
        );
        assert_eq!(rust_raw, c_raw, "buffer mismatch: w={w}, h={h}, bpp={bpp}");
    }
}

fn filtered_bytes(rng: &mut Rng, w: usize, h: usize, bpp: usize, filters: &[u8]) -> Vec<u8> {
    let row_bytes = w * bpp;
    let mut raw = Vec::with_capacity(h * (row_bytes + 1));
    for row in 0..h {
        raw.push(filters[row]);
        raw.extend((0..row_bytes).map(|_| rng.byte()));
    }
    raw
}

#[test]
fn exported_data_symbols_match() {
    let _guard = ABI_LOCK.lock().unwrap();
    unsafe {
        let c = Library::new(c_library_path()).unwrap();
        let rust = Library::new(rust_library_path()).unwrap();
        for (name, size) in [
            (&b"cp_fixed_table"[..], 320usize),
            (&b"cp_permutation_order"[..], 19),
            (&b"cp_len_extra_bits"[..], 31),
            (&b"cp_len_base"[..], 31 * 4),
            (&b"cp_dist_extra_bits"[..], 32),
            (&b"cp_dist_base"[..], 32 * 4),
        ] {
            let c_symbol: Symbol<*const u8> = c.get(name).unwrap();
            let rust_symbol: Symbol<*const u8> = rust.get(name).unwrap();
            assert_eq!(
                std::slice::from_raw_parts(*rust_symbol, size),
                std::slice::from_raw_parts(*c_symbol, size),
                "data symbol {} differs",
                String::from_utf8_lossy(name)
            );
        }
    }
}

#[test]
fn unfilter_configuration_matrix() {
    let _guard = ABI_LOCK.lock().unwrap();
    let mut rng = Rng::new(0x8a5c_91e2_d4b7_603f);
    unsafe {
        let (c, rust) = load_apis();

        for h in [-3, -1, 0] {
            compare_unfilter(&c, &rust, 7, h, 4, &[0x5a]);
        }

        for &w in &[1usize, 2, 3, 9, 31] {
            for bpp in 1..=8 {
                for first_filter in 0..=4 {
                    for _ in 0..64 {
                        let raw = filtered_bytes(&mut rng, w, 1, bpp, &[first_filter]);
                        compare_unfilter(&c, &rust, w as c_int, 1, bpp as c_int, &raw);
                    }
                }
            }
        }

        for &w in &[1usize, 2, 3, 11] {
            for bpp in 1..=8 {
                for first_filter in 0..=4 {
                    for later_filter in 0..=4 {
                        for _ in 0..64 {
                            let raw =
                                filtered_bytes(&mut rng, w, 2, bpp, &[first_filter, later_filter]);
                            compare_unfilter(&c, &rust, w as c_int, 2, bpp as c_int, &raw);
                        }
                    }
                }
            }
        }

        for &w in &[1usize, 2, 7] {
            for bpp in 1..=8 {
                for _ in 0..1_000 {
                    let h = 3 + rng.usize(8);
                    let filters: Vec<_> = (0..h).map(|_| rng.usize(5) as u8).collect();
                    let raw = filtered_bytes(&mut rng, w, h, bpp, &filters);
                    compare_unfilter(&c, &rust, w as c_int, h as c_int, bpp as c_int, &raw);
                }
            }
        }
    }
}

#[test]
fn inflate_stored_fixed_and_multiblock_configurations() {
    let _guard = ABI_LOCK.lock().unwrap();
    let mut rng = Rng::new(0x184f_2ca9_e037_65bd);
    unsafe {
        let (c, rust) = load_apis();

        let empty = stored_blocks(&[b""]);
        assert_eq!(compare_inflate(&c, &rust, &empty, 0).0, 1);

        for length in [1usize, 2, 3, 7, 31, 255, 1024] {
            for _ in 0..32 {
                let payload: Vec<_> = (0..length).map(|_| rng.byte()).collect();
                let stream = stored_blocks(&[&payload]);
                compare_inflate(&c, &rust, &stream, payload.len());
                compare_inflate(&c, &rust, &stream, payload.len() + 17);
            }
        }

        let fixed_empty = fixed_stream(b"", None);
        assert_eq!(compare_inflate(&c, &rust, &fixed_empty, 0).0, 1);
        for length in [1usize, 2, 17, 257] {
            for _ in 0..64 {
                let payload: Vec<_> = (0..length).map(|_| rng.byte()).collect();
                let stream = fixed_stream(&payload, None);
                let (_, output) = compare_inflate(&c, &rust, &stream, payload.len());
                assert_eq!(&output[..payload.len()], payload);
            }
        }

        for _ in 0..128 {
            let prefix: Vec<_> = (0..(1 + rng.usize(32))).map(|_| rng.byte()).collect();
            let distance_one = fixed_stream(&prefix, Some(0));
            compare_inflate(&c, &rust, &distance_one, prefix.len() + 3);

            let distance_symbol = 1 + rng.usize(3);
            let prefix: Vec<_> = (0..(distance_symbol + 1 + rng.usize(32)))
                .map(|_| rng.byte())
                .collect();
            let distance_many = fixed_stream(&prefix, Some(distance_symbol));
            compare_inflate(&c, &rust, &distance_many, prefix.len() + 3);
        }

        for _ in 0..64 {
            let first: Vec<_> = (0..(1 + rng.usize(128))).map(|_| rng.byte()).collect();
            let second: Vec<_> = (0..(1 + rng.usize(128))).map(|_| rng.byte()).collect();
            let stream = stored_blocks(&[&first, &second]);
            compare_inflate(&c, &rust, &stream, first.len() + second.len());
        }
    }
}

#[test]
fn inflate_dynamic_configurations() {
    let _guard = ABI_LOCK.lock().unwrap();
    let mut rng = Rng::new(0xc6e4_a913_5d7b_208f);
    unsafe {
        let (c, rust) = load_apis();

        for _ in 0..64 {
            let token_count = 1 + rng.usize(128);
            let tokens: Vec<_> = (0..token_count)
                .map(|_| DynamicToken::Literal(65 + rng.usize(4) as u8))
                .collect();
            let (literal_only, expected) = dynamic_stream(
                &[(65, 3), (66, 3), (67, 3), (68, 3), (256, 1)],
                1,
                &[(0, 1)],
                &tokens,
                ZeroRepeat::Eighteen,
                true,
            );
            let (_, output) = compare_inflate(&c, &rust, &literal_only, expected.len());
            assert_eq!(&output[..expected.len()], expected);

            let mut tokens = vec![DynamicToken::Literal(65)];
            tokens.extend((0..(1 + rng.usize(32))).map(|_| DynamicToken::Length3Distance(0)));
            let (distance_one, expected) = dynamic_stream(
                &[(65, 1), (256, 2), (257, 2)],
                1,
                &[(0, 1)],
                &tokens,
                ZeroRepeat::Eighteen,
                false,
            );
            let (_, output) = compare_inflate(&c, &rust, &distance_one, expected.len());
            assert_eq!(&output[..expected.len()], expected);

            let mut tokens = vec![
                DynamicToken::Literal(65),
                DynamicToken::Literal(66),
                DynamicToken::Literal(67),
            ];
            tokens.extend((0..(1 + rng.usize(32))).map(|_| DynamicToken::Length3Distance(2)));
            let (distance_three, expected) = dynamic_stream(
                &[(65, 2), (66, 2), (67, 2), (256, 3), (257, 3)],
                3,
                &[(2, 1)],
                &tokens,
                ZeroRepeat::Eighteen,
                false,
            );
            let (_, output) = compare_inflate(&c, &rust, &distance_three, expected.len());
            assert_eq!(&output[..expected.len()], expected);

            for zero_repeat in [ZeroRepeat::Seventeen, ZeroRepeat::Eighteen] {
                let tokens = vec![DynamicToken::Literal(65); 1 + rng.usize(128)];
                let (stream, expected) = dynamic_stream(
                    &[(65, 1), (256, 1)],
                    1,
                    &[(0, 1)],
                    &tokens,
                    zero_repeat,
                    false,
                );
                let (_, output) = compare_inflate(&c, &rust, &stream, expected.len());
                assert_eq!(&output[..expected.len()], expected);
            }
        }

        for length in [512usize, 1024, 4096] {
            let data: Vec<_> = (0..length).map(|index| (index % 251) as u8).collect();
            let stream = compress(&data, Compression::best());
            if (stream[0] >> 1) & 3 == 2 {
                let (_, output) = compare_inflate(&c, &rust, &stream, data.len());
                assert_eq!(&output[..data.len()], data);
            }
        }
    }
}

#[test]
fn inflate_compressor_generated_randomized_streams() {
    let _guard = ABI_LOCK.lock().unwrap();
    let mut rng = Rng::new(0x4bc9_173e_a862_d50f);
    unsafe {
        let (c, rust) = load_apis();
        for case in 0..512 {
            let length = rng.usize(4097);
            let data: Vec<_> = match case % 3 {
                0 => (0..length).map(|_| rng.byte()).collect(),
                1 => {
                    let value = rng.byte();
                    vec![value; length]
                }
                _ => {
                    let period = 1 + rng.usize(64);
                    let pattern: Vec<_> = (0..period).map(|_| rng.byte()).collect();
                    (0..length).map(|index| pattern[index % period]).collect()
                }
            };
            let compression = match case % 4 {
                0 => Compression::none(),
                1 => Compression::fast(),
                2 => Compression::default(),
                _ => Compression::best(),
            };
            let stream = compress(&data, compression);
            compare_inflate(&c, &rust, &stream, data.len() + 8);
        }
    }
}

#[test]
fn inflate_alignment_and_final_word_configurations() {
    let _guard = ABI_LOCK.lock().unwrap();
    let mut rng = Rng::new(0x712b_5ce0_9a4d_f386);
    unsafe {
        let (c, rust) = load_apis();
        for first_bytes in 0..4 {
            for last_bytes in 0..4 {
                for _ in 0..32 {
                    let payload: Vec<_> = (0..(1 + rng.usize(128))).map(|_| rng.byte()).collect();
                    let stream = fixed_stream(&payload, None);
                    for extra_capacity in [0usize, 13] {
                        compare_positioned_inflate(
                            &c,
                            &rust,
                            &stream,
                            first_bytes,
                            last_bytes,
                            payload.len() + extra_capacity,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn explicit_error_returns_match() {
    let _guard = ABI_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_apis();

        let bad_complement = [1, 0, 0, 0, 0];
        assert_eq!(compare_inflate(&c, &rust, &bad_complement, 0).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."
                    .as_slice()
            )
        );

        let stored_extra_input = [1, 0, 0, 0xff, 0xff, 0xa5];
        assert_eq!(compare_inflate(&c, &rust, &stored_extra_input, 0).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(b"Stored block extends beyond end of input stream.".as_slice())
        );

        let literal = fixed_stream(b"A", None);
        assert_eq!(compare_inflate(&c, &rust, &literal, 0).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(b"Attempted to overwrite out buffer while outputting a symbol.".as_slice())
        );

        let invalid_distance = fixed_stream(b"", Some(0));
        assert_eq!(compare_inflate(&c, &rust, &invalid_distance, 3).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(b"Attempted to write before out buffer (invalid backwards distance).".as_slice())
        );

        let string_overflow = fixed_stream(b"A", Some(0));
        assert_eq!(compare_inflate(&c, &rust, &string_overflow, 2).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(b"Attempted to overwrite out buffer while outputting a string.".as_slice())
        );

        assert_eq!(compare_inflate(&c, &rust, &[7], 0).0, 0);
        assert_eq!(
            c.error().as_deref(),
            Some(b"Detected unknown block type within input stream.".as_slice())
        );

        for filter in [5u8, 6, 127, 255] {
            compare_unfilter(&c, &rust, 1, 1, 1, &[filter, 0x5a]);
            compare_unfilter(&c, &rust, 1, 2, 1, &[0, 0x11, filter, 0x22]);
        }
    }
}

#[test]
fn generic_ffi_boundaries_match_without_undefined_pointer_access() {
    let _guard = ABI_LOCK.lock().unwrap();
    unsafe {
        let (c, rust) = load_apis();

        compare_unfilter(&c, &rust, 0, 0, 0, &[0xa5]);
        compare_unfilter(&c, &rust, 0, 1, 0, &[0]);
        compare_unfilter(&c, &rust, 0, 2, 0, &[0, 0]);
        compare_unfilter(&c, &rust, 1, 1, 1, &[0, 0xff]);
        compare_unfilter(&c, &rust, 4096, 1, 1, &{
            let mut raw = vec![0; 4097];
            raw[0] = 1;
            raw
        });

        let payload = vec![0x5a; 65_535];
        let stream = stored_blocks(&[&payload]);
        compare_inflate(&c, &rust, &stream, payload.len());
    }
}

#[cfg(unix)]
fn status_signature(status: ExitStatus) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    (status.code(), status.signal())
}

#[test]
fn ffi_process_worker() {
    let Some(case) = std::env::var_os("DIFFERENTIAL_WORKER_CASE") else {
        return;
    };
    let implementation = std::env::var("DIFFERENTIAL_WORKER_IMPL").unwrap();
    let path = if implementation == "c" {
        c_library_path()
    } else {
        rust_library_path()
    };
    unsafe {
        let api = Api::load(&path);
        let mut input = [0u8; 8];
        let mut output = [0u8; 8];
        match case.to_str().unwrap() {
            "inflate_zero_input" => {
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    0,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "inflate_negative_input" => {
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    -1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "inflate_null_input" => {
                (api.inflate)(
                    std::ptr::null_mut(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "inflate_null_output_literal" => {
                let mut stream = fixed_stream(b"A", None);
                (api.inflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    std::ptr::null_mut(),
                    1,
                );
            }
            "inflate_negative_output" => {
                let mut stream = fixed_stream(b"", None);
                (api.inflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    -1,
                );
            }
            "inflate_truncated_fixed" => {
                input[0] = 3;
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "inflate_truncated_dynamic" => {
                input[0] = 5;
                (api.inflate)(
                    input.as_mut_ptr().cast(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "unfilter_null_no_rows" => {
                (api.unfilter)(0, 0, 0, std::ptr::null_mut());
            }
            "unfilter_null_first_row" => {
                (api.unfilter)(1, 1, 1, std::ptr::null_mut());
            }
            unknown => panic!("unknown worker case {unknown}"),
        }
    }
}

#[test]
#[cfg(unix)]
fn process_level_error_boundaries_match() {
    let executable = std::env::current_exe().unwrap();
    for case in [
        "inflate_zero_input",
        "inflate_negative_input",
        "inflate_null_input",
        "inflate_null_output_literal",
        "inflate_negative_output",
        "inflate_truncated_fixed",
        "inflate_truncated_dynamic",
        "unfilter_null_no_rows",
        "unfilter_null_first_row",
    ] {
        let run = |implementation: &str| {
            Command::new(&executable)
                .args([
                    "--exact",
                    "ffi_process_worker",
                    "--test-threads=1",
                    "--nocapture",
                ])
                .env("DIFFERENTIAL_WORKER_CASE", case)
                .env("DIFFERENTIAL_WORKER_IMPL", implementation)
                .status()
                .unwrap()
        };
        let c_status = run("c");
        let rust_status = run("rust");
        assert_eq!(
            status_signature(rust_status),
            status_signature(c_status),
            "process outcome differs for {case}: C={c_status}, Rust={rust_status}"
        );
    }
}

#[test]
fn internal_assertion_surface_is_mirrored() {
    let c = std::fs::read_to_string(manifest_dir().join("../c_src/src/lib.c")).unwrap();
    let rust = std::fs::read_to_string(manifest_dir().join("src/lib.rs")).unwrap();
    let c_assertions = [
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
    let rust_assertions = [
        "assert_eq!((*s).bits_left & 7, 0);",
        "assert!((*s).word_index <= (*s).word_count);",
        "assert!((*s).count >= num_bits_to_read);",
        "assert!(num_bits_to_read <= 32);",
        "assert!(num_bits_to_read >= 0);",
        "assert!((*s).bits_left > 0);",
        "assert!((*s).count <= 64);",
        "assert!(!would_overflow(s, num_bits_to_read));",
        "assert!(len < 16);",
        "assert_eq!(search >> len, key >> len);",
    ];
    for assertion in c_assertions {
        assert!(
            c.contains(assertion),
            "missing C assertion inventory item: {assertion}"
        );
    }
    for assertion in rust_assertions {
        assert!(
            rust.contains(assertion),
            "Rust does not mirror assertion: {assertion}"
        );
    }
}
