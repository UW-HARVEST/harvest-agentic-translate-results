use libloading::Library;
use std::ffi::{c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bounds {
    error: usize,
    lower_bound: c_int,
    upper_bound: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InBuffer {
    src: *const c_void,
    size: usize,
    pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct OutBuffer {
    dst: *mut c_void,
    size: usize,
    pos: usize,
}

struct Pair {
    c: Library,
    rust: Library,
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libzstd.so"),
        crate_root.join("target/release/libzstd.so"),
    )
}

fn libraries() -> Pair {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    unsafe {
        Pair {
            c: Library::new(c_path).expect("load C shared library"),
            rust: Library::new(rust_path).expect("load Rust shared library"),
        }
    }
}

unsafe fn get<T: Copy>(library: &Library, name: &[u8]) -> T {
    *unsafe { library.get::<T>(name) }
        .unwrap_or_else(|error| panic!("load {}: {error}", String::from_utf8_lossy(name)))
}

fn random_bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    (0..len)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            (*seed >> 24) as u8
        })
        .collect()
}

fn payloads() -> Vec<Vec<u8>> {
    let mut seed = 0x4d59_5df4_d0f3_3173;
    let mut values = vec![
        Vec::new(),
        vec![0],
        vec![0xff],
        (0..128).map(|value| value as u8).collect(),
        vec![b'a'; 1024],
    ];
    for size in [2, 3, 7, 8, 15, 31, 63, 127, 129, 255, 511, 4096, 16_383] {
        values.push(random_bytes(&mut seed, size));
    }
    values
}

fn assert_same_error(pair: &Pair, left: usize, right: usize, context: &str) {
    type IsError = unsafe extern "C" fn(usize) -> c_uint;
    let c_is_error: IsError = unsafe { get(&pair.c, b"ZSTD_isError") };
    let rust_is_error: IsError = unsafe { get(&pair.rust, b"ZSTD_isError") };
    assert_eq!(left, right, "{context}: exact result differs");
    assert_eq!(
        unsafe { c_is_error(left) },
        unsafe { rust_is_error(right) },
        "{context}: error classification differs"
    );
}

#[test]
fn every_dynamic_symbol_resolves_from_both_libraries() {
    let pair = libraries();
    let symbols = include_str!("../SYMBOLS.md");
    let mut count = 0;
    for line in symbols.lines().filter(|line| line.starts_with("| ")) {
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        if fields
            .get(1)
            .is_none_or(|field| field.parse::<usize>().is_err())
        {
            continue;
        }
        let name = fields[2].trim_matches('`');
        let nul_name = format!("{name}\0");
        unsafe {
            pair.c
                .get::<*mut c_void>(nul_name.as_bytes())
                .unwrap_or_else(|error| panic!("C does not export {name}: {error}"));
            pair.rust
                .get::<*mut c_void>(nul_name.as_bytes())
                .unwrap_or_else(|error| panic!("Rust does not export {name}: {error}"));
        }
        count += 1;
    }
    assert_eq!(count, 615);
}

