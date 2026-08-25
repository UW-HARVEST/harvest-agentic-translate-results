mod common;

use common::*;
use std::ffi::{CStr, c_char, c_int, c_uint, c_ulonglong, c_void};
use std::ptr;

#[test]
fn all_c_exports_resolve_from_both_shared_objects() {
    unsafe {
        let libs = Libraries::load();
        let symbols = include_str!("../SYMBOLS.md")
            .lines()
            .filter_map(|line| {
                if !line.starts_with("| ") {
                    return None;
                }
                let mut ticks = line.match_indices('`');
                let start = ticks.next()?.0 + 1;
                let end = ticks.next()?.0;
                Some(&line[start..end])
            })
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 143);
        for symbol in symbols {
            let mut name = symbol.as_bytes().to_vec();
            name.push(0);
            let _: libloading::Symbol<'_, *mut c_void> = libs
                .c
                .get(&name)
                .unwrap_or_else(|_| panic!("C missing {symbol}"));
            let _: libloading::Symbol<'_, *mut c_void> = libs
                .rust
                .get(&name)
                .unwrap_or_else(|_| panic!("Rust missing {symbol}"));
        }
    }
}

#[test]
fn metadata_bounds_and_error_helpers_match() {
    unsafe {
        let libs = Libraries::load();
        for name in [
            b"LZ4_versionNumber\0".as_slice(),
            b"LZ4F_compressionLevel_max\0",
        ] {
            let (c, r) = libs.pair::<unsafe extern "C" fn() -> c_int>(name);
            assert_eq!(
                c(),
                r(),
                "{}",
                CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()
            );
        }
        for name in [b"LZ4F_getVersion\0".as_slice(), b"LZ4_XXH_versionNumber\0"] {
            let (c, r) = libs.pair::<unsafe extern "C" fn() -> c_uint>(name);
            assert_eq!(c(), r());
        }
        let (c_version, r_version) =
            libs.pair::<unsafe extern "C" fn() -> *const c_char>(b"LZ4_versionString\0");
        assert_eq!(
            CStr::from_ptr(c_version()).to_bytes(),
            CStr::from_ptr(r_version()).to_bytes()
        );

        let (c_bound, r_bound) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let (c_ring, r_ring) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_decoderRingBufferSize\0");
        for value in [
            -1,
            0,
            1,
            15,
            16,
            65535,
            65536,
            LZ4_MAX_INPUT_SIZE,
            LZ4_MAX_INPUT_SIZE + 1,
        ] {
            assert_eq!(c_bound(value), r_bound(value), "bound({value})");
            assert_eq!(c_ring(value), r_ring(value), "ring({value})");
        }

        let (c_is_error, r_is_error) =
            libs.pair::<unsafe extern "C" fn(usize) -> c_uint>(b"LZ4F_isError\0");
        let (c_error_code, r_error_code) =
            libs.pair::<unsafe extern "C" fn(usize) -> c_int>(b"LZ4F_getErrorCode\0");
        let (c_error_name, r_error_name) =
            libs.pair::<unsafe extern "C" fn(usize) -> *const c_char>(b"LZ4F_getErrorName\0");
        for code in (0..=24).map(|value| 0usize.wrapping_sub(value)) {
            assert_eq!(c_is_error(code), r_is_error(code));
            assert_eq!(c_error_code(code), r_error_code(code));
            assert_eq!(
                CStr::from_ptr(c_error_name(code)).to_bytes(),
                CStr::from_ptr(r_error_name(code)).to_bytes()
            );
        }
    }
}

#[test]
fn xxhash_one_shot_streaming_copy_and_canonical_match() {
    unsafe {
        let libs = Libraries::load();
        let (c32, r32) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint>(b"LZ4_XXH32\0");
        let (c64, r64) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize, c_ulonglong) -> c_ulonglong>(
                b"LZ4_XXH64\0",
            );
        let mut rng = Rng::new(0x4c5a_3444_4946_4655);
        let sizes = [0, 1, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 255, 1024];
        for iteration in 0..96 {
            let size = if iteration < sizes.len() {
                sizes[iteration]
            } else {
                (rng.next_u64() as usize) % 8192
            };
            let mut storage = rng.bytes(size + 8);
            for offset in [0usize, 1] {
                let input = storage.as_mut_ptr().add(offset);
                for seed in [0, 1, rng.next_u64()] {
                    assert_eq!(
                        c32(input.cast(), size, seed as c_uint),
                        r32(input.cast(), size, seed as c_uint)
                    );
                    assert_eq!(c64(input.cast(), size, seed), r64(input.cast(), size, seed));
                }
            }
        }

        exercise_streaming_hash32(&libs, &mut rng);
        exercise_streaming_hash64(&libs, &mut rng);
        exercise_canonical_hashes(&libs, &mut rng);
    }
}

