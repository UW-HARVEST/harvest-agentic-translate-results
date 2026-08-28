use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{Mutex, MutexGuard};

type Pinflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

const C_LIBRARY: &str = "../c_src/build/libharvest-work-aRikqo.so";
const RUST_LIBRARY: &str = "target/release/libpinflate_lib.so";
const PERMUTATION: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
const LEN_BASE: [usize; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [usize; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_tests() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

struct Api {
    _library: Library,
    pinflate: Pinflate,
    error_reason: *mut *const c_char,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let pinflate = *unsafe { library.get::<Pinflate>(b"pinflate\0") }.unwrap();
        let error_reason =
            *unsafe { library.get::<*mut *const c_char>(b"cp_error_reason\0") }.unwrap();
        Self {
            _library: library,
            pinflate,
            error_reason,
        }
    }

    unsafe fn call(
        &self,
        input: *mut u8,
        input_len: usize,
        output: *mut u8,
        output_len: usize,
    ) -> (c_int, Option<String>) {
        unsafe { self.error_reason.write(std::ptr::null()) };
        let result = unsafe {
            (self.pinflate)(
                input.cast(),
                input_len as c_int,
                output.cast(),
                output_len as c_int,
            )
        };
        let reason_ptr = unsafe { self.error_reason.read() };
        let reason = if reason_ptr.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(reason_ptr) }
                    .to_str()
                    .unwrap()
                    .to_owned(),
            )
        };
        (result, reason)
    }
}

