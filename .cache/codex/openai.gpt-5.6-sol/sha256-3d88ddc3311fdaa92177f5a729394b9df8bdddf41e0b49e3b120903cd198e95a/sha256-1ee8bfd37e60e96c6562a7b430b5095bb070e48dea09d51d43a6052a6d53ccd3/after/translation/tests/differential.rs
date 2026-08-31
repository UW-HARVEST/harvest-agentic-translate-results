#![allow(unsafe_op_in_unsafe_fn, unused_unsafe)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

const LZ4_MAX_INPUT_SIZE: c_int = 0x7e00_0000;
const LZ4F_VERSION: c_uint = 100;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct FrameInfo {
    block_size_id: c_int,
    block_mode: c_int,
    content_checksum_flag: c_int,
    frame_type: c_int,
    content_size: u64,
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
    skip_checksums: c_uint,
    reserved: [c_uint; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CustomMem {
    custom_alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    custom_calloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    custom_free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque_state: *mut c_void,
}

unsafe extern "C" fn failing_alloc(_: *mut c_void, _: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn no_op_free(_: *mut c_void, _: *mut c_void) {}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/liblz4.so");
        let rust_path = root.join("target/release/liblz4.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {} (run cargo build --release first)",
            rust_path.display()
        );
        Self {
            c: unsafe { Library::new(c_path).unwrap() },
            rust: unsafe { Library::new(rust_path).unwrap() },
        }
    }
}

unsafe fn symbol<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    unsafe { lib.get(name).unwrap() }
}

fn random_bytes(seed: &mut u64, len: usize) -> Vec<u8> {
    if len == 0 {
        return Vec::with_capacity(1);
    }
    let mut out = vec![0; len];
    for byte in &mut out {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        *byte = (*seed >> 24) as u8;
    }
    out
}

fn patterned(seed: &mut u64, len: usize, mode: usize) -> Vec<u8> {
    if len == 0 {
        return Vec::with_capacity(1);
    }
    match mode % 4 {
        0 => vec![0; len],
        1 => (0..len).map(|i| (i % 251) as u8).collect(),
        2 => {
            let pattern = random_bytes(seed, 31);
            (0..len).map(|i| pattern[i % pattern.len()]).collect()
        }
        _ => random_bytes(seed, len),
    }
}

unsafe fn compress_bound(lib: &Library, size: c_int) -> c_int {
    let f: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { symbol(lib, b"LZ4_compressBound") };
    unsafe { f(size) }
}

unsafe fn compress_call(
    lib: &Library,
    name: &[u8],
    input: &[u8],
    capacity: usize,
    extra: Option<c_int>,
) -> (c_int, Vec<u8>) {
    let mut out = vec![0xa5; capacity.max(1)];
    let result = if let Some(value) = extra {
        let f: Symbol<
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int,
        > = unsafe { symbol(lib, name) };
        unsafe {
            f(
                input.as_ptr().cast(),
                out.as_mut_ptr().cast(),
                input.len() as c_int,
                capacity as c_int,
                value,
            )
        }
    } else {
        let f: Symbol<unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int> =
            unsafe { symbol(lib, name) };
        unsafe {
            f(
                input.as_ptr().cast(),
                out.as_mut_ptr().cast(),
                input.len() as c_int,
                capacity as c_int,
            )
        }
    };
    if result > 0 {
        out.truncate(result as usize);
    }
    (result, out)
}

unsafe fn decompress_safe(lib: &Library, compressed: &[u8], capacity: usize) -> (c_int, Vec<u8>) {
    let f: Symbol<unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int> =
        unsafe { symbol(lib, b"LZ4_decompress_safe") };
    let mut out = vec![0xcc; capacity.max(1)];
    let result = unsafe {
        f(
            compressed.as_ptr().cast(),
            out.as_mut_ptr().cast(),
            compressed.len() as c_int,
            capacity as c_int,
        )
    };
    if result >= 0 {
        out.truncate(result as usize);
    }
    (result, out)
}

#[test]
fn all_dynamic_symbols_load_from_both_libraries() {
    unsafe {
        let libs = Libraries::load();
        let manifest =
            std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("SYMBOLS.md"))
                .unwrap();
        let names: Vec<_> = manifest
            .lines()
            .filter(|line| line.starts_with("LZ4"))
            .collect();
        assert_eq!(names.len(), 143);
        for name in names {
            let mut nul_name = name.as_bytes().to_vec();
            nul_name.push(0);
            let _: Symbol<*mut c_void> = unsafe { symbol(&libs.c, &nul_name) };
            let _: Symbol<*mut c_void> = unsafe { symbol(&libs.rust, &nul_name) };
        }
    }
}

#[test]
fn metadata_bounds_and_error_names_match() {
    unsafe {
        let libs = Libraries::load();
        for name in [
            &b"LZ4_versionNumber"[..],
            b"LZ4_sizeofState",
            b"LZ4_sizeofStateHC",
            b"LZ4_sizeofStreamState",
            b"LZ4_sizeofStreamStateHC",
            b"LZ4F_compressionLevel_max",
        ] {
            let c: Symbol<unsafe extern "C" fn() -> c_int> = unsafe { symbol(&libs.c, name) };
            let r: Symbol<unsafe extern "C" fn() -> c_int> = unsafe { symbol(&libs.rust, name) };
            assert_eq!(unsafe { c() }, unsafe { r() }, "{name:?}");
        }

        let c_version: Symbol<unsafe extern "C" fn() -> *const c_char> =
            unsafe { symbol(&libs.c, b"LZ4_versionString") };
        let r_version: Symbol<unsafe extern "C" fn() -> *const c_char> =
            unsafe { symbol(&libs.rust, b"LZ4_versionString") };
        assert_eq!(unsafe { CStr::from_ptr(c_version()) }, unsafe {
            CStr::from_ptr(r_version())
        });

        for value in [-1, 0, 1, 65_536, LZ4_MAX_INPUT_SIZE, LZ4_MAX_INPUT_SIZE + 1] {
            assert_eq!(unsafe { compress_bound(&libs.c, value) }, unsafe {
                compress_bound(&libs.rust, value)
            });
        }

        let c_ring: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            unsafe { symbol(&libs.c, b"LZ4_decoderRingBufferSize") };
        let r_ring: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            unsafe { symbol(&libs.rust, b"LZ4_decoderRingBufferSize") };
        for value in [-1, 0, 1, 15, 16, 65_536, LZ4_MAX_INPUT_SIZE] {
            assert_eq!(unsafe { c_ring(value) }, unsafe { r_ring(value) });
        }

        let c_block: Symbol<unsafe extern "C" fn(c_uint) -> usize> =
            unsafe { symbol(&libs.c, b"LZ4F_getBlockSize") };
        let r_block: Symbol<unsafe extern "C" fn(c_uint) -> usize> =
            unsafe { symbol(&libs.rust, b"LZ4F_getBlockSize") };
        let c_is_error: Symbol<unsafe extern "C" fn(usize) -> c_uint> =
            unsafe { symbol(&libs.c, b"LZ4F_isError") };
        let r_is_error: Symbol<unsafe extern "C" fn(usize) -> c_uint> =
            unsafe { symbol(&libs.rust, b"LZ4F_isError") };
        let c_error_name: Symbol<unsafe extern "C" fn(usize) -> *const c_char> =
            unsafe { symbol(&libs.c, b"LZ4F_getErrorName") };
        let r_error_name: Symbol<unsafe extern "C" fn(usize) -> *const c_char> =
            unsafe { symbol(&libs.rust, b"LZ4F_getErrorName") };
        for value in [0, 1, 3, 4, 5, 6, 7, 8, c_uint::MAX] {
            let c_result = unsafe { c_block(value) };
            let r_result = unsafe { r_block(value) };
            assert_eq!(c_result, r_result);
            assert_eq!(unsafe { c_is_error(c_result) }, unsafe {
                r_is_error(r_result)
            });
            assert_eq!(unsafe { CStr::from_ptr(c_error_name(c_result)) }, unsafe {
                CStr::from_ptr(r_error_name(r_result))
            });
        }
    }
}

