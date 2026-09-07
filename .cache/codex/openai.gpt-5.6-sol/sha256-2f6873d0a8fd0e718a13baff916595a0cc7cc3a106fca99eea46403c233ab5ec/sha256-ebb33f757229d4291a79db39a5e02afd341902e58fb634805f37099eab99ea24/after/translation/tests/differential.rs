use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::slice;

type Inflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;
type Unfilter = unsafe extern "C" fn(c_int, c_int, c_int, *mut u8) -> c_int;

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-fDrOX9.so")
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libunfilter_lib.so")
}

unsafe fn load_pair() -> (Library, Library) {
    unsafe {
        (
            Library::new(c_library_path()).expect("load C shared object"),
            Library::new(rust_library_path()).expect("load Rust shared object"),
        )
    }
}

unsafe fn exported_bytes(lib: &Library, name: &[u8], len: usize) -> Vec<u8> {
    unsafe {
        let symbol: Symbol<*const u8> = lib.get(name).unwrap();
        slice::from_raw_parts(*symbol, len).to_vec()
    }
}

unsafe fn exported_words(lib: &Library, name: &[u8], len: usize) -> Vec<u32> {
    unsafe {
        let symbol: Symbol<*const u32> = lib.get(name).unwrap();
        slice::from_raw_parts(*symbol, len).to_vec()
    }
}

unsafe fn error_reason(lib: &Library) -> Option<Vec<u8>> {
    unsafe {
        let symbol: Symbol<*mut *const c_char> = lib.get(b"cp_error_reason\0").unwrap();
        let pointer = **symbol;
        (!pointer.is_null()).then(|| CStr::from_ptr(pointer).to_bytes().to_vec())
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

    fn range(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    bit: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit: 0,
        }
    }

    fn bits(&mut self, value: u32, count: u8) {
        for shift in 0..count {
            if self.bit == 0 {
                self.bytes.push(0);
            }
            if ((value >> shift) & 1) != 0 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 1 << self.bit;
            }
            self.bit = (self.bit + 1) & 7;
        }
    }

    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
        }
    }

    fn raw(&mut self, bytes: &[u8]) {
        assert_eq!(self.bit, 0);
        self.bytes.extend_from_slice(bytes);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn stored_stream(payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() <= u16::MAX as usize);
    let mut writer = BitWriter::new();
    writer.bits(1, 1);
    writer.bits(0, 2);
    writer.align();
    let len = payload.len() as u16;
    writer.raw(&len.to_le_bytes());
    writer.raw(&(!len).to_le_bytes());
    writer.raw(payload);
    writer.finish()
}

fn multiple_stored_stream(first: &[u8], second: &[u8]) -> Vec<u8> {
    assert!(first.len() <= u16::MAX as usize);
    assert!(second.len() <= u16::MAX as usize);
    let mut writer = BitWriter::new();
    writer.bits(0, 1);
    writer.bits(0, 2);
    writer.align();
    let first_len = first.len() as u16;
    writer.raw(&first_len.to_le_bytes());
    writer.raw(&(!first_len).to_le_bytes());
    writer.raw(first);
    writer.bits(1, 1);
    writer.bits(0, 2);
    writer.align();
    let second_len = second.len() as u16;
    writer.raw(&second_len.to_le_bytes());
    writer.raw(&(!second_len).to_le_bytes());
    writer.raw(second);
    writer.finish()
}

fn fixed_len(symbol: usize) -> u8 {
    match symbol {
        0..=143 => 8,
        144..=255 => 9,
        256..=279 => 7,
        280..=287 => 8,
        _ => panic!("bad fixed symbol"),
    }
}

fn fixed_code(symbol: usize) -> (u32, u8) {
    let mut counts = [0u32; 16];
    for sym in 0..288 {
        counts[fixed_len(sym) as usize] += 1;
    }
    let mut next = [0u32; 16];
    let mut code = 0;
    for bits in 1..=15 {
        code = (code + counts[bits - 1]) << 1;
        next[bits] = code;
    }
    for sym in 0..=symbol {
        let len = fixed_len(sym) as usize;
        let assigned = next[len];
        next[len] += 1;
        if sym == symbol {
            return (assigned.reverse_bits() >> (32 - len), len as u8);
        }
    }
    unreachable!()
}