unsafe fn exercise_streaming_hash32(libs: &Libraries, rng: &mut Rng) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
    type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
    type Digest = unsafe extern "C" fn(*const c_void) -> c_uint;
    type Copy = unsafe extern "C" fn(*mut c_void, *const c_void);
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_XXH32_createState\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_XXH32_freeState\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_XXH32_reset\0") };
    let (update_c, update_r) = unsafe { libs.pair::<Update>(b"LZ4_XXH32_update\0") };
    let (digest_c, digest_r) = unsafe { libs.pair::<Digest>(b"LZ4_XXH32_digest\0") };
    let (copy_c, copy_r) = unsafe { libs.pair::<Copy>(b"LZ4_XXH32_copyState\0") };
    let c = unsafe { create_c() };
    let r = unsafe { create_r() };
    let c_copy = unsafe { create_c() };
    let r_copy = unsafe { create_r() };
    assert!(!c.is_null() && !r.is_null() && !c_copy.is_null() && !r_copy.is_null());
    assert_eq!(unsafe { reset_c(c, 0x1234_5678) }, unsafe {
        reset_r(r, 0x1234_5678)
    });
    for size in [0, 1, 2, 13, 16, 17, 31, 64, 257] {
        let bytes = rng.bytes(size);
        assert_eq!(
            unsafe { update_c(c, bytes.as_ptr().cast(), bytes.len()) },
            unsafe { update_r(r, bytes.as_ptr().cast(), bytes.len()) }
        );
        assert_eq!(unsafe { digest_c(c) }, unsafe { digest_r(r) });
    }
    unsafe {
        copy_c(c_copy, c);
        copy_r(r_copy, r);
    }
    let suffix = rng.bytes(79);
    assert_eq!(
        unsafe { update_c(c_copy, suffix.as_ptr().cast(), suffix.len()) },
        unsafe { update_r(r_copy, suffix.as_ptr().cast(), suffix.len()) }
    );
    assert_eq!(unsafe { digest_c(c_copy) }, unsafe { digest_r(r_copy) });
    assert_eq!(unsafe { free_c(c) }, unsafe { free_r(r) });
    assert_eq!(unsafe { free_c(c_copy) }, unsafe { free_r(r_copy) });
}

unsafe fn exercise_streaming_hash64(libs: &Libraries, rng: &mut Rng) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> c_int;
    type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
    type Digest = unsafe extern "C" fn(*const c_void) -> c_ulonglong;
    type Copy = unsafe extern "C" fn(*mut c_void, *const c_void);
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_XXH64_createState\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_XXH64_freeState\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_XXH64_reset\0") };
    let (update_c, update_r) = unsafe { libs.pair::<Update>(b"LZ4_XXH64_update\0") };
    let (digest_c, digest_r) = unsafe { libs.pair::<Digest>(b"LZ4_XXH64_digest\0") };
    let (copy_c, copy_r) = unsafe { libs.pair::<Copy>(b"LZ4_XXH64_copyState\0") };
    let c = unsafe { create_c() };
    let r = unsafe { create_r() };
    let c_copy = unsafe { create_c() };
    let r_copy = unsafe { create_r() };
    assert_eq!(unsafe { reset_c(c, u64::MAX - 7) }, unsafe {
        reset_r(r, u64::MAX - 7)
    });
    for size in [0, 1, 7, 8, 29, 32, 33, 65, 509] {
        let bytes = rng.bytes(size);
        assert_eq!(
            unsafe { update_c(c, bytes.as_ptr().cast(), bytes.len()) },
            unsafe { update_r(r, bytes.as_ptr().cast(), bytes.len()) }
        );
        assert_eq!(unsafe { digest_c(c) }, unsafe { digest_r(r) });
    }
    unsafe {
        copy_c(c_copy, c);
        copy_r(r_copy, r);
    }
    let suffix = rng.bytes(93);
    assert_eq!(
        unsafe { update_c(c_copy, suffix.as_ptr().cast(), suffix.len()) },
        unsafe { update_r(r_copy, suffix.as_ptr().cast(), suffix.len()) }
    );
    assert_eq!(unsafe { digest_c(c_copy) }, unsafe { digest_r(r_copy) });
    assert_eq!(unsafe { free_c(c) }, unsafe { free_r(r) });
    assert_eq!(unsafe { free_c(c_copy) }, unsafe { free_r(r_copy) });
}