#[test]
fn xxhash_one_shot_streaming_and_canonical_match() {
    unsafe {
        let libs = Libraries::load();
        type H32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
        type H64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
        let c32: Symbol<H32> = unsafe { symbol(&libs.c, b"LZ4_XXH32") };
        let r32: Symbol<H32> = unsafe { symbol(&libs.rust, b"LZ4_XXH32") };
        let c64: Symbol<H64> = unsafe { symbol(&libs.c, b"LZ4_XXH64") };
        let r64: Symbol<H64> = unsafe { symbol(&libs.rust, b"LZ4_XXH64") };
        let mut seed = 0x4c5a_3456_7812_90abu64;
        for len in [0, 1, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 255, 4097] {
            for _ in 0..12 {
                let input = random_bytes(&mut seed, len);
                let hash_seed = seed as u32;
                assert_eq!(
                    unsafe { c32(input.as_ptr().cast(), len, hash_seed) },
                    unsafe { r32(input.as_ptr().cast(), len, hash_seed) }
                );
                assert_eq!(unsafe { c64(input.as_ptr().cast(), len, seed) }, unsafe {
                    r64(input.as_ptr().cast(), len, seed)
                });
            }
        }

        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Reset32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type Digest32 = unsafe extern "C" fn(*const c_void) -> u32;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        let input = random_bytes(&mut seed, 10_003);
        let mut digests = Vec::new();
        for lib in [&libs.c, &libs.rust] {
            let create: Symbol<Create> = unsafe { symbol(lib, b"LZ4_XXH32_createState") };
            let reset: Symbol<Reset32> = unsafe { symbol(lib, b"LZ4_XXH32_reset") };
            let update: Symbol<Update> = unsafe { symbol(lib, b"LZ4_XXH32_update") };
            let digest: Symbol<Digest32> = unsafe { symbol(lib, b"LZ4_XXH32_digest") };
            let free: Symbol<Free> = unsafe { symbol(lib, b"LZ4_XXH32_freeState") };
            assert_eq!(unsafe { update(ptr::null_mut(), ptr::null(), 0) }, 1);
            let state = unsafe { create() };
            assert!(!state.is_null());
            assert_eq!(unsafe { reset(state, 0x1234_5678) }, 0);
            let mut offset = 0;
            for chunk in [1, 15, 16, 31, 1024, 4096, usize::MAX] {
                let take = chunk.min(input.len() - offset);
                assert_eq!(
                    unsafe { update(state, input[offset..].as_ptr().cast(), take) },
                    0
                );
                offset += take;
                if offset == input.len() {
                    break;
                }
            }
            digests.push(unsafe { digest(state) });
            assert_eq!(unsafe { free(state) }, 0);
        }
        assert_eq!(digests[0], digests[1]);

        let hash = digests[0];
        let mut canonical_c = [0u8; 4];
        let mut canonical_r = [0u8; 4];
        type ToCanonical = unsafe extern "C" fn(*mut c_void, u32);
        type FromCanonical = unsafe extern "C" fn(*const c_void) -> u32;
        let c_to: Symbol<ToCanonical> = unsafe { symbol(&libs.c, b"LZ4_XXH32_canonicalFromHash") };
        let r_to: Symbol<ToCanonical> =
            unsafe { symbol(&libs.rust, b"LZ4_XXH32_canonicalFromHash") };
        let c_from: Symbol<FromCanonical> =
            unsafe { symbol(&libs.c, b"LZ4_XXH32_hashFromCanonical") };
        let r_from: Symbol<FromCanonical> =
            unsafe { symbol(&libs.rust, b"LZ4_XXH32_hashFromCanonical") };
        unsafe { c_to(canonical_c.as_mut_ptr().cast(), hash) };
        unsafe { r_to(canonical_r.as_mut_ptr().cast(), hash) };
        assert_eq!(canonical_c, canonical_r);
        assert_eq!(unsafe { c_from(canonical_c.as_ptr().cast()) }, unsafe {
            r_from(canonical_r.as_ptr().cast())
        });
    }
}

