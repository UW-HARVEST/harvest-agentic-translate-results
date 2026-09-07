use libloading::Library;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

type Pinflate = unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, c_int) -> c_int;

struct Api {
    _library: Library,
    pinflate: Pinflate,
    reason: *mut *const c_char,
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    result: c_int,
    output: Vec<u8>,
    reason: Option<String>,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }.unwrap();
        let pinflate = unsafe { *library.get::<Pinflate>(b"pinflate").unwrap() };
        let reason = unsafe {
            *library
                .get::<*mut *const c_char>(b"cp_error_reason")
                .unwrap()
        };
        Self {
            _library: library,
            pinflate,
            reason,
        }
    }

    unsafe fn call(&self, input: &[u8], output_len: usize, first_bytes: usize) -> Outcome {
        let mut storage = vec![0_u8; input.len() + 8];
        let wanted_mod = (4 - first_bytes) & 3;
        let start = (0..4)
            .find(|offset| (storage.as_ptr() as usize + offset) & 3 == wanted_mod)
            .unwrap();
        storage[start..start + input.len()].copy_from_slice(input);
        let mut output = vec![0xa5_u8; output_len.max(1)];
        unsafe { *self.reason = ptr::null() };
        let result = unsafe {
            (self.pinflate)(
                storage.as_mut_ptr().add(start).cast(),
                input.len() as c_int,
                output.as_mut_ptr().cast(),
                output_len as c_int,
            )
        };
        let reason_ptr = unsafe { *self.reason };
        let reason = if reason_ptr.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(reason_ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        output.truncate(output_len);
        Outcome {
            result,
            output,
            reason,
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root
            .parent()
            .unwrap()
            .join("c_src/build/libharvest-work-kvAlTr.so"),
        crate_root.join("target/release/libpinflate_lib.so"),
    )
}

fn apis() -> (Api, Api) {
    let (c, rust) = library_paths();
    assert!(c.exists(), "missing C library: {}", c.display());
    assert!(
        rust.exists(),
        "missing release Rust library: {}",
        rust.display()
    );
    unsafe { (Api::load(&c), Api::load(&rust)) }
}

fn compare(c: &Api, rust: &Api, input: &[u8], output_len: usize, first_bytes: usize) {
    let c_outcome = unsafe { c.call(input, output_len, first_bytes) };
    let rust_outcome = unsafe { rust.call(input, output_len, first_bytes) };
    assert_eq!(
        c_outcome,
        rust_outcome,
        "input={}, output_len={output_len}, first_bytes={first_bytes}",
        hex(input)
    );
}

fn compare_success(
    c: &Api,
    rust: &Api,
    input: &[u8],
    expected: &[u8],
    output_len: usize,
    first_bytes: usize,
) {
    compare(c, rust, input, output_len, first_bytes);
    let outcome = unsafe { c.call(input, output_len, first_bytes) };
    assert_eq!(outcome.result, 1, "C rejected {}", hex(input));
    assert_eq!(&outcome.output[..expected.len()], expected);
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    count: usize,
}

impl Bits {
    fn write(&mut self, value: u32, width: usize) {
        for bit in 0..width {
            if self.count & 7 == 0 {
                self.bytes.push(0);
            }
            self.bytes[self.count / 8] |= (((value >> bit) & 1) as u8) << (self.count & 7);
            self.count += 1;
        }
    }

    fn align_byte(&mut self) {
        while self.count & 7 != 0 {
            self.write(0, 1);
        }
    }

    fn symbol(&mut self, lengths: &[u8], symbol: usize) {
        let codes = canonical_codes(lengths);
        let (code, width) = codes[symbol];
        self.write(reverse(code, width), width as usize);
    }
}

fn reverse(mut value: u32, width: u8) -> u32 {
    let mut reversed = 0;
    for _ in 0..width {
        reversed = (reversed << 1) | (value & 1);
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
    for width in 1..=15 {
        code = (code + counts[width - 1]) << 1;
        next[width] = code;
    }
    lengths
        .iter()
        .map(|&width| {
            if width == 0 {
                (0, 0)
            } else {
                let result = next[width as usize];
                next[width as usize] += 1;
                (result, width)
            }
        })
        .collect()
}

fn fixed_lengths() -> Vec<u8> {
    let mut lengths = vec![5; 320];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..288].fill(8);
    lengths
}

const LEN_BASE: [u32; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LEN_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u32; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

#[derive(Clone, Copy)]
enum Token {
    Literal(u8),
    Match(u32, u32),
}

fn write_tokens(bits: &mut Bits, lit: &[u8], dist: &[u8], tokens: &[Token]) {
    for token in tokens {
        match *token {
            Token::Literal(byte) => bits.symbol(lit, byte as usize),
            Token::Match(length, distance) => {
                let li = (0..LEN_BASE.len())
                    .find(|&i| {
                        let span = if LEN_EXTRA[i] == 0 {
                            1
                        } else {
                            1 << LEN_EXTRA[i]
                        };
                        length >= LEN_BASE[i] && length < LEN_BASE[i] + span
                    })
                    .unwrap();
                bits.symbol(lit, 257 + li);
                bits.write(length - LEN_BASE[li], LEN_EXTRA[li] as usize);
                let di = (0..DIST_BASE.len())
                    .find(|&i| {
                        let span = if DIST_EXTRA[i] == 0 {
                            1
                        } else {
                            1 << DIST_EXTRA[i]
                        };
                        distance >= DIST_BASE[i] && distance < DIST_BASE[i] + span
                    })
                    .unwrap();
                bits.symbol(dist, di);
                bits.write(distance - DIST_BASE[di], DIST_EXTRA[di] as usize);
            }
        }
    }
    bits.symbol(lit, 256);
}

fn fixed_block(bits: &mut Bits, final_block: bool, tokens: &[Token]) {
    bits.write(final_block as u32, 1);
    bits.write(1, 2);
    let lengths = fixed_lengths();
    write_tokens(bits, &lengths[..288], &lengths[288..], tokens);
}

fn fixed_stream(tokens: &[Token]) -> Vec<u8> {
    let mut bits = Bits::default();
    fixed_block(&mut bits, true, tokens);
    bits.bytes
}

fn stored_stream(payload: &[u8]) -> Vec<u8> {
    let mut bits = Bits::default();
    bits.write(1, 1);
    bits.write(0, 2);
    bits.align_byte();
    bits.write(payload.len() as u32, 16);
    bits.write(!(payload.len() as u16) as u32, 16);
    for &byte in payload {
        bits.write(byte as u32, 8);
    }
    bits.bytes
}

const CL_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

fn dynamic_header(
    bits: &mut Bits,
    nlit: usize,
    ndist: usize,
    cl_lengths: &[u8; 19],
    ncl: usize,
    encoded_lengths: &[(usize, u32, usize)],
) {
    bits.write(1, 1);
    bits.write(2, 2);
    bits.write((nlit - 257) as u32, 5);
    bits.write((ndist - 1) as u32, 5);
    bits.write((ncl - 4) as u32, 4);
    for &symbol in &CL_ORDER[..ncl] {
        bits.write(cl_lengths[symbol] as u32, 3);
    }
    for &(symbol, extra, width) in encoded_lengths {
        bits.symbol(cl_lengths, symbol);
        bits.write(extra, width);
    }
}

fn zeros_17(encoded: &mut Vec<(usize, u32, usize)>, mut count: usize) {
    while count >= 3 {
        let take = count.min(10);
        encoded.push((17, (take - 3) as u32, 3));
        count -= take;
    }
    encoded.extend((0..count).map(|_| (0, 0, 0)));
}

fn zeros_18(encoded: &mut Vec<(usize, u32, usize)>, mut count: usize) {
    while count >= 11 {
        let take = count.min(138);
        encoded.push((18, (take - 11) as u32, 7));
        count -= take;
    }
    encoded.extend((0..count).map(|_| (0, 0, 0)));
}

fn dynamic_stream(mode: u8, payload_len: usize) -> Vec<u8> {
    let mut bits = Bits::default();
    let mut cl = [0_u8; 19];
    let mut encoded = Vec::new();
    match mode {
        0 => {
            cl[0] = 1;
            cl[1] = 1;
            encoded.extend((0..65).map(|_| (0, 0, 0)));
            encoded.push((1, 0, 0));
            encoded.extend((0..190).map(|_| (0, 0, 0)));
            encoded.push((1, 0, 0));
            encoded.push((1, 0, 0));
        }
        16 => {
            for symbol in [0, 1, 3, 16, 18] {
                cl[symbol] = 3;
            }
            zeros_18(&mut encoded, 65);
            encoded.push((3, 0, 0));
            encoded.push((16, 0, 2));
            zeros_18(&mut encoded, 187);
            encoded.push((3, 0, 0));
            encoded.push((1, 0, 0));
        }
        17 => {
            for symbol in [0, 1, 17, 18] {
                cl[symbol] = 2;
            }
            zeros_17(&mut encoded, 65);
            encoded.push((1, 0, 0));
            zeros_17(&mut encoded, 190);
            encoded.push((1, 0, 0));
            encoded.push((1, 0, 0));
        }
        18 => {
            for symbol in [0, 1, 17, 18] {
                cl[symbol] = 2;
            }
            zeros_18(&mut encoded, 65);
            encoded.push((1, 0, 0));
            zeros_18(&mut encoded, 190);
            encoded.push((1, 0, 0));
            encoded.push((1, 0, 0));
        }
        _ => unreachable!(),
    }
    dynamic_header(&mut bits, 257, 1, &cl, 18, &encoded);
    let mut lit = vec![0_u8; 257];
    let data_width = if mode == 16 { 3 } else { 1 };
    if mode == 16 {
        lit[65..=68].fill(data_width);
    } else {
        lit[65] = data_width;
    }
    lit[256] = data_width;
    let dist = [1_u8];
    let tokens = vec![Token::Literal(b'A'); payload_len];
    write_tokens(&mut bits, &lit, &dist, &tokens);
    bits.bytes
}

fn dynamic_copy_stream(matches: usize) -> Vec<u8> {
    let mut bits = Bits::default();
    let mut cl = [0_u8; 19];
    for symbol in [0, 1, 2, 18] {
        cl[symbol] = 2;
    }
    let mut encoded = Vec::new();
    zeros_18(&mut encoded, 65);
    encoded.push((2, 0, 0));
    encoded.push((2, 0, 0));
    zeros_18(&mut encoded, 189);
    encoded.push((2, 0, 0));
    encoded.push((2, 0, 0));
    encoded.push((1, 0, 0));
    dynamic_header(&mut bits, 258, 1, &cl, 18, &encoded);
    let mut lit = vec![0_u8; 258];
    for symbol in [65, 66, 256, 257] {
        lit[symbol] = 2;
    }
    let dist = [1_u8];
    let mut tokens = vec![Token::Literal(b'A')];
    tokens.extend((0..matches).map(|_| Token::Match(3, 1)));
    write_tokens(&mut bits, &lit, &dist, &tokens);
    bits.bytes
}

fn random_bytes(state: &mut u64, length: usize) -> Vec<u8> {
    (0..length)
        .map(|_| {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            *state as u8
        })
        .collect()
}

#[test]
fn phase_b_stored_fixed_and_shape_rows() {
    let (c, rust) = apis();
    compare_success(&c, &rust, &stored_stream(&[]), &[], 0, 0);
    compare_success(&c, &rust, &stored_stream(b"x"), b"x", 1, 1);

    let mut seed = 0x43a1_7b9d_d00d_f00d;
    for length in [2, 3, 4, 7, 31, 255, 1024] {
        for _ in 0..20 {
            let payload = random_bytes(&mut seed, length);
            let stream = stored_stream(&payload);
            compare_success(&c, &rust, &stream, &payload, length, length & 3);
            compare_success(&c, &rust, &stream, &payload, length + 9, (length + 1) & 3);
        }
    }

    compare_success(&c, &rust, &fixed_stream(&[]), &[], 0, 2);
    for length in [1, 2, 7, 64, 257] {
        for _ in 0..30 {
            let payload = random_bytes(&mut seed, length);
            let tokens: Vec<_> = payload.iter().copied().map(Token::Literal).collect();
            compare_success(
                &c,
                &rust,
                &fixed_stream(&tokens),
                &payload,
                length,
                length & 3,
            );
        }
    }

    let distance_one = fixed_stream(&[Token::Literal(b'Q'), Token::Match(40, 1)]);
    compare_success(&c, &rust, &distance_one, &vec![b'Q'; 41], 41, 3);
    let distance_many = fixed_stream(&[
        Token::Literal(b'a'),
        Token::Literal(b'b'),
        Token::Literal(b'c'),
        Token::Match(30, 3),
    ]);
    compare_success(&c, &rust, &distance_many, &b"abc".repeat(11), 33, 0);
    let extras = fixed_stream(&[
        Token::Literal(b'a'),
        Token::Literal(b'b'),
        Token::Literal(b'c'),
        Token::Literal(b'd'),
        Token::Literal(b'e'),
        Token::Match(11, 5),
        Token::Match(258, 1),
    ]);
    let mut expected = b"abcdeabcdeabcdea".to_vec();
    expected.extend(std::iter::repeat_n(b'a', 258));
    compare_success(&c, &rust, &extras, &expected, expected.len(), 1);

    let mut multi = Bits::default();
    fixed_block(&mut multi, false, &[Token::Literal(b'a')]);
    fixed_block(
        &mut multi,
        true,
        &[Token::Literal(b'b'), Token::Literal(b'c')],
    );
    compare_success(&c, &rust, &multi.bytes, b"abc", 3, 2);

    for first_bytes in 0..4 {
        compare_success(
            &c,
            &rust,
            &fixed_stream(&[Token::Literal(b'z')]),
            b"z",
            1,
            first_bytes,
        );
    }
    for tail in 0..4 {
        let mut stream = fixed_stream(&[]);
        while stream.len() & 3 != tail {
            stream.push(0);
        }
        compare_success(&c, &rust, &stream, &[], 0, 0);
    }
}

#[test]
fn phase_b_dynamic_rows() {
    let (c, rust) = apis();
    for mode in [0, 16, 17, 18] {
        for length in [0, 1, 2, 9, 63] {
            let expected = vec![b'A'; length];
            compare_success(
                &c,
                &rust,
                &dynamic_stream(mode, length),
                &expected,
                length,
                mode as usize & 3,
            );
        }
    }
    for matches in [1, 2, 7, 31] {
        let expected = vec![b'A'; 1 + 3 * matches];
        compare_success(
            &c,
            &rust,
            &dynamic_copy_stream(matches),
            &expected,
            expected.len(),
            matches & 3,
        );
    }
}

#[test]
fn phase_c_explicit_error_rows() {
    let (c, rust) = apis();
    let cases = [
        (
            vec![1, 1, 0, 0, 0],
            8,
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        ),
        (
            vec![1, 0, 0, 0xff, 0xff, 0],
            8,
            "Stored block extends beyond end of input stream.",
        ),
        (
            fixed_stream(&[Token::Literal(b'x')]),
            0,
            "Attempted to overwrite out buffer while outputting a symbol.",
        ),
        (
            fixed_stream(&[Token::Match(3, 1)]),
            8,
            "Attempted to write before out buffer (invalid backwards distance).",
        ),
        (
            fixed_stream(&[Token::Literal(b'x'), Token::Match(3, 1)]),
            3,
            "Attempted to overwrite out buffer while outputting a string.",
        ),
        (
            vec![7],
            8,
            "Detected unknown block type within input stream.",
        ),
    ];
    for (input, output_len, expected_reason) in cases {
        compare(&c, &rust, &input, output_len, 0);
        let outcome = unsafe { c.call(&input, output_len, 0) };
        assert_eq!(outcome.result, 0);
        assert_eq!(outcome.reason.as_deref(), Some(expected_reason));
    }
}

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

fn child_status(path: &Path, input: Option<&[u8]>, in_len: c_int, out_len: c_int) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0);
        if pid == 0 {
            close(2);
            let api = Api::load(path);
            let mut input_storage = input.unwrap_or(&[]).to_vec();
            if input_storage.is_empty() {
                input_storage.push(0);
            }
            let mut output = vec![0_u8; 16];
            let input_ptr = if input.is_none() {
                ptr::null_mut()
            } else {
                input_storage.as_mut_ptr().cast()
            };
            let output_ptr = if out_len == c_int::MIN {
                ptr::null_mut()
            } else {
                output.as_mut_ptr().cast()
            };
            let actual_out_len = if out_len == c_int::MIN { 1 } else { out_len };
            let result = (api.pinflate)(input_ptr, in_len, output_ptr, actual_out_len);
            _exit((result & 0xff) as c_int);
        }
        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);
        status
    }
}