fn distance_code(symbol: usize) -> (u32, u8) {
    ((symbol as u32).reverse_bits() >> 27, 5)
}

const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const LENGTH_BASE: [usize; 29] = [
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

fn write_fixed_symbol(writer: &mut BitWriter, symbol: usize) {
    let (code, bits) = fixed_code(symbol);
    writer.bits(code, bits);
}

fn write_match(writer: &mut BitWriter, length: usize, distance: usize) {
    let length_symbol = (0..29)
        .find(|&i| {
            let max = LENGTH_BASE[i] + ((1usize << LENGTH_EXTRA[i]) - 1);
            length >= LENGTH_BASE[i] && length <= max
        })
        .unwrap();
    write_fixed_symbol(writer, 257 + length_symbol);
    writer.bits(
        (length - LENGTH_BASE[length_symbol]) as u32,
        LENGTH_EXTRA[length_symbol],
    );

    let distance_symbol = (0..30)
        .find(|&i| {
            let max = DIST_BASE[i] + ((1usize << DIST_EXTRA[i]) - 1);
            distance >= DIST_BASE[i] && distance <= max
        })
        .unwrap();
    let (code, bits) = distance_code(distance_symbol);
    writer.bits(code, bits);
    writer.bits(
        (distance - DIST_BASE[distance_symbol]) as u32,
        DIST_EXTRA[distance_symbol],
    );
}

fn fixed_literal_stream(payload: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for &byte in payload {
        write_fixed_symbol(&mut writer, byte as usize);
    }
    write_fixed_symbol(&mut writer, 256);
    writer.finish()
}

fn fixed_match_stream(prefix: &[u8], matches: &[(usize, usize)]) -> (Vec<u8>, Vec<u8>) {
    let mut writer = BitWriter::new();
    let mut output = prefix.to_vec();
    writer.bits(1, 1);
    writer.bits(1, 2);
    for &byte in prefix {
        write_fixed_symbol(&mut writer, byte as usize);
    }
    for &(length, distance) in matches {
        write_match(&mut writer, length, distance);
        if distance <= output.len() {
            for _ in 0..length {
                let byte = output[output.len() - distance];
                output.push(byte);
            }
        }
    }
    write_fixed_symbol(&mut writer, 256);
    (writer.finish(), output)
}

fn multiple_fixed_stream(first: &[u8], second: &[u8]) -> Vec<u8> {
    let mut writer = BitWriter::new();
    writer.bits(0, 1);
    writer.bits(1, 2);
    for &byte in first {
        write_fixed_symbol(&mut writer, byte as usize);
    }
    write_fixed_symbol(&mut writer, 256);
    writer.bits(1, 1);
    writer.bits(1, 2);
    for &byte in second {
        write_fixed_symbol(&mut writer, byte as usize);
    }
    write_fixed_symbol(&mut writer, 256);
    writer.finish()
}

fn python_raw_deflate(payload: &[u8]) -> Vec<u8> {
    let script = "import sys,zlib\n\
d=sys.stdin.buffer.read()\n\
z=zlib.compressobj(6,zlib.DEFLATED,-15)\n\
sys.stdout.buffer.write(z.compress(d)+z.flush())\n";
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn python zlib");
    child.stdin.take().unwrap().write_all(payload).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    output.stdout
}

fn block_type(stream: &[u8]) -> u8 {
    (stream[0] >> 1) & 3
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 15) as usize] as char);
    }
    result
}

fn from_hex(value: &str) -> Vec<u8> {
    fn digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("invalid hex"),
        }
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (digit(pair[0]) << 4) | digit(pair[1]))
        .collect()
}