#[test]
fn xxhash64_stream_copy_and_canonical_match() {
    unsafe {
        let libs = Libraries::load();
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Reset = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type CopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
        type Digest = unsafe extern "C" fn(*const c_void) -> u64;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Canonical = unsafe extern "C" fn(*mut c_void, u64);
        type FromCanonical = unsafe extern "C" fn(*const c_void) -> u64;
        let mut seed = 0x9182_7364_5546_3728;
        let input = random_bytes(&mut seed, 20_003);
        let mut results = Vec::new();
        for lib in [&libs.c, &libs.rust] {
            let create: Symbol<Create> = symbol(lib, b"LZ4_XXH64_createState");
            let reset: Symbol<Reset> = symbol(lib, b"LZ4_XXH64_reset");
            let update: Symbol<Update> = symbol(lib, b"LZ4_XXH64_update");
            let copy: Symbol<CopyState> = symbol(lib, b"LZ4_XXH64_copyState");
            let digest: Symbol<Digest> = symbol(lib, b"LZ4_XXH64_digest");
            let free: Symbol<Free> = symbol(lib, b"LZ4_XXH64_freeState");
            let to_canonical: Symbol<Canonical> = symbol(lib, b"LZ4_XXH64_canonicalFromHash");
            let from_canonical: Symbol<FromCanonical> = symbol(lib, b"LZ4_XXH64_hashFromCanonical");
            assert_eq!(update(ptr::null_mut(), ptr::null(), 0), 1);
            let state = create();
            let copied = create();
            assert_eq!(reset(state, seed), 0);
            assert_eq!(update(state, input.as_ptr().cast(), 31), 0);
            copy(copied, state);
            assert_eq!(digest(state), digest(copied));
            assert_eq!(
                update(state, input[31..].as_ptr().cast(), input.len() - 31),
                0
            );
            let hash = digest(state);
            let mut canonical = [0u8; 8];
            to_canonical(canonical.as_mut_ptr().cast(), hash);
            assert_eq!(from_canonical(canonical.as_ptr().cast()), hash);
            results.push((hash, canonical));
            assert_eq!(free(state), 0);
            assert_eq!(free(copied), 0);
        }
        assert_eq!(results[0], results[1]);
    }
}

#[test]
fn block_and_hc_compression_are_byte_identical() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0x1234_9876_fedc_ba09;
        let sizes = [0, 1, 3, 4, 12, 13, 31, 255, 4096, 65_535, 70_000];
        for (case, size) in sizes.into_iter().enumerate() {
            for mode in 0..4 {
                for _ in 0..8 {
                    let input = patterned(&mut seed, size, mode);
                    let bound = unsafe { compress_bound(&libs.c, size as c_int) } as usize;
                    for acceleration in [-3, 0, 1, 2, 65_537, 65_538] {
                        let c = unsafe {
                            compress_call(
                                &libs.c,
                                b"LZ4_compress_fast",
                                &input,
                                bound,
                                Some(acceleration),
                            )
                        };
                        let r = unsafe {
                            compress_call(
                                &libs.rust,
                                b"LZ4_compress_fast",
                                &input,
                                bound,
                                Some(acceleration),
                            )
                        };
                        assert_eq!(c, r, "fast case={case} mode={mode} acc={acceleration}");
                        let decoded_c = unsafe { decompress_safe(&libs.c, &c.1, size) };
                        let decoded_r = unsafe { decompress_safe(&libs.rust, &r.1, size) };
                        assert_eq!(decoded_c, decoded_r);
                        assert_eq!(decoded_c.1, input);
                    }
                    for level in [-1, 1, 2, 9, 10, 11, 12, 13] {
                        let c = unsafe {
                            compress_call(&libs.c, b"LZ4_compress_HC", &input, bound, Some(level))
                        };
                        let r = unsafe {
                            compress_call(
                                &libs.rust,
                                b"LZ4_compress_HC",
                                &input,
                                bound,
                                Some(level),
                            )
                        };
                        assert_eq!(c, r, "HC case={case} mode={mode} level={level}");
                    }
                }
            }
        }
    }
}

#[test]
fn external_state_dest_size_partial_and_malformed_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0xdec0_de01_5678_9876;
        type Size = unsafe extern "C" fn() -> c_int;
        type Ext = unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int;
        for size in [1, 7, 64, 1024, 65_536] {
            let input = patterned(&mut seed, size, size);
            let capacity = unsafe { compress_bound(&libs.c, size as c_int) } as usize;
            for (size_name, function_name) in [
                (&b"LZ4_sizeofState"[..], &b"LZ4_compress_fast_extState"[..]),
                (
                    &b"LZ4_sizeofStateHC"[..],
                    &b"LZ4_compress_HC_extStateHC"[..],
                ),
            ] {
                let c_size: Symbol<Size> = unsafe { symbol(&libs.c, size_name) };
                let r_size: Symbol<Size> = unsafe { symbol(&libs.rust, size_name) };
                assert_eq!(unsafe { c_size() }, unsafe { r_size() });
                let words = (unsafe { c_size() } as usize).div_ceil(8);
                let mut c_state = vec![0u64; words];
                let mut r_state = vec![0u64; words];
                let mut c_out = vec![0u8; capacity];
                let mut r_out = vec![0u8; capacity];
                let c_fn: Symbol<Ext> = unsafe { symbol(&libs.c, function_name) };
                let r_fn: Symbol<Ext> = unsafe { symbol(&libs.rust, function_name) };
                let c_result = unsafe {
                    c_fn(
                        c_state.as_mut_ptr().cast(),
                        input.as_ptr().cast(),
                        c_out.as_mut_ptr().cast(),
                        size as c_int,
                        capacity as c_int,
                        9,
                    )
                };
                let r_result = unsafe {
                    r_fn(
                        r_state.as_mut_ptr().cast(),
                        input.as_ptr().cast(),
                        r_out.as_mut_ptr().cast(),
                        size as c_int,
                        capacity as c_int,
                        9,
                    )
                };
                assert_eq!(c_result, r_result);
                assert_eq!(&c_out[..c_result as usize], &r_out[..r_result as usize]);
            }

            type DestSize =
                unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
            let c_dest: Symbol<DestSize> = unsafe { symbol(&libs.c, b"LZ4_compress_destSize") };
            let r_dest: Symbol<DestSize> = unsafe { symbol(&libs.rust, b"LZ4_compress_destSize") };
            for target in [1usize, 8, capacity / 2, capacity] {
                let mut c_consumed = size as c_int;
                let mut r_consumed = size as c_int;
                let mut c_out = vec![0u8; target.max(1)];
                let mut r_out = vec![0u8; target.max(1)];
                let c_result = unsafe {
                    c_dest(
                        input.as_ptr().cast(),
                        c_out.as_mut_ptr().cast(),
                        &mut c_consumed,
                        target as c_int,
                    )
                };
                let r_result = unsafe {
                    r_dest(
                        input.as_ptr().cast(),
                        r_out.as_mut_ptr().cast(),
                        &mut r_consumed,
                        target as c_int,
                    )
                };
                assert_eq!((c_result, c_consumed), (r_result, r_consumed));
                assert_eq!(&c_out[..c_result as usize], &r_out[..r_result as usize]);
            }
        }

        type Partial =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let valid = patterned(&mut seed, 10_000, 2);
        let bound = unsafe { compress_bound(&libs.c, valid.len() as c_int) } as usize;
        let compressed =
            unsafe { compress_call(&libs.c, b"LZ4_compress_default", &valid, bound, None) }.1;
        let c_partial: Symbol<Partial> = unsafe { symbol(&libs.c, b"LZ4_decompress_safe_partial") };
        let r_partial: Symbol<Partial> =
            unsafe { symbol(&libs.rust, b"LZ4_decompress_safe_partial") };
        for target in [0, 1, 17, 9999, 10_000, 10_001] {
            let mut c_out = vec![0u8; 10_001];
            let mut r_out = vec![0u8; 10_001];
            let c_result = unsafe {
                c_partial(
                    compressed.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    compressed.len() as c_int,
                    target,
                    10_001,
                )
            };
            let r_result = unsafe {
                r_partial(
                    compressed.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    compressed.len() as c_int,
                    target,
                    10_001,
                )
            };
            assert_eq!(c_result, r_result);
            assert_eq!(
                &c_out[..c_result.max(0) as usize],
                &r_out[..r_result.max(0) as usize]
            );
        }

        for malformed in [&[][..], &[1][..], &[0x10][..], &[0xf0, 0xff, 0xff][..]] {
            let c = unsafe { decompress_safe(&libs.c, malformed, 32) };
            let r = unsafe { decompress_safe(&libs.rust, malformed, 32) };
            assert_eq!(c.0, r.0);
        }
        let c_null = unsafe { decompress_safe_raw_null(&libs.c) };
        let r_null = unsafe { decompress_safe_raw_null(&libs.rust) };
        assert_eq!(c_null, r_null);
    }
}

