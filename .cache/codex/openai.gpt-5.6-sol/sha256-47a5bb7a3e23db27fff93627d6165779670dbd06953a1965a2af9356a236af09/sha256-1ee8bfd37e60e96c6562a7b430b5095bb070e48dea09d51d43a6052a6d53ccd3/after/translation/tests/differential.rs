#![allow(unused_unsafe, unused_parens)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::path::PathBuf;
use std::ptr;

const LZ4_MAX_INPUT_SIZE: c_int = 0x7E00_0000;
const LZ4F_VERSION: c_uint = 100;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct FrameInfo {
    block_size_id: c_int,
    block_mode: c_int,
    content_checksum_flag: c_int,
    frame_type: c_int,
    content_size: c_ulonglong,
    dict_id: c_uint,
    block_checksum_flag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Preferences {
    frame_info: FrameInfo,
    compression_level: c_int,
    auto_flush: c_uint,
    favor_dec_speed: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct CompressOptions {
    stable_src: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct DecompressOptions {
    stable_dst: c_uint,
    reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CustomMem {
    custom_alloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    custom_calloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    custom_free:
        Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>,
    opaque_state: *mut c_void,
}

impl Default for CustomMem {
    fn default() -> Self {
        Self {
            custom_alloc: None,
            custom_calloc: None,
            custom_free: None,
            opaque_state: ptr::null_mut(),
        }
    }
}

struct Libraries {
    c: Library,
    rust: Library,
}

unsafe extern "C" {
    fn tmpfile() -> *mut c_void;
    fn rewind(stream: *mut c_void);
    fn fwrite(ptr: *const c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fclose(stream: *mut c_void) -> c_int;
}

impl Libraries {
    unsafe fn open() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/liblz4.so");
        let rust_path = manifest.join("target/release/liblz4.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).unwrap() },
            rust: unsafe { Library::new(rust_path).unwrap() },
        }
    }
}

unsafe fn symbol<T: Copy>(library: &Library, name: &str) -> T {
    let mut bytes = Vec::from(name.as_bytes());
    bytes.push(0);
    unsafe { *library.get::<T>(&bytes).unwrap() }
}

fn random_bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    let mut output = vec![0; len];
    for byte in &mut output {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *byte = (*seed >> 24) as u8;
    }
    output
}

fn patterned_bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    let alphabet = random_bytes(seed, 17);
    (0..len).map(|index| alphabet[index % alphabet.len()]).collect()
}

fn data_ptr(data: &[u8]) -> *const u8 {
    static ZERO: u8 = 0;
    if data.is_empty() {
        &ZERO
    } else {
        data.as_ptr()
    }
}

fn assert_result_and_bytes(
    label: &str,
    c_result: c_int,
    rust_result: c_int,
    c_output: &[u8],
    rust_output: &[u8],
) {
    assert_eq!(c_result, rust_result, "{label}: return value");
    if c_result > 0 {
        let count = c_result as usize;
        assert_eq!(
            &c_output[..count],
            &rust_output[..count],
            "{label}: output bytes"
        );
    }
}

#[test]
fn dynamic_symbol_surface_is_identical() {
    unsafe {
        let libraries = Libraries::open();
        let symbols = include_str!("../backend-symbols.txt");
        let mut count = 0;
        for name in symbols.lines().filter(|line| !line.is_empty()) {
            let _: libloading::Symbol<'_, *mut c_void> = libraries
                .c
                .get(format!("{name}\0").as_bytes())
                .unwrap_or_else(|error| panic!("C symbol {name}: {error}"));
            let _: libloading::Symbol<'_, *mut c_void> = libraries
                .rust
                .get(format!("{name}\0").as_bytes())
                .unwrap_or_else(|error| panic!("Rust symbol {name}: {error}"));
            count += 1;
        }
        assert_eq!(count, 143);
    }
}

#[test]
fn metadata_bounds_and_error_helpers_match() {
    unsafe {
        let libraries = Libraries::open();
        for name in [
            "LZ4_versionNumber",
            "LZ4_sizeofState",
            "LZ4_sizeofStateHC",
            "LZ4_sizeofStreamState",
            "LZ4_sizeofStreamStateHC",
            "LZ4F_compressionLevel_max",
        ] {
            let c_fn: unsafe extern "C" fn() -> c_int = symbol(&libraries.c, name);
            let rust_fn: unsafe extern "C" fn() -> c_int = symbol(&libraries.rust, name);
            assert_eq!(unsafe { c_fn() }, unsafe { rust_fn() }, "{name}");
        }

        let c_version_string: unsafe extern "C" fn() -> *const c_char =
            symbol(&libraries.c, "LZ4_versionString");
        let rust_version_string: unsafe extern "C" fn() -> *const c_char =
            symbol(&libraries.rust, "LZ4_versionString");
        let c_string = unsafe { std::ffi::CStr::from_ptr(c_version_string()) };
        let rust_string = unsafe { std::ffi::CStr::from_ptr(rust_version_string()) };
        assert_eq!(c_string.to_bytes_with_nul(), rust_string.to_bytes_with_nul());

        let c_bound: unsafe extern "C" fn(c_int) -> c_int =
            symbol(&libraries.c, "LZ4_compressBound");
        let rust_bound: unsafe extern "C" fn(c_int) -> c_int =
            symbol(&libraries.rust, "LZ4_compressBound");
        for input in [
            -1,
            0,
            1,
            15,
            16,
            255,
            65_535,
            65_536,
            LZ4_MAX_INPUT_SIZE,
            LZ4_MAX_INPUT_SIZE + 1,
        ] {
            assert_eq!(unsafe { c_bound(input) }, unsafe { rust_bound(input) });
        }

        let c_ring: unsafe extern "C" fn(c_int) -> c_int =
            symbol(&libraries.c, "LZ4_decoderRingBufferSize");
        let rust_ring: unsafe extern "C" fn(c_int) -> c_int =
            symbol(&libraries.rust, "LZ4_decoderRingBufferSize");
        for input in [-1, 0, 1, 65_535, 65_536, LZ4_MAX_INPUT_SIZE, LZ4_MAX_INPUT_SIZE + 1] {
            assert_eq!(unsafe { c_ring(input) }, unsafe { rust_ring(input) });
        }

        let c_f_version: unsafe extern "C" fn() -> c_uint =
            symbol(&libraries.c, "LZ4F_getVersion");
        let rust_f_version: unsafe extern "C" fn() -> c_uint =
            symbol(&libraries.rust, "LZ4F_getVersion");
        assert_eq!(unsafe { c_f_version() }, unsafe { rust_f_version() });

        let c_block: unsafe extern "C" fn(c_uint) -> usize =
            symbol(&libraries.c, "LZ4F_getBlockSize");
        let rust_block: unsafe extern "C" fn(c_uint) -> usize =
            symbol(&libraries.rust, "LZ4F_getBlockSize");
        let c_is_error: unsafe extern "C" fn(usize) -> c_uint =
            symbol(&libraries.c, "LZ4F_isError");
        let rust_is_error: unsafe extern "C" fn(usize) -> c_uint =
            symbol(&libraries.rust, "LZ4F_isError");
        let c_error_code: unsafe extern "C" fn(usize) -> c_int =
            symbol(&libraries.c, "LZ4F_getErrorCode");
        let rust_error_code: unsafe extern "C" fn(usize) -> c_int =
            symbol(&libraries.rust, "LZ4F_getErrorCode");
        let c_error_name: unsafe extern "C" fn(usize) -> *const c_char =
            symbol(&libraries.c, "LZ4F_getErrorName");
        let rust_error_name: unsafe extern "C" fn(usize) -> *const c_char =
            symbol(&libraries.rust, "LZ4F_getErrorName");
        for id in [0, 1, 3, 4, 5, 6, 7, 8, c_uint::MAX] {
            let c_value = unsafe { c_block(id) };
            let rust_value = unsafe { rust_block(id) };
            assert_eq!(c_value, rust_value, "block size id {id}");
            assert_eq!(
                unsafe { c_is_error(c_value) },
                unsafe { rust_is_error(rust_value) }
            );
            assert_eq!(
                unsafe { c_error_code(c_value) },
                unsafe { rust_error_code(rust_value) }
            );
            let c_name = unsafe { std::ffi::CStr::from_ptr(c_error_name(c_value)) };
            let rust_name = unsafe { std::ffi::CStr::from_ptr(rust_error_name(rust_value)) };
            assert_eq!(c_name.to_bytes(), rust_name.to_bytes());
        }
    }
}

#[test]
fn xxhash_one_shot_streaming_copy_and_canonical_match() {
    unsafe {
        let libraries = Libraries::open();
        type Hash32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;
        type Hash64 =
            unsafe extern "C" fn(*const c_void, usize, c_ulonglong) -> c_ulonglong;
        let c_hash32: Hash32 = symbol(&libraries.c, "LZ4_XXH32");
        let rust_hash32: Hash32 = symbol(&libraries.rust, "LZ4_XXH32");
        let c_hash64: Hash64 = symbol(&libraries.c, "LZ4_XXH64");
        let rust_hash64: Hash64 = symbol(&libraries.rust, "LZ4_XXH64");
        let mut seed = 0x1234_5678_9ABC_DEF0;

        for len in [0, 1, 2, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 63, 255, 4097] {
            for offset in [0, 1] {
                let mut storage = random_bytes(&mut seed, len + offset);
                let input = unsafe { storage.as_mut_ptr().add(offset) };
                for hash_seed in [0, 1, 0x9E37_79B1, c_uint::MAX] {
                    assert_eq!(
                        unsafe { c_hash32(input.cast(), len, hash_seed) },
                        unsafe { rust_hash32(input.cast(), len, hash_seed) },
                        "XXH32 len={len} offset={offset}"
                    );
                    assert_eq!(
                        unsafe { c_hash64(input.cast(), len, hash_seed as u64 * 0x1_0000_0001) },
                        unsafe {
                            rust_hash64(input.cast(), len, hash_seed as u64 * 0x1_0000_0001)
                        },
                        "XXH64 len={len} offset={offset}"
                    );
                }
            }
        }

        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Reset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type Digest32 = unsafe extern "C" fn(*const c_void) -> c_uint;
        type CopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
        type Canon32 = unsafe extern "C" fn(*mut c_void, c_uint);
        type FromCanon32 = unsafe extern "C" fn(*const c_void) -> c_uint;

        let _: Create = symbol(&libraries.c, "LZ4_XXH32_createState");
        let _: Create = symbol(&libraries.rust, "LZ4_XXH32_createState");
        let _: Free = symbol(&libraries.c, "LZ4_XXH32_freeState");
        let _: Free = symbol(&libraries.rust, "LZ4_XXH32_freeState");

        for prefix in ["LZ4_XXH32", "LZ4_XXH64"] {
            let c_create: Create = symbol(&libraries.c, &format!("{prefix}_createState"));
            let rust_create: Create = symbol(&libraries.rust, &format!("{prefix}_createState"));
            let c_free: Free = symbol(&libraries.c, &format!("{prefix}_freeState"));
            let rust_free: Free = symbol(&libraries.rust, &format!("{prefix}_freeState"));
            let c_state = unsafe { c_create() };
            let rust_state = unsafe { rust_create() };
            assert_eq!(c_state.is_null(), rust_state.is_null());
            assert!(!c_state.is_null());

            if prefix.ends_with("32") {
                let c_reset: Reset32 = symbol(&libraries.c, "LZ4_XXH32_reset");
                let rust_reset: Reset32 = symbol(&libraries.rust, "LZ4_XXH32_reset");
                let c_update: Update = symbol(&libraries.c, "LZ4_XXH32_update");
                let rust_update: Update = symbol(&libraries.rust, "LZ4_XXH32_update");
                let c_digest: Digest32 = symbol(&libraries.c, "LZ4_XXH32_digest");
                let rust_digest: Digest32 = symbol(&libraries.rust, "LZ4_XXH32_digest");
                assert_eq!(
                    unsafe { c_reset(c_state, 0xA5A5_5A5A) },
                    unsafe { rust_reset(rust_state, 0xA5A5_5A5A) }
                );
                let data = random_bytes(&mut seed, 777);
                for chunk in [0usize, 1, 13, 64, 255, 444] {
                    let start = chunk.min(data.len());
                    let end = (start + 37).min(data.len());
                    assert_eq!(
                        unsafe { c_update(c_state, data[start..].as_ptr().cast(), end - start) },
                        unsafe {
                            rust_update(rust_state, data[start..].as_ptr().cast(), end - start)
                        }
                    );
                }
                assert_eq!(
                    unsafe { c_update(c_state, ptr::null(), 0) },
                    unsafe { rust_update(rust_state, ptr::null(), 0) }
                );
                assert_eq!(
                    unsafe { c_update(c_state, ptr::null(), 1) },
                    unsafe { rust_update(rust_state, ptr::null(), 1) }
                );
                let c_digest_value = unsafe { c_digest(c_state) };
                let rust_digest_value = unsafe { rust_digest(rust_state) };
                assert_eq!(c_digest_value, rust_digest_value);

                let c_copy_state: CopyState = symbol(&libraries.c, "LZ4_XXH32_copyState");
                let rust_copy_state: CopyState =
                    symbol(&libraries.rust, "LZ4_XXH32_copyState");
                let c_copy = unsafe { c_create() };
                let rust_copy = unsafe { rust_create() };
                unsafe {
                    c_copy_state(c_copy, c_state);
                    rust_copy_state(rust_copy, rust_state);
                }
                assert_eq!(unsafe { c_digest(c_copy) }, unsafe { rust_digest(rust_copy) });
                assert_eq!(unsafe { c_free(c_copy) }, unsafe { rust_free(rust_copy) });

                let c_canon: Canon32 = symbol(&libraries.c, "LZ4_XXH32_canonicalFromHash");
                let rust_canon: Canon32 =
                    symbol(&libraries.rust, "LZ4_XXH32_canonicalFromHash");
                let c_from: FromCanon32 =
                    symbol(&libraries.c, "LZ4_XXH32_hashFromCanonical");
                let rust_from: FromCanon32 =
                    symbol(&libraries.rust, "LZ4_XXH32_hashFromCanonical");
                let mut c_bytes = [0u8; 4];
                let mut rust_bytes = [0u8; 4];
                unsafe {
                    c_canon(c_bytes.as_mut_ptr().cast(), c_digest_value);
                    rust_canon(rust_bytes.as_mut_ptr().cast(), rust_digest_value);
                }
                assert_eq!(c_bytes, rust_bytes);
                assert_eq!(
                    unsafe { c_from(c_bytes.as_ptr().cast()) },
                    unsafe { rust_from(rust_bytes.as_ptr().cast()) }
                );
            } else {
                type Reset64 =
                    unsafe extern "C" fn(*mut c_void, c_ulonglong) -> c_int;
                type Digest64 = unsafe extern "C" fn(*const c_void) -> c_ulonglong;
                type Canon64 = unsafe extern "C" fn(*mut c_void, c_ulonglong);
                type FromCanon64 =
                    unsafe extern "C" fn(*const c_void) -> c_ulonglong;
                let c_reset: Reset64 = symbol(&libraries.c, "LZ4_XXH64_reset");
                let rust_reset: Reset64 = symbol(&libraries.rust, "LZ4_XXH64_reset");
                let c_update: Update = symbol(&libraries.c, "LZ4_XXH64_update");
                let rust_update: Update = symbol(&libraries.rust, "LZ4_XXH64_update");
                let c_digest: Digest64 = symbol(&libraries.c, "LZ4_XXH64_digest");
                let rust_digest: Digest64 = symbol(&libraries.rust, "LZ4_XXH64_digest");
                assert_eq!(
                    unsafe { c_reset(c_state, 0x0123_4567_89AB_CDEF) },
                    unsafe { rust_reset(rust_state, 0x0123_4567_89AB_CDEF) }
                );
                let data = random_bytes(&mut seed, 1025);
                let mut position = 0;
                while position < data.len() {
                    let amount = ((position * 17 + 31) % 97 + 1).min(data.len() - position);
                    assert_eq!(
                        unsafe {
                            c_update(c_state, data[position..].as_ptr().cast(), amount)
                        },
                        unsafe {
                            rust_update(rust_state, data[position..].as_ptr().cast(), amount)
                        }
                    );
                    position += amount;
                }
                assert_eq!(
                    unsafe { c_update(c_state, ptr::null(), 0) },
                    unsafe { rust_update(rust_state, ptr::null(), 0) }
                );
                assert_eq!(
                    unsafe { c_update(c_state, ptr::null(), 1) },
                    unsafe { rust_update(rust_state, ptr::null(), 1) }
                );
                let c_digest_value = unsafe { c_digest(c_state) };
                let rust_digest_value = unsafe { rust_digest(rust_state) };
                assert_eq!(c_digest_value, rust_digest_value);
                let c_canon: Canon64 = symbol(&libraries.c, "LZ4_XXH64_canonicalFromHash");
                let rust_canon: Canon64 =
                    symbol(&libraries.rust, "LZ4_XXH64_canonicalFromHash");
                let c_from: FromCanon64 =
                    symbol(&libraries.c, "LZ4_XXH64_hashFromCanonical");
                let rust_from: FromCanon64 =
                    symbol(&libraries.rust, "LZ4_XXH64_hashFromCanonical");
                let mut c_bytes = [0u8; 8];
                let mut rust_bytes = [0u8; 8];
                unsafe {
                    c_canon(c_bytes.as_mut_ptr().cast(), c_digest_value);
                    rust_canon(rust_bytes.as_mut_ptr().cast(), rust_digest_value);
                }
                assert_eq!(c_bytes, rust_bytes);
                assert_eq!(
                    unsafe { c_from(c_bytes.as_ptr().cast()) },
                    unsafe { rust_from(rust_bytes.as_ptr().cast()) }
                );
            }
            assert_eq!(unsafe { c_free(c_state) }, unsafe { rust_free(rust_state) });
        }
    }
}

#[test]
fn block_fast_hc_partial_and_error_paths_match() {
    unsafe {
        let libraries = Libraries::open();
        type Bound = unsafe extern "C" fn(c_int) -> c_int;
        type Compress =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type CompressFast = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        type CompressHc = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        type Decompress =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type Partial = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        let c_bound: Bound = symbol(&libraries.c, "LZ4_compressBound");
        let rust_bound: Bound = symbol(&libraries.rust, "LZ4_compressBound");
        let c_default: Compress = symbol(&libraries.c, "LZ4_compress_default");
        let rust_default: Compress = symbol(&libraries.rust, "LZ4_compress_default");
        let c_fast: CompressFast = symbol(&libraries.c, "LZ4_compress_fast");
        let rust_fast: CompressFast = symbol(&libraries.rust, "LZ4_compress_fast");
        let c_hc: CompressHc = symbol(&libraries.c, "LZ4_compress_HC");
        let rust_hc: CompressHc = symbol(&libraries.rust, "LZ4_compress_HC");
        let c_decompress: Decompress = symbol(&libraries.c, "LZ4_decompress_safe");
        let rust_decompress: Decompress = symbol(&libraries.rust, "LZ4_decompress_safe");
        let c_partial: Partial = symbol(&libraries.c, "LZ4_decompress_safe_partial");
        let rust_partial: Partial =
            symbol(&libraries.rust, "LZ4_decompress_safe_partial");
        let mut seed = 0xD1FF_E2E3_4455_6677;

        for iteration in 0..96 {
            let len = match iteration {
                0 => 0,
                1 => 1,
                2 => 12,
                3 => 13,
                4 => 65_535,
                5 => 65_536,
                _ => (seed as usize % 131_073),
            };
            let input = if iteration % 3 == 0 {
                patterned_bytes(&mut seed, len)
            } else {
                random_bytes(&mut seed, len)
            };
            let bound = unsafe { c_bound(len as c_int) };
            assert_eq!(bound, unsafe { rust_bound(len as c_int) });
            let capacities = [0, 1, (bound / 2).max(1), bound.saturating_sub(1), bound];
            for capacity in capacities {
                let mut c_output = vec![0xA5; capacity.max(1) as usize];
                let mut rust_output = vec![0xA5; capacity.max(1) as usize];
                let c_result = unsafe {
                    c_default(
                        data_ptr(&input).cast(),
                        c_output.as_mut_ptr().cast(),
                        len as c_int,
                        capacity,
                    )
                };
                let rust_result = unsafe {
                    rust_default(
                        data_ptr(&input).cast(),
                        rust_output.as_mut_ptr().cast(),
                        len as c_int,
                        capacity,
                    )
                };
                assert_result_and_bytes(
                    "LZ4_compress_default",
                    c_result,
                    rust_result,
                    &c_output,
                    &rust_output,
                );
            }

            for acceleration in [0, 1, 2, 17, 65_537, 65_538] {
                let mut c_output = vec![0; bound as usize];
                let mut rust_output = vec![0; bound as usize];
                let c_result = unsafe {
                    c_fast(
                        data_ptr(&input).cast(),
                        c_output.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                        acceleration,
                    )
                };
                let rust_result = unsafe {
                    rust_fast(
                        data_ptr(&input).cast(),
                        rust_output.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                        acceleration,
                    )
                };
                assert_result_and_bytes(
                    "LZ4_compress_fast",
                    c_result,
                    rust_result,
                    &c_output,
                    &rust_output,
                );
            }

            for level in [-1, 0, 1, 2, 9, 10, 12, 13] {
                let mut c_output = vec![0; bound as usize];
                let mut rust_output = vec![0; bound as usize];
                let c_size = unsafe {
                    c_hc(
                        data_ptr(&input).cast(),
                        c_output.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                        level,
                    )
                };
                let rust_size = unsafe {
                    rust_hc(
                        data_ptr(&input).cast(),
                        rust_output.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                        level,
                    )
                };
                assert_result_and_bytes(
                    "LZ4_compress_HC",
                    c_size,
                    rust_size,
                    &c_output,
                    &rust_output,
                );
            }

            let mut compressed = vec![0; bound as usize];
            let compressed_size = unsafe {
                c_default(
                    data_ptr(&input).cast(),
                    compressed.as_mut_ptr().cast(),
                    len as c_int,
                    bound,
                )
            };
            assert!(compressed_size > 0);
            for capacity in [0, 1, len.saturating_sub(1), len, len + 7] {
                let mut c_output = vec![0xCC; capacity.max(1)];
                let mut rust_output = vec![0xCC; capacity.max(1)];
                let c_result = unsafe {
                    c_decompress(
                        compressed.as_ptr().cast(),
                        c_output.as_mut_ptr().cast(),
                        compressed_size,
                        capacity as c_int,
                    )
                };
                let rust_result = unsafe {
                    rust_decompress(
                        compressed.as_ptr().cast(),
                        rust_output.as_mut_ptr().cast(),
                        compressed_size,
                        capacity as c_int,
                    )
                };
                assert_result_and_bytes(
                    "LZ4_decompress_safe",
                    c_result,
                    rust_result,
                    &c_output,
                    &rust_output,
                );
                if capacity >= len && c_result >= 0 {
                    assert_eq!(&c_output[..len], &input);
                }
            }
            for target in [0, 1, len / 2, len, len + 1] {
                let capacity = len.max(1);
                let mut c_output = vec![0; capacity];
                let mut rust_output = vec![0; capacity];
                let c_result = unsafe {
                    c_partial(
                        compressed.as_ptr().cast(),
                        c_output.as_mut_ptr().cast(),
                        compressed_size,
                        target as c_int,
                        capacity as c_int,
                    )
                };
                let rust_result = unsafe {
                    rust_partial(
                        compressed.as_ptr().cast(),
                        rust_output.as_mut_ptr().cast(),
                        compressed_size,
                        target as c_int,
                        capacity as c_int,
                    )
                };
                assert_result_and_bytes(
                    "LZ4_decompress_safe_partial",
                    c_result,
                    rust_result,
                    &c_output,
                    &rust_output,
                );
            }
        }

        let malformed = [0u8, 0, 0, 0, 0xFF, 0xFF];
        for compressed_size in [-1, 0, 1, malformed.len() as c_int] {
            for capacity in [-1, 0, 1, 32] {
                let mut c_output = [0u8; 32];
                let mut rust_output = [0u8; 32];
                let c_result = unsafe {
                    c_decompress(
                        malformed.as_ptr().cast(),
                        c_output.as_mut_ptr().cast(),
                        compressed_size,
                        capacity,
                    )
                };
                let rust_result = unsafe {
                    rust_decompress(
                        malformed.as_ptr().cast(),
                        rust_output.as_mut_ptr().cast(),
                        compressed_size,
                        capacity,
                    )
                };
                assert_eq!(c_result, rust_result);
                assert_eq!(c_output, rust_output);
            }
        }
    }
}

#[test]
fn external_state_stream_dictionary_and_compatibility_apis_match() {
    unsafe {
        let libraries = Libraries::open();
        type Size = unsafe extern "C" fn() -> c_int;
        type Ext = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        type Init = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        type Save = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
        type Continue = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        type Attach = unsafe extern "C" fn(*mut c_void, *const c_void);
        let c_size: Size = symbol(&libraries.c, "LZ4_sizeofState");
        let rust_size: Size = symbol(&libraries.rust, "LZ4_sizeofState");
        assert_eq!(unsafe { c_size() }, unsafe { rust_size() });
        let state_size = unsafe { c_size() } as usize;
        let c_ext: Ext = symbol(&libraries.c, "LZ4_compress_fast_extState");
        let rust_ext: Ext = symbol(&libraries.rust, "LZ4_compress_fast_extState");
        let c_init: Init = symbol(&libraries.c, "LZ4_initStream");
        let rust_init: Init = symbol(&libraries.rust, "LZ4_initStream");

        let mut c_state = vec![0u64; state_size.div_ceil(8) + 1];
        let mut rust_state = vec![0u64; state_size.div_ceil(8) + 1];
        assert_eq!(
            unsafe { c_init(ptr::null_mut(), state_size) }.is_null(),
            unsafe { rust_init(ptr::null_mut(), state_size) }.is_null()
        );
        assert_eq!(
            unsafe { c_init(c_state.as_mut_ptr().cast(), state_size - 1) }.is_null(),
            unsafe { rust_init(rust_state.as_mut_ptr().cast(), state_size - 1) }.is_null()
        );
        assert_eq!(
            unsafe { c_init(c_state.as_mut_ptr().cast::<u8>().add(1).cast(), state_size) }
                .is_null(),
            unsafe {
                rust_init(
                    rust_state.as_mut_ptr().cast::<u8>().add(1).cast(),
                    state_size,
                )
            }
            .is_null()
        );

        let mut seed = 0xCAF5_BABE_0102_0304;
        for _ in 0..32 {
            let len = (seed as usize % 70_000) + 1;
            let input = random_bytes(&mut seed, len);
            let bound = len + len / 255 + 16;
            let mut c_output = vec![0; bound];
            let mut rust_output = vec![0; bound];
            let c_result = unsafe {
                c_ext(
                    c_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    c_output.as_mut_ptr().cast(),
                    len as c_int,
                    bound as c_int,
                    1,
                )
            };
            let rust_result = unsafe {
                rust_ext(
                    rust_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    rust_output.as_mut_ptr().cast(),
                    len as c_int,
                    bound as c_int,
                    1,
                )
            };
            assert_result_and_bytes(
                "LZ4_compress_fast_extState",
                c_result,
                rust_result,
                &c_output,
                &rust_output,
            );
        }

        let c_create: Create = symbol(&libraries.c, "LZ4_createStream");
        let rust_create: Create = symbol(&libraries.rust, "LZ4_createStream");
        let c_free: Free = symbol(&libraries.c, "LZ4_freeStream");
        let rust_free: Free = symbol(&libraries.rust, "LZ4_freeStream");
        let c_load: Load = symbol(&libraries.c, "LZ4_loadDict");
        let rust_load: Load = symbol(&libraries.rust, "LZ4_loadDict");
        let c_save: Save = symbol(&libraries.c, "LZ4_saveDict");
        let rust_save: Save = symbol(&libraries.rust, "LZ4_saveDict");
        let c_continue: Continue = symbol(&libraries.c, "LZ4_compress_fast_continue");
        let rust_continue: Continue =
            symbol(&libraries.rust, "LZ4_compress_fast_continue");
        let c_attach: Attach = symbol(&libraries.c, "LZ4_attach_dictionary");
        let rust_attach: Attach = symbol(&libraries.rust, "LZ4_attach_dictionary");
        let c_stream = unsafe { c_create() };
        let rust_stream = unsafe { rust_create() };
        let c_dictionary_stream = unsafe { c_create() };
        let rust_dictionary_stream = unsafe { rust_create() };
        assert!(!c_stream.is_null() && !rust_stream.is_null());
        let dictionary = patterned_bytes(&mut seed, 70_000);
        assert_eq!(
            unsafe { c_load(c_dictionary_stream, dictionary.as_ptr().cast(), dictionary.len() as c_int) },
            unsafe {
                rust_load(
                    rust_dictionary_stream,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                )
            }
        );
        unsafe {
            c_attach(c_stream, c_dictionary_stream);
            rust_attach(rust_stream, rust_dictionary_stream);
        }
        for _ in 0..24 {
            let len = (seed as usize % 32_000) + 1;
            let input = patterned_bytes(&mut seed, len);
            let bound = len + len / 255 + 16;
            let mut c_output = vec![0; bound];
            let mut rust_output = vec![0; bound];
            let c_result = unsafe {
                c_continue(
                    c_stream,
                    input.as_ptr().cast(),
                    c_output.as_mut_ptr().cast(),
                    len as c_int,
                    bound as c_int,
                    2,
                )
            };
            let rust_result = unsafe {
                rust_continue(
                    rust_stream,
                    input.as_ptr().cast(),
                    rust_output.as_mut_ptr().cast(),
                    len as c_int,
                    bound as c_int,
                    2,
                )
            };
            assert_result_and_bytes(
                "LZ4_compress_fast_continue",
                c_result,
                rust_result,
                &c_output,
                &rust_output,
            );
        }
        let mut c_saved = vec![0; 65_536];
        let mut rust_saved = vec![0; 65_536];
        let c_saved_size =
            unsafe { c_save(c_stream, c_saved.as_mut_ptr().cast(), c_saved.len() as c_int) };
        let rust_saved_size = unsafe {
            rust_save(
                rust_stream,
                rust_saved.as_mut_ptr().cast(),
                rust_saved.len() as c_int,
            )
        };
        assert_eq!(c_saved_size, rust_saved_size);
        assert_eq!(
            &c_saved[..c_saved_size as usize],
            &rust_saved[..rust_saved_size as usize]
        );
        assert_eq!(unsafe { c_free(c_stream) }, unsafe { rust_free(rust_stream) });
        assert_eq!(
            unsafe { c_free(c_dictionary_stream) },
            unsafe { rust_free(rust_dictionary_stream) }
        );
    }
}

unsafe fn frame_one_shot(
    library: &Library,
    input: &[u8],
    preferences: &Preferences,
) -> (usize, Vec<u8>) {
    type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
    type Compress = unsafe extern "C" fn(
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const Preferences,
    ) -> usize;
    let bound: Bound = unsafe { symbol(library, "LZ4F_compressFrameBound") };
    let compress: Compress = unsafe { symbol(library, "LZ4F_compressFrame") };
    let capacity = unsafe { bound(input.len(), preferences) };
    let mut output = vec![0; capacity.max(1)];
    let result = unsafe {
        compress(
            output.as_mut_ptr().cast(),
            capacity,
            input.as_ptr().cast(),
            input.len(),
            preferences,
        )
    };
    (result, output)
}

#[test]
fn frame_option_cross_product_streaming_and_dictionary_match() {
    unsafe {
        let libraries = Libraries::open();
        let mut seed = 0x0DDC_0FFE_EE11_2233;
        let block_sizes = [0, 4, 5, 6, 7];
        let levels = [-1, 0, 9, 10, 12, 13];
        let mut cases = 0usize;
        for block_size_id in block_sizes {
            for block_mode in [0, 1] {
                for content_checksum_flag in [0, 1] {
                    for block_checksum_flag in [0, 1] {
                        for compression_level in levels {
                            for auto_flush in [0, 1] {
                                for favor_dec_speed in [0, 1] {
                                    let len = match cases % 8 {
                                        0 => 0,
                                        1 => 1,
                                        2 => 63,
                                        3 => 64,
                                        4 => 4097,
                                        5 => 65_535,
                                        6 => 65_536,
                                        _ => 65_537 + (seed as usize % 8192),
                                    };
                                    let input = if cases % 2 == 0 {
                                        random_bytes(&mut seed, len)
                                    } else {
                                        patterned_bytes(&mut seed, len)
                                    };
                                    let preferences = Preferences {
                                        frame_info: FrameInfo {
                                            block_size_id,
                                            block_mode,
                                            content_checksum_flag,
                                            frame_type: 0,
                                            content_size: if cases % 3 == 0 {
                                                len as u64
                                            } else {
                                                0
                                            },
                                            dict_id: if cases % 5 == 0 {
                                                0x1234_5678
                                            } else {
                                                0
                                            },
                                            block_checksum_flag,
                                        },
                                        compression_level,
                                        auto_flush,
                                        favor_dec_speed,
                                        reserved: [0; 3],
                                    };
                                    let (c_result, c_output) =
                                        unsafe { frame_one_shot(&libraries.c, &input, &preferences) };
                                    let (rust_result, rust_output) = unsafe {
                                        frame_one_shot(&libraries.rust, &input, &preferences)
                                    };
                                    assert_eq!(c_result, rust_result, "frame case {cases}");
                                    assert_eq!(
                                        &c_output[..c_result],
                                        &rust_output[..rust_result],
                                        "frame bytes case {cases}"
                                    );
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 960);

        type CreateCctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
        type Begin = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const Preferences,
        ) -> usize;
        type Update = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        type Flush = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        type End = Flush;
        type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
        let c_create: CreateCctx =
            symbol(&libraries.c, "LZ4F_createCompressionContext");
        let rust_create: CreateCctx =
            symbol(&libraries.rust, "LZ4F_createCompressionContext");
        let c_free: FreeCctx = symbol(&libraries.c, "LZ4F_freeCompressionContext");
        let rust_free: FreeCctx =
            symbol(&libraries.rust, "LZ4F_freeCompressionContext");
        let c_begin: Begin = symbol(&libraries.c, "LZ4F_compressBegin");
        let rust_begin: Begin = symbol(&libraries.rust, "LZ4F_compressBegin");
        let c_update: Update = symbol(&libraries.c, "LZ4F_compressUpdate");
        let rust_update: Update = symbol(&libraries.rust, "LZ4F_compressUpdate");
        let c_flush: Flush = symbol(&libraries.c, "LZ4F_flush");
        let rust_flush: Flush = symbol(&libraries.rust, "LZ4F_flush");
        let c_end: End = symbol(&libraries.c, "LZ4F_compressEnd");
        let rust_end: End = symbol(&libraries.rust, "LZ4F_compressEnd");
        let c_bound: Bound = symbol(&libraries.c, "LZ4F_compressBound");
        let rust_bound: Bound = symbol(&libraries.rust, "LZ4F_compressBound");

        for index in 0..32 {
            let preferences = Preferences {
                frame_info: FrameInfo {
                    block_size_id: block_sizes[index % block_sizes.len()],
                    block_mode: (index % 2) as c_int,
                    content_checksum_flag: ((index / 2) % 2) as c_int,
                    frame_type: 0,
                    content_size: 0,
                    dict_id: 0,
                    block_checksum_flag: ((index / 4) % 2) as c_int,
                },
                compression_level: levels[index % levels.len()],
                auto_flush: ((index / 8) % 2) as c_uint,
                favor_dec_speed: ((index / 16) % 2) as c_uint,
                reserved: [0; 3],
            };
            let mut c_ctx = ptr::null_mut();
            let mut rust_ctx = ptr::null_mut();
            assert_eq!(
                unsafe { c_create(&mut c_ctx, LZ4F_VERSION) },
                unsafe { rust_create(&mut rust_ctx, LZ4F_VERSION) }
            );
            assert!(!c_ctx.is_null() && !rust_ctx.is_null());
            let mut c_header = [0u8; 64];
            let mut rust_header = [0u8; 64];
            let c_header_size = unsafe {
                c_begin(
                    c_ctx,
                    c_header.as_mut_ptr().cast(),
                    c_header.len(),
                    &preferences,
                )
            };
            let rust_header_size = unsafe {
                rust_begin(
                    rust_ctx,
                    rust_header.as_mut_ptr().cast(),
                    rust_header.len(),
                    &preferences,
                )
            };
            assert_eq!(c_header_size, rust_header_size);
            assert_eq!(
                &c_header[..c_header_size],
                &rust_header[..rust_header_size]
            );
            let input = random_bytes(&mut seed, 1000 + index * 137);
            let capacity = unsafe { c_bound(input.len(), &preferences) };
            assert_eq!(capacity, unsafe { rust_bound(input.len(), &preferences) });
            let mut c_output = vec![0; capacity.max(64)];
            let mut rust_output = vec![0; capacity.max(64)];
            let options = CompressOptions {
                stable_src: (index % 2) as c_uint,
                reserved: [0; 3],
            };
            let c_size = unsafe {
                c_update(
                    c_ctx,
                    c_output.as_mut_ptr().cast(),
                    c_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    &options,
                )
            };
            let rust_size = unsafe {
                rust_update(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    rust_output.len(),
                    input.as_ptr().cast(),
                    input.len(),
                    &options,
                )
            };
            assert_eq!(c_size, rust_size);
            assert_eq!(&c_output[..c_size], &rust_output[..rust_size]);
            let c_flush_size = unsafe {
                c_flush(c_ctx, c_output.as_mut_ptr().cast(), c_output.len(), &options)
            };
            let rust_flush_size = unsafe {
                rust_flush(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    rust_output.len(),
                    &options,
                )
            };
            assert_eq!(c_flush_size, rust_flush_size);
            assert_eq!(
                &c_output[..c_flush_size],
                &rust_output[..rust_flush_size]
            );
            let c_end_size = unsafe {
                c_end(c_ctx, c_output.as_mut_ptr().cast(), c_output.len(), &options)
            };
            let rust_end_size = unsafe {
                rust_end(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    rust_output.len(),
                    &options,
                )
            };
            assert_eq!(c_end_size, rust_end_size);
            assert_eq!(&c_output[..c_end_size], &rust_output[..rust_end_size]);
            assert_eq!(unsafe { c_free(c_ctx) }, unsafe { rust_free(rust_ctx) });
        }

        type CreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
        type FreeCDict = unsafe extern "C" fn(*mut c_void);
        type CompressCDict = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const c_void,
            *const Preferences,
        ) -> usize;
        let c_create_cdict: CreateCDict = symbol(&libraries.c, "LZ4F_createCDict");
        let rust_create_cdict: CreateCDict =
            symbol(&libraries.rust, "LZ4F_createCDict");
        let c_free_cdict: FreeCDict = symbol(&libraries.c, "LZ4F_freeCDict");
        let rust_free_cdict: FreeCDict = symbol(&libraries.rust, "LZ4F_freeCDict");
        let c_compress_cdict: CompressCDict =
            symbol(&libraries.c, "LZ4F_compressFrame_usingCDict");
        let rust_compress_cdict: CompressCDict =
            symbol(&libraries.rust, "LZ4F_compressFrame_usingCDict");
        for dict_len in [0, 1, 64, 65_535, 65_536, 65_537] {
            let dictionary = random_bytes(&mut seed, dict_len);
            let c_cdict =
                unsafe { c_create_cdict(data_ptr(&dictionary).cast(), dictionary.len()) };
            let rust_cdict =
                unsafe { rust_create_cdict(data_ptr(&dictionary).cast(), dictionary.len()) };
            assert_eq!(c_cdict.is_null(), rust_cdict.is_null());
            if c_cdict.is_null() {
                continue;
            }
            let input = patterned_bytes(&mut seed, 8193);
            let preferences = Preferences::default();
            let capacity = 9000;
            let mut c_output = vec![0; capacity];
            let mut rust_output = vec![0; capacity];
            let mut c_ctx = ptr::null_mut();
            let mut rust_ctx = ptr::null_mut();
            assert_eq!(
                unsafe { c_create(&mut c_ctx, LZ4F_VERSION) },
                unsafe { rust_create(&mut rust_ctx, LZ4F_VERSION) }
            );
            let c_result = unsafe {
                c_compress_cdict(
                    c_ctx,
                    c_output.as_mut_ptr().cast(),
                    capacity,
                    input.as_ptr().cast(),
                    input.len(),
                    c_cdict,
                    &preferences,
                )
            };
            let rust_result = unsafe {
                rust_compress_cdict(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    capacity,
                    input.as_ptr().cast(),
                    input.len(),
                    rust_cdict,
                    &preferences,
                )
            };
            assert_eq!(c_result, rust_result);
            assert_eq!(&c_output[..c_result], &rust_output[..rust_result]);
            assert_eq!(unsafe { c_free(c_ctx) }, unsafe { rust_free(rust_ctx) });
            unsafe {
                c_free_cdict(c_cdict);
                rust_free_cdict(rust_cdict);
            }
        }
    }
}

#[test]
fn frame_decompression_and_header_error_paths_match() {
    unsafe {
        let libraries = Libraries::open();
        type HeaderSize = unsafe extern "C" fn(*const c_void, usize) -> usize;
        type CreateDctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
        type Decompress = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
            *const DecompressOptions,
        ) -> usize;
        let c_header: HeaderSize = symbol(&libraries.c, "LZ4F_headerSize");
        let rust_header: HeaderSize = symbol(&libraries.rust, "LZ4F_headerSize");
        for size in [0, 1, 4, 5, 6, 7, 19] {
            let bytes = [0u8; 19];
            assert_eq!(
                unsafe { c_header(bytes.as_ptr().cast(), size) },
                unsafe { rust_header(bytes.as_ptr().cast(), size) }
            );
        }
        assert_eq!(
            unsafe { c_header(ptr::null(), 5) },
            unsafe { rust_header(ptr::null(), 5) }
        );

        let c_create: CreateDctx =
            symbol(&libraries.c, "LZ4F_createDecompressionContext");
        let rust_create: CreateDctx =
            symbol(&libraries.rust, "LZ4F_createDecompressionContext");
        let c_free: FreeDctx = symbol(&libraries.c, "LZ4F_freeDecompressionContext");
        let rust_free: FreeDctx =
            symbol(&libraries.rust, "LZ4F_freeDecompressionContext");
        let c_decompress: Decompress = symbol(&libraries.c, "LZ4F_decompress");
        let rust_decompress: Decompress = symbol(&libraries.rust, "LZ4F_decompress");
        let mut seed = 0xACED_0001_3344_5566;
        for index in 0..80 {
            let len = if index < 8 {
                [0, 1, 4, 64, 65_535, 65_536, 65_537, 100_000][index]
            } else {
                seed as usize % 100_001
            };
            let input = if index % 2 == 0 {
                random_bytes(&mut seed, len)
            } else {
                patterned_bytes(&mut seed, len)
            };
            let preferences = Preferences {
                frame_info: FrameInfo {
                    block_size_id: [0, 4, 5, 6, 7][index % 5],
                    block_mode: (index % 2) as c_int,
                    content_checksum_flag: ((index / 2) % 2) as c_int,
                    frame_type: 0,
                    content_size: if index % 3 == 0 { len as u64 } else { 0 },
                    dict_id: 0,
                    block_checksum_flag: ((index / 4) % 2) as c_int,
                },
                compression_level: [-1, 0, 9, 10, 12, 13][index % 6],
                auto_flush: ((index / 8) % 2) as c_uint,
                favor_dec_speed: ((index / 16) % 2) as c_uint,
                reserved: [0; 3],
            };
            let (frame_size, frame) =
                unsafe { frame_one_shot(&libraries.c, &input, &preferences) };
            let frame = &frame[..frame_size];
            let mut c_ctx = ptr::null_mut();
            let mut rust_ctx = ptr::null_mut();
            assert_eq!(
                unsafe { c_create(&mut c_ctx, LZ4F_VERSION) },
                unsafe { rust_create(&mut rust_ctx, LZ4F_VERSION) }
            );
            let mut c_output = vec![0u8; len.max(1)];
            let mut rust_output = vec![0u8; len.max(1)];
            let mut c_src_size = frame.len();
            let mut rust_src_size = frame.len();
            let mut c_dst_size = len;
            let mut rust_dst_size = len;
            let options = DecompressOptions {
                stable_dst: (index % 2) as c_uint,
                reserved: [0; 3],
            };
            let c_result = unsafe {
                c_decompress(
                    c_ctx,
                    c_output.as_mut_ptr().cast(),
                    &mut c_dst_size,
                    frame.as_ptr().cast(),
                    &mut c_src_size,
                    &options,
                )
            };
            let rust_result = unsafe {
                rust_decompress(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    &mut rust_dst_size,
                    frame.as_ptr().cast(),
                    &mut rust_src_size,
                    &options,
                )
            };
            assert_eq!(c_result, rust_result);
            assert_eq!(c_src_size, rust_src_size);
            assert_eq!(c_dst_size, rust_dst_size);
            assert_eq!(&c_output[..c_dst_size], &rust_output[..rust_dst_size]);
            assert_eq!(&c_output[..c_dst_size], &input[..c_dst_size]);
            assert_eq!(unsafe { c_free(c_ctx) }, unsafe { rust_free(rust_ctx) });
        }

        for malformed in [
            vec![],
            vec![0],
            vec![0x04, 0x22, 0x4D, 0x18, 0],
            vec![0x50, 0x2A, 0x4D, 0x18, 0, 0, 0, 0],
            vec![0x04, 0x22, 0x4D, 0x18, 0xFF, 0xFF, 0],
        ] {
            let mut c_ctx = ptr::null_mut();
            let mut rust_ctx = ptr::null_mut();
            unsafe {
                c_create(&mut c_ctx, LZ4F_VERSION);
                rust_create(&mut rust_ctx, LZ4F_VERSION);
            }
            let mut c_output = [0u8; 64];
            let mut rust_output = [0u8; 64];
            let mut c_src_size = malformed.len();
            let mut rust_src_size = malformed.len();
            let mut c_dst_size = c_output.len();
            let mut rust_dst_size = rust_output.len();
            let c_result = unsafe {
                c_decompress(
                    c_ctx,
                    c_output.as_mut_ptr().cast(),
                    &mut c_dst_size,
                    malformed.as_ptr().cast(),
                    &mut c_src_size,
                    ptr::null(),
                )
            };
            let rust_result = unsafe {
                rust_decompress(
                    rust_ctx,
                    rust_output.as_mut_ptr().cast(),
                    &mut rust_dst_size,
                    malformed.as_ptr().cast(),
                    &mut rust_src_size,
                    ptr::null(),
                )
            };
            assert_eq!(c_result, rust_result);
            assert_eq!(c_src_size, rust_src_size);
            assert_eq!(c_dst_size, rust_dst_size);
            assert_eq!(unsafe { c_free(c_ctx) }, unsafe { rust_free(rust_ctx) });
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HcMatch {
    off: c_int,
    len: c_int,
    back: c_int,
}

#[test]
fn compatibility_state_hc_and_dictionary_exports_match() {
    unsafe {
        let libraries = Libraries::open();
        let mut seed = 0x5151_A0A0_7788_99AA;
        let input = patterned_bytes(&mut seed, 8192);
        type Bound = unsafe extern "C" fn(c_int) -> c_int;
        let c_bound: Bound = symbol(&libraries.c, "LZ4_compressBound");
        let bound = c_bound(input.len() as c_int);

        type Compress3 =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        for name in ["LZ4_compress", "LZ4_compressHC"] {
            let c_fn: Compress3 = symbol(&libraries.c, name);
            let rust_fn: Compress3 = symbol(&libraries.rust, name);
            let mut c_output = vec![0; bound as usize];
            let mut rust_output = vec![0; bound as usize];
            let c_result = c_fn(
                data_ptr(&input).cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            let rust_result = rust_fn(
                data_ptr(&input).cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }

        type Compress4 =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        for name in [
            "LZ4_compress_limitedOutput",
            "LZ4_compressHC_limitedOutput",
        ] {
            let c_fn: Compress4 = symbol(&libraries.c, name);
            let rust_fn: Compress4 = symbol(&libraries.rust, name);
            let mut c_output = vec![0; bound as usize];
            let mut rust_output = vec![0; bound as usize];
            let c_result = c_fn(
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            let rust_result = rust_fn(
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }

        type CompressLevel =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let c_hc2: CompressLevel = symbol(&libraries.c, "LZ4_compressHC2");
        let rust_hc2: CompressLevel = symbol(&libraries.rust, "LZ4_compressHC2");
        let mut c_output = vec![0; bound as usize];
        let mut rust_output = vec![0; bound as usize];
        let c_result = c_hc2(
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            11,
        );
        let rust_result = rust_hc2(
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            11,
        );
        assert_result_and_bytes(
            "LZ4_compressHC2",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type CompressCapLevel = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        let c_hc2_limited: CompressCapLevel =
            symbol(&libraries.c, "LZ4_compressHC2_limitedOutput");
        let rust_hc2_limited: CompressCapLevel =
            symbol(&libraries.rust, "LZ4_compressHC2_limitedOutput");
        let c_result = c_hc2_limited(
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            11,
        );
        let rust_result = rust_hc2_limited(
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            11,
        );
        assert_result_and_bytes(
            "LZ4_compressHC2_limitedOutput",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type Size = unsafe extern "C" fn() -> c_int;
        let c_state_size: Size = symbol(&libraries.c, "LZ4_sizeofStreamState");
        let rust_state_size: Size = symbol(&libraries.rust, "LZ4_sizeofStreamState");
        assert_eq!(c_state_size(), rust_state_size());
        let fast_size = c_state_size() as usize;
        let c_hc_size: Size = symbol(&libraries.c, "LZ4_sizeofStateHC");
        let rust_hc_size: Size = symbol(&libraries.rust, "LZ4_sizeofStateHC");
        assert_eq!(c_hc_size(), rust_hc_size());
        let hc_size = c_hc_size() as usize;

        type State4 =
            unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type State5 = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int;
        type State6 = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        for name in ["LZ4_compress_withState", "LZ4_compressHC_withStateHC"] {
            let c_fn: State4 = symbol(&libraries.c, name);
            let rust_fn: State4 = symbol(&libraries.rust, name);
            let size = if name.contains("HC") { hc_size } else { fast_size };
            let mut c_state = vec![0u64; size.div_ceil(8)];
            let mut rust_state = vec![0u64; size.div_ceil(8)];
            let c_result = c_fn(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            let rust_result = rust_fn(
                rust_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }
        for name in [
            "LZ4_compress_limitedOutput_withState",
            "LZ4_compressHC_limitedOutput_withStateHC",
        ] {
            let c_fn: State5 = symbol(&libraries.c, name);
            let rust_fn: State5 = symbol(&libraries.rust, name);
            let size = if name.contains("HC") { hc_size } else { fast_size };
            let mut c_state = vec![0u64; size.div_ceil(8)];
            let mut rust_state = vec![0u64; size.div_ceil(8)];
            let c_result = c_fn(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            let rust_result = rust_fn(
                rust_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }
        {
            let name = "LZ4_compressHC2_withStateHC";
            let c_fn: State5 = symbol(&libraries.c, name);
            let rust_fn: State5 = symbol(&libraries.rust, name);
            let mut c_state = vec![0u64; hc_size.div_ceil(8)];
            let mut rust_state = vec![0u64; hc_size.div_ceil(8)];
            let c_result = c_fn(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                11,
            );
            let rust_result = rust_fn(
                rust_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
                11,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }
        for name in [
            "LZ4_compressHC2_limitedOutput_withStateHC",
            "LZ4_compress_HC_extStateHC",
        ] {
            let c_fn: State6 = symbol(&libraries.c, name);
            let rust_fn: State6 = symbol(&libraries.rust, name);
            let mut c_state = vec![0u64; hc_size.div_ceil(8)];
            let mut rust_state = vec![0u64; hc_size.div_ceil(8)];
            let c_result = c_fn(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
                11,
            );
            let rust_result = rust_fn(
                rust_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
                11,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }

        let c_hc_fast_reset: State6 =
            symbol(&libraries.c, "LZ4_compress_HC_extStateHC_fastReset");
        let rust_hc_fast_reset: State6 =
            symbol(&libraries.rust, "LZ4_compress_HC_extStateHC_fastReset");
        let c_hc_ext: State6 = symbol(&libraries.c, "LZ4_compress_HC_extStateHC");
        let rust_hc_ext: State6 = symbol(&libraries.rust, "LZ4_compress_HC_extStateHC");
        let mut c_state = vec![0u64; hc_size.div_ceil(8)];
        let mut rust_state = vec![0u64; hc_size.div_ceil(8)];
        c_hc_ext(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            9,
        );
        rust_hc_ext(
            rust_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            9,
        );
        let c_result = c_hc_fast_reset(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            10,
        );
        let rust_result = rust_hc_fast_reset(
            rust_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            10,
        );
        assert_result_and_bytes(
            "LZ4_compress_HC_extStateHC_fastReset",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type DestSize =
            unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
        let c_dest: DestSize = symbol(&libraries.c, "LZ4_compress_destSize");
        let rust_dest: DestSize = symbol(&libraries.rust, "LZ4_compress_destSize");
        let mut c_consumed = input.len() as c_int;
        let mut rust_consumed = input.len() as c_int;
        let c_result = c_dest(
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            &mut c_consumed,
            256,
        );
        let rust_result = rust_dest(
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            &mut rust_consumed,
            256,
        );
        assert_eq!(c_consumed, rust_consumed);
        assert_result_and_bytes(
            "LZ4_compress_destSize",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type DestState = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            *mut c_int,
            c_int,
            c_int,
        ) -> c_int;
        let c_dest_state: DestState =
            symbol(&libraries.c, "LZ4_compress_destSize_extState");
        let rust_dest_state: DestState =
            symbol(&libraries.rust, "LZ4_compress_destSize_extState");
        let mut c_fast_state = vec![0u64; fast_size.div_ceil(8)];
        let mut rust_fast_state = vec![0u64; fast_size.div_ceil(8)];
        c_consumed = input.len() as c_int;
        rust_consumed = input.len() as c_int;
        let c_result = c_dest_state(
            c_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            &mut c_consumed,
            256,
            3,
        );
        let rust_result = rust_dest_state(
            rust_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            &mut rust_consumed,
            256,
            3,
        );
        assert_eq!(c_consumed, rust_consumed);
        assert_result_and_bytes(
            "LZ4_compress_destSize_extState",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_fast_ext: State6 = symbol(&libraries.c, "LZ4_compress_fast_extState");
        let rust_fast_ext: State6 =
            symbol(&libraries.rust, "LZ4_compress_fast_extState");
        let c_fast_reset: State6 =
            symbol(&libraries.c, "LZ4_compress_fast_extState_fastReset");
        let rust_fast_reset: State6 =
            symbol(&libraries.rust, "LZ4_compress_fast_extState_fastReset");
        c_fast_ext(
            c_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            1,
        );
        rust_fast_ext(
            rust_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            1,
        );
        let c_result = c_fast_reset(
            c_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            2,
        );
        let rust_result = rust_fast_reset(
            rust_fast_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            2,
        );
        assert_result_and_bytes(
            "LZ4_compress_fast_extState_fastReset",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type HcDest = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            *mut c_int,
            c_int,
            c_int,
        ) -> c_int;
        let c_hc_dest_state: HcDest = symbol(&libraries.c, "LZ4_compress_HC_destSize");
        let rust_hc_dest_state: HcDest =
            symbol(&libraries.rust, "LZ4_compress_HC_destSize");
        let mut c_hc_dest_memory = vec![0u64; hc_size.div_ceil(8)];
        let mut rust_hc_dest_memory = vec![0u64; hc_size.div_ceil(8)];
        c_consumed = input.len() as c_int;
        rust_consumed = input.len() as c_int;
        let c_result = c_hc_dest_state(
            c_hc_dest_memory.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            &mut c_consumed,
            512,
            10,
        );
        let rust_result = rust_hc_dest_state(
            rust_hc_dest_memory.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            &mut rust_consumed,
            512,
            10,
        );
        assert_eq!(c_consumed, rust_consumed);
        assert_result_and_bytes(
            "LZ4_compress_HC_destSize",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );

        type Create = unsafe extern "C" fn() -> *mut c_void;
        type CreateOld = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Reset = unsafe extern "C" fn(*mut c_void);
        type ResetState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
        type Load =
            unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        type LoadInternal =
            unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
        type Continue5 = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int;
        let dictionary = patterned_bytes(&mut seed, 65_536);
        let c_create_old: CreateOld = symbol(&libraries.c, "LZ4_create");
        let rust_create_old: CreateOld = symbol(&libraries.rust, "LZ4_create");
        let c_stream = c_create_old(ptr::null_mut());
        let rust_stream = rust_create_old(ptr::null_mut());
        let c_load_slow: Load = symbol(&libraries.c, "LZ4_loadDictSlow");
        let rust_load_slow: Load = symbol(&libraries.rust, "LZ4_loadDictSlow");
        assert_eq!(
            c_load_slow(c_stream, dictionary.as_ptr().cast(), dictionary.len() as c_int),
            rust_load_slow(
                rust_stream,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int
            )
        );
        let c_load_internal: LoadInternal = symbol(&libraries.c, "LZ4_loadDict_internal");
        let rust_load_internal: LoadInternal =
            symbol(&libraries.rust, "LZ4_loadDict_internal");
        for mode in [0, 1, 2] {
            assert_eq!(
                c_load_internal(
                    c_stream,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode,
                ),
                rust_load_internal(
                    rust_stream,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode,
                )
            );
        }
        let c_force: State4 = symbol(&libraries.c, "LZ4_compress_forceExtDict");
        let rust_force: State4 = symbol(&libraries.rust, "LZ4_compress_forceExtDict");
        let c_result = c_force(
            c_stream,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        let rust_result = rust_force(
            rust_stream,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        assert_result_and_bytes(
            "LZ4_compress_forceExtDict",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_reset: Reset = symbol(&libraries.c, "LZ4_resetStream");
        let rust_reset: Reset = symbol(&libraries.rust, "LZ4_resetStream");
        c_reset(c_stream);
        rust_reset(rust_stream);
        let c_reset_fast: Reset = symbol(&libraries.c, "LZ4_resetStream_fast");
        let rust_reset_fast: Reset = symbol(&libraries.rust, "LZ4_resetStream_fast");
        c_reset_fast(c_stream);
        rust_reset_fast(rust_stream);
        let c_reset_state: ResetState = symbol(&libraries.c, "LZ4_resetStreamState");
        let rust_reset_state: ResetState =
            symbol(&libraries.rust, "LZ4_resetStreamState");
        assert_eq!(
            c_reset_state(c_stream, ptr::null_mut()),
            rust_reset_state(rust_stream, ptr::null_mut())
        );
        let c_continue_old: State4 = symbol(&libraries.c, "LZ4_compress_continue");
        let rust_continue_old: State4 = symbol(&libraries.rust, "LZ4_compress_continue");
        let c_result = c_continue_old(
            c_stream,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        let rust_result = rust_continue_old(
            rust_stream,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        assert_result_and_bytes(
            "LZ4_compress_continue",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_continue_limited: Continue5 =
            symbol(&libraries.c, "LZ4_compress_limitedOutput_continue");
        let rust_continue_limited: Continue5 =
            symbol(&libraries.rust, "LZ4_compress_limitedOutput_continue");
        let c_result = c_continue_limited(
            c_stream,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
        );
        let rust_result = rust_continue_limited(
            rust_stream,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
        );
        assert_result_and_bytes(
            "LZ4_compress_limitedOutput_continue",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_slide: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
            symbol(&libraries.c, "LZ4_slideInputBuffer");
        let rust_slide: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
            symbol(&libraries.rust, "LZ4_slideInputBuffer");
        assert_eq!(c_slide(c_stream).is_null(), rust_slide(rust_stream).is_null());
        let c_free: Free = symbol(&libraries.c, "LZ4_freeStream");
        let rust_free: Free = symbol(&libraries.rust, "LZ4_freeStream");
        assert_eq!(c_free(c_stream), rust_free(rust_stream));

        let c_create_hc: Create = symbol(&libraries.c, "LZ4_createStreamHC");
        let rust_create_hc: Create = symbol(&libraries.rust, "LZ4_createStreamHC");
        let c_hc_stream = c_create_hc();
        let rust_hc_stream = rust_create_hc();
        let c_load_hc: Load = symbol(&libraries.c, "LZ4_loadDictHC");
        let rust_load_hc: Load = symbol(&libraries.rust, "LZ4_loadDictHC");
        assert_eq!(
            c_load_hc(
                c_hc_stream,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int
            ),
            rust_load_hc(
                rust_hc_stream,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int
            )
        );
        type SearchExtDict = unsafe extern "C" fn(
            *const u8,
            c_uint,
            *const u8,
            *const u8,
            *const c_void,
            c_uint,
            c_int,
            c_int,
        ) -> HcMatch;
        let c_search: SearchExtDict = symbol(&libraries.c, "LZ4HC_searchExtDict");
        let rust_search: SearchExtDict =
            symbol(&libraries.rust, "LZ4HC_searchExtDict");
        let c_match = c_search(
            input.as_ptr(),
            65_536,
            input.as_ptr(),
            input.as_ptr().add(input.len()),
            c_hc_stream,
            65_536,
            3,
            16,
        );
        let rust_match = rust_search(
            input.as_ptr(),
            65_536,
            input.as_ptr(),
            input.as_ptr().add(input.len()),
            rust_hc_stream,
            65_536,
            3,
            16,
        );
        assert_eq!(c_match, rust_match);
        let c_set_level: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.c, "LZ4_setCompressionLevel");
        let rust_set_level: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.rust, "LZ4_setCompressionLevel");
        let c_favor: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.c, "LZ4_favorDecompressionSpeed");
        let rust_favor: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.rust, "LZ4_favorDecompressionSpeed");
        c_set_level(c_hc_stream, 11);
        rust_set_level(rust_hc_stream, 11);
        c_favor(c_hc_stream, 1);
        rust_favor(rust_hc_stream, 1);
        let c_attach_hc: unsafe extern "C" fn(*mut c_void, *const c_void) =
            symbol(&libraries.c, "LZ4_attach_HC_dictionary");
        let rust_attach_hc: unsafe extern "C" fn(*mut c_void, *const c_void) =
            symbol(&libraries.rust, "LZ4_attach_HC_dictionary");
        c_attach_hc(c_hc_stream, ptr::null());
        rust_attach_hc(rust_hc_stream, ptr::null());
        let c_hc_continue: Continue5 = symbol(&libraries.c, "LZ4_compress_HC_continue");
        let rust_hc_continue: Continue5 =
            symbol(&libraries.rust, "LZ4_compress_HC_continue");
        let c_result = c_hc_continue(
            c_hc_stream,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
        );
        let rust_result = rust_hc_continue(
            rust_hc_stream,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
        );
        assert_result_and_bytes(
            "LZ4_compress_HC_continue",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        {
            let name = "LZ4_compressHC_continue";
            let c_fn: State4 = symbol(&libraries.c, name);
            let rust_fn: State4 = symbol(&libraries.rust, name);
            let c_result = c_fn(
                c_hc_stream,
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            let rust_result = rust_fn(
                rust_hc_stream,
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }
        {
            let name = "LZ4_compressHC_limitedOutput_continue";
            let c_fn: Continue5 = symbol(&libraries.c, name);
            let rust_fn: Continue5 = symbol(&libraries.rust, name);
            let c_result = c_fn(
                c_hc_stream,
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            let rust_result = rust_fn(
                rust_hc_stream,
                input.as_ptr().cast(),
                rust_output.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
            );
            assert_result_and_bytes(name, c_result, rust_result, &c_output, &rust_output);
        }
        type ContinueDest = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            *mut c_int,
            c_int,
        ) -> c_int;
        let c_hc_dest: ContinueDest =
            symbol(&libraries.c, "LZ4_compress_HC_continue_destSize");
        let rust_hc_dest: ContinueDest =
            symbol(&libraries.rust, "LZ4_compress_HC_continue_destSize");
        c_consumed = input.len() as c_int;
        rust_consumed = input.len() as c_int;
        let c_result = c_hc_dest(
            c_hc_stream,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            &mut c_consumed,
            512,
        );
        let rust_result = rust_hc_dest(
            rust_hc_stream,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            &mut rust_consumed,
            512,
        );
        assert_eq!(c_consumed, rust_consumed);
        assert_result_and_bytes(
            "LZ4_compress_HC_continue_destSize",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_save_hc: unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int =
            symbol(&libraries.c, "LZ4_saveDictHC");
        let rust_save_hc: unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int =
            symbol(&libraries.rust, "LZ4_saveDictHC");
        let mut c_saved = vec![0; 65_536];
        let mut rust_saved = vec![0; 65_536];
        let c_saved_size =
            c_save_hc(c_hc_stream, c_saved.as_mut_ptr().cast(), c_saved.len() as c_int);
        let rust_saved_size = rust_save_hc(
            rust_hc_stream,
            rust_saved.as_mut_ptr().cast(),
            rust_saved.len() as c_int,
        );
        assert_eq!(c_saved_size, rust_saved_size);
        assert_eq!(
            &c_saved[..c_saved_size as usize],
            &rust_saved[..rust_saved_size as usize]
        );
        let c_reset_hc: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.c, "LZ4_resetStreamHC");
        let rust_reset_hc: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.rust, "LZ4_resetStreamHC");
        let c_reset_hc_fast: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.c, "LZ4_resetStreamHC_fast");
        let rust_reset_hc_fast: unsafe extern "C" fn(*mut c_void, c_int) =
            symbol(&libraries.rust, "LZ4_resetStreamHC_fast");
        c_reset_hc(c_hc_stream, 9);
        rust_reset_hc(rust_hc_stream, 9);
        c_reset_hc_fast(c_hc_stream, 10);
        rust_reset_hc_fast(rust_hc_stream, 10);
        let c_free_hc: Free = symbol(&libraries.c, "LZ4_freeStreamHC");
        let rust_free_hc: Free = symbol(&libraries.rust, "LZ4_freeStreamHC");
        assert_eq!(c_free_hc(c_hc_stream), rust_free_hc(rust_hc_stream));

        type InitHc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        let c_init_hc: InitHc = symbol(&libraries.c, "LZ4_initStreamHC");
        let rust_init_hc: InitHc = symbol(&libraries.rust, "LZ4_initStreamHC");
        let mut c_hc_state = vec![0u64; hc_size.div_ceil(8)];
        let mut rust_hc_state = vec![0u64; hc_size.div_ceil(8)];
        assert_eq!(
            c_init_hc(c_hc_state.as_mut_ptr().cast(), hc_size).is_null(),
            rust_init_hc(rust_hc_state.as_mut_ptr().cast(), hc_size).is_null()
        );
        let c_reset_hc_state: ResetState =
            symbol(&libraries.c, "LZ4_resetStreamStateHC");
        let rust_reset_hc_state: ResetState =
            symbol(&libraries.rust, "LZ4_resetStreamStateHC");
        assert_eq!(
            c_reset_hc_state(c_hc_state.as_mut_ptr().cast(), input.as_ptr() as *mut c_char),
            rust_reset_hc_state(
                rust_hc_state.as_mut_ptr().cast(),
                input.as_ptr() as *mut c_char
            )
        );

        let c_create_hc_old: unsafe extern "C" fn(*const c_char) -> *mut c_void =
            symbol(&libraries.c, "LZ4_createHC");
        let rust_create_hc_old: unsafe extern "C" fn(*const c_char) -> *mut c_void =
            symbol(&libraries.rust, "LZ4_createHC");
        let c_old_hc = c_create_hc_old(input.as_ptr().cast());
        let rust_old_hc = rust_create_hc_old(input.as_ptr().cast());
        type OldHc5 = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int;
        type OldHc6 = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        let c_old_continue: OldHc5 = symbol(&libraries.c, "LZ4_compressHC2_continue");
        let rust_old_continue: OldHc5 =
            symbol(&libraries.rust, "LZ4_compressHC2_continue");
        let c_result = c_old_continue(
            c_old_hc,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            10,
        );
        let rust_result = rust_old_continue(
            rust_old_hc,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            10,
        );
        assert_result_and_bytes(
            "LZ4_compressHC2_continue",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_old_limited: OldHc6 =
            symbol(&libraries.c, "LZ4_compressHC2_limitedOutput_continue");
        let rust_old_limited: OldHc6 =
            symbol(&libraries.rust, "LZ4_compressHC2_limitedOutput_continue");
        let c_result = c_old_limited(
            c_old_hc,
            input.as_ptr().cast(),
            c_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            10,
        );
        let rust_result = rust_old_limited(
            rust_old_hc,
            input.as_ptr().cast(),
            rust_output.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
            10,
        );
        assert_result_and_bytes(
            "LZ4_compressHC2_limitedOutput_continue",
            c_result,
            rust_result,
            &c_output,
            &rust_output,
        );
        let c_slide_hc: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
            symbol(&libraries.c, "LZ4_slideInputBufferHC");
        let rust_slide_hc: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
            symbol(&libraries.rust, "LZ4_slideInputBufferHC");
        assert_eq!(
            c_slide_hc(c_old_hc).is_null(),
            rust_slide_hc(rust_old_hc).is_null()
        );
        let c_free_old_hc: Free = symbol(&libraries.c, "LZ4_freeHC");
        let rust_free_old_hc: Free = symbol(&libraries.rust, "LZ4_freeHC");
        assert_eq!(c_free_old_hc(c_old_hc), rust_free_old_hc(rust_old_hc));

        let mut compressed = vec![0; bound as usize];
        type CompressDefault =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let c_default: CompressDefault = symbol(&libraries.c, "LZ4_compress_default");
        let compressed_size = c_default(
            input.as_ptr().cast(),
            compressed.as_mut_ptr().cast(),
            input.len() as c_int,
            bound,
        );
        type Decode4 =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type DecodeDict6 = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int;
        type DecodeForce6 = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            *const c_void,
            usize,
        ) -> c_int;
        for name in [
            "LZ4_decompress_safe_withPrefix64k",
            "LZ4_uncompress_unknownOutputSize",
        ] {
            let c_fn: Decode4 = symbol(&libraries.c, name);
            let rust_fn: Decode4 = symbol(&libraries.rust, name);
            let mut c_decode = vec![0u8; 65_536 + input.len()];
            let mut rust_decode = vec![0u8; 65_536 + input.len()];
            let c_dest = c_decode.as_mut_ptr().add(65_536);
            let rust_dest = rust_decode.as_mut_ptr().add(65_536);
            let c_result = c_fn(
                compressed.as_ptr().cast(),
                c_dest.cast(),
                compressed_size,
                input.len() as c_int,
            );
            let rust_result = rust_fn(
                compressed.as_ptr().cast(),
                rust_dest.cast(),
                compressed_size,
                input.len() as c_int,
            );
            assert_eq!(c_result, rust_result, "{name}");
            assert_eq!(
                &c_decode[65_536..65_536 + input.len()],
                &rust_decode[65_536..65_536 + input.len()]
            );
        }
        let c_safe_dict: DecodeDict6 =
            symbol(&libraries.c, "LZ4_decompress_safe_usingDict");
        let rust_safe_dict: DecodeDict6 =
            symbol(&libraries.rust, "LZ4_decompress_safe_usingDict");
        let mut c_decode = vec![0; input.len()];
        let mut rust_decode = vec![0; input.len()];
        let c_result = c_safe_dict(
            compressed.as_ptr().cast(),
            c_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        let rust_result = rust_safe_dict(
            compressed.as_ptr().cast(),
            rust_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        assert_result_and_bytes(
            "LZ4_decompress_safe_usingDict",
            c_result,
            rust_result,
            &c_decode,
            &rust_decode,
        );
        let c_force_decode: DecodeForce6 =
            symbol(&libraries.c, "LZ4_decompress_safe_forceExtDict");
        let rust_force_decode: DecodeForce6 =
            symbol(&libraries.rust, "LZ4_decompress_safe_forceExtDict");
        let c_result = c_force_decode(
            compressed.as_ptr().cast(),
            c_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        let rust_result = rust_force_decode(
            compressed.as_ptr().cast(),
            rust_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        assert_result_and_bytes(
            "LZ4_decompress_safe_forceExtDict",
            c_result,
            rust_result,
            &c_decode,
            &rust_decode,
        );
        type PartialDict7 = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int;
        let c_partial_dict: PartialDict7 =
            symbol(&libraries.c, "LZ4_decompress_safe_partial_usingDict");
        let rust_partial_dict: PartialDict7 =
            symbol(&libraries.rust, "LZ4_decompress_safe_partial_usingDict");
        let c_result = c_partial_dict(
            compressed.as_ptr().cast(),
            c_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int / 2,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        let rust_result = rust_partial_dict(
            compressed.as_ptr().cast(),
            rust_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int / 2,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        assert_result_and_bytes(
            "LZ4_decompress_safe_partial_usingDict",
            c_result,
            rust_result,
            &c_decode,
            &rust_decode,
        );
        type PartialForce7 = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
            *const c_void,
            usize,
        ) -> c_int;
        let c_partial_force: PartialForce7 =
            symbol(&libraries.c, "LZ4_decompress_safe_partial_forceExtDict");
        let rust_partial_force: PartialForce7 =
            symbol(&libraries.rust, "LZ4_decompress_safe_partial_forceExtDict");
        let c_result = c_partial_force(
            compressed.as_ptr().cast(),
            c_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int / 2,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        let rust_result = rust_partial_force(
            compressed.as_ptr().cast(),
            rust_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int / 2,
            input.len() as c_int,
            ptr::null(),
            0,
        );
        assert_result_and_bytes(
            "LZ4_decompress_safe_partial_forceExtDict",
            c_result,
            rust_result,
            &c_decode,
            &rust_decode,
        );

        type DecodeFast =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        for name in ["LZ4_decompress_fast", "LZ4_uncompress"] {
            let c_fn: DecodeFast = symbol(&libraries.c, name);
            let rust_fn: DecodeFast = symbol(&libraries.rust, name);
            let c_result = c_fn(
                compressed.as_ptr().cast(),
                c_decode.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            let rust_result = rust_fn(
                compressed.as_ptr().cast(),
                rust_decode.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            assert_eq!(c_result, rust_result, "{name}");
            assert_eq!(c_decode, rust_decode);
        }
        type DecodeFastDict = unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int;
        let c_fast_dict: DecodeFastDict =
            symbol(&libraries.c, "LZ4_decompress_fast_usingDict");
        let rust_fast_dict: DecodeFastDict =
            symbol(&libraries.rust, "LZ4_decompress_fast_usingDict");
        assert_eq!(
            c_fast_dict(
                compressed.as_ptr().cast(),
                c_decode.as_mut_ptr().cast(),
                input.len() as c_int,
                ptr::null(),
                0
            ),
            rust_fast_dict(
                compressed.as_ptr().cast(),
                rust_decode.as_mut_ptr().cast(),
                input.len() as c_int,
                ptr::null(),
                0
            )
        );
        let mut c_prefix = vec![0u8; 65_536 + input.len()];
        let mut rust_prefix = vec![0u8; 65_536 + input.len()];
        type DecodeFastPrefix =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        let c_fast_prefix: DecodeFastPrefix =
            symbol(&libraries.c, "LZ4_decompress_fast_withPrefix64k");
        let rust_fast_prefix: DecodeFastPrefix =
            symbol(&libraries.rust, "LZ4_decompress_fast_withPrefix64k");
        assert_eq!(
            c_fast_prefix(
                compressed.as_ptr().cast(),
                c_prefix.as_mut_ptr().add(65_536).cast(),
                input.len() as c_int
            ),
            rust_fast_prefix(
                compressed.as_ptr().cast(),
                rust_prefix.as_mut_ptr().add(65_536).cast(),
                input.len() as c_int
            )
        );
        assert_eq!(c_prefix, rust_prefix);

        let c_create_decode: Create = symbol(&libraries.c, "LZ4_createStreamDecode");
        let rust_create_decode: Create = symbol(&libraries.rust, "LZ4_createStreamDecode");
        let c_decode_stream = c_create_decode();
        let rust_decode_stream = rust_create_decode();
        type SetDecode =
            unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        let c_set_decode: SetDecode = symbol(&libraries.c, "LZ4_setStreamDecode");
        let rust_set_decode: SetDecode = symbol(&libraries.rust, "LZ4_setStreamDecode");
        assert_eq!(
            c_set_decode(c_decode_stream, ptr::null(), 0),
            rust_set_decode(rust_decode_stream, ptr::null(), 0)
        );
        type DecodeContinue = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int;
        let c_safe_continue: DecodeContinue =
            symbol(&libraries.c, "LZ4_decompress_safe_continue");
        let rust_safe_continue: DecodeContinue =
            symbol(&libraries.rust, "LZ4_decompress_safe_continue");
        let c_result = c_safe_continue(
            c_decode_stream,
            compressed.as_ptr().cast(),
            c_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
        );
        let rust_result = rust_safe_continue(
            rust_decode_stream,
            compressed.as_ptr().cast(),
            rust_decode.as_mut_ptr().cast(),
            compressed_size,
            input.len() as c_int,
        );
        assert_result_and_bytes(
            "LZ4_decompress_safe_continue",
            c_result,
            rust_result,
            &c_decode,
            &rust_decode,
        );
        let c_free_decode: Free = symbol(&libraries.c, "LZ4_freeStreamDecode");
        let rust_free_decode: Free = symbol(&libraries.rust, "LZ4_freeStreamDecode");
        assert_eq!(
            c_free_decode(c_decode_stream),
            rust_free_decode(rust_decode_stream)
        );

        let c_decode_stream = c_create_decode();
        let rust_decode_stream = rust_create_decode();
        let c_fast_continue: unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
        ) -> c_int = symbol(&libraries.c, "LZ4_decompress_fast_continue");
        let rust_fast_continue: unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
        ) -> c_int = symbol(&libraries.rust, "LZ4_decompress_fast_continue");
        assert_eq!(
            c_fast_continue(
                c_decode_stream,
                compressed.as_ptr().cast(),
                c_decode.as_mut_ptr().cast(),
                input.len() as c_int
            ),
            rust_fast_continue(
                rust_decode_stream,
                compressed.as_ptr().cast(),
                rust_decode.as_mut_ptr().cast(),
                input.len() as c_int
            )
        );
        assert_eq!(
            c_free_decode(c_decode_stream),
            rust_free_decode(rust_decode_stream)
        );

        let c_xxh_version: unsafe extern "C" fn() -> c_uint =
            symbol(&libraries.c, "LZ4_XXH_versionNumber");
        let rust_xxh_version: unsafe extern "C" fn() -> c_uint =
            symbol(&libraries.rust, "LZ4_XXH_versionNumber");
        assert_eq!(c_xxh_version(), rust_xxh_version());
        type XxhCreate = unsafe extern "C" fn() -> *mut c_void;
        type XxhFree = unsafe extern "C" fn(*mut c_void) -> c_int;
        type XxhReset64 = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> c_int;
        type XxhCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
        type XxhDigest64 = unsafe extern "C" fn(*const c_void) -> c_ulonglong;
        let c_xxh_create: XxhCreate = symbol(&libraries.c, "LZ4_XXH64_createState");
        let rust_xxh_create: XxhCreate =
            symbol(&libraries.rust, "LZ4_XXH64_createState");
        let c_xxh_free: XxhFree = symbol(&libraries.c, "LZ4_XXH64_freeState");
        let rust_xxh_free: XxhFree = symbol(&libraries.rust, "LZ4_XXH64_freeState");
        let c_xxh_reset: XxhReset64 = symbol(&libraries.c, "LZ4_XXH64_reset");
        let rust_xxh_reset: XxhReset64 = symbol(&libraries.rust, "LZ4_XXH64_reset");
        let c_xxh_copy: XxhCopy = symbol(&libraries.c, "LZ4_XXH64_copyState");
        let rust_xxh_copy: XxhCopy = symbol(&libraries.rust, "LZ4_XXH64_copyState");
        let c_xxh_digest: XxhDigest64 = symbol(&libraries.c, "LZ4_XXH64_digest");
        let rust_xxh_digest: XxhDigest64 =
            symbol(&libraries.rust, "LZ4_XXH64_digest");
        let c_source = c_xxh_create();
        let rust_source = rust_xxh_create();
        let c_copy = c_xxh_create();
        let rust_copy = rust_xxh_create();
        assert_eq!(
            c_xxh_reset(c_source, 0xDEAD_BEEF_1234_5678),
            rust_xxh_reset(rust_source, 0xDEAD_BEEF_1234_5678)
        );
        c_xxh_copy(c_copy, c_source);
        rust_xxh_copy(rust_copy, rust_source);
        assert_eq!(c_xxh_digest(c_copy), rust_xxh_digest(rust_copy));
        assert_eq!(c_xxh_free(c_source), rust_xxh_free(rust_source));
        assert_eq!(c_xxh_free(c_copy), rust_xxh_free(rust_copy));
    }
}

#[test]
fn supplemental_frame_low_level_and_file_lifecycles_match() {
    unsafe {
        let libraries = Libraries::open();
        let mut seed = 0xF11E_1234_5678_9ABC;
        let dictionary = random_bytes(&mut seed, 65_536);
        let input = patterned_bytes(&mut seed, 100_000);
        let preferences = Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 1,
                content_checksum_flag: 1,
                frame_type: 0,
                content_size: input.len() as u64,
                dict_id: 7,
                block_checksum_flag: 1,
            },
            compression_level: 10,
            auto_flush: 1,
            favor_dec_speed: 1,
            reserved: [0; 3],
        };
        type CreateCctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_create: CreateCctx =
            symbol(&libraries.c, "LZ4F_createCompressionContext");
        let rust_create: CreateCctx =
            symbol(&libraries.rust, "LZ4F_createCompressionContext");
        let c_free: FreeCctx = symbol(&libraries.c, "LZ4F_freeCompressionContext");
        let rust_free: FreeCctx =
            symbol(&libraries.rust, "LZ4F_freeCompressionContext");
        type CreateCDictAdvanced =
            unsafe extern "C" fn(CustomMem, *const c_void, usize) -> *mut c_void;
        let c_create_cdict: CreateCDictAdvanced =
            symbol(&libraries.c, "LZ4F_createCDict_advanced");
        let rust_create_cdict: CreateCDictAdvanced =
            symbol(&libraries.rust, "LZ4F_createCDict_advanced");
        type FreeCDict = unsafe extern "C" fn(*mut c_void);
        let c_free_cdict: FreeCDict = symbol(&libraries.c, "LZ4F_freeCDict");
        let rust_free_cdict: FreeCDict = symbol(&libraries.rust, "LZ4F_freeCDict");
        let c_cdict = c_create_cdict(
            CustomMem::default(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
        );
        let rust_cdict = rust_create_cdict(
            CustomMem::default(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
        );
        assert_eq!(c_cdict.is_null(), rust_cdict.is_null());

        type BeginInternal = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const c_void,
            *const Preferences,
        ) -> usize;
        type BeginDict = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const Preferences,
        ) -> usize;
        type BeginCDict = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            *const Preferences,
        ) -> usize;
        for name in [
            "LZ4F_compressBegin_usingDict",
            "LZ4F_compressBegin_usingDictOnce",
        ] {
            let c_fn: BeginDict = symbol(&libraries.c, name);
            let rust_fn: BeginDict = symbol(&libraries.rust, name);
            let mut c_ctx = ptr::null_mut();
            let mut rust_ctx = ptr::null_mut();
            assert_eq!(
                c_create(&mut c_ctx, LZ4F_VERSION),
                rust_create(&mut rust_ctx, LZ4F_VERSION)
            );
            let mut c_header = [0u8; 64];
            let mut rust_header = [0u8; 64];
            let c_result = c_fn(
                c_ctx,
                c_header.as_mut_ptr().cast(),
                c_header.len(),
                dictionary.as_ptr().cast(),
                dictionary.len(),
                &preferences,
            );
            let rust_result = rust_fn(
                rust_ctx,
                rust_header.as_mut_ptr().cast(),
                rust_header.len(),
                dictionary.as_ptr().cast(),
                dictionary.len(),
                &preferences,
            );
            assert_eq!(c_result, rust_result, "{name}");
            assert_eq!(&c_header[..c_result], &rust_header[..rust_result]);
            assert_eq!(c_free(c_ctx), rust_free(rust_ctx));
        }
        let c_begin_internal: BeginInternal =
            symbol(&libraries.c, "LZ4F_compressBegin_internal");
        let rust_begin_internal: BeginInternal =
            symbol(&libraries.rust, "LZ4F_compressBegin_internal");
        let mut c_ctx = ptr::null_mut();
        let mut rust_ctx = ptr::null_mut();
        assert_eq!(
            c_create(&mut c_ctx, LZ4F_VERSION),
            rust_create(&mut rust_ctx, LZ4F_VERSION)
        );
        let mut c_header = [0u8; 64];
        let mut rust_header = [0u8; 64];
        let c_result = c_begin_internal(
            c_ctx,
            c_header.as_mut_ptr().cast(),
            c_header.len(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
            ptr::null(),
            &preferences,
        );
        let rust_result = rust_begin_internal(
            rust_ctx,
            rust_header.as_mut_ptr().cast(),
            rust_header.len(),
            dictionary.as_ptr().cast(),
            dictionary.len(),
            ptr::null(),
            &preferences,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(&c_header[..c_result], &rust_header[..rust_result]);
        assert_eq!(c_free(c_ctx), rust_free(rust_ctx));

        let c_begin_cdict: BeginCDict =
            symbol(&libraries.c, "LZ4F_compressBegin_usingCDict");
        let rust_begin_cdict: BeginCDict =
            symbol(&libraries.rust, "LZ4F_compressBegin_usingCDict");
        let mut c_ctx = ptr::null_mut();
        let mut rust_ctx = ptr::null_mut();
        assert_eq!(
            c_create(&mut c_ctx, LZ4F_VERSION),
            rust_create(&mut rust_ctx, LZ4F_VERSION)
        );
        let c_result = c_begin_cdict(
            c_ctx,
            c_header.as_mut_ptr().cast(),
            c_header.len(),
            c_cdict,
            &preferences,
        );
        let rust_result = rust_begin_cdict(
            rust_ctx,
            rust_header.as_mut_ptr().cast(),
            rust_header.len(),
            rust_cdict,
            &preferences,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(&c_header[..c_result], &rust_header[..rust_result]);
        assert_eq!(c_free(c_ctx), rust_free(rust_ctx));
        c_free_cdict(c_cdict);
        rust_free_cdict(rust_cdict);

        let independent = Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 1,
                ..FrameInfo::default()
            },
            auto_flush: 1,
            ..Preferences::default()
        };
        type Begin = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const Preferences,
        ) -> usize;
        type Update = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        let c_begin: Begin = symbol(&libraries.c, "LZ4F_compressBegin");
        let rust_begin: Begin = symbol(&libraries.rust, "LZ4F_compressBegin");
        let c_uncompressed: Update = symbol(&libraries.c, "LZ4F_uncompressedUpdate");
        let rust_uncompressed: Update =
            symbol(&libraries.rust, "LZ4F_uncompressedUpdate");
        let mut c_ctx = ptr::null_mut();
        let mut rust_ctx = ptr::null_mut();
        assert_eq!(
            c_create(&mut c_ctx, LZ4F_VERSION),
            rust_create(&mut rust_ctx, LZ4F_VERSION)
        );
        c_begin(c_ctx, c_header.as_mut_ptr().cast(), c_header.len(), &independent);
        rust_begin(
            rust_ctx,
            rust_header.as_mut_ptr().cast(),
            rust_header.len(),
            &independent,
        );
        let mut c_output = vec![0; input.len() + 64];
        let mut rust_output = vec![0; input.len() + 64];
        let c_result = c_uncompressed(
            c_ctx,
            c_output.as_mut_ptr().cast(),
            c_output.len(),
            input.as_ptr().cast(),
            input.len(),
            ptr::null(),
        );
        let rust_result = rust_uncompressed(
            rust_ctx,
            rust_output.as_mut_ptr().cast(),
            rust_output.len(),
            input.as_ptr().cast(),
            input.len(),
            ptr::null(),
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(&c_output[..c_result], &rust_output[..rust_result]);
        assert_eq!(c_free(c_ctx), rust_free(rust_ctx));

        let (frame_size, frame) = frame_one_shot(&libraries.c, &input, &preferences);
        let frame = &frame[..frame_size];
        type CreateDctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
        type GetInfo = unsafe extern "C" fn(
            *mut c_void,
            *mut FrameInfo,
            *const c_void,
            *mut usize,
        ) -> usize;
        type ResetDctx = unsafe extern "C" fn(*mut c_void);
        type DecompressDict = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
            *const c_void,
            usize,
            *const DecompressOptions,
        ) -> usize;
        let c_create_d: CreateDctx =
            symbol(&libraries.c, "LZ4F_createDecompressionContext");
        let rust_create_d: CreateDctx =
            symbol(&libraries.rust, "LZ4F_createDecompressionContext");
        let c_free_d: FreeDctx = symbol(&libraries.c, "LZ4F_freeDecompressionContext");
        let rust_free_d: FreeDctx =
            symbol(&libraries.rust, "LZ4F_freeDecompressionContext");
        let c_info: GetInfo = symbol(&libraries.c, "LZ4F_getFrameInfo");
        let rust_info: GetInfo = symbol(&libraries.rust, "LZ4F_getFrameInfo");
        let c_reset_d: ResetDctx =
            symbol(&libraries.c, "LZ4F_resetDecompressionContext");
        let rust_reset_d: ResetDctx =
            symbol(&libraries.rust, "LZ4F_resetDecompressionContext");
        let c_decompress_dict: DecompressDict =
            symbol(&libraries.c, "LZ4F_decompress_usingDict");
        let rust_decompress_dict: DecompressDict =
            symbol(&libraries.rust, "LZ4F_decompress_usingDict");
        let mut c_dctx = ptr::null_mut();
        let mut rust_dctx = ptr::null_mut();
        assert_eq!(
            c_create_d(&mut c_dctx, LZ4F_VERSION),
            rust_create_d(&mut rust_dctx, LZ4F_VERSION)
        );
        let mut c_info_value = FrameInfo::default();
        let mut rust_info_value = FrameInfo::default();
        let mut c_src = frame.len();
        let mut rust_src = frame.len();
        let c_result = c_info(
            c_dctx,
            &mut c_info_value,
            frame.as_ptr().cast(),
            &mut c_src,
        );
        let rust_result = rust_info(
            rust_dctx,
            &mut rust_info_value,
            frame.as_ptr().cast(),
            &mut rust_src,
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(c_src, rust_src);
        assert_eq!(c_info_value.block_size_id, rust_info_value.block_size_id);
        c_reset_d(c_dctx);
        rust_reset_d(rust_dctx);
        let mut c_decoded = vec![0; input.len()];
        let mut rust_decoded = vec![0; input.len()];
        c_src = frame.len();
        rust_src = frame.len();
        let mut c_dst = c_decoded.len();
        let mut rust_dst = rust_decoded.len();
        let c_result = c_decompress_dict(
            c_dctx,
            c_decoded.as_mut_ptr().cast(),
            &mut c_dst,
            frame.as_ptr().cast(),
            &mut c_src,
            ptr::null(),
            0,
            ptr::null(),
        );
        let rust_result = rust_decompress_dict(
            rust_dctx,
            rust_decoded.as_mut_ptr().cast(),
            &mut rust_dst,
            frame.as_ptr().cast(),
            &mut rust_src,
            ptr::null(),
            0,
            ptr::null(),
        );
        assert_eq!(c_result, rust_result);
        assert_eq!(c_src, rust_src);
        assert_eq!(c_dst, rust_dst);
        assert_eq!(&c_decoded[..c_dst], &rust_decoded[..rust_dst]);
        assert_eq!(c_free_d(c_dctx), rust_free_d(rust_dctx));

        type WriteOpen =
            unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
        type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
        type Close = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_write_open: WriteOpen = symbol(&libraries.c, "LZ4F_writeOpen");
        let rust_write_open: WriteOpen = symbol(&libraries.rust, "LZ4F_writeOpen");
        let c_write: Write = symbol(&libraries.c, "LZ4F_write");
        let rust_write: Write = symbol(&libraries.rust, "LZ4F_write");
        let c_write_close: Close = symbol(&libraries.c, "LZ4F_writeClose");
        let rust_write_close: Close = symbol(&libraries.rust, "LZ4F_writeClose");
        let c_file = tmpfile();
        let rust_file = tmpfile();
        assert!(!c_file.is_null() && !rust_file.is_null());
        let mut c_writer = ptr::null_mut();
        let mut rust_writer = ptr::null_mut();
        assert_eq!(
            c_write_open(&mut c_writer, c_file, &preferences),
            rust_write_open(&mut rust_writer, rust_file, &preferences)
        );
        for chunk in input.chunks(7777) {
            assert_eq!(
                c_write(c_writer, chunk.as_ptr().cast(), chunk.len()),
                rust_write(rust_writer, chunk.as_ptr().cast(), chunk.len())
            );
        }
        assert_eq!(c_write_close(c_writer), rust_write_close(rust_writer));
        rewind(c_file);
        rewind(rust_file);
        let mut c_file_bytes = Vec::new();
        let mut rust_file_bytes = Vec::new();
        loop {
            let mut c_chunk = [0u8; 4096];
            let mut rust_chunk = [0u8; 4096];
            let c_count = fread(c_chunk.as_mut_ptr().cast(), 1, c_chunk.len(), c_file);
            let rust_count =
                fread(rust_chunk.as_mut_ptr().cast(), 1, rust_chunk.len(), rust_file);
            assert_eq!(c_count, rust_count);
            c_file_bytes.extend_from_slice(&c_chunk[..c_count]);
            rust_file_bytes.extend_from_slice(&rust_chunk[..rust_count]);
            if c_count == 0 {
                break;
            }
        }
        assert_eq!(c_file_bytes, rust_file_bytes);
        assert_eq!(fclose(c_file), 0);
        assert_eq!(fclose(rust_file), 0);

        type ReadOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
        type Read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
        let c_read_open: ReadOpen = symbol(&libraries.c, "LZ4F_readOpen");
        let rust_read_open: ReadOpen = symbol(&libraries.rust, "LZ4F_readOpen");
        let c_read: Read = symbol(&libraries.c, "LZ4F_read");
        let rust_read: Read = symbol(&libraries.rust, "LZ4F_read");
        let c_read_close: Close = symbol(&libraries.c, "LZ4F_readClose");
        let rust_read_close: Close = symbol(&libraries.rust, "LZ4F_readClose");
        let c_file = tmpfile();
        let rust_file = tmpfile();
        assert_eq!(
            fwrite(
                c_file_bytes.as_ptr().cast(),
                1,
                c_file_bytes.len(),
                c_file
            ),
            c_file_bytes.len()
        );
        assert_eq!(
            fwrite(
                rust_file_bytes.as_ptr().cast(),
                1,
                rust_file_bytes.len(),
                rust_file
            ),
            rust_file_bytes.len()
        );
        rewind(c_file);
        rewind(rust_file);
        let mut c_reader = ptr::null_mut();
        let mut rust_reader = ptr::null_mut();
        assert_eq!(
            c_read_open(&mut c_reader, c_file),
            rust_read_open(&mut rust_reader, rust_file)
        );
        let mut c_plain = Vec::new();
        let mut rust_plain = Vec::new();
        loop {
            let mut c_chunk = [0u8; 3333];
            let mut rust_chunk = [0u8; 3333];
            let c_count = c_read(c_reader, c_chunk.as_mut_ptr().cast(), c_chunk.len());
            let rust_count =
                rust_read(rust_reader, rust_chunk.as_mut_ptr().cast(), rust_chunk.len());
            assert_eq!(c_count, rust_count);
            c_plain.extend_from_slice(&c_chunk[..c_count]);
            rust_plain.extend_from_slice(&rust_chunk[..rust_count]);
            if c_count == 0 {
                break;
            }
        }
        assert_eq!(c_plain, rust_plain);
        assert_eq!(c_plain, input);
        assert_eq!(c_read_close(c_reader), rust_read_close(rust_reader));
        assert_eq!(fclose(c_file), 0);
        assert_eq!(fclose(rust_file), 0);
    }
}

#[test]
fn frame_error_mutations_and_state_rejections_match() {
    unsafe {
        let libraries = Libraries::open();
        type CreateCctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
        type Update = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        type Flush = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        type Begin = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const Preferences,
        ) -> usize;
        type CompressFrame = unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const Preferences,
        ) -> usize;
        let c_create: CreateCctx =
            symbol(&libraries.c, "LZ4F_createCompressionContext");
        let rust_create: CreateCctx =
            symbol(&libraries.rust, "LZ4F_createCompressionContext");
        let c_free: FreeCctx = symbol(&libraries.c, "LZ4F_freeCompressionContext");
        let rust_free: FreeCctx =
            symbol(&libraries.rust, "LZ4F_freeCompressionContext");
        let c_update: Update = symbol(&libraries.c, "LZ4F_compressUpdate");
        let rust_update: Update = symbol(&libraries.rust, "LZ4F_compressUpdate");
        let c_flush: Flush = symbol(&libraries.c, "LZ4F_flush");
        let rust_flush: Flush = symbol(&libraries.rust, "LZ4F_flush");
        let c_end: Flush = symbol(&libraries.c, "LZ4F_compressEnd");
        let rust_end: Flush = symbol(&libraries.rust, "LZ4F_compressEnd");
        let c_begin: Begin = symbol(&libraries.c, "LZ4F_compressBegin");
        let rust_begin: Begin = symbol(&libraries.rust, "LZ4F_compressBegin");
        let c_frame: CompressFrame = symbol(&libraries.c, "LZ4F_compressFrame");
        let rust_frame: CompressFrame = symbol(&libraries.rust, "LZ4F_compressFrame");
        let input = vec![0x5Au8; 4096];

        let mut c_ctx = ptr::null_mut();
        let mut rust_ctx = ptr::null_mut();
        assert_eq!(
            c_create(&mut c_ctx, LZ4F_VERSION),
            rust_create(&mut rust_ctx, LZ4F_VERSION)
        );
        let mut c_output = vec![0u8; 8192];
        let mut rust_output = vec![0u8; 8192];
        assert_eq!(
            c_update(
                c_ctx,
                c_output.as_mut_ptr().cast(),
                c_output.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            ),
            rust_update(
                rust_ctx,
                rust_output.as_mut_ptr().cast(),
                rust_output.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        );
        assert_eq!(
            c_flush(c_ctx, c_output.as_mut_ptr().cast(), c_output.len(), ptr::null()),
            rust_flush(
                rust_ctx,
                rust_output.as_mut_ptr().cast(),
                rust_output.len(),
                ptr::null()
            )
        );
        assert_eq!(
            c_end(c_ctx, c_output.as_mut_ptr().cast(), 3, ptr::null()),
            rust_end(rust_ctx, rust_output.as_mut_ptr().cast(), 3, ptr::null())
        );
        assert_eq!(c_free(c_ctx), rust_free(rust_ctx));

        let mut c_ctx = ptr::null_mut();
        let mut rust_ctx = ptr::null_mut();
        c_create(&mut c_ctx, LZ4F_VERSION);
        rust_create(&mut rust_ctx, LZ4F_VERSION);
        let preferences = Preferences::default();
        assert_eq!(
            c_begin(c_ctx, c_output.as_mut_ptr().cast(), 18, &preferences),
            rust_begin(rust_ctx, rust_output.as_mut_ptr().cast(), 18, &preferences)
        );
        assert_eq!(c_free(c_ctx), rust_free(rust_ctx));
        assert_eq!(
            c_frame(
                c_output.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                &preferences,
            ),
            rust_frame(
                rust_output.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                &preferences,
            )
        );

        type CreateDctx =
            unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type FreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
        type Decompress = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
            *const DecompressOptions,
        ) -> usize;
        let c_create_d: CreateDctx =
            symbol(&libraries.c, "LZ4F_createDecompressionContext");
        let rust_create_d: CreateDctx =
            symbol(&libraries.rust, "LZ4F_createDecompressionContext");
        let c_free_d: FreeDctx = symbol(&libraries.c, "LZ4F_freeDecompressionContext");
        let rust_free_d: FreeDctx =
            symbol(&libraries.rust, "LZ4F_freeDecompressionContext");
        let c_decompress: Decompress = symbol(&libraries.c, "LZ4F_decompress");
        let rust_decompress: Decompress = symbol(&libraries.rust, "LZ4F_decompress");

        let checked_preferences = Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 1,
                content_checksum_flag: 1,
                frame_type: 0,
                content_size: input.len() as u64,
                dict_id: 0,
                block_checksum_flag: 1,
            },
            auto_flush: 1,
            ..Preferences::default()
        };
        let (frame_size, frame) =
            frame_one_shot(&libraries.c, &input, &checked_preferences);
        let base = frame[..frame_size].to_vec();
        let header_size = 15usize;
        let block_header = header_size;
        let encoded_block_size = u32::from_le_bytes(
            base[block_header..block_header + 4]
                .try_into()
                .unwrap(),
        );
        let payload_size = (encoded_block_size & 0x7FFF_FFFF) as usize;
        let payload_start = block_header + 4;
        let block_checksum = payload_start + payload_size;

        let mut mutations = Vec::new();
        let mut reserved_flg = base.clone();
        reserved_flg[4] |= 0x02;
        mutations.push(reserved_flg);
        let mut wrong_version = base.clone();
        wrong_version[4] &= 0x3F;
        mutations.push(wrong_version);
        let mut reserved_bd_high = base.clone();
        reserved_bd_high[5] |= 0x80;
        mutations.push(reserved_bd_high);
        let mut invalid_block_id = base.clone();
        invalid_block_id[5] = (invalid_block_id[5] & 0x0F) | 0x30;
        mutations.push(invalid_block_id);
        let mut reserved_bd_low = base.clone();
        reserved_bd_low[5] |= 0x01;
        mutations.push(reserved_bd_low);
        let mut bad_header_checksum = base.clone();
        bad_header_checksum[header_size - 1] ^= 0x80;
        mutations.push(bad_header_checksum);
        let mut oversized_block = base.clone();
        oversized_block[block_header..block_header + 4]
            .copy_from_slice(&(65_537u32).to_le_bytes());
        mutations.push(oversized_block);
        let mut bad_payload = base.clone();
        bad_payload[payload_start] ^= 0xFF;
        mutations.push(bad_payload);
        let mut bad_block_checksum = base.clone();
        bad_block_checksum[block_checksum] ^= 0x01;
        mutations.push(bad_block_checksum);
        let mut bad_content_checksum = base.clone();
        let last = bad_content_checksum.len() - 1;
        bad_content_checksum[last] ^= 0x01;
        mutations.push(bad_content_checksum);

        for (index, mutation) in mutations.iter().enumerate() {
            let mut c_dctx = ptr::null_mut();
            let mut rust_dctx = ptr::null_mut();
            assert_eq!(
                c_create_d(&mut c_dctx, LZ4F_VERSION),
                rust_create_d(&mut rust_dctx, LZ4F_VERSION)
            );
            let mut c_plain = vec![0u8; input.len()];
            let mut rust_plain = vec![0u8; input.len()];
            let mut c_src = mutation.len();
            let mut rust_src = mutation.len();
            let mut c_dst = c_plain.len();
            let mut rust_dst = rust_plain.len();
            let c_result = c_decompress(
                c_dctx,
                c_plain.as_mut_ptr().cast(),
                &mut c_dst,
                mutation.as_ptr().cast(),
                &mut c_src,
                ptr::null(),
            );
            let rust_result = rust_decompress(
                rust_dctx,
                rust_plain.as_mut_ptr().cast(),
                &mut rust_dst,
                mutation.as_ptr().cast(),
                &mut rust_src,
                ptr::null(),
            );
            assert_eq!(c_result, rust_result, "mutation {index}");
            assert_eq!(c_src, rust_src, "mutation {index} consumed");
            assert_eq!(c_dst, rust_dst, "mutation {index} produced");
            assert_eq!(c_free_d(c_dctx), rust_free_d(rust_dctx));
        }
    }
}

#[test]
fn context_custom_memory_and_file_null_rejections_match() {
    unsafe {
        let libraries = Libraries::open();
        type CreateAdvanced = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
        type FreeContext = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_create_c: CreateAdvanced =
            symbol(&libraries.c, "LZ4F_createCompressionContext_advanced");
        let rust_create_c: CreateAdvanced =
            symbol(&libraries.rust, "LZ4F_createCompressionContext_advanced");
        let c_create_d: CreateAdvanced =
            symbol(&libraries.c, "LZ4F_createDecompressionContext_advanced");
        let rust_create_d: CreateAdvanced =
            symbol(&libraries.rust, "LZ4F_createDecompressionContext_advanced");
        let c_free_c: FreeContext = symbol(&libraries.c, "LZ4F_freeCompressionContext");
        let rust_free_c: FreeContext =
            symbol(&libraries.rust, "LZ4F_freeCompressionContext");
        let c_free_d: FreeContext = symbol(&libraries.c, "LZ4F_freeDecompressionContext");
        let rust_free_d: FreeContext =
            symbol(&libraries.rust, "LZ4F_freeDecompressionContext");
        let memory = CustomMem::default();
        let c_cctx = unsafe { c_create_c(memory, LZ4F_VERSION) };
        let rust_cctx = unsafe { rust_create_c(memory, LZ4F_VERSION) };
        assert_eq!(c_cctx.is_null(), rust_cctx.is_null());
        let c_dctx = unsafe { c_create_d(memory, LZ4F_VERSION) };
        let rust_dctx = unsafe { rust_create_d(memory, LZ4F_VERSION) };
        assert_eq!(c_dctx.is_null(), rust_dctx.is_null());
        assert_eq!(unsafe { c_free_c(c_cctx) }, unsafe { rust_free_c(rust_cctx) });
        assert_eq!(unsafe { c_free_d(c_dctx) }, unsafe { rust_free_d(rust_dctx) });

        type FileOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
        type FileWriteOpen =
            unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
        type FileIo = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
        type FileWrite =
            unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
        type FileClose = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_read_open: FileOpen = symbol(&libraries.c, "LZ4F_readOpen");
        let rust_read_open: FileOpen = symbol(&libraries.rust, "LZ4F_readOpen");
        let c_write_open: FileWriteOpen = symbol(&libraries.c, "LZ4F_writeOpen");
        let rust_write_open: FileWriteOpen = symbol(&libraries.rust, "LZ4F_writeOpen");
        let c_read: FileIo = symbol(&libraries.c, "LZ4F_read");
        let rust_read: FileIo = symbol(&libraries.rust, "LZ4F_read");
        let c_write: FileWrite = symbol(&libraries.c, "LZ4F_write");
        let rust_write: FileWrite = symbol(&libraries.rust, "LZ4F_write");
        let c_read_close: FileClose = symbol(&libraries.c, "LZ4F_readClose");
        let rust_read_close: FileClose = symbol(&libraries.rust, "LZ4F_readClose");
        let c_write_close: FileClose = symbol(&libraries.c, "LZ4F_writeClose");
        let rust_write_close: FileClose = symbol(&libraries.rust, "LZ4F_writeClose");
        let mut c_state = ptr::null_mut();
        let mut rust_state = ptr::null_mut();
        assert_eq!(
            unsafe { c_read_open(&mut c_state, ptr::null_mut()) },
            unsafe { rust_read_open(&mut rust_state, ptr::null_mut()) }
        );
        assert_eq!(
            unsafe { c_read_open(ptr::null_mut(), ptr::null_mut()) },
            unsafe { rust_read_open(ptr::null_mut(), ptr::null_mut()) }
        );
        assert_eq!(
            unsafe { c_write_open(&mut c_state, ptr::null_mut(), ptr::null()) },
            unsafe { rust_write_open(&mut rust_state, ptr::null_mut(), ptr::null()) }
        );
        let mut byte = 0u8;
        assert_eq!(
            unsafe { c_read(ptr::null_mut(), (&mut byte as *mut u8).cast(), 1) },
            unsafe { rust_read(ptr::null_mut(), (&mut byte as *mut u8).cast(), 1) }
        );
        assert_eq!(
            unsafe { c_write(ptr::null_mut(), (&byte as *const u8).cast(), 1) },
            unsafe { rust_write(ptr::null_mut(), (&byte as *const u8).cast(), 1) }
        );
        assert_eq!(
            unsafe { c_read_close(ptr::null_mut()) },
            unsafe { rust_read_close(ptr::null_mut()) }
        );
        assert_eq!(
            unsafe { c_write_close(ptr::null_mut()) },
            unsafe { rust_write_close(ptr::null_mut()) }
        );
    }
}