fn aligned_input(stream: &[u8], alignment: usize) -> (Vec<u8>, usize) {
    let mut storage = vec![0xa5; stream.len() + 8];
    let base = storage.as_ptr() as usize;
    let offset = (alignment + 4 - (base & 3)) & 3;
    storage[offset..offset + stream.len()].copy_from_slice(stream);
    (storage, offset)
}

unsafe fn compare_inflate(
    c: &Library,
    rust: &Library,
    stream: &[u8],
    alignment: usize,
    output_capacity: usize,
    expected: Option<&[u8]>,
) -> c_int {
    unsafe {
        let c_fn: Symbol<Inflate> = c.get(b"cp_inflate\0").unwrap();
        let r_fn: Symbol<Inflate> = rust.get(b"cp_inflate\0").unwrap();
        let (mut c_input, c_offset) = aligned_input(stream, alignment);
        let (mut r_input, r_offset) = aligned_input(stream, alignment);
        let mut c_output = vec![0xcd; output_capacity];
        let mut r_output = vec![0xcd; output_capacity];
        let c_result = c_fn(
            c_input.as_mut_ptr().add(c_offset).cast(),
            stream.len() as c_int,
            c_output.as_mut_ptr().cast(),
            output_capacity as c_int,
        );
        let r_result = r_fn(
            r_input.as_mut_ptr().add(r_offset).cast(),
            stream.len() as c_int,
            r_output.as_mut_ptr().cast(),
            output_capacity as c_int,
        );
        assert_eq!(c_result, r_result, "return mismatch for {stream:02x?}");
        assert_eq!(c_output, r_output, "output mismatch for {stream:02x?}");
        if c_result == 0 {
            assert_eq!(error_reason(c), error_reason(rust));
        }
        if let Some(expected) = expected {
            assert_eq!(c_result, 1, "C rejected valid stream {stream:02x?}");
            assert_eq!(&c_output[..expected.len()], expected);
            assert!(c_output[expected.len()..].iter().all(|&b| b == 0xcd));
        }
        c_result
    }
}

unsafe fn compare_unfilter(
    c: &Library,
    rust: &Library,
    w: c_int,
    h: c_int,
    bpp: c_int,
    input: &[u8],
) -> c_int {
    unsafe {
        let c_fn: Symbol<Unfilter> = c.get(b"unfilter\0").unwrap();
        let r_fn: Symbol<Unfilter> = rust.get(b"unfilter\0").unwrap();
        let mut c_raw = input.to_vec();
        let mut r_raw = input.to_vec();
        let c_result = c_fn(w, h, bpp, c_raw.as_mut_ptr());
        let r_result = r_fn(w, h, bpp, r_raw.as_mut_ptr());
        assert_eq!(c_result, r_result);
        assert_eq!(c_raw, r_raw, "unfilter mismatch w={w} h={h} bpp={bpp}");
        c_result
    }
}

fn random_scanlines(rng: &mut Rng, w: usize, h: usize, bpp: usize, filters: &[u8]) -> Vec<u8> {
    let row = w * bpp;
    let mut raw = Vec::with_capacity(h * (row + 1));
    for y in 0..h {
        raw.push(filters[y % filters.len()]);
        for _ in 0..row {
            raw.push(rng.byte());
        }
    }
    raw
}