unsafe fn decompress_safe_raw_null(lib: &Library) -> c_int {
    let f: Symbol<unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int> =
        unsafe { symbol(lib, b"LZ4_decompress_safe") };
    let mut out = [0u8; 1];
    unsafe { f(ptr::null(), out.as_mut_ptr().cast(), 1, 1) }
}

unsafe fn stream_compress(lib: &Library, hc: bool, input: &[u8]) -> Vec<u8> {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Continue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
    let (create_name, free_name, load_name, continue_name) = if hc {
        (
            &b"LZ4_createStreamHC"[..],
            &b"LZ4_freeStreamHC"[..],
            &b"LZ4_loadDictHC"[..],
            &b"LZ4_compress_HC_continue"[..],
        )
    } else {
        (
            &b"LZ4_createStream"[..],
            &b"LZ4_freeStream"[..],
            &b"LZ4_loadDict"[..],
            &b"LZ4_compress_fast_continue"[..],
        )
    };
    let create: Symbol<Create> = unsafe { symbol(lib, create_name) };
    let free: Symbol<Free> = unsafe { symbol(lib, free_name) };
    let load: Symbol<Load> = unsafe { symbol(lib, load_name) };
    let next: Symbol<Continue> = unsafe { symbol(lib, continue_name) };
    let stream = unsafe { create() };
    assert!(!stream.is_null());
    let dict_len = input.len().min(70_000);
    let expected_loaded = if !hc && dict_len < std::mem::size_of::<usize>() {
        0
    } else {
        dict_len.min(65_536) as c_int
    };
    assert_eq!(
        unsafe { load(stream, input.as_ptr().cast(), dict_len as c_int) },
        expected_loaded
    );
    let mut result = Vec::new();
    for chunk in input.chunks(997) {
        let capacity = unsafe { compress_bound(lib, chunk.len() as c_int) } as usize;
        let mut out = vec![0u8; capacity];
        let written = unsafe {
            next(
                stream,
                chunk.as_ptr().cast(),
                out.as_mut_ptr().cast(),
                chunk.len() as c_int,
                capacity as c_int,
            )
        };
        assert!(written > 0);
        result.extend_from_slice(&out[..written as usize]);
    }
    assert_eq!(unsafe { free(stream) }, 0);
    result
}

#[test]
fn low_level_stream_and_dictionary_state_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0xa501_91cc_7744_0021;
        for len in [1, 31, 4096, 70_000] {
            for mode in 0..4 {
                let input = patterned(&mut seed, len, mode);
                assert_eq!(unsafe { stream_compress(&libs.c, false, &input) }, unsafe {
                    stream_compress(&libs.rust, false, &input)
                });
                assert_eq!(unsafe { stream_compress(&libs.c, true, &input) }, unsafe {
                    stream_compress(&libs.rust, true, &input)
                });
            }
        }

        type Init = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        for (name, size_name) in [
            (&b"LZ4_initStream"[..], &b"LZ4_sizeofState"[..]),
            (&b"LZ4_initStreamHC"[..], &b"LZ4_sizeofStateHC"[..]),
        ] {
            let c_init: Symbol<Init> = unsafe { symbol(&libs.c, name) };
            let r_init: Symbol<Init> = unsafe { symbol(&libs.rust, name) };
            let c_size: Symbol<unsafe extern "C" fn() -> c_int> =
                unsafe { symbol(&libs.c, size_name) };
            let size = unsafe { c_size() } as usize;
            assert!(unsafe { c_init(ptr::null_mut(), size) }.is_null());
            assert!(unsafe { r_init(ptr::null_mut(), size) }.is_null());
            let mut c_words = vec![0u64; size.div_ceil(8)];
            let mut r_words = vec![0u64; size.div_ceil(8)];
            assert!(unsafe { c_init(c_words.as_mut_ptr().cast(), size - 1) }.is_null());
            assert!(unsafe { r_init(r_words.as_mut_ptr().cast(), size - 1) }.is_null());
            assert!(!unsafe { c_init(c_words.as_mut_ptr().cast(), size) }.is_null());
            assert!(!unsafe { r_init(r_words.as_mut_ptr().cast(), size) }.is_null());
        }
    }
}

