use libloading::Library;
use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::ptr;

type Hex2Bin = unsafe extern "C" fn(
    *mut u8,
    usize,
    *const c_char,
    usize,
    *const c_char,
    *mut *const c_char,
) -> c_int;

const CASES: usize = 128;
const SENTINEL: u8 = 0xa5;

#[derive(Clone, Debug)]
struct Spec {
    input: Option<Vec<u8>>,
    hex_len: usize,
    ignore: Option<Vec<u8>>,
    bin_storage_len: usize,
    bin_maxlen: usize,
    null_bin: bool,
    endpoint: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    ret: c_int,
    bin: Vec<u8>,
    end_offset: Option<usize>,
}

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
        (self.next_u64() as usize) % upper
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_build = manifest.join("../c_src/build");
    let mut c_libraries: Vec<_> = std::fs::read_dir(&c_build)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", c_build.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    c_libraries.sort();
    assert_eq!(
        c_libraries.len(),
        1,
        "expected one C shared library in {}",
        c_build.display()
    );
    (
        c_libraries.pop().unwrap(),
        manifest.join("target/release/libhex2bin_lib.so"),
    )
}

fn with_functions<T>(body: impl FnOnce(Hex2Bin, Hex2Bin) -> T) -> T {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing release Rust library: {}; run cargo build --release",
        rust_path.display()
    );

    unsafe {
        let c_library = Library::new(&c_path).unwrap();
        let rust_library = Library::new(&rust_path).unwrap();
        let c_function: Hex2Bin = *c_library.get(b"hex2bin\0").unwrap();
        let rust_function: Hex2Bin = *rust_library.get(b"hex2bin\0").unwrap();
        body(c_function, rust_function)
    }
}

fn invoke(function: Hex2Bin, spec: &Spec) -> Outcome {
    let input = spec.input.clone();
    let hex = input
        .as_ref()
        .map_or(ptr::null(), |bytes| bytes.as_ptr().cast::<c_char>());

    let mut ignore_storage = spec.ignore.clone();
    if let Some(bytes) = &mut ignore_storage {
        assert!(!bytes.contains(&0), "ignore payload must be a C string");
        bytes.push(0);
    }
    let ignore = ignore_storage
        .as_ref()
        .map_or(ptr::null(), |bytes| bytes.as_ptr().cast::<c_char>());

    let mut bin = vec![SENTINEL; spec.bin_storage_len];
    let bin_pointer = if spec.null_bin {
        ptr::null_mut()
    } else {
        bin.as_mut_ptr()
    };

    let mut end = usize::MAX as *const c_char;
    let end_pointer = if spec.endpoint {
        &mut end
    } else {
        ptr::null_mut()
    };

    let ret = unsafe {
        function(
            bin_pointer,
            spec.bin_maxlen,
            hex,
            spec.hex_len,
            ignore,
            end_pointer,
        )
    };
    let end_offset = spec
        .endpoint
        .then(|| (end as usize).wrapping_sub(hex as usize));

    Outcome {
        ret,
        bin,
        end_offset,
    }
}

fn assert_match(c_function: Hex2Bin, rust_function: Hex2Bin, spec: &Spec, row: &str) -> Outcome {
    let c = invoke(c_function, spec);
    let rust = invoke(rust_function, spec);
    assert_eq!(rust, c, "{row} diverged for {spec:?}");
    c
}

fn valid_spec(
    input: Vec<u8>,
    decoded_len: usize,
    bin_maxlen: usize,
    endpoint: bool,
    ignore: Option<Vec<u8>>,
) -> Spec {
    let hex_len = input.len();
    Spec {
        input: Some(input),
        hex_len,
        ignore,
        bin_storage_len: decoded_len + 4,
        bin_maxlen,
        null_bin: false,
        endpoint,
    }
}

fn alpha(nibble: u8, uppercase: bool) -> u8 {
    debug_assert!((10..=15).contains(&nibble));
    (if uppercase { b'A' } else { b'a' }) + nibble - 10
}