#[test]
fn differential_valid_and_explicit_error_surface() {
    unsafe {
        let (c, rust) = load_pair();

        assert_eq!(
            exported_bytes(&c, b"cp_fixed_table\0", 320),
            exported_bytes(&rust, b"cp_fixed_table\0", 320)
        );
        assert_eq!(
            exported_bytes(&c, b"cp_permutation_order\0", 19),
            exported_bytes(&rust, b"cp_permutation_order\0", 19)
        );
        assert_eq!(
            exported_bytes(&c, b"cp_len_extra_bits\0", 31),
            exported_bytes(&rust, b"cp_len_extra_bits\0", 31)
        );
        assert_eq!(
            exported_words(&c, b"cp_len_base\0", 31),
            exported_words(&rust, b"cp_len_base\0", 31)
        );
        assert_eq!(
            exported_bytes(&c, b"cp_dist_extra_bits\0", 32),
            exported_bytes(&rust, b"cp_dist_extra_bits\0", 32)
        );
        assert_eq!(
            exported_words(&c, b"cp_dist_base\0", 32),
            exported_words(&rust, b"cp_dist_base\0", 32)
        );
        assert_eq!(error_reason(&c), None);
        assert_eq!(error_reason(&rust), None);

        let c_unfilter: Symbol<Unfilter> = c.get(b"unfilter\0").unwrap();
        let r_unfilter: Symbol<Unfilter> = rust.get(b"unfilter\0").unwrap();
        assert_eq!(c_unfilter(1, 0, 1, std::ptr::null_mut()), 1);
        assert_eq!(r_unfilter(1, 0, 1, std::ptr::null_mut()), 1);
        assert_eq!(c_unfilter(1, -1, 1, std::ptr::null_mut()), 1);
        assert_eq!(r_unfilter(1, -1, 1, std::ptr::null_mut()), 1);

        let mut rng = Rng::new(0x5eed_d1ff_cafe_babe);
        for filter in 0..=4u8 {
            for &(w, bpp) in &[(0, 1), (1, 1), (7, 1), (1, 4), (6, 4)] {
                for _ in 0..48 {
                    let raw = random_scanlines(&mut rng, w, 1, bpp, &[filter]);
                    assert_eq!(
                        compare_unfilter(&c, &rust, w as c_int, 1, bpp as c_int, &raw),
                        1
                    );
                }
            }
        }
        for first_filter in 0..=4u8 {
            for later_filter in 0..=4u8 {
                for &(w, bpp) in &[(1, 1), (9, 1), (1, 4), (5, 4)] {
                    for _ in 0..32 {
                        let raw =
                            random_scanlines(&mut rng, w, 4, bpp, &[first_filter, later_filter]);
                        assert_eq!(
                            compare_unfilter(&c, &rust, w as c_int, 4, bpp as c_int, &raw),
                            1
                        );
                    }
                }
            }
        }
        for filter in [5u8, 255] {
            assert_eq!(
                compare_unfilter(&c, &rust, 4, 1, 1, &[filter, 1, 2, 3, 4]),
                0
            );
            assert_eq!(
                compare_unfilter(&c, &rust, 2, 2, 1, &[0, 1, 2, filter, 3, 4]),
                0
            );
        }
        for filter in 0..=4u8 {
            compare_unfilter(&c, &rust, 0, 3, 1, &[filter; 3]);
            assert_eq!(compare_unfilter(&c, &rust, 5, 3, 0, &[filter; 3]), 1);
        }

        for alignment in 0..4 {
            let empty = stored_stream(&[]);
            assert_eq!(
                compare_inflate(&c, &rust, &empty, alignment, 0, Some(&[])),
                1
            );
            for _ in 0..40 {
                let len = 1 + rng.range(384);
                let payload: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
                let stream = stored_stream(&payload);
                assert_eq!(
                    compare_inflate(&c, &rust, &stream, alignment, len, Some(&payload)),
                    1
                );
                assert_eq!(
                    compare_inflate(&c, &rust, &stream, alignment, len + 17, Some(&payload)),
                    1
                );
            }
        }

        for _ in 0..32 {
            let first: Vec<u8> = (0..rng.range(4)).map(|_| rng.byte()).collect();
            let second: Vec<u8> = (0..(20 + rng.range(40))).map(|_| rng.byte()).collect();
            let multi_stored = multiple_stored_stream(&first, &second);
            assert_eq!(
                compare_inflate(&c, &rust, &multi_stored, rng.range(4), 128, None),
                0
            );
        }

        let fixed_empty = fixed_literal_stream(&[]);
        assert_eq!(compare_inflate(&c, &rust, &fixed_empty, 0, 0, Some(&[])), 1);
        for alignment in 0..4 {
            for _ in 0..64 {
                let len = 1 + rng.range(300);
                let payload: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
                let stream = fixed_literal_stream(&payload);
                assert_eq!(
                    compare_inflate(&c, &rust, &stream, alignment, len, Some(&payload)),
                    1
                );
                assert_eq!(
                    compare_inflate(&c, &rust, &stream, alignment, len + 31, Some(&payload)),
                    1
                );
            }
        }

        for _ in 0..64 {
            let byte = rng.byte();
            let (stream, output) = fixed_match_stream(&[byte], &[(3 + rng.range(200), 1)]);
            assert_eq!(
                compare_inflate(
                    &c,
                    &rust,
                    &stream,
                    rng.range(4),
                    output.len(),
                    Some(&output)
                ),
                1
            );
        }
        for _ in 0..64 {
            let distance = 2 + rng.range(30);
            let prefix: Vec<u8> = (0..distance).map(|_| rng.byte()).collect();
            let length = 3 + rng.range(180);
            let (stream, output) = fixed_match_stream(&prefix, &[(length, distance)]);
            assert_eq!(
                compare_inflate(
                    &c,
                    &rust,
                    &stream,
                    rng.range(4),
                    output.len(),
                    Some(&output)
                ),
                1
            );
        }
        let prefix: Vec<u8> = (0..300).map(|_| rng.byte()).collect();
        let matches = [
            (3, 1),
            (11, 4),
            (19, 5),
            (35, 17),
            (67, 33),
            (131, 129),
            (258, 257),
        ];
        let (stream, output) = fixed_match_stream(&prefix, &matches);
        for alignment in 0..4 {
            assert_eq!(
                compare_inflate(&c, &rust, &stream, alignment, output.len(), Some(&output)),
                1
            );
        }

        let mut dynamic_entropy_cases = 0;
        let mut dynamic_repetitive_cases = 0;
        for attempt in 0..96 {
            let len = 700 + rng.range(1400);
            let payload: Vec<u8> = (0..len)
                .map(|_| {
                    if attempt & 1 == 0 {
                        b'a' + rng.range(16) as u8
                    } else {
                        rng.byte()
                    }
                })
                .collect();
            let compressed = python_raw_deflate(&payload);
            if block_type(&compressed) == 2 {
                assert_eq!(
                    compare_inflate(
                        &c,
                        &rust,
                        &compressed,
                        dynamic_entropy_cases & 3,
                        payload.len() + 23,
                        Some(&payload)
                    ),
                    1
                );
                dynamic_entropy_cases += 1;
                if dynamic_entropy_cases == 24 {
                    break;
                }
            }
        }
        assert_eq!(dynamic_entropy_cases, 24);

        for attempt in 0..64 {
            let mut payload = Vec::new();
            for i in 0..(300 + attempt * 3) {
                payload.extend_from_slice(if i % 7 == 0 {
                    b"abracadabra!"
                } else {
                    b"abracadabra?"
                });
            }
            let compressed = python_raw_deflate(&payload);
            if block_type(&compressed) == 2 {
                assert_eq!(
                    compare_inflate(
                        &c,
                        &rust,
                        &compressed,
                        dynamic_repetitive_cases & 3,
                        payload.len(),
                        Some(&payload)
                    ),
                    1
                );
                dynamic_repetitive_cases += 1;
                if dynamic_repetitive_cases == 16 {
                    break;
                }
            }
        }
        assert_eq!(dynamic_repetitive_cases, 16);

        for _ in 0..48 {
            let first: Vec<u8> = (0..(1 + rng.range(100))).map(|_| rng.byte()).collect();
            let second: Vec<u8> = (0..(1 + rng.range(100))).map(|_| rng.byte()).collect();
            let stream = multiple_fixed_stream(&first, &second);
            let expected: Vec<u8> = first.iter().chain(&second).copied().collect();
            assert_eq!(
                compare_inflate(
                    &c,
                    &rust,
                    &stream,
                    rng.range(4),
                    expected.len(),
                    Some(&expected)
                ),
                1
            );
        }

        let complement_error = [1u8, 1, 0, 0, 0];
        assert_eq!(compare_inflate(&c, &rust, &complement_error, 0, 8, None), 0);
        let stored_end_error = [1u8, 0, 0, 255, 255, 0xaa];
        assert_eq!(compare_inflate(&c, &rust, &stored_end_error, 0, 8, None), 0);
        let unknown_block = [7u8];
        assert_eq!(compare_inflate(&c, &rust, &unknown_block, 0, 8, None), 0);

        let literal = fixed_literal_stream(b"x");
        assert_eq!(compare_inflate(&c, &rust, &literal, 0, 0, None), 0);
        let (distance_before, _) = fixed_match_stream(&[], &[(3, 1)]);
        assert_eq!(compare_inflate(&c, &rust, &distance_before, 0, 8, None), 0);
        let (string_overflow, _) = fixed_match_stream(b"x", &[(3, 1)]);
        assert_eq!(compare_inflate(&c, &rust, &string_overflow, 0, 1, None), 0);
    }
}