#[test]
fn legacy_aliases_and_decode_streams_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0x6633_aa55_1234_9876;
        for size in [0, 1, 31, 4096, 65_536] {
            let input = patterned(&mut seed, size, size);
            let bound = compress_bound(&libs.c, size as c_int) as usize;
            type Compress3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
            for name in [&b"LZ4_compress"[..], b"LZ4_compressHC"] {
                let c_fn: Symbol<Compress3> = symbol(&libs.c, name);
                let r_fn: Symbol<Compress3> = symbol(&libs.rust, name);
                let mut c_out = vec![0u8; bound.max(1)];
                let mut r_out = vec![0u8; bound.max(1)];
                let c_result = c_fn(
                    input.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    size as c_int,
                );
                let r_result = r_fn(
                    input.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    size as c_int,
                );
                assert_eq!(c_result, r_result);
                assert_eq!(&c_out[..c_result as usize], &r_out[..r_result as usize]);
            }

            let compressed = compress_call(&libs.c, b"LZ4_compress_default", &input, bound, None).1;
            type Create = unsafe extern "C" fn() -> *mut c_void;
            type Set = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
            type Decode = unsafe extern "C" fn(
                *mut c_void,
                *const c_char,
                *mut c_char,
                c_int,
                c_int,
            ) -> c_int;
            type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
            let mut decoded = Vec::new();
            for lib in [&libs.c, &libs.rust] {
                let create: Symbol<Create> = symbol(lib, b"LZ4_createStreamDecode");
                let set: Symbol<Set> = symbol(lib, b"LZ4_setStreamDecode");
                let decode: Symbol<Decode> = symbol(lib, b"LZ4_decompress_safe_continue");
                let free: Symbol<Free> = symbol(lib, b"LZ4_freeStreamDecode");
                let stream = create();
                assert_eq!(set(stream, ptr::null(), 0), 1);
                let mut out = vec![0u8; size.max(1)];
                let result = decode(
                    stream,
                    compressed.as_ptr().cast(),
                    out.as_mut_ptr().cast(),
                    compressed.len() as c_int,
                    size as c_int,
                );
                out.truncate(result.max(0) as usize);
                decoded.push((result, out));
                assert_eq!(free(stream), 0);
            }
            assert_eq!(decoded[0], decoded[1]);
            assert_eq!(decoded[0].1, input);
        }
    }
}

unsafe fn frame_compress(
    lib: &Library,
    input: &[u8],
    prefs: Option<&Preferences>,
) -> (usize, Vec<u8>) {
    type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
    type Compress =
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
    let bound: Symbol<Bound> = unsafe { symbol(lib, b"LZ4F_compressFrameBound") };
    let compress: Symbol<Compress> = unsafe { symbol(lib, b"LZ4F_compressFrame") };
    let prefs_ptr = prefs.map_or(ptr::null(), |p| p);
    let capacity = unsafe { bound(input.len(), prefs_ptr) };
    let mut output = vec![0u8; capacity.max(1)];
    let result = unsafe {
        compress(
            output.as_mut_ptr().cast(),
            capacity,
            input.as_ptr().cast(),
            input.len(),
            prefs_ptr,
        )
    };
    if result <= capacity {
        output.truncate(result);
    }
    (result, output)
}

unsafe fn frame_decompress(lib: &Library, frame: &[u8], fragmented: bool) -> (usize, Vec<u8>) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Decompress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
        *const DecompressOptions,
    ) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    let create: Symbol<Create> = unsafe { symbol(lib, b"LZ4F_createDecompressionContext") };
    let decompress: Symbol<Decompress> = unsafe { symbol(lib, b"LZ4F_decompress") };
    let free: Symbol<Free> = unsafe { symbol(lib, b"LZ4F_freeDecompressionContext") };
    let mut context = ptr::null_mut();
    assert_eq!(unsafe { create(&mut context, LZ4F_VERSION) }, 0);
    let mut source_offset = 0;
    let mut output = Vec::new();
    let mut hint = 1;
    while source_offset < frame.len() || hint != 0 {
        let available = if fragmented {
            (frame.len() - source_offset).min(1 + source_offset % 97)
        } else {
            frame.len() - source_offset
        };
        let mut src_size = available;
        let mut chunk = vec![0u8; if fragmented { 257 } else { 5_000_000 }];
        let mut dst_size = chunk.len();
        hint = unsafe {
            decompress(
                context,
                chunk.as_mut_ptr().cast(),
                &mut dst_size,
                frame[source_offset..].as_ptr().cast(),
                &mut src_size,
                ptr::null(),
            )
        };
        assert_eq!(
            hint >> (usize::BITS - 8),
            0,
            "frame decompression error {hint}"
        );
        source_offset += src_size;
        output.extend_from_slice(&chunk[..dst_size]);
        if hint == 0 {
            break;
        }
        assert!(src_size > 0 || dst_size > 0);
    }
    let free_result = unsafe { free(context) };
    (free_result, output)
}

#[test]
fn frame_preference_cross_product_and_fragmentation_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0xf00d_cafe_9988_1234;
        for block_size_id in [0, 4, 5, 6, 7] {
            for block_mode in [0, 1] {
                for content_checksum in [0, 1] {
                    for block_checksum in [0, 1] {
                        for level in [-3, 0, 2, 9, 10, 12, 13] {
                            let len = [0, 1, 31, 4096, 70_003][(seed as usize) % 5];
                            let mode = seed as usize;
                            let input = patterned(&mut seed, len, mode);
                            let prefs = Preferences {
                                frame_info: FrameInfo {
                                    block_size_id,
                                    block_mode,
                                    content_checksum_flag: content_checksum,
                                    frame_type: 0,
                                    content_size: if seed & 1 == 0 { len as u64 } else { 0 },
                                    dict_id: if seed & 2 == 0 { 0x1234_5678 } else { 0 },
                                    block_checksum_flag: block_checksum,
                                },
                                compression_level: level,
                                auto_flush: (seed & 4 != 0) as u32,
                                favor_dec_speed: (seed & 8 != 0) as u32,
                                reserved: [0; 3],
                            };
                            let c = unsafe { frame_compress(&libs.c, &input, Some(&prefs)) };
                            let r = unsafe { frame_compress(&libs.rust, &input, Some(&prefs)) };
                            assert_eq!(
                                c, r,
                                "bs={block_size_id} mode={block_mode} cc={content_checksum} bc={block_checksum} level={level}"
                            );
                            let c_decoded = unsafe { frame_decompress(&libs.c, &c.1, true) };
                            let r_decoded = unsafe { frame_decompress(&libs.rust, &r.1, true) };
                            assert_eq!(c_decoded, r_decoded);
                            assert_eq!(c_decoded.1, input);
                        }
                    }
                }
            }
        }
    }
}