fn mixed_hex(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        for nibble in [byte >> 4, byte & 0x0f] {
            encoded.push(if nibble < 10 {
                b'0' + nibble
            } else {
                alpha(nibble, rng.usize(2) == 0)
            });
        }
    }
    encoded
}

fn random_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

#[test]
fn phase_b_all_configuration_rows() {
    with_functions(|c_function, rust_function| {
        let mut rng = Rng::new(0x8b5a_2d91_74c3_e60f);

        // C01: empty input and every data pointer null.
        for _ in 0..CASES {
            let spec = Spec {
                input: None,
                hex_len: 0,
                ignore: None,
                bin_storage_len: 0,
                bin_maxlen: rng.next_u64() as usize,
                null_bin: true,
                endpoint: false,
            };
            assert_eq!(assert_match(c_function, rust_function, &spec, "C01").ret, 0);
        }

        // C02: empty input with a writable endpoint.
        for _ in 0..CASES {
            let spec = Spec {
                input: Some(vec![rng.byte()]),
                hex_len: 0,
                ignore: None,
                bin_storage_len: 0,
                bin_maxlen: 0,
                null_bin: true,
                endpoint: true,
            };
            let outcome = assert_match(c_function, rust_function, &spec, "C02");
            assert_eq!((outcome.ret, outcome.end_offset), (0, Some(0)));
        }

        // C03: decimal nibbles, one byte, exact capacity, no endpoint.
        for _ in 0..CASES {
            let high = rng.usize(10) as u8;
            let low = rng.usize(10) as u8;
            let spec = valid_spec(vec![b'0' + high, b'0' + low], 1, 1, false, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C03");
            assert_eq!(outcome.ret, 1);
            assert_eq!(outcome.bin[0], high * 16 + low);
        }

        // C04: uppercase alpha nibbles and spare capacity.
        for _ in 0..CASES {
            let high = 10 + rng.usize(6) as u8;
            let low = 10 + rng.usize(6) as u8;
            let spec = valid_spec(
                vec![alpha(high, true), alpha(low, true)],
                1,
                1 + rng.usize(8),
                true,
                None,
            );
            let outcome = assert_match(c_function, rust_function, &spec, "C04");
            assert_eq!((outcome.ret, outcome.end_offset), (1, Some(2)));
        }

        // C05: lowercase alpha mixed with decimal nibbles.
        for case in 0..CASES {
            let digit = rng.usize(10) as u8;
            let letter = 10 + rng.usize(6) as u8;
            let input = if case % 2 == 0 {
                vec![alpha(letter, false), b'0' + digit]
            } else {
                vec![b'0' + digit, alpha(letter, false)]
            };
            let spec = valid_spec(input, 1, 1, case % 3 == 0, None);
            assert_eq!(assert_match(c_function, rust_function, &spec, "C05").ret, 1);
        }

        // C06: many bytes, all classifier paths, exact capacity.
        for _ in 0..CASES {
            let len = 2 + rng.usize(128);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let spec = valid_spec(input, len, len, rng.usize(2) == 0, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C06");
            assert_eq!(outcome.ret, len as c_int);
            assert_eq!(&outcome.bin[..len], bytes);
        }

        // C07: many bytes and capacity strictly larger than the result.
        for _ in 0..CASES {
            let len = 2 + rng.usize(128);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let spec = valid_spec(input, len, len + 1 + rng.usize(64), true, None);
            assert_eq!(
                assert_match(c_function, rust_function, &spec, "C07").ret,
                len as c_int
            );
        }

        // C08: repeated separators before, between, and after complete bytes.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let encoded = mixed_hex(&bytes, &mut rng);
            let separators = [b' ', b':', b'-'];
            let mut input = vec![b' ', b' '];
            for (index, pair) in encoded.chunks_exact(2).enumerate() {
                input.extend_from_slice(pair);
                let count = if index + 1 == len {
                    2
                } else {
                    1 + rng.usize(3)
                };
                for _ in 0..count {
                    input.push(separators[rng.usize(separators.len())]);
                }
            }
            let spec = valid_spec(input, len, len, true, Some(separators.to_vec()));
            let outcome = assert_match(c_function, rust_function, &spec, "C08");
            assert_eq!(outcome.ret, len as c_int);
            assert_eq!(&outcome.bin[..len], bytes);
        }

        // C09: ignore is configured but does not match valid input.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let spec = valid_spec(input, len, len, false, Some(b":_-".to_vec()));
            assert_eq!(
                assert_match(c_function, rust_function, &spec, "C09").ret,
                len as c_int
            );
        }

        // C10: every member of a multi-byte ignore set is exercised.
        for _ in 0..CASES {
            let len = 4 + rng.usize(32);
            let bytes = random_bytes(&mut rng, len);
            let encoded = mixed_hex(&bytes, &mut rng);
            let ignored = b" \t:,_-";
            let mut input = Vec::new();
            for (index, pair) in encoded.chunks_exact(2).enumerate() {
                input.push(ignored[index % ignored.len()]);
                input.extend_from_slice(pair);
            }
            input.extend_from_slice(ignored);
            let spec = valid_spec(
                input,
                bytes.len(),
                bytes.len(),
                true,
                Some(ignored.to_vec()),
            );
            assert_eq!(
                assert_match(c_function, rust_function, &spec, "C10").ret,
                bytes.len() as c_int
            );
        }

        // C11: strchr matches its own NUL terminator at byte boundaries.
        for _ in 0..CASES {
            let len = 1 + rng.usize(32);
            let bytes = random_bytes(&mut rng, len);
            let encoded = mixed_hex(&bytes, &mut rng);
            let mut input = vec![0];
            for pair in encoded.chunks_exact(2) {
                input.extend_from_slice(pair);
                input.push(0);
            }
            let spec = valid_spec(input, bytes.len(), bytes.len(), true, Some(Vec::new()));
            let outcome = assert_match(c_function, rust_function, &spec, "C11");
            assert_eq!(outcome.ret, bytes.len() as c_int);
            assert_eq!(&outcome.bin[..bytes.len()], bytes);
        }

        // C12: signed-char platforms still pass high-bit ignore bytes to strchr.
        for _ in 0..CASES {
            let separator = 0x80 + rng.usize(0x80) as u8;
            let len = 1 + rng.usize(32);
            let bytes = random_bytes(&mut rng, len);
            let encoded = mixed_hex(&bytes, &mut rng);
            let mut input = vec![separator];
            for pair in encoded.chunks_exact(2) {
                input.extend_from_slice(pair);
                input.push(separator);
            }
            let spec = valid_spec(input, bytes.len(), bytes.len(), true, Some(vec![separator]));
            assert_eq!(
                assert_match(c_function, rust_function, &spec, "C12").ret,
                bytes.len() as c_int
            );
        }

        // C13: a terminator after complete bytes is a successful partial parse.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let mut input = mixed_hex(&bytes, &mut rng);
            input.push(b'!');
            input.extend_from_slice(b"not parsed");
            let expected_end = len * 2;
            let spec = valid_spec(input, len, len, true, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C13");
            assert_eq!(
                (outcome.ret, outcome.end_offset),
                (len as c_int, Some(expected_end))
            );
        }

        // C14: an immediate terminator returns an empty successful prefix.
        const TERMINATORS: &[u8] = b"!gG/_,";
        for _ in 0..CASES {
            let terminator = TERMINATORS[rng.usize(TERMINATORS.len())];
            let spec = valid_spec(vec![terminator, b'0', b'0'], 0, 0, true, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C14");
            assert_eq!((outcome.ret, outcome.end_offset), (0, Some(0)));
        }

        // C15: high-bit bytes are also immediate non-hex terminators.
        for _ in 0..CASES {
            let terminator = 0x80 + rng.usize(0x80) as u8;
            let spec = valid_spec(vec![terminator, b'0', b'0'], 0, 0, true, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C15");
            assert_eq!((outcome.ret, outcome.end_offset), (0, Some(0)));
        }

        // C16: hex_len, rather than a terminator, delimits a backing buffer prefix.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let mut input = mixed_hex(&bytes, &mut rng);
            let prefix_len = input.len();
            input.extend_from_slice(b"0123456789abcdef!");
            let mut spec = valid_spec(input, len, len, rng.usize(2) == 0, None);
            spec.hex_len = prefix_len;
            let outcome = assert_match(c_function, rust_function, &spec, "C16");
            assert_eq!(outcome.ret, len as c_int);
            assert_eq!(&outcome.bin[..len], bytes);
        }

        // C17: a full parse stores exactly the one-past-end pointer.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let input_len = input.len();
            let spec = valid_spec(input, len, len, true, None);
            let outcome = assert_match(c_function, rust_function, &spec, "C17");
            assert_eq!(outcome.end_offset, Some(input_len));
        }

        // C18: huge lengths that remain defined because parsing short-circuits.
        for _ in 0..CASES {
            let len = 1 + rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let spec = valid_spec(input, len, usize::MAX, true, None);
            assert_eq!(
                assert_match(c_function, rust_function, &spec, "C18/bin_max").ret,
                len as c_int
            );

            let spec = Spec {
                input: Some(vec![b'!', rng.byte()]),
                hex_len: usize::MAX,
                ignore: None,
                bin_storage_len: 4,
                bin_maxlen: usize::MAX,
                null_bin: false,
                endpoint: true,
            };
            let outcome = assert_match(c_function, rust_function, &spec, "C18/hex_len");
            assert_eq!((outcome.ret, outcome.end_offset), (0, Some(0)));
        }
    });
}