#[test]
fn zero_input_assertion_matches() {
    if env::var_os("DIFFERENTIAL_ABORT_PROBE").is_some() {
        return;
    }
    use std::os::unix::process::ExitStatusExt;
    let executable = env::current_exe().unwrap();
    let run = |library: &str| {
        Command::new(&executable)
            .arg("--exact")
            .arg("abort_probe")
            .arg("--nocapture")
            .env("DIFFERENTIAL_ABORT_PROBE", library)
            .status()
            .unwrap()
    };
    let c_status = run("c");
    let rust_status = run("rust");
    assert!(!c_status.success());
    assert_eq!(c_status.signal(), rust_status.signal());
    assert_eq!(c_status.code(), rust_status.code());
}

fn run_boundary_probe(library: &str, mode: &str, input: &[u8], output_len: c_int) -> Output {
    Command::new("timeout")
        .arg("0.5")
        .arg(env::current_exe().unwrap())
        .arg("--exact")
        .arg("boundary_probe")
        .arg("--nocapture")
        .env("DIFFERENTIAL_BOUNDARY_LIBRARY", library)
        .env("DIFFERENTIAL_BOUNDARY_MODE", mode)
        .env("DIFFERENTIAL_BOUNDARY_INPUT", hex(input))
        .env("DIFFERENTIAL_BOUNDARY_OUTPUT_LEN", output_len.to_string())
        .output()
        .unwrap()
}