#[test]
fn metadata_bounds_and_error_helpers_match() {
    let pair = libraries();
    type U0 = unsafe extern "C" fn() -> c_uint;
    type I0 = unsafe extern "C" fn() -> c_int;
    type Z1 = unsafe extern "C" fn(usize) -> usize;
    type U1 = unsafe extern "C" fn(usize) -> c_uint;
    type BoundsFn = unsafe extern "C" fn(c_int) -> Bounds;

    for name in [
        b"ZSTD_versionNumber".as_slice(),
        b"FSE_versionNumber",
        b"ZSTD_XXH_versionNumber",
    ] {
        let c: U0 = unsafe { get(&pair.c, name) };
        let rust: U0 = unsafe { get(&pair.rust, name) };
        assert_eq!(unsafe { c() }, unsafe { rust() });
    }
    for name in [
        b"ZSTD_minCLevel".as_slice(),
        b"ZSTD_maxCLevel",
        b"ZSTD_defaultCLevel",
    ] {
        let c: I0 = unsafe { get(&pair.c, name) };
        let rust: I0 = unsafe { get(&pair.rust, name) };
        assert_eq!(unsafe { c() }, unsafe { rust() });
    }
    for name in [
        b"ZSTD_compressBound".as_slice(),
        b"FSE_compressBound",
        b"HUF_compressBound",
        b"ZSTD_sequenceBound",
    ] {
        let c: Z1 = unsafe { get(&pair.c, name) };
        let rust: Z1 = unsafe { get(&pair.rust, name) };
        for value in [
            0,
            1,
            2,
            127,
            128,
            129,
            131_071,
            131_072,
            usize::MAX / 2,
            usize::MAX,
        ] {
            assert_eq!(unsafe { c(value) }, unsafe { rust(value) }, "{name:?}");
        }
    }
    for name in [
        b"ZSTD_isError".as_slice(),
        b"FSE_isError",
        b"HUF_isError",
        b"HIST_isError",
        b"ZDICT_isError",
    ] {
        let c: U1 = unsafe { get(&pair.c, name) };
        let rust: U1 = unsafe { get(&pair.rust, name) };
        for value in [0, 1, 10, usize::MAX - 200, usize::MAX - 1, usize::MAX] {
            assert_eq!(unsafe { c(value) }, unsafe { rust(value) }, "{name:?}");
        }
    }

    for (name, values) in [
        (
            b"ZSTD_cParam_getBounds".as_slice(),
            vec![
                -1,
                0,
                99,
                100,
                101,
                102,
                103,
                104,
                105,
                106,
                107,
                108,
                130,
                160,
                161,
                162,
                163,
                164,
                200,
                201,
                202,
                400,
                401,
                402,
                499,
                500,
                1000,
                1017,
                1018,
                c_int::MAX,
            ],
        ),
        (
            b"ZSTD_dParam_getBounds".as_slice(),
            vec![-1, 0, 99, 100, 101, 999, 1000, 1001, 1005, 1006, c_int::MAX],
        ),
    ] {
        let c: BoundsFn = unsafe { get(&pair.c, name) };
        let rust: BoundsFn = unsafe { get(&pair.rust, name) };
        for value in values {
            assert_eq!(
                unsafe { c(value) },
                unsafe { rust(value) },
                "{name:?}({value})"
            );
        }
    }
}