#[test]
fn phase_c_crash_and_generic_boundary_rows() {
    let (c_path, rust_path) = library_paths();
    let cases: Vec<(Option<Vec<u8>>, c_int, c_int)> = vec![
        (Some(vec![]), 0, 0),
        (None, 0, 0),
        (None, 1, 1),
        (Some(fixed_stream(&[Token::Literal(b'x')])), 2, c_int::MIN),
        (Some(vec![7, 0, 0, 0]), c_int::MAX, 8),
        (Some(vec![0]), -1, 8),
        (Some(fixed_stream(&[])), 2, -1),
    ];
    for (input, in_len, out_len) in cases {
        let c_status = child_status(&c_path, input.as_deref(), in_len, out_len);
        let rust_status = child_status(&rust_path, input.as_deref(), in_len, out_len);
        assert_eq!(
            c_status, rust_status,
            "boundary mismatch input={input:?}, in_len={in_len}, out_len={out_len}"
        );
    }
}

#[test]
fn translated_assertion_surface_is_release_active() {
    let source = include_str!("../src/lib.rs");
    assert_eq!(source.matches("assert!(").count(), 8);
    assert_eq!(source.matches("assert_eq!(").count(), 2);
    assert!(!source.contains("debug_assert"));
}

#[test]
fn phase_d_exported_data_symbols_match_byte_for_byte() {
    let (c_path, rust_path) = library_paths();
    unsafe {
        let c = Library::new(c_path).unwrap();
        let rust = Library::new(rust_path).unwrap();
        for (name, size) in [
            (&b"cp_fixed_table\0"[..], 320),
            (&b"cp_permutation_order\0"[..], 19),
            (&b"cp_len_extra_bits\0"[..], 31),
            (&b"cp_len_base\0"[..], 31 * 4),
            (&b"cp_dist_extra_bits\0"[..], 32),
            (&b"cp_dist_base\0"[..], 32 * 4),
        ] {
            let c_ptr = *c.get::<*const u8>(name).unwrap();
            let rust_ptr = *rust.get::<*const u8>(name).unwrap();
            assert_eq!(
                std::slice::from_raw_parts(c_ptr, size),
                std::slice::from_raw_parts(rust_ptr, size),
                "data symbol {} differs",
                CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()
            );
        }
        let _: libloading::Symbol<'_, *mut *const c_char> = c.get(b"cp_error_reason").unwrap();
        let _: libloading::Symbol<'_, *mut *const c_char> = rust.get(b"cp_error_reason").unwrap();
    }
}