fn assert_probe_pair(mode: &str, input: &[u8], output_len: c_int) {
    use std::os::unix::process::ExitStatusExt;
    let c = run_boundary_probe("c", mode, input, output_len);
    let rust = run_boundary_probe("rust", mode, input, output_len);
    assert_eq!(
        (c.status.code(), c.status.signal()),
        (rust.status.code(), rust.status.signal()),
        "process outcome mismatch mode={mode} input={:02x?}\nC stderr: {}\nRust stderr: {}",
        input,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr)
    );
    if c.status.success() {
        let record = |output: &[u8]| {
            String::from_utf8_lossy(output)
                .lines()
                .find(|line| line.starts_with("result="))
                .map(str::to_owned)
        };
        assert_eq!(
            record(&c.stdout),
            record(&rust.stdout),
            "returned-value mismatch mode={mode} input={:02x?}",
            input
        );
    }
}

#[test]
fn malformed_and_boundary_subprocesses_match() {
    if env::var_os("DIFFERENTIAL_BOUNDARY_LIBRARY").is_some() {
        return;
    }

    let fixed = fixed_literal_stream(b"abcdef0123456789");
    for end in 0..fixed.len() {
        assert_probe_pair("inflate", &fixed[..end], 64);
    }

    let dynamic_payload = b"the quick brown fox jumps over the lazy dog. ".repeat(200);
    let dynamic = python_raw_deflate(&dynamic_payload);
    assert_eq!(block_type(&dynamic), 2);
    let dynamic_prefixes = [
        0,
        1,
        2,
        3,
        dynamic.len() / 4,
        dynamic.len() / 2,
        dynamic.len().saturating_sub(1),
    ];
    for end in dynamic_prefixes {
        assert_probe_pair("inflate", &dynamic[..end], dynamic_payload.len() as c_int);
    }

    let stored = stored_stream(b"stored payload");
    for end in 0..stored.len() {
        assert_probe_pair("inflate", &stored[..end], 64);
    }

    let mut rng = Rng::new(0xbad5_eed0_1234_5678);
    for len in 1..=8 {
        for _ in 0..4 {
            let input: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            assert_probe_pair("inflate", &input, 64);
        }
    }

    assert_probe_pair("unfilter_null", &[], 0);
    assert_probe_pair("inflate_null_input", &[], 8);
    assert_probe_pair("inflate_null_output", &fixed_literal_stream(b"x"), 1);
    assert_probe_pair("inflate_negative_input", &[], 8);
    assert_probe_pair(
        "inflate_oversized_output",
        &fixed_literal_stream(&[]),
        c_int::MAX,
    );
}