#[test]
fn randomized_one_shot_compression_is_byte_identical() {
    let pair = libraries();
    type Bound = unsafe extern "C" fn(usize) -> usize;
    type Compress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
    type Decompress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
    type FrameSize = unsafe extern "C" fn(*const c_void, usize) -> u64;
    type FindSize = unsafe extern "C" fn(*const c_void, usize) -> usize;

    let c_bound: Bound = unsafe { get(&pair.c, b"ZSTD_compressBound") };
    let r_bound: Bound = unsafe { get(&pair.rust, b"ZSTD_compressBound") };
    let c_compress: Compress = unsafe { get(&pair.c, b"ZSTD_compress") };
    let r_compress: Compress = unsafe { get(&pair.rust, b"ZSTD_compress") };
    let c_decompress: Decompress = unsafe { get(&pair.c, b"ZSTD_decompress") };
    let r_decompress: Decompress = unsafe { get(&pair.rust, b"ZSTD_decompress") };
    let c_frame_size: FrameSize = unsafe { get(&pair.c, b"ZSTD_getFrameContentSize") };
    let r_frame_size: FrameSize = unsafe { get(&pair.rust, b"ZSTD_getFrameContentSize") };
    let c_find_size: FindSize = unsafe { get(&pair.c, b"ZSTD_findFrameCompressedSize") };
    let r_find_size: FindSize = unsafe { get(&pair.rust, b"ZSTD_findFrameCompressedSize") };

    for (case, input) in payloads().iter().enumerate() {
        for level in [-5, 0, 1, 3, 9, 19] {
            let c_capacity = unsafe { c_bound(input.len()) };
            let r_capacity = unsafe { r_bound(input.len()) };
            assert_eq!(c_capacity, r_capacity);
            let mut c_output = vec![0xa5; c_capacity];
            let mut r_output = vec![0xa5; r_capacity];
            let c_size = unsafe {
                c_compress(
                    c_output.as_mut_ptr().cast(),
                    c_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    level,
                )
            };
            let r_size = unsafe {
                r_compress(
                    r_output.as_mut_ptr().cast(),
                    r_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    level,
                )
            };
            assert_eq!(c_size, r_size, "case={case} level={level}");
            assert_eq!(
                &c_output[..c_size],
                &r_output[..r_size],
                "case={case} level={level}"
            );

            assert_eq!(
                unsafe { c_frame_size(c_output.as_ptr().cast(), c_size) },
                unsafe { r_frame_size(r_output.as_ptr().cast(), r_size) }
            );
            assert_eq!(
                unsafe { c_find_size(c_output.as_ptr().cast(), c_size) },
                unsafe { r_find_size(r_output.as_ptr().cast(), r_size) }
            );

            let mut c_decoded = vec![0xcc; input.len()];
            let mut r_decoded = vec![0xcc; input.len()];
            let c_decoded_size = unsafe {
                c_decompress(
                    c_decoded.as_mut_ptr().cast(),
                    c_decoded.len(),
                    c_output.as_ptr().cast(),
                    c_size,
                )
            };
            let r_decoded_size = unsafe {
                r_decompress(
                    r_decoded.as_mut_ptr().cast(),
                    r_decoded.len(),
                    r_output.as_ptr().cast(),
                    r_size,
                )
            };
            assert_eq!(c_decoded_size, r_decoded_size);
            assert_eq!(c_decoded, r_decoded);
            assert_eq!(c_decoded, *input);
        }
    }
}

unsafe fn advanced_compress(
    library: &Library,
    input: &[u8],
    level: c_int,
    strategy: c_int,
    content_size: c_int,
    checksum: c_int,
) -> (Vec<usize>, Vec<u8>) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Set = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
    type Bound = unsafe extern "C" fn(usize) -> usize;
    type Compress =
        unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;

    let create: Create = unsafe { get(library, b"ZSTD_createCCtx") };
    let free: Free = unsafe { get(library, b"ZSTD_freeCCtx") };
    let set: Set = unsafe { get(library, b"ZSTD_CCtx_setParameter") };
    let bound: Bound = unsafe { get(library, b"ZSTD_compressBound") };
    let compress: Compress = unsafe { get(library, b"ZSTD_compress2") };
    let context = unsafe { create() };
    assert!(!context.is_null());
    let results = vec![
        unsafe { set(context, 100, level) },
        unsafe { set(context, 107, strategy) },
        unsafe { set(context, 200, content_size) },
        unsafe { set(context, 201, checksum) },
    ];
    let mut output = vec![0; unsafe { bound(input.len()) }];
    let size = unsafe {
        compress(
            context,
            output.as_mut_ptr().cast(),
            output.len(),
            input.as_ptr().cast(),
            input.len(),
        )
    };
    output.truncate(size);
    assert_eq!(unsafe { free(context) }, 0);
    (results, output)
}

#[test]
fn advanced_context_option_cross_product_matches() {
    let pair = libraries();
    let mut seed = 0xa076_1d64_78bd_642f;
    for size in [0, 1, 31, 128, 1024, 8192] {
        let input = random_bytes(&mut seed, size);
        for level in [-3, 0, 3, 12] {
            for strategy in [1, 3, 6, 9] {
                for (content_size, checksum) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let c = unsafe {
                        advanced_compress(&pair.c, &input, level, strategy, content_size, checksum)
                    };
                    let rust = unsafe {
                        advanced_compress(
                            &pair.rust,
                            &input,
                            level,
                            strategy,
                            content_size,
                            checksum,
                        )
                    };
                    assert_eq!(c, rust);
                }
            }
        }
    }
}