unsafe fn streaming_frame(lib: &Library, input: &[u8], uncompressed: bool) -> Vec<u8> {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Begin = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const Preferences) -> usize;
    type Update = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const CompressOptions,
    ) -> usize;
    type End =
        unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const CompressOptions) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    let create: Symbol<Create> = unsafe { symbol(lib, b"LZ4F_createCompressionContext") };
    let begin: Symbol<Begin> = unsafe { symbol(lib, b"LZ4F_compressBegin") };
    let update: Symbol<Update> = unsafe {
        symbol(
            lib,
            if uncompressed {
                b"LZ4F_uncompressedUpdate"
            } else {
                b"LZ4F_compressUpdate"
            },
        )
    };
    let end: Symbol<End> = unsafe { symbol(lib, b"LZ4F_compressEnd") };
    let free: Symbol<Free> = unsafe { symbol(lib, b"LZ4F_freeCompressionContext") };
    let prefs = Preferences {
        frame_info: FrameInfo {
            block_size_id: 4,
            block_mode: if uncompressed { 1 } else { 0 },
            content_checksum_flag: 1,
            frame_type: 0,
            content_size: input.len() as u64,
            dict_id: 7,
            block_checksum_flag: 1,
        },
        compression_level: 10,
        auto_flush: 0,
        favor_dec_speed: 1,
        reserved: [0; 3],
    };
    let options = CompressOptions {
        stable_src: 0,
        reserved: [0; 3],
    };
    let mut context = ptr::null_mut();
    assert_eq!(unsafe { create(&mut context, LZ4F_VERSION) }, 0);
    let mut output = Vec::new();
    let mut buffer = vec![0u8; 200_000];
    let header = unsafe { begin(context, buffer.as_mut_ptr().cast(), buffer.len(), &prefs) };
    output.extend_from_slice(&buffer[..header]);
    for chunk in input.chunks(3333) {
        let written = unsafe {
            update(
                context,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                chunk.as_ptr().cast(),
                chunk.len(),
                &options,
            )
        };
        output.extend_from_slice(&buffer[..written]);
    }
    let tail = unsafe { end(context, buffer.as_mut_ptr().cast(), buffer.len(), &options) };
    output.extend_from_slice(&buffer[..tail]);
    assert_eq!(unsafe { free(context) }, 0);
    output
}

#[test]
fn streaming_frame_compressed_and_uncompressed_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0x7788_1234_abcd_9876;
        for len in [0, 1, 4096, 70_000, 140_003] {
            let input = patterned(&mut seed, len, len);
            for uncompressed in [false, true] {
                let c = unsafe { streaming_frame(&libs.c, &input, uncompressed) };
                let r = unsafe { streaming_frame(&libs.rust, &input, uncompressed) };
                assert_eq!(c, r);
                assert_eq!(unsafe { frame_decompress(&libs.c, &c, true) }.1, input);
            }
        }
    }
}

unsafe fn dictionary_frame(lib: &Library, input: &[u8], dict: &[u8]) -> Vec<u8> {
    type CreateDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
    type FreeDict = unsafe extern "C" fn(*mut c_void);
    type CreateContext = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type FreeContext = unsafe extern "C" fn(*mut c_void) -> usize;
    type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
    type Compress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        *const Preferences,
    ) -> usize;
    let create_dict: Symbol<CreateDict> = symbol(lib, b"LZ4F_createCDict");
    let free_dict: Symbol<FreeDict> = symbol(lib, b"LZ4F_freeCDict");
    let create_context: Symbol<CreateContext> = symbol(lib, b"LZ4F_createCompressionContext");
    let free_context: Symbol<FreeContext> = symbol(lib, b"LZ4F_freeCompressionContext");
    let bound: Symbol<Bound> = symbol(lib, b"LZ4F_compressFrameBound");
    let compress: Symbol<Compress> = symbol(lib, b"LZ4F_compressFrame_usingCDict");
    let cdict = create_dict(dict.as_ptr().cast(), dict.len());
    assert!(!cdict.is_null());
    let mut context = ptr::null_mut();
    assert_eq!(create_context(&mut context, LZ4F_VERSION), 0);
    let prefs = Preferences {
        frame_info: FrameInfo {
            block_size_id: 4,
            block_mode: 0,
            content_checksum_flag: 1,
            frame_type: 0,
            content_size: input.len() as u64,
            dict_id: 0x4455,
            block_checksum_flag: 1,
        },
        compression_level: 9,
        auto_flush: 1,
        favor_dec_speed: 0,
        reserved: [0; 3],
    };
    let capacity = bound(input.len(), &prefs);
    let mut output = vec![0u8; capacity];
    let written = compress(
        context,
        output.as_mut_ptr().cast(),
        capacity,
        input.as_ptr().cast(),
        input.len(),
        cdict,
        &prefs,
    );
    output.truncate(written);
    assert_eq!(free_context(context), 0);
    free_dict(cdict);
    output
}

#[test]
fn frame_dictionary_info_and_custom_memory_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0x88aa_44cc_22ee_1199;
        for dict_len in [0, 31, 65_536, 70_000] {
            let dict = patterned(&mut seed, dict_len, dict_len);
            let mut input = dict
                .iter()
                .copied()
                .cycle()
                .take(20_003)
                .collect::<Vec<_>>();
            input.extend_from_slice(&random_bytes(&mut seed, 97));
            let c_frame = dictionary_frame(&libs.c, &input, &dict);
            let r_frame = dictionary_frame(&libs.rust, &input, &dict);
            assert_eq!(c_frame, r_frame);

            type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
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
            type Info = unsafe extern "C" fn(
                *mut c_void,
                *mut FrameInfo,
                *const c_void,
                *mut usize,
            ) -> usize;
            type Free = unsafe extern "C" fn(*mut c_void) -> usize;
            let mut infos = Vec::new();
            for lib in [&libs.c, &libs.rust] {
                let create: Symbol<Create> = symbol(lib, b"LZ4F_createDecompressionContext");
                let info_fn: Symbol<Info> = symbol(lib, b"LZ4F_getFrameInfo");
                let free: Symbol<Free> = symbol(lib, b"LZ4F_freeDecompressionContext");
                let mut context = ptr::null_mut();
                assert_eq!(create(&mut context, LZ4F_VERSION), 0);
                let mut info = FrameInfo::default();
                let mut source_size = c_frame.len();
                let hint = info_fn(
                    context,
                    &mut info,
                    c_frame.as_ptr().cast(),
                    &mut source_size,
                );
                infos.push((hint, source_size, format!("{info:?}")));
                free(context);
            }
            assert_eq!(infos[0], infos[1]);

            let mut decoded = Vec::new();
            for lib in [&libs.c, &libs.rust] {
                let create: Symbol<Create> = symbol(lib, b"LZ4F_createDecompressionContext");
                let decompress: Symbol<DecompressDict> = symbol(lib, b"LZ4F_decompress_usingDict");
                let free: Symbol<Free> = symbol(lib, b"LZ4F_freeDecompressionContext");
                let mut context = ptr::null_mut();
                assert_eq!(create(&mut context, LZ4F_VERSION), 0);
                let mut source_size = c_frame.len();
                let mut output = vec![0u8; input.len().max(1)];
                let mut output_size = input.len();
                let hint = decompress(
                    context,
                    output.as_mut_ptr().cast(),
                    &mut output_size,
                    c_frame.as_ptr().cast(),
                    &mut source_size,
                    dict.as_ptr().cast(),
                    dict.len(),
                    ptr::null(),
                );
                output.truncate(output_size);
                decoded.push((hint, source_size, output));
                free(context);
            }
            assert_eq!(decoded[0], decoded[1]);
            assert_eq!(decoded[0].2, input);
        }

        let failing_memory = CustomMem {
            custom_alloc: Some(failing_alloc),
            custom_calloc: Some(failing_alloc),
            custom_free: Some(no_op_free),
            opaque_state: ptr::null_mut(),
        };
        type AdvancedContext = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
        type AdvancedDict = unsafe extern "C" fn(CustomMem, *const c_void, usize) -> *mut c_void;
        let dict = [1u8; 16];
        for name in [
            &b"LZ4F_createCompressionContext_advanced"[..],
            b"LZ4F_createDecompressionContext_advanced",
        ] {
            let c_fn: Symbol<AdvancedContext> = symbol(&libs.c, name);
            let r_fn: Symbol<AdvancedContext> = symbol(&libs.rust, name);
            assert!(c_fn(failing_memory, LZ4F_VERSION).is_null());
            assert!(r_fn(failing_memory, LZ4F_VERSION).is_null());
        }
        let c_dict: Symbol<AdvancedDict> = symbol(&libs.c, b"LZ4F_createCDict_advanced");
        let r_dict: Symbol<AdvancedDict> = symbol(&libs.rust, b"LZ4F_createCDict_advanced");
        assert!(c_dict(failing_memory, dict.as_ptr().cast(), dict.len()).is_null());
        assert!(r_dict(failing_memory, dict.as_ptr().cast(), dict.len()).is_null());
    }
}