#[test]
fn boundary_probe() {
    let Some(which) = env::var_os("DIFFERENTIAL_BOUNDARY_LIBRARY") else {
        return;
    };
    let mode = env::var("DIFFERENTIAL_BOUNDARY_MODE").unwrap();
    let input = from_hex(&env::var("DIFFERENTIAL_BOUNDARY_INPUT").unwrap());
    let output_len: c_int = env::var("DIFFERENTIAL_BOUNDARY_OUTPUT_LEN")
        .unwrap()
        .parse()
        .unwrap();
    unsafe {
        let path = if which == "c" {
            c_library_path()
        } else {
            rust_library_path()
        };
        let library = Library::new(path).unwrap();
        match mode.as_str() {
            "inflate" => {
                let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
                let mut input = input;
                let mut output = vec![0xcd; output_len.max(0) as usize];
                let result = inflate(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output_len,
                );
                println!(
                    "result={result};reason={};output={}",
                    error_reason(&library)
                        .map(|value| hex(&value))
                        .unwrap_or_default(),
                    hex(&output)
                );
            }
            "unfilter_null" => {
                let unfilter: Symbol<Unfilter> = library.get(b"unfilter\0").unwrap();
                let result = unfilter(1, 1, 1, std::ptr::null_mut());
                println!("result={result}");
            }
            "inflate_null_input" => {
                let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
                let mut output = [0xcdu8; 8];
                let result = inflate(
                    std::ptr::null_mut(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
                println!("result={result};output={}", hex(&output));
            }
            "inflate_null_output" => {
                let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
                let mut input = input;
                let result = inflate(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    std::ptr::null_mut(),
                    output_len,
                );
                println!("result={result}");
            }
            "inflate_negative_input" => {
                let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
                let mut input = [0u8; 8];
                let mut output = [0xcdu8; 8];
                let result = inflate(
                    input.as_mut_ptr().cast(),
                    -1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
                println!("result={result};output={}", hex(&output));
            }
            "inflate_oversized_output" => {
                let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
                let mut input = input;
                let mut output = [0xcdu8; 1];
                let result = inflate(
                    input.as_mut_ptr().cast(),
                    input.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output_len,
                );
                println!("result={result};output={}", hex(&output));
            }
            _ => panic!("unknown probe mode"),
        }
    }
}

#[test]
fn abort_probe() {
    let Some(which) = env::var_os("DIFFERENTIAL_ABORT_PROBE") else {
        return;
    };
    unsafe {
        let path = if which == "c" {
            c_library_path()
        } else {
            rust_library_path()
        };
        let library = Library::new(path).unwrap();
        let inflate: Symbol<Inflate> = library.get(b"cp_inflate\0").unwrap();
        let mut input = Vec::<u8>::new();
        let mut output = Vec::<u8>::new();
        let result = inflate(input.as_mut_ptr().cast(), 0, output.as_mut_ptr().cast(), 0);
        panic!("zero-length input unexpectedly returned {result}");
    }
}