unsafe fn stream_compress(library: &Library, input: &[u8], chunk_size: usize) -> Vec<u8> {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Init = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
    type Bound = unsafe extern "C" fn(usize) -> usize;
    type Stream = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *mut usize,
        *const c_void,
        usize,
        *mut usize,
        c_int,
    ) -> usize;

    let create: Create = unsafe { get(library, b"ZSTD_createCStream") };
    let free: Free = unsafe { get(library, b"ZSTD_freeCStream") };
    let init: Init = unsafe { get(library, b"ZSTD_initCStream") };
    let bound: Bound = unsafe { get(library, b"ZSTD_compressBound") };
    let stream: Stream = unsafe { get(library, b"ZSTD_compressStream2_simpleArgs") };
    let context = unsafe { create() };
    assert!(!context.is_null());
    assert_eq!(unsafe { init(context, 5) }, 0);
    let mut output = vec![0; unsafe { bound(input.len()) } + 64];
    let mut output_pos = 0;
    for chunk in input.chunks(chunk_size) {
        let mut input_pos = 0;
        let result = unsafe {
            stream(
                context,
                output.as_mut_ptr().cast(),
                output.len(),
                &mut output_pos,
                chunk.as_ptr().cast(),
                chunk.len(),
                &mut input_pos,
                0,
            )
        };
        assert_eq!(input_pos, chunk.len());
        assert!(result <= output.len());
    }
    let mut input_pos = 0;
    let result = unsafe {
        stream(
            context,
            output.as_mut_ptr().cast(),
            output.len(),
            &mut output_pos,
            ptr::null(),
            0,
            &mut input_pos,
            2,
        )
    };
    assert_eq!(result, 0);
    output.truncate(output_pos);
    assert_eq!(unsafe { free(context) }, 0);
    output
}

unsafe fn stream_decompress(library: &Library, frame: &[u8], output_size: usize) -> Vec<u8> {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Init = unsafe extern "C" fn(*mut c_void) -> usize;
    type Stream = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *mut usize,
        *const c_void,
        usize,
        *mut usize,
    ) -> usize;

    let create: Create = unsafe { get(library, b"ZSTD_createDStream") };
    let free: Free = unsafe { get(library, b"ZSTD_freeDStream") };
    let init: Init = unsafe { get(library, b"ZSTD_initDStream") };
    let stream: Stream = unsafe { get(library, b"ZSTD_decompressStream_simpleArgs") };
    let context = unsafe { create() };
    assert!(!context.is_null());
    assert!(unsafe { init(context) } > 0);
    let mut output = vec![0; output_size];
    let mut output_pos = 0;
    let mut input_pos = 0;
    let result = unsafe {
        stream(
            context,
            output.as_mut_ptr().cast(),
            output.len(),
            &mut output_pos,
            frame.as_ptr().cast(),
            frame.len(),
            &mut input_pos,
        )
    };
    assert_eq!(result, 0);
    assert_eq!(input_pos, frame.len());
    output.truncate(output_pos);
    assert_eq!(unsafe { free(context) }, 0);
    output
}

#[test]
fn streaming_chunk_shapes_match() {
    let pair = libraries();
    let mut seed = 0xe703_7ed1_a0b4_28db;
    for size in [0, 1, 127, 128, 129, 4096, 32_768] {
        let input = random_bytes(&mut seed, size);
        for chunk_size in [1, 3, 64, 1024, usize::MAX] {
            let c = unsafe { stream_compress(&pair.c, &input, chunk_size) };
            let rust = unsafe { stream_compress(&pair.rust, &input, chunk_size) };
            assert_eq!(c, rust, "size={size} chunk={chunk_size}");
            let c_decoded = unsafe { stream_decompress(&pair.c, &c, input.len()) };
            let rust_decoded = unsafe { stream_decompress(&pair.rust, &rust, input.len()) };
            assert_eq!(c_decoded, rust_decoded);
            assert_eq!(c_decoded, input);
        }
    }
}