#[test]
fn frame_error_surface_matches_exact_codes() {
    unsafe {
        let libs = Libraries::load();
        type Header = unsafe extern "C" fn(*const c_void, usize) -> usize;
        let c_header: Symbol<Header> = unsafe { symbol(&libs.c, b"LZ4F_headerSize") };
        let r_header: Symbol<Header> = unsafe { symbol(&libs.rust, b"LZ4F_headerSize") };
        let malformed = [
            vec![],
            vec![0; 4],
            vec![0; 5],
            vec![4, 0x22, 0x4d, 0x18, 0],
            vec![4, 0x22, 0x4d, 0x18, 0xff],
        ];
        for bytes in malformed {
            let c = unsafe { c_header(bytes.as_ptr().cast(), bytes.len()) };
            let r = unsafe { r_header(bytes.as_ptr().cast(), bytes.len()) };
            assert_eq!(c, r);
        }
        assert_eq!(unsafe { c_header(ptr::null(), 5) }, unsafe {
            r_header(ptr::null(), 5)
        });

        type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        let c_create: Symbol<Create> = unsafe { symbol(&libs.c, b"LZ4F_createCompressionContext") };
        let r_create: Symbol<Create> =
            unsafe { symbol(&libs.rust, b"LZ4F_createCompressionContext") };
        assert_eq!(unsafe { c_create(ptr::null_mut(), LZ4F_VERSION) }, unsafe {
            r_create(ptr::null_mut(), LZ4F_VERSION)
        });

        type Begin =
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const Preferences) -> usize;
        type Free = unsafe extern "C" fn(*mut c_void) -> usize;
        for lib in [&libs.c, &libs.rust] {
            let create: Symbol<Create> = unsafe { symbol(lib, b"LZ4F_createCompressionContext") };
            let begin: Symbol<Begin> = unsafe { symbol(lib, b"LZ4F_compressBegin") };
            let free: Symbol<Free> = unsafe { symbol(lib, b"LZ4F_freeCompressionContext") };
            let mut context = ptr::null_mut();
            assert_eq!(unsafe { create(&mut context, LZ4F_VERSION) }, 0);
            let mut output = [0u8; 19];
            let error = unsafe { begin(context, output.as_mut_ptr().cast(), 18, ptr::null()) };
            assert_ne!(error, 0);
            assert_eq!(unsafe { free(context) }, 0);
        }

        type Update = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize;
        let mut errors = Vec::new();
        for lib in [&libs.c, &libs.rust] {
            let create: Symbol<Create> = unsafe { symbol(lib, b"LZ4F_createCompressionContext") };
            let update: Symbol<Update> = unsafe { symbol(lib, b"LZ4F_compressUpdate") };
            let free: Symbol<Free> = unsafe { symbol(lib, b"LZ4F_freeCompressionContext") };
            let mut context = ptr::null_mut();
            assert_eq!(unsafe { create(&mut context, LZ4F_VERSION) }, 0);
            let mut output = [0u8; 64];
            errors.push(unsafe {
                update(
                    context,
                    output.as_mut_ptr().cast(),
                    output.len(),
                    ptr::null(),
                    0,
                    ptr::null(),
                )
            });
            unsafe { free(context) };
        }
        assert_eq!(errors[0], errors[1]);
    }
}