unsafe fn exercise_canonical_hashes(libs: &Libraries, rng: &mut Rng) {
    type To32 = unsafe extern "C" fn(*mut c_void, c_uint);
    type From32 = unsafe extern "C" fn(*const c_void) -> c_uint;
    type To64 = unsafe extern "C" fn(*mut c_void, c_ulonglong);
    type From64 = unsafe extern "C" fn(*const c_void) -> c_ulonglong;
    let (to32_c, to32_r) = unsafe { libs.pair::<To32>(b"LZ4_XXH32_canonicalFromHash\0") };
    let (from32_c, from32_r) = unsafe { libs.pair::<From32>(b"LZ4_XXH32_hashFromCanonical\0") };
    let (to64_c, to64_r) = unsafe { libs.pair::<To64>(b"LZ4_XXH64_canonicalFromHash\0") };
    let (from64_c, from64_r) = unsafe { libs.pair::<From64>(b"LZ4_XXH64_hashFromCanonical\0") };
    for _ in 0..64 {
        let value = rng.next_u64();
        let mut c32 = [0u8; 4];
        let mut r32 = [0u8; 4];
        unsafe {
            to32_c(c32.as_mut_ptr().cast(), value as u32);
            to32_r(r32.as_mut_ptr().cast(), value as u32);
        }
        assert_eq!(c32, r32);
        assert_eq!(unsafe { from32_c(c32.as_ptr().cast()) }, unsafe {
            from32_r(r32.as_ptr().cast())
        });
        let mut c64 = [0u8; 8];
        let mut r64 = [0u8; 8];
        unsafe {
            to64_c(c64.as_mut_ptr().cast(), value);
            to64_r(r64.as_mut_ptr().cast(), value);
        }
        assert_eq!(c64, r64);
        assert_eq!(unsafe { from64_c(c64.as_ptr().cast()) }, unsafe {
            from64_r(r64.as_ptr().cast())
        });
    }
}

#[test]
fn randomized_core_and_hc_block_paths_match_byte_for_byte() {
    unsafe {
        let libs = Libraries::load();
        let (bound_c, bound_r) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let (compress_c, compress_r) = libs.pair::<Compress>(b"LZ4_compress_default\0");
        let (fast_c, fast_r) = libs.pair::<CompressFast>(b"LZ4_compress_fast\0");
        let (hc_c, hc_r) = libs.pair::<CompressFast>(b"LZ4_compress_HC\0");
        let (decompress_c, decompress_r) = libs.pair::<Decompress>(b"LZ4_decompress_safe\0");
        type Partial =
            unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let (partial_c, partial_r) = libs.pair::<Partial>(b"LZ4_decompress_safe_partial\0");
        let mut rng = Rng::new(0x8ac7_2371_42d0_91ee);
        let boundaries = [0, 1, 4, 12, 13, 15, 16, 63, 255, 1024, 65535, 65536, 65547];
        for iteration in 0..80 {
            let size = if iteration < boundaries.len() {
                boundaries[iteration]
            } else {
                (rng.next_u64() as usize) % 131_072
            };
            let generated = if iteration % 2 == 0 {
                patterned(size)
            } else {
                rng.bytes(size)
            };
            let storage = if generated.is_empty() {
                vec![0]
            } else {
                generated
            };
            let input = &storage[..size];
            let bound = bound_c(size as c_int);
            assert_eq!(bound, bound_r(size as c_int));
            let (c_result, c_bytes) = compress_with(&compress_c, &input, bound as usize);
            let (r_result, r_bytes) = compress_with(&compress_r, &input, bound as usize);
            assert_eq!(c_result, r_result);
            assert_eq!(c_bytes, r_bytes);

            let (c_dec_result, c_dec) = decompress_with(&decompress_c, &c_bytes, input.len());
            let (r_dec_result, r_dec) = decompress_with(&decompress_r, &r_bytes, input.len());
            assert_eq!(c_dec_result, r_dec_result);
            assert_eq!(c_dec, r_dec);
            assert_eq!(c_dec, input);

            for acceleration in [-1, 0, 1, 2, 17, 65537, 65538] {
                let mut c_out = vec![0u8; bound as usize];
                let mut r_out = vec![0u8; bound as usize];
                let c_size = fast_c(
                    input.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound,
                    acceleration,
                );
                let r_size = fast_r(
                    input.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound,
                    acceleration,
                );
                assert_eq!(c_size, r_size);
                assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
            }

            for level in [-1, 0, 1, 2, 9, 10, 11, 12, 13] {
                let mut c_out = vec![0u8; bound as usize];
                let mut r_out = vec![0u8; bound as usize];
                let c_size = hc_c(
                    input.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound,
                    level,
                );
                let r_size = hc_r(
                    input.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound,
                    level,
                );
                assert_eq!(c_size, r_size);
                assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
            }

            if !input.is_empty() {
                for target in [0, 1, size / 2, size, size + 3] {
                    let capacity = target.min(size);
                    let mut c_out = vec![0u8; capacity.max(1)];
                    let mut r_out = vec![0u8; capacity.max(1)];
                    let c_size = partial_c(
                        c_bytes.as_ptr().cast(),
                        c_out.as_mut_ptr().cast(),
                        c_bytes.len() as c_int,
                        target as c_int,
                        capacity as c_int,
                    );
                    let r_size = partial_r(
                        r_bytes.as_ptr().cast(),
                        r_out.as_mut_ptr().cast(),
                        r_bytes.len() as c_int,
                        target as c_int,
                        capacity as c_int,
                    );
                    assert_eq!(c_size, r_size);
                    if c_size >= 0 {
                        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
                    }
                }
            }
        }

        assert_eq!(
            decompress_c(ptr::null(), ptr::null_mut(), 0, 0),
            decompress_r(ptr::null(), ptr::null_mut(), 0, 0)
        );
    }
}