unsafe fn dictionary_round_trip(
    library: &Library,
    input: &[u8],
    dictionary: &[u8],
) -> (Vec<usize>, Vec<u8>, Vec<u8>) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Bound = unsafe extern "C" fn(usize) -> usize;
    type Compress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        usize,
        c_int,
    ) -> usize;
    type Decompress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize;

    let create_c: Create = unsafe { get(library, b"ZSTD_createCCtx") };
    let create_d: Create = unsafe { get(library, b"ZSTD_createDCtx") };
    let free_c: Free = unsafe { get(library, b"ZSTD_freeCCtx") };
    let free_d: Free = unsafe { get(library, b"ZSTD_freeDCtx") };
    let bound: Bound = unsafe { get(library, b"ZSTD_compressBound") };
    let compress: Compress = unsafe { get(library, b"ZSTD_compress_usingDict") };
    let decompress: Decompress = unsafe { get(library, b"ZSTD_decompress_usingDict") };
    let cctx = unsafe { create_c() };
    let dctx = unsafe { create_d() };
    assert!(!cctx.is_null() && !dctx.is_null());
    let mut compressed = vec![0; unsafe { bound(input.len()) }];
    let compressed_size = unsafe {
        compress(
            cctx,
            compressed.as_mut_ptr().cast(),
            compressed.len(),
            input.as_ptr().cast(),
            input.len(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
            7,
        )
    };
    compressed.truncate(compressed_size);
    let mut decoded = vec![0; input.len()];
    let decoded_size = unsafe {
        decompress(
            dctx,
            decoded.as_mut_ptr().cast(),
            decoded.len(),
            compressed.as_ptr().cast(),
            compressed.len(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
        )
    };
    decoded.truncate(decoded_size);
    let frees = vec![unsafe { free_c(cctx) }, unsafe { free_d(dctx) }];
    (frees, compressed, decoded)
}

#[test]
fn raw_dictionary_paths_match() {
    let pair = libraries();
    let mut seed = 0x8ebc_6af0_9c88_c6e3;
    for dict_size in [0, 1, 64, 1024, 8192] {
        let dictionary = random_bytes(&mut seed, dict_size);
        for input_size in [0, 1, 31, 1024, 8192] {
            let input = random_bytes(&mut seed, input_size);
            let c = unsafe { dictionary_round_trip(&pair.c, &input, &dictionary) };
            let rust = unsafe { dictionary_round_trip(&pair.rust, &input, &dictionary) };
            assert_eq!(c, rust);
            assert_eq!(c.2, input);
        }
    }
}

#[test]
fn skippable_frames_and_invalid_variants_match() {
    let pair = libraries();
    type Write = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
    type Read =
        unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize;
    type Is = unsafe extern "C" fn(*const c_void, usize) -> c_uint;
    let c_write: Write = unsafe { get(&pair.c, b"ZSTD_writeSkippableFrame") };
    let r_write: Write = unsafe { get(&pair.rust, b"ZSTD_writeSkippableFrame") };
    let c_read: Read = unsafe { get(&pair.c, b"ZSTD_readSkippableFrame") };
    let r_read: Read = unsafe { get(&pair.rust, b"ZSTD_readSkippableFrame") };
    let c_is: Is = unsafe { get(&pair.c, b"ZSTD_isSkippableFrame") };
    let r_is: Is = unsafe { get(&pair.rust, b"ZSTD_isSkippableFrame") };
    let mut seed = 0x5899_65cc_7537_4cc3;

    for size in [0, 1, 7, 128, 4096] {
        let input = random_bytes(&mut seed, size);
        for variant in 0..=15 {
            let mut c_frame = vec![0; size + 8];
            let mut r_frame = vec![0; size + 8];
            let c_size = unsafe {
                c_write(
                    c_frame.as_mut_ptr().cast(),
                    c_frame.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    variant,
                )
            };
            let r_size = unsafe {
                r_write(
                    r_frame.as_mut_ptr().cast(),
                    r_frame.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    variant,
                )
            };
            assert_eq!(c_size, r_size);
            assert_eq!(c_frame, r_frame);
            assert_eq!(unsafe { c_is(c_frame.as_ptr().cast(), c_size) }, unsafe {
                r_is(r_frame.as_ptr().cast(), r_size)
            });
            let mut c_output = vec![0; size];
            let mut r_output = vec![0; size];
            let mut c_variant = c_uint::MAX;
            let mut r_variant = c_uint::MAX;
            let c_read_size = unsafe {
                c_read(
                    c_output.as_mut_ptr().cast(),
                    c_output.len(),
                    &mut c_variant,
                    c_frame.as_ptr().cast(),
                    c_size,
                )
            };
            let r_read_size = unsafe {
                r_read(
                    r_output.as_mut_ptr().cast(),
                    r_output.len(),
                    &mut r_variant,
                    r_frame.as_ptr().cast(),
                    r_size,
                )
            };
            assert_eq!(
                (c_read_size, c_variant, c_output),
                (r_read_size, r_variant, r_output)
            );
        }

        for invalid_variant in [16, 17, c_uint::MAX] {
            let mut c_output = vec![0; size + 8];
            let mut r_output = vec![0; size + 8];
            let c_result = unsafe {
                c_write(
                    c_output.as_mut_ptr().cast(),
                    c_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    invalid_variant,
                )
            };
            let r_result = unsafe {
                r_write(
                    r_output.as_mut_ptr().cast(),
                    r_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    invalid_variant,
                )
            };
            assert_same_error(&pair, c_result, r_result, "invalid magic variant");
        }
    }
}

#[test]
fn xxhash_and_histogram_low_level_outputs_match() {
    let pair = libraries();
    type Xxh32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
    type Xxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
    type Hist = unsafe extern "C" fn(*mut c_uint, *mut c_uint, *const c_void, usize) -> usize;
    let c_xxh32: Xxh32 = unsafe { get(&pair.c, b"ZSTD_XXH32") };
    let r_xxh32: Xxh32 = unsafe { get(&pair.rust, b"ZSTD_XXH32") };
    let c_xxh64: Xxh64 = unsafe { get(&pair.c, b"ZSTD_XXH64") };
    let r_xxh64: Xxh64 = unsafe { get(&pair.rust, b"ZSTD_XXH64") };
    let c_hist: Hist = unsafe { get(&pair.c, b"HIST_count") };
    let r_hist: Hist = unsafe { get(&pair.rust, b"HIST_count") };
    let mut seed = 0xd6e8_feb8_6659_fd93;

    for size in [0, 1, 3, 8, 31, 32, 33, 255, 1024, 16_384] {
        for _ in 0..16 {
            let input = random_bytes(&mut seed, size);
            for hash_seed in [0, 1, u32::MAX] {
                assert_eq!(
                    unsafe { c_xxh32(input.as_ptr().cast(), input.len(), hash_seed) },
                    unsafe { r_xxh32(input.as_ptr().cast(), input.len(), hash_seed) }
                );
            }
            for hash_seed in [0, 1, u64::MAX] {
                assert_eq!(
                    unsafe { c_xxh64(input.as_ptr().cast(), input.len(), hash_seed) },
                    unsafe { r_xxh64(input.as_ptr().cast(), input.len(), hash_seed) }
                );
            }
            if !input.is_empty() {
                let mut c_counts = [0; 256];
                let mut r_counts = [0; 256];
                let mut c_max = 255;
                let mut r_max = 255;
                let c_result = unsafe {
                    c_hist(
                        c_counts.as_mut_ptr(),
                        &mut c_max,
                        input.as_ptr().cast(),
                        input.len(),
                    )
                };
                let r_result = unsafe {
                    r_hist(
                        r_counts.as_mut_ptr(),
                        &mut r_max,
                        input.as_ptr().cast(),
                        input.len(),
                    )
                };
                assert_eq!((c_result, c_max, c_counts), (r_result, r_max, r_counts));
            }
        }
    }
}

#[test]
fn exact_error_results_match_for_public_boundaries() {
    let pair = libraries();
    type Compress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize;
    type Decompress = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize;
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Set = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
    type Reset = unsafe extern "C" fn(*mut c_void, c_int) -> usize;

    let c_compress: Compress = unsafe { get(&pair.c, b"ZSTD_compress") };
    let r_compress: Compress = unsafe { get(&pair.rust, b"ZSTD_compress") };
    let c_decompress: Decompress = unsafe { get(&pair.c, b"ZSTD_decompress") };
    let r_decompress: Decompress = unsafe { get(&pair.rust, b"ZSTD_decompress") };
    let input = b"boundary test input";
    for capacity in [0, 1, 2, 4, 8] {
        let mut c_output = vec![0; capacity];
        let mut r_output = vec![0; capacity];
        let c_result = unsafe {
            c_compress(
                c_output.as_mut_ptr().cast(),
                c_output.len(),
                input.as_ptr().cast(),
                input.len(),
                3,
            )
        };
        let r_result = unsafe {
            r_compress(
                r_output.as_mut_ptr().cast(),
                r_output.len(),
                input.as_ptr().cast(),
                input.len(),
                3,
            )
        };
        assert_same_error(&pair, c_result, r_result, "undersized compression output");
    }

    let malformed_cases: &[&[u8]] = &[
        &[],
        &[0],
        &[0x28, 0xb5],
        &[0x28, 0xb5, 0x2f, 0xfd],
        &[0xff; 32],
    ];
    for malformed in malformed_cases {
        for capacity in [0, 1, 64, 4096] {
            let mut c_output = vec![0; capacity];
            let mut r_output = vec![0; capacity];
            let c_result = unsafe {
                c_decompress(
                    c_output.as_mut_ptr().cast(),
                    c_output.len(),
                    malformed.as_ptr().cast(),
                    malformed.len(),
                )
            };
            let r_result = unsafe {
                r_decompress(
                    r_output.as_mut_ptr().cast(),
                    r_output.len(),
                    malformed.as_ptr().cast(),
                    malformed.len(),
                )
            };
            assert_same_error(&pair, c_result, r_result, "malformed frame");
        }
    }

    for (create_name, free_name, set_name, reset_name, params) in [
        (
            b"ZSTD_createCCtx".as_slice(),
            b"ZSTD_freeCCtx".as_slice(),
            b"ZSTD_CCtx_setParameter".as_slice(),
            b"ZSTD_CCtx_reset".as_slice(),
            vec![-1, 0, 99, 108, 499, 999, 1018, c_int::MAX],
        ),
        (
            b"ZSTD_createDCtx".as_slice(),
            b"ZSTD_freeDCtx".as_slice(),
            b"ZSTD_DCtx_setParameter".as_slice(),
            b"ZSTD_DCtx_reset".as_slice(),
            vec![-1, 0, 99, 101, 999, 1006, c_int::MAX],
        ),
    ] {
        let c_create: Create = unsafe { get(&pair.c, create_name) };
        let r_create: Create = unsafe { get(&pair.rust, create_name) };
        let c_free: Free = unsafe { get(&pair.c, free_name) };
        let r_free: Free = unsafe { get(&pair.rust, free_name) };
        let c_set: Set = unsafe { get(&pair.c, set_name) };
        let r_set: Set = unsafe { get(&pair.rust, set_name) };
        let c_reset: Reset = unsafe { get(&pair.c, reset_name) };
        let r_reset: Reset = unsafe { get(&pair.rust, reset_name) };
        let c_context = unsafe { c_create() };
        let r_context = unsafe { r_create() };
        for param in params {
            let c_result = unsafe { c_set(c_context, param, 0) };
            let r_result = unsafe { r_set(r_context, param, 0) };
            assert_same_error(&pair, c_result, r_result, "out-of-range parameter enum");
        }
        for directive in [-1, 0, 1, 2, 3, 4, c_int::MAX] {
            let c_result = unsafe { c_reset(c_context, directive) };
            let r_result = unsafe { r_reset(r_context, directive) };
            assert_same_error(&pair, c_result, r_result, "reset enum boundary");
        }
        assert_eq!(unsafe { c_free(c_context) }, unsafe { r_free(r_context) });
        assert_eq!(unsafe { c_free(ptr::null_mut()) }, unsafe {
            r_free(ptr::null_mut())
        });
    }
}

#[test]
fn exported_data_and_object_lifecycle_match() {
    let pair = libraries();
    unsafe {
        let c_debug = pair
            .c
            .get::<*mut c_int>(b"g_debuglevel")
            .expect("C g_debuglevel");
        let r_debug = pair
            .rust
            .get::<*mut c_int>(b"g_debuglevel")
            .expect("Rust g_debuglevel");
        assert_eq!(**c_debug, **r_debug);
        **c_debug = 3;
        **r_debug = 3;
        assert_eq!(**c_debug, **r_debug);
        **c_debug = 0;
        **r_debug = 0;
    }

    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Sizeof = unsafe extern "C" fn(*const c_void) -> usize;
    for (create_name, free_name, sizeof_name) in [
        (
            b"ZSTD_createCCtx".as_slice(),
            b"ZSTD_freeCCtx".as_slice(),
            b"ZSTD_sizeof_CCtx".as_slice(),
        ),
        (
            b"ZSTD_createDCtx".as_slice(),
            b"ZSTD_freeDCtx".as_slice(),
            b"ZSTD_sizeof_DCtx".as_slice(),
        ),
        (
            b"ZSTD_createCStream".as_slice(),
            b"ZSTD_freeCStream".as_slice(),
            b"ZSTD_sizeof_CStream".as_slice(),
        ),
        (
            b"ZSTD_createDStream".as_slice(),
            b"ZSTD_freeDStream".as_slice(),
            b"ZSTD_sizeof_DStream".as_slice(),
        ),
    ] {
        let c_create: Create = unsafe { get(&pair.c, create_name) };
        let r_create: Create = unsafe { get(&pair.rust, create_name) };
        let c_free: Free = unsafe { get(&pair.c, free_name) };
        let r_free: Free = unsafe { get(&pair.rust, free_name) };
        let c_sizeof: Sizeof = unsafe { get(&pair.c, sizeof_name) };
        let r_sizeof: Sizeof = unsafe { get(&pair.rust, sizeof_name) };
        let c_object = unsafe { c_create() };
        let r_object = unsafe { r_create() };
        assert_eq!(
            unsafe { c_sizeof(c_object) },
            unsafe { r_sizeof(r_object) },
            "{sizeof_name:?}"
        );
        assert_eq!(unsafe { c_free(c_object) }, unsafe { r_free(r_object) });
    }
}

#[test]
fn struct_buffer_stream_entry_points_match() {
    let pair = libraries();
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Init = unsafe extern "C" fn(*mut c_void, c_int) -> usize;
    type Stream = unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;
    let input = b"struct-buffer streaming path";

    let run = |library: &Library| unsafe {
        let create: Create = get(library, b"ZSTD_createCStream");
        let free: Free = get(library, b"ZSTD_freeCStream");
        let init: Init = get(library, b"ZSTD_initCStream");
        let stream: Stream = get(library, b"ZSTD_compressStream2");
        let context = create();
        let init_result = init(context, 3);
        let mut bytes = vec![0; 256];
        let mut output = OutBuffer {
            dst: bytes.as_mut_ptr().cast(),
            size: bytes.len(),
            pos: 0,
        };
        let mut input_buffer = InBuffer {
            src: input.as_ptr().cast(),
            size: input.len(),
            pos: 0,
        };
        let result = stream(context, &mut output, &mut input_buffer, 2);
        bytes.truncate(output.pos);
        let free_result = free(context);
        (init_result, result, input_buffer.pos, bytes, free_result)
    };

    assert_eq!(run(&pair.c), run(&pair.rust));
}