#[test]
fn explicit_rejection_matrix_matches() {
    unsafe {
        let libs = Libraries::load();
        type FileOpen = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
        type FileRead = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
        type FileClose = unsafe extern "C" fn(*mut c_void) -> usize;
        type FileWriteOpen =
            unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
        type FileWrite = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
        let mut byte = 0u8;
        for (name, kind) in [(&b"LZ4F_readClose"[..], 0), (&b"LZ4F_writeClose"[..], 0)] {
            let c_fn: Symbol<FileClose> = symbol(&libs.c, name);
            let r_fn: Symbol<FileClose> = symbol(&libs.rust, name);
            assert_eq!(c_fn(ptr::null_mut()), r_fn(ptr::null_mut()), "{kind}");
        }
        let c_read_open: Symbol<FileOpen> = symbol(&libs.c, b"LZ4F_readOpen");
        let r_read_open: Symbol<FileOpen> = symbol(&libs.rust, b"LZ4F_readOpen");
        assert_eq!(
            c_read_open(ptr::null_mut(), ptr::null_mut()),
            r_read_open(ptr::null_mut(), ptr::null_mut())
        );
        let c_read: Symbol<FileRead> = symbol(&libs.c, b"LZ4F_read");
        let r_read: Symbol<FileRead> = symbol(&libs.rust, b"LZ4F_read");
        assert_eq!(
            c_read(ptr::null_mut(), (&mut byte as *mut u8).cast(), 1),
            r_read(ptr::null_mut(), (&mut byte as *mut u8).cast(), 1)
        );
        let c_write_open: Symbol<FileWriteOpen> = symbol(&libs.c, b"LZ4F_writeOpen");
        let r_write_open: Symbol<FileWriteOpen> = symbol(&libs.rust, b"LZ4F_writeOpen");
        assert_eq!(
            c_write_open(ptr::null_mut(), ptr::null_mut(), ptr::null()),
            r_write_open(ptr::null_mut(), ptr::null_mut(), ptr::null())
        );
        let c_write: Symbol<FileWrite> = symbol(&libs.c, b"LZ4F_write");
        let r_write: Symbol<FileWrite> = symbol(&libs.rust, b"LZ4F_write");
        assert_eq!(
            c_write(ptr::null_mut(), (&byte as *const u8).cast(), 1),
            r_write(ptr::null_mut(), (&byte as *const u8).cast(), 1)
        );

        type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        for name in [
            &b"LZ4F_createCompressionContext"[..],
            b"LZ4F_createDecompressionContext",
        ] {
            let c_fn: Symbol<Create> = symbol(&libs.c, name);
            let r_fn: Symbol<Create> = symbol(&libs.rust, name);
            assert_eq!(
                c_fn(ptr::null_mut(), c_uint::MAX),
                r_fn(ptr::null_mut(), c_uint::MAX)
            );
        }

        type Compress =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let source = [0u8; 1];
        let mut c_out = [0u8; 32];
        let mut r_out = [0u8; 32];
        for name in [&b"LZ4_compress_fast"[..], b"LZ4_compress_HC"] {
            let c_fn: Symbol<Compress> = symbol(&libs.c, name);
            let r_fn: Symbol<Compress> = symbol(&libs.rust, name);
            for (source_size, capacity) in [(-1, 32), (1, 0), (1, -1)] {
                assert_eq!(
                    c_fn(
                        source.as_ptr().cast(),
                        c_out.as_mut_ptr().cast(),
                        source_size,
                        capacity,
                        1,
                    ),
                    r_fn(
                        source.as_ptr().cast(),
                        r_out.as_mut_ptr().cast(),
                        source_size,
                        capacity,
                        1,
                    )
                );
            }
        }
    }
}

unsafe fn libc() -> Library {
    unsafe { Library::new("libc.so.6").unwrap() }
}

unsafe fn write_lz4_file(lib: &Library, path: &Path, input: &[u8]) -> usize {
    type Fopen = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
    type Fclose = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Open = unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
    type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
    type Close = unsafe extern "C" fn(*mut c_void) -> usize;
    let libc = unsafe { libc() };
    let fopen: Symbol<Fopen> = unsafe { symbol(&libc, b"fopen") };
    let fclose: Symbol<Fclose> = unsafe { symbol(&libc, b"fclose") };
    let open: Symbol<Open> = unsafe { symbol(lib, b"LZ4F_writeOpen") };
    let write: Symbol<Write> = unsafe { symbol(lib, b"LZ4F_write") };
    let close: Symbol<Close> = unsafe { symbol(lib, b"LZ4F_writeClose") };
    let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let mode = CString::new("wb").unwrap();
    let file = unsafe { fopen(path.as_ptr(), mode.as_ptr()) };
    assert!(!file.is_null());
    let mut state = ptr::null_mut();
    assert_eq!(unsafe { open(&mut state, file, ptr::null()) }, 0);
    for chunk in input.chunks(777) {
        assert_eq!(
            unsafe { write(state, chunk.as_ptr().cast(), chunk.len()) },
            chunk.len()
        );
    }
    let close_result = unsafe { close(state) };
    assert_eq!(unsafe { fclose(file) }, 0);
    close_result
}

unsafe fn read_lz4_file(lib: &Library, path: &Path) -> (usize, Vec<u8>, usize) {
    type Fopen = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void;
    type Fclose = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Open = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
    type Read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
    type Close = unsafe extern "C" fn(*mut c_void) -> usize;
    let libc = libc();
    let fopen: Symbol<Fopen> = symbol(&libc, b"fopen");
    let fclose: Symbol<Fclose> = symbol(&libc, b"fclose");
    let open: Symbol<Open> = symbol(lib, b"LZ4F_readOpen");
    let read: Symbol<Read> = symbol(lib, b"LZ4F_read");
    let close: Symbol<Close> = symbol(lib, b"LZ4F_readClose");
    let path = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let mode = CString::new("rb").unwrap();
    let file = fopen(path.as_ptr(), mode.as_ptr());
    assert!(!file.is_null());
    let mut state = ptr::null_mut();
    let open_result = open(&mut state, file);
    let mut output = Vec::new();
    if open_result == 0 {
        loop {
            let mut chunk = [0u8; 509];
            let read_result = read(state, chunk.as_mut_ptr().cast(), chunk.len());
            assert!(read_result <= chunk.len(), "LZ4F_read error {read_result}");
            if read_result == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..read_result]);
        }
    }
    let close_result = if state.is_null() { 0 } else { close(state) };
    assert_eq!(fclose(file), 0);
    (open_result, output, close_result)
}

#[test]
fn file_write_wrappers_match() {
    unsafe {
        let libs = Libraries::load();
        let mut seed = 0x1020_3040_5060_7080;
        let base = std::env::temp_dir().join(format!("lz4-diff-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        for len in [0, 1, 4096, 70_000] {
            let input = patterned(&mut seed, len, len);
            let c_path = base.join(format!("{len}-c.lz4"));
            let r_path = base.join(format!("{len}-r.lz4"));
            let c_close = unsafe { write_lz4_file(&libs.c, &c_path, &input) };
            let r_close = unsafe { write_lz4_file(&libs.rust, &r_path, &input) };
            assert_eq!(c_close, r_close);
            assert_eq!(
                std::fs::read(&c_path).unwrap(),
                std::fs::read(&r_path).unwrap()
            );
            if len >= 4096 {
                let c_read = unsafe { read_lz4_file(&libs.c, &c_path) };
                let r_read = unsafe { read_lz4_file(&libs.rust, &c_path) };
                assert_eq!(c_read, r_read);
                assert_eq!(c_read.1, input);
            }
        }
        std::fs::remove_dir_all(base).unwrap();
    }
}