#[derive(Clone, Debug)]
enum Token {
    Literal(u8),
    Match { length: usize, distance: usize },
}

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl Bits {
    fn write(&mut self, value: u32, count: u8) {
        for bit in 0..count {
            self.current |= (((value >> bit) & 1) as u8) << self.used;
            self.used += 1;
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

    fn finish(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

fn reverse(mut value: u32, width: u8) -> u32 {
    let mut result = 0;
    for _ in 0..width {
        result = (result << 1) | (value & 1);
        value >>= 1;
    }
    result
}

fn codes(lengths: &[u8]) -> Vec<(u32, u8)> {
    let mut counts = [0_u32; 16];
    for &length in lengths {
        counts[length as usize] += 1;
    }
    counts[0] = 0;
    let mut next = [0_u32; 16];
    let mut code = 0;
    for length in 1..=15 {
        code = (code + counts[length - 1]) << 1;
        next[length] = code;
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

fn emit_symbol(bits: &mut Bits, table: &[(u32, u8)], symbol: usize) {
    let (code, width) = table[symbol];
    assert_ne!(width, 0, "symbol {symbol} has no Huffman code");
    bits.write(code, width);
}

fn fixed_lengths() -> (Vec<u8>, Vec<u8>) {
    let mut literal = vec![0_u8; 288];
    literal[..144].fill(8);
    literal[144..256].fill(9);
    literal[256..280].fill(7);
    literal[280..].fill(8);
    (literal, vec![5; 32])
}

fn find_range(value: usize, bases: &[usize], extras: &[u8]) -> (usize, u32, u8) {
    for (symbol, (&base, &extra)) in bases.iter().zip(extras).enumerate() {
        let maximum = base + ((1_usize << extra) - 1);
        if value >= base && value <= maximum {
            return (symbol, (value - base) as u32, extra);
        }
    }
    panic!("value {value} has no DEFLATE range");
}

fn emit_tokens(
    bits: &mut Bits,
    tokens: &[Token],
    literal_codes: &[(u32, u8)],
    distance_codes: &[(u32, u8)],
) {
    for token in tokens {
        match *token {
            Token::Literal(value) => emit_symbol(bits, literal_codes, value as usize),
            Token::Match { length, distance } => {
                let (length_index, length_value, length_bits) =
                    find_range(length, &LEN_BASE, &LEN_EXTRA);
                emit_symbol(bits, literal_codes, 257 + length_index);
                bits.write(length_value, length_bits);

                let (distance_symbol, distance_value, distance_bits) =
                    find_range(distance, &DIST_BASE, &DIST_EXTRA);
                emit_symbol(bits, distance_codes, distance_symbol);
                bits.write(distance_value, distance_bits);
            }
        }
    }
    emit_symbol(bits, literal_codes, 256);
}

fn append_fixed(bits: &mut Bits, final_block: bool, tokens: &[Token]) {
    bits.write(u32::from(final_block), 1);
    bits.write(1, 2);
    let (literal_lengths, distance_lengths) = fixed_lengths();
    emit_tokens(
        bits,
        tokens,
        &codes(&literal_lengths),
        &codes(&distance_lengths),
    );
}

fn fixed_stream(tokens: &[Token]) -> Vec<u8> {
    let mut bits = Bits::default();
    append_fixed(&mut bits, true, tokens);
    bits.finish()
}

fn stored_stream(payload: &[u8], trailing: &[u8], valid_complement: bool) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.write(1, 1);
    bits.write(0, 2);
    bits.align();
    let length = payload.len() as u16;
    bits.bytes.extend_from_slice(&length.to_le_bytes());
    let complement = if valid_complement { !length } else { length };
    bits.bytes.extend_from_slice(&complement.to_le_bytes());
    bits.bytes.extend_from_slice(payload);
    bits.bytes.extend_from_slice(trailing);
    bits.finish()
}

fn append_dynamic_direct(
    bits: &mut Bits,
    final_block: bool,
    tokens: &[Token],
    literal_lengths: &[u8],
    distance_lengths: &[u8],
) {
    assert!((257..=286).contains(&literal_lengths.len()));
    assert!((1..=30).contains(&distance_lengths.len()));
    assert!(
        literal_lengths
            .iter()
            .chain(distance_lengths)
            .all(|&length| length <= 3)
    );

    bits.write(u32::from(final_block), 1);
    bits.write(2, 2);
    bits.write((literal_lengths.len() - 257) as u32, 5);
    bits.write((distance_lengths.len() - 1) as u32, 5);
    bits.write(14, 4); // 18 code-length code lengths

    let mut code_length_lengths = [0_u8; 19];
    code_length_lengths[0..=3].fill(2);
    for &symbol in &PERMUTATION[..18] {
        bits.write(code_length_lengths[symbol] as u32, 3);
    }
    let code_length_codes = codes(&code_length_lengths);
    for &length in literal_lengths.iter().chain(distance_lengths) {
        emit_symbol(bits, &code_length_codes, length as usize);
    }

    emit_tokens(
        bits,
        tokens,
        &codes(literal_lengths),
        &codes(distance_lengths),
    );
}

fn dynamic_direct_stream(
    tokens: &[Token],
    literal_lengths: &[u8],
    distance_lengths: &[u8],
) -> Vec<u8> {
    let mut bits = Bits::default();
    append_dynamic_direct(&mut bits, true, tokens, literal_lengths, distance_lengths);
    bits.finish()
}

fn dynamic_literal_stream(payload: &[u8]) -> Vec<u8> {
    let mut literal_lengths = vec![0_u8; 257];
    literal_lengths[65..=68].fill(3);
    literal_lengths[256] = 1;
    let tokens: Vec<_> = payload.iter().copied().map(Token::Literal).collect();
    dynamic_direct_stream(&tokens, &literal_lengths, &[0])
}

fn append_dynamic_literals(bits: &mut Bits, final_block: bool, payload: &[u8]) {
    let mut literal_lengths = vec![0_u8; 257];
    literal_lengths[65..=68].fill(3);
    literal_lengths[256] = 1;
    let tokens: Vec<_> = payload.iter().copied().map(Token::Literal).collect();
    append_dynamic_direct(bits, final_block, &tokens, &literal_lengths, &[0]);
}

fn dynamic_repeat_stream(payload: &[u8]) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.write(1, 1);
    bits.write(2, 2);
    bits.write(0, 5); // 257 literal/length symbols
    bits.write(0, 5); // one distance symbol
    bits.write(14, 4); // 18 code-length code lengths

    let mut code_length_lengths = [0_u8; 19];
    code_length_lengths[0] = 2;
    code_length_lengths[1] = 2;
    code_length_lengths[3] = 3;
    code_length_lengths[16] = 3;
    code_length_lengths[17] = 3;
    code_length_lengths[18] = 3;
    for &symbol in &PERMUTATION[..18] {
        bits.write(code_length_lengths[symbol] as u32, 3);
    }
    let code_length_codes = codes(&code_length_lengths);

    emit_symbol(&mut bits, &code_length_codes, 18);
    bits.write(54, 7); // 65 zero lengths
    emit_symbol(&mut bits, &code_length_codes, 3);
    emit_symbol(&mut bits, &code_length_codes, 16);
    bits.write(0, 2); // repeat length 3 three times: symbols 65..=68
    emit_symbol(&mut bits, &code_length_codes, 18);
    bits.write(127, 7); // 138 zeros
    emit_symbol(&mut bits, &code_length_codes, 18);
    bits.write(29, 7); // 40 zeros
    emit_symbol(&mut bits, &code_length_codes, 17);
    bits.write(6, 3); // 9 zeros, reaching symbol 255
    emit_symbol(&mut bits, &code_length_codes, 1); // EOB at 256
    emit_symbol(&mut bits, &code_length_codes, 0); // unused distance symbol

    let mut literal_lengths = vec![0_u8; 257];
    literal_lengths[65..=68].fill(3);
    literal_lengths[256] = 1;
    let tokens: Vec<_> = payload.iter().copied().map(Token::Literal).collect();
    emit_tokens(&mut bits, &tokens, &codes(&literal_lengths), &[]);
    bits.finish()
}

fn dynamic_match_stream(tokens: &[Token], distance_symbol: usize) -> Vec<u8> {
    let mut literal_lengths = vec![0_u8; 266];
    literal_lengths[65..=68].fill(3);
    literal_lengths[256] = 3;
    literal_lengths[257] = 3;
    literal_lengths[265] = 3;
    let mut distance_lengths = vec![0_u8; distance_symbol + 1];
    distance_lengths[distance_symbol] = 1;
    dynamic_direct_stream(tokens, &literal_lengths, &distance_lengths)
}

fn expected_from_tokens(tokens: &[Token]) -> Vec<u8> {
    let mut output = Vec::new();
    for token in tokens {
        match *token {
            Token::Literal(value) => output.push(value),
            Token::Match { length, distance } => {
                for _ in 0..length {
                    output.push(output[output.len() - distance]);
                }
            }
        }
    }
    output
}

fn aligned_buffer(bytes: &[u8], alignment: usize) -> (Vec<u8>, usize) {
    let mut storage = vec![0_u8; bytes.len() + 7];
    let base = storage.as_ptr() as usize;
    let offset = (alignment + 4 - (base & 3)) & 3;
    storage[offset..offset + bytes.len()].copy_from_slice(bytes);
    assert_eq!((storage[offset..].as_ptr() as usize) & 3, alignment);
    (storage, offset)
}

fn compare(
    c: &Api,
    rust: &Api,
    stream: &[u8],
    alignment: usize,
    output_capacity: usize,
) -> (c_int, Option<String>, Vec<u8>) {
    let (mut c_input, c_offset) = aligned_buffer(stream, alignment);
    let (mut rust_input, rust_offset) = aligned_buffer(stream, alignment);
    let mut c_output = vec![0xa5_u8; output_capacity.max(1)];
    let mut rust_output = c_output.clone();
    let c_output_ptr = c_output.as_mut_ptr();
    let rust_output_ptr = rust_output.as_mut_ptr();
    let (c_result, c_reason) = unsafe {
        c.call(
            c_input.as_mut_ptr().add(c_offset),
            stream.len(),
            c_output_ptr,
            output_capacity,
        )
    };
    let (rust_result, rust_reason) = unsafe {
        rust.call(
            rust_input.as_mut_ptr().add(rust_offset),
            stream.len(),
            rust_output_ptr,
            output_capacity,
        )
    };
    assert_eq!(rust_result, c_result, "return mismatch for {stream:02x?}");
    assert_eq!(rust_reason, c_reason, "error mismatch for {stream:02x?}");
    assert_eq!(rust_output, c_output, "output mismatch for {stream:02x?}");
    (c_result, c_reason, c_output)
}

fn load_pair() -> (Api, Api) {
    unsafe {
        (
            Api::load(&manifest_path(C_LIBRARY)),
            Api::load(&manifest_path(RUST_LIBRARY)),
        )
    }
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn exported_object_bytes_match() {
    let _guard = lock_tests();
    let (c, rust) = load_pair();
    let objects = [
        ("cp_fixed_table", 320),
        ("cp_permutation_order", 19),
        ("cp_len_extra_bits", 31),
        ("cp_len_base", 124),
        ("cp_dist_extra_bits", 32),
        ("cp_dist_base", 128),
    ];
    for (name, size) in objects {
        unsafe {
            let symbol = format!("{name}\0");
            let c_ptr = *c._library.get::<*const u8>(symbol.as_bytes()).unwrap();
            let rust_ptr = *rust._library.get::<*const u8>(symbol.as_bytes()).unwrap();
            assert_eq!(
                std::slice::from_raw_parts(rust_ptr, size),
                std::slice::from_raw_parts(c_ptr, size),
                "{name}"
            );
        }
    }
    assert!(unsafe { c.error_reason.read() }.is_null());
    assert!(unsafe { rust.error_reason.read() }.is_null());
}

#[test]
fn valid_stored_and_fixed_configurations_match() {
    let _guard = lock_tests();
    let (c, rust) = load_pair();
    let mut random = 0x5eed_cafe_d00d_f00d;

    for alignment in 0..4 {
        for length in [0, 1, 2, 7, 31, 255] {
            for _ in 0..12 {
                let payload: Vec<_> = (0..length)
                    .map(|_| next_random(&mut random) as u8)
                    .collect();
                let stream = stored_stream(&payload, &[], true);
                let capacity = payload.len() + usize::from(alignment == 3);
                let (result, reason, output) = compare(&c, &rust, &stream, alignment, capacity);
                assert_eq!(result, 1);
                assert_eq!(reason, None);
                assert_eq!(output.len(), capacity.max(1));
            }
        }
    }

    for alignment in 0..4 {
        for length in [0, 1, 2, 7, 31, 127] {
            for _ in 0..20 {
                let payload: Vec<_> = (0..length)
                    .map(|_| next_random(&mut random) as u8)
                    .collect();
                let tokens: Vec<_> = payload.iter().copied().map(Token::Literal).collect();
                let stream = fixed_stream(&tokens);
                let capacity = payload.len() + usize::from(alignment == 1);
                let (result, reason, output) = compare(&c, &rust, &stream, alignment, capacity);
                assert_eq!(result, 1);
                assert_eq!(reason, None);
                assert_eq!(&output[..payload.len()], payload);
            }
        }
    }

    for length in [3, 11, 12, 31, 67, 130, 258] {
        let tokens = [
            Token::Literal(b'Q'),
            Token::Match {
                length,
                distance: 1,
            },
        ];
        let expected = expected_from_tokens(&tokens);
        let (result, reason, output) = compare(
            &c,
            &rust,
            &fixed_stream(&tokens),
            length & 3,
            expected.len(),
        );
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for &(distance, length) in &[(2, 3), (4, 11), (8, 12), (9, 31)] {
        let mut tokens: Vec<_> = b"ABCDABCDX".iter().copied().map(Token::Literal).collect();
        tokens.push(Token::Match { length, distance });
        let expected = expected_from_tokens(&tokens);
        let (result, reason, output) = compare(
            &c,
            &rust,
            &fixed_stream(&tokens),
            distance & 3,
            expected.len() + 3,
        );
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..128 {
        let length = 3 + (next_random(&mut random) % 256) as usize;
        let tokens = [
            Token::Literal(next_random(&mut random) as u8),
            Token::Match {
                length,
                distance: 1,
            },
        ];
        let expected = expected_from_tokens(&tokens);
        let capacity = expected.len() + (next_random(&mut random) & 1) as usize;
        let alignment = (next_random(&mut random) & 3) as usize;
        let (result, reason, output) =
            compare(&c, &rust, &fixed_stream(&tokens), alignment, capacity);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..128 {
        let distance = 2 + (next_random(&mut random) % 63) as usize;
        let length = 3 + (next_random(&mut random) % 256) as usize;
        let mut tokens: Vec<_> = (0..64)
            .map(|_| Token::Literal(next_random(&mut random) as u8))
            .collect();
        tokens.push(Token::Match { length, distance });
        let expected = expected_from_tokens(&tokens);
        let capacity = expected.len() + (next_random(&mut random) & 1) as usize;
        let alignment = (next_random(&mut random) & 3) as usize;
        let (result, reason, output) =
            compare(&c, &rust, &fixed_stream(&tokens), alignment, capacity);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..64 {
        let left_len = (next_random(&mut random) % 24) as usize;
        let right_len = (next_random(&mut random) % 24) as usize;
        let left: Vec<_> = (0..left_len)
            .map(|_| Token::Literal(next_random(&mut random) as u8))
            .collect();
        let right: Vec<_> = (0..right_len)
            .map(|_| Token::Literal(next_random(&mut random) as u8))
            .collect();
        let mut bits = Bits::default();
        append_fixed(&mut bits, false, &left);
        append_fixed(&mut bits, true, &right);
        let stream = bits.finish();
        let mut expected = expected_from_tokens(&left);
        expected.extend(expected_from_tokens(&right));
        let (result, reason, output) = compare(
            &c,
            &rust,
            &stream,
            (left_len + right_len) & 3,
            expected.len(),
        );
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }
}

#[test]
fn valid_dynamic_configurations_match() {
    let _guard = lock_tests();
    let (c, rust) = load_pair();
    let mut random = 0x1234_5678_9abc_def0;

    for alignment in 0..4 {
        for length in [0, 1, 2, 7, 32, 127] {
            for _ in 0..20 {
                let payload: Vec<_> = (0..length)
                    .map(|_| b'A' + (next_random(&mut random) & 3) as u8)
                    .collect();
                for stream in [
                    dynamic_literal_stream(&payload),
                    dynamic_repeat_stream(&payload),
                ] {
                    let capacity = payload.len() + usize::from(alignment == 2);
                    let (result, reason, output) = compare(&c, &rust, &stream, alignment, capacity);
                    assert_eq!(result, 1);
                    assert_eq!(reason, None);
                    assert_eq!(&output[..payload.len()], payload);
                }
            }
        }
    }

    for length in [3, 11, 12] {
        let tokens = [
            Token::Literal(b'A'),
            Token::Match {
                length,
                distance: 1,
            },
        ];
        let expected = expected_from_tokens(&tokens);
        let stream = dynamic_match_stream(&tokens, 0);
        let (result, reason, output) = compare(&c, &rust, &stream, length & 3, expected.len());
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..128 {
        let length = [3, 11, 12][(next_random(&mut random) % 3) as usize];
        let tokens = [
            Token::Literal(b'A'),
            Token::Match {
                length,
                distance: 1,
            },
        ];
        let expected = expected_from_tokens(&tokens);
        let stream = dynamic_match_stream(&tokens, 0);
        let capacity = expected.len() + (next_random(&mut random) & 1) as usize;
        let alignment = (next_random(&mut random) & 3) as usize;
        let (result, reason, output) = compare(&c, &rust, &stream, alignment, capacity);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for length in [3, 11, 12] {
        let mut tokens: Vec<_> = b"ABCDABCD".iter().copied().map(Token::Literal).collect();
        tokens.push(Token::Match {
            length,
            distance: 8,
        });
        let expected = expected_from_tokens(&tokens);
        let stream = dynamic_match_stream(&tokens, 5);
        let (result, reason, output) =
            compare(&c, &rust, &stream, (length + 1) & 3, expected.len() + 2);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..128 {
        let length = [3, 11, 12][(next_random(&mut random) % 3) as usize];
        let distance = 7 + (next_random(&mut random) & 1) as usize;
        let mut tokens: Vec<_> = b"ABCDABCD".iter().copied().map(Token::Literal).collect();
        tokens.push(Token::Match { length, distance });
        let expected = expected_from_tokens(&tokens);
        let stream = dynamic_match_stream(&tokens, 5);
        let capacity = expected.len() + (next_random(&mut random) & 1) as usize;
        let alignment = (next_random(&mut random) & 3) as usize;
        let (result, reason, output) = compare(&c, &rust, &stream, alignment, capacity);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    for _ in 0..64 {
        let fixed_payload: Vec<_> = (0..(next_random(&mut random) % 32))
            .map(|_| b'A' + (next_random(&mut random) & 3) as u8)
            .collect();
        let dynamic_payload: Vec<_> = (0..(next_random(&mut random) % 32))
            .map(|_| b'A' + (next_random(&mut random) & 3) as u8)
            .collect();

        let fixed_tokens: Vec<_> = fixed_payload.iter().copied().map(Token::Literal).collect();
        let mut fixed_then_dynamic = Bits::default();
        append_fixed(&mut fixed_then_dynamic, false, &fixed_tokens);
        append_dynamic_literals(&mut fixed_then_dynamic, true, &dynamic_payload);
        let mut expected = fixed_payload.clone();
        expected.extend_from_slice(&dynamic_payload);
        let stream = fixed_then_dynamic.finish();
        let (result, reason, output) =
            compare(&c, &rust, &stream, expected.len() & 3, expected.len());
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);

        let mut dynamic_then_fixed = Bits::default();
        append_dynamic_literals(&mut dynamic_then_fixed, false, &dynamic_payload);
        append_fixed(&mut dynamic_then_fixed, true, &fixed_tokens);
        let mut expected = dynamic_payload.clone();
        expected.extend_from_slice(&fixed_payload);
        let stream = dynamic_then_fixed.finish();
        let (result, reason, output) =
            compare(&c, &rust, &stream, stream.len() & 3, expected.len() + 1);
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..expected.len()], expected);
    }

    let mut observed_tails = [false; 4];
    for length in 0..256 {
        let payload: Vec<_> = (0..length).map(|index| b'A' + (index & 3) as u8).collect();
        let stream = dynamic_literal_stream(&payload);
        observed_tails[stream.len() & 3] = true;
        let (result, reason, output) = compare(&c, &rust, &stream, length & 3, payload.len());
        assert_eq!((result, reason), (1, None));
        assert_eq!(&output[..payload.len()], payload);
        if observed_tails.iter().all(|observed| *observed) {
            break;
        }
    }
    assert!(observed_tails.iter().all(|observed| *observed));
}

fn assert_error(c: &Api, rust: &Api, stream: &[u8], capacity: usize, expected_reason: &str) {
    let (result, reason, _) = compare(c, rust, stream, 0, capacity);
    assert_eq!(result, 0);
    assert_eq!(reason.as_deref(), Some(expected_reason));
}

#[test]
fn explicit_error_configurations_match() {
    let _guard = lock_tests();
    let (c, rust) = load_pair();

    assert_error(
        &c,
        &rust,
        &stored_stream(b"x", &[], false),
        1,
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
    );
    assert_error(
        &c,
        &rust,
        &stored_stream(&[], &[0], true),
        0,
        "Stored block extends beyond end of input stream.",
    );
    assert_error(
        &c,
        &rust,
        &fixed_stream(&[Token::Literal(b'A')]),
        0,
        "Attempted to overwrite out buffer while outputting a symbol.",
    );
    assert_error(
        &c,
        &rust,
        &fixed_stream(&[Token::Match {
            length: 3,
            distance: 1,
        }]),
        3,
        "Attempted to write before out buffer (invalid backwards distance).",
    );
    assert_error(
        &c,
        &rust,
        &fixed_stream(&[
            Token::Literal(b'A'),
            Token::Match {
                length: 3,
                distance: 1,
            },
        ]),
        3,
        "Attempted to overwrite out buffer while outputting a string.",
    );
    assert_error(
        &c,
        &rust,
        &[0b0000_0111],
        0,
        "Detected unknown block type within input stream.",
    );
}

fn child_status(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("crash_case_driver")
        .arg("--nocapture")
        .env("PINFLATE_CHILD_LIBRARY", library)
        .env("PINFLATE_CHILD_CASE", case)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn assert_same_signal(case: &str) {
    use std::os::unix::process::ExitStatusExt;
    let c = child_status(C_LIBRARY, case);
    let rust = child_status(RUST_LIBRARY, case);
    assert_eq!(rust.signal(), c.signal(), "{case}: C={c:?}, Rust={rust:?}");
    assert!(c.signal().is_some(), "{case} did not terminate by signal");
}

#[test]
fn process_error_boundaries_match() {
    let _guard = lock_tests();
    for case in [
        "zero_input",
        "null_input",
        "null_output",
        "short_stored_header",
        "dynamic_bit_overflow",
        "oversized_bit_request",
        "oversized_code_length",
        "invalid_huffman_prefix",
    ] {
        assert_same_signal(case);
    }
}

#[test]
fn unreachable_c_assertions_remain_exact_rust_assertions() {
    let _guard = lock_tests();
    let c = include_str!("../../c_src/src/lib.c");
    let rust = include_str!("../src/lib.rs");
    let invariant_pairs = [
        (
            "assert(!(s->bits_left & 7));",
            "assert_eq!(s.bits_left & 7, 0);",
        ),
        (
            "assert(s->word_index <= s->word_count);",
            "assert!(s.word_index <= s.word_count);",
        ),
        (
            "assert(num_bits_to_read >= 0);",
            "assert!(num_bits_to_read >= 0);",
        ),
        ("assert(s->count <= 64);", "assert!(s.count <= 64);"),
    ];
    for (c_assertion, rust_assertion) in invariant_pairs {
        assert!(c.contains(c_assertion), "missing C invariant {c_assertion}");
        assert!(
            rust.contains(rust_assertion),
            "missing Rust invariant {rust_assertion}"
        );
    }
}

#[test]
fn crash_case_driver() {
    let Ok(library) = std::env::var("PINFLATE_CHILD_LIBRARY") else {
        return;
    };
    let case = std::env::var("PINFLATE_CHILD_CASE").unwrap();
    let api = unsafe { Api::load(&manifest_path(&library)) };
    let mut output = [0_u8; 8];
    let mut one_byte = [0_u8; 1];
    let mut literal = fixed_stream(&[Token::Literal(b'A')]);

    unsafe {
        match case.as_str() {
            "zero_input" => {
                (api.pinflate)(
                    one_byte.as_mut_ptr().cast(),
                    0,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "null_input" => {
                (api.pinflate)(
                    std::ptr::null_mut(),
                    1,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "null_output" => {
                (api.pinflate)(
                    literal.as_mut_ptr().cast(),
                    literal.len() as c_int,
                    std::ptr::null_mut(),
                    1,
                );
            }
            "short_stored_header" => {
                (api.pinflate)(
                    [1_u8, 0].as_mut_ptr().cast(),
                    2,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "dynamic_bit_overflow" => {
                (api.pinflate)(
                    [5_u8, 0, 0].as_mut_ptr().cast(),
                    3,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "oversized_bit_request" => {
                let table = *api._library.get::<*mut u8>(b"cp_len_extra_bits\0").unwrap();
                table.write(33);
                let mut stream = fixed_stream(&[
                    Token::Literal(b'A'),
                    Token::Match {
                        length: 3,
                        distance: 1,
                    },
                ]);
                (api.pinflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "oversized_code_length" => {
                let table = *api._library.get::<*mut u8>(b"cp_fixed_table\0").unwrap();
                table.write(16);
                (api.pinflate)(
                    literal.as_mut_ptr().cast(),
                    literal.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            "invalid_huffman_prefix" => {
                let table = *api._library.get::<*mut u8>(b"cp_fixed_table\0").unwrap();
                std::ptr::write_bytes(table, 0, 320);
                table.add(65).write(1);
                table.add(288).write(1);
                let mut stream = [0b0000_1011_u8];
                (api.pinflate)(
                    stream.as_mut_ptr().cast(),
                    stream.len() as c_int,
                    output.as_mut_ptr().cast(),
                    output.len() as c_int,
                );
            }
            _ => panic!("unknown crash case {case}"),
        }
    }
    std::process::exit(64);
}