#[test]
fn phase_c_e01_output_capacity_rejection() {
    with_functions(|c_function, rust_function| {
        let mut rng = Rng::new(0x1122_3344_5566_7788);
        for case in 0..CASES {
            let capacity = case % 33;
            let len = capacity + 1 + rng.usize(16);
            let bytes = random_bytes(&mut rng, len);
            let input = mixed_hex(&bytes, &mut rng);
            let mut spec = valid_spec(input, bytes.len(), capacity, true, None);
            spec.bin_storage_len = bytes.len() + 4;
            spec.null_bin = capacity == 0 && case % 2 == 0;
            let outcome = assert_match(c_function, rust_function, &spec, "E01");
            assert_eq!((outcome.ret, outcome.end_offset), (-1, Some(capacity * 2)));
            if !spec.null_bin {
                assert_eq!(&outcome.bin[..capacity], &bytes[..capacity]);
                assert!(outcome.bin[capacity..].iter().all(|byte| *byte == SENTINEL));
            }
        }
    });
}

#[test]
fn phase_c_e02_unmatched_high_nibble_rejection() {
    with_functions(|c_function, rust_function| {
        let mut rng = Rng::new(0xa1b2_c3d4_e5f6_0718);
        for case in 0..CASES {
            let len = rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let mut input = mixed_hex(&bytes, &mut rng);
            input.push(b"0123456789abcdefABCDEF"[rng.usize(22)]);
            if case % 2 != 0 {
                input.push(if case % 4 == 1 { b'!' } else { b':' });
            }
            let ignore = (case % 4 == 3).then(|| vec![b':']);
            let spec = valid_spec(input, len, len + 1, true, ignore);
            let outcome = assert_match(c_function, rust_function, &spec, "E02");
            assert_eq!((outcome.ret, outcome.end_offset), (-1, Some(len * 2)));
            assert_eq!(&outcome.bin[..len], bytes);
        }
    });
}

#[test]
fn phase_c_e03_missing_endpoint_rejects_partial_parse() {
    with_functions(|c_function, rust_function| {
        let mut rng = Rng::new(0x0f1e_2d3c_4b5a_6978);
        const TERMINATORS: &[u8] = b"!gG/_,";
        for _ in 0..CASES {
            let len = rng.usize(64);
            let bytes = random_bytes(&mut rng, len);
            let mut input = mixed_hex(&bytes, &mut rng);
            input.push(TERMINATORS[rng.usize(TERMINATORS.len())]);
            input.extend_from_slice(b"unconsumed");
            let spec = valid_spec(input, len, len + 1, false, None);
            let outcome = assert_match(c_function, rust_function, &spec, "E03");
            assert_eq!(outcome.ret, -1);
            assert_eq!(&outcome.bin[..len], bytes);
        }
    });
}
