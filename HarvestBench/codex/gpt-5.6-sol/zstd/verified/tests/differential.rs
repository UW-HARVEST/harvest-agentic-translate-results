use libloading::Library;
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CompressionParameters {
    window_log: c_uint,
    chain_log: c_uint,
    hash_log: c_uint,
    search_log: c_uint,
    min_match: c_uint,
    target_length: c_uint,
    strategy: c_uint,
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

struct Api {
    library: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            library: Library::new(path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display())),
        }
    }

    unsafe fn symbol<T: Copy>(&self, name: &[u8]) -> T {
        *self.library.get::<T>(name).unwrap_or_else(|error| {
            panic!("missing {:?}: {error}", CStr::from_bytes_with_nul(name))
        })
    }

    unsafe fn is_error(&self, code: usize) -> bool {
        let function: unsafe extern "C" fn(usize) -> c_uint = self.symbol(b"ZSTD_isError\0");
        function(code) != 0
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

unsafe fn libraries() -> (Api, Api) {
    (
        Api::open(&root().join("c_src/build/libzstd.so")),
        Api::open(&root().join("target/release/libzstd.so")),
    )
}

fn random_bytes(seed: &mut u64, size: usize) -> Vec<u8> {
    (0..size)
        .map(|_| {
            *seed ^= *seed << 13;
            *seed ^= *seed >> 7;
            *seed ^= *seed << 17;
            (*seed >> 24) as u8
        })
        .collect()
}

unsafe fn compress(api: &Api, input: &[u8], level: c_int, capacity: usize) -> (usize, Vec<u8>) {
    let function: unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int) -> usize =
        api.symbol(b"ZSTD_compress\0");
    let mut output = vec![0xa5; capacity.max(1)];
    let result = function(
        output.as_mut_ptr().cast(),
        capacity,
        input.as_ptr().cast(),
        input.len(),
        level,
    );
    if !api.is_error(result) {
        output.truncate(result);
    }
    (result, output)
}

unsafe fn decompress(api: &Api, input: &[u8], capacity: usize) -> (usize, Vec<u8>) {
    let function: unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> usize =
        api.symbol(b"ZSTD_decompress\0");
    let mut output = vec![0x5a; capacity.max(1)];
    let result = function(
        output.as_mut_ptr().cast(),
        capacity,
        input.as_ptr().cast(),
        input.len(),
    );
    if !api.is_error(result) {
        output.truncate(result);
    }
    (result, output)
}

#[test]
fn every_c_dynamic_symbol_loads_from_both_libraries() {
    unsafe {
        let (c, rust) = libraries();
        let symbols = std::fs::read_to_string(root().join("SYMBOLS.md")).unwrap();
        let mut count = 0;
        for line in symbols
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains(" | `"))
        {
            let Some(start) = line.find('`') else {
                continue;
            };
            let Some(end) = line[start + 1..].find('`') else {
                continue;
            };
            let mut name = line[start + 1..start + 1 + end].as_bytes().to_vec();
            name.push(0);
            let _: *mut c_void = c.symbol(&name);
            let _: *mut c_void = rust.symbol(&name);
            count += 1;
        }
        assert_eq!(count, 615);
    }
}

#[test]
fn metadata_bounds_and_error_surface_match() {
    unsafe {
        let (c, rust) = libraries();
        let version: unsafe extern "C" fn() -> c_uint = c.symbol(b"ZSTD_versionNumber\0");
        let rust_version: unsafe extern "C" fn() -> c_uint = rust.symbol(b"ZSTD_versionNumber\0");
        assert_eq!(version(), rust_version());

        for name in [
            b"ZSTD_versionString\0".as_slice(),
            b"ZSTD_getErrorName\0",
            b"FSE_getErrorName\0",
            b"HUF_getErrorName\0",
            b"ZDICT_getErrorName\0",
        ] {
            let left: unsafe extern "C" fn(usize) -> *const c_char = c.symbol(name);
            let right: unsafe extern "C" fn(usize) -> *const c_char = rust.symbol(name);
            for code in [0, 1, usize::MAX, usize::MAX - 1, usize::MAX - 120] {
                assert_eq!(
                    CStr::from_ptr(left(code)),
                    CStr::from_ptr(right(code)),
                    "{name:?} {code}"
                );
            }
        }

        for name in [
            b"ZSTD_minCLevel\0".as_slice(),
            b"ZSTD_maxCLevel\0",
            b"ZSTD_defaultCLevel\0",
        ] {
            let left: unsafe extern "C" fn() -> c_int = c.symbol(name);
            let right: unsafe extern "C" fn() -> c_int = rust.symbol(name);
            assert_eq!(left(), right(), "{name:?}");
        }

        let c_bound: unsafe extern "C" fn(usize) -> usize = c.symbol(b"ZSTD_compressBound\0");
        let r_bound: unsafe extern "C" fn(usize) -> usize = rust.symbol(b"ZSTD_compressBound\0");
        for size in [
            0,
            1,
            63,
            64,
            127,
            128,
            1024,
            131_071,
            131_072,
            usize::MAX,
            0xff00_ff00_ff00_feffusize,
            0xff00_ff00_ff00_ff00usize,
        ] {
            assert_eq!(c_bound(size), r_bound(size), "compressBound({size})");
        }

        for name in [
            b"ZSTD_cParam_getBounds\0".as_slice(),
            b"ZSTD_dParam_getBounds\0",
        ] {
            let left: unsafe extern "C" fn(c_int) -> Bounds = c.symbol(name);
            let right: unsafe extern "C" fn(c_int) -> Bounds = rust.symbol(name);
            for parameter in [
                c_int::MIN,
                -1,
                0,
                10,
                99,
                100,
                101,
                107,
                130,
                164,
                200,
                201,
                202,
                400,
                402,
                500,
                1000,
                1017,
                c_int::MAX,
            ] {
                assert_eq!(left(parameter), right(parameter), "{name:?}({parameter})");
            }
        }
    }
}

#[test]
fn xxhash_one_shot_and_streaming_match() {
    unsafe {
        let (c, rust) = libraries();
        let version_c: unsafe extern "C" fn() -> c_uint = c.symbol(b"ZSTD_XXH_versionNumber\0");
        let version_r: unsafe extern "C" fn() -> c_uint = rust.symbol(b"ZSTD_XXH_versionNumber\0");
        assert_eq!(version_c(), version_r());
        let hash32_c: unsafe extern "C" fn(*const c_void, usize, u32) -> u32 =
            c.symbol(b"ZSTD_XXH32\0");
        let hash32_r: unsafe extern "C" fn(*const c_void, usize, u32) -> u32 =
            rust.symbol(b"ZSTD_XXH32\0");
        let hash64_c: unsafe extern "C" fn(*const c_void, usize, u64) -> u64 =
            c.symbol(b"ZSTD_XXH64\0");
        let hash64_r: unsafe extern "C" fn(*const c_void, usize, u64) -> u64 =
            rust.symbol(b"ZSTD_XXH64\0");
        let mut seed = 0x3141_5926_5358_9793;
        for size in [0, 1, 3, 4, 7, 8, 15, 16, 31, 32, 33, 255, 4096] {
            let input = random_bytes(&mut seed, size);
            for hash_seed in [0, 1, u32::MAX, seed as u32] {
                assert_eq!(
                    hash32_c(input.as_ptr().cast(), size, hash_seed),
                    hash32_r(input.as_ptr().cast(), size, hash_seed)
                );
            }
            for hash_seed in [0, 1, u64::MAX, seed] {
                assert_eq!(
                    hash64_c(input.as_ptr().cast(), size, hash_seed),
                    hash64_r(input.as_ptr().cast(), size, hash_seed)
                );
            }
        }

        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Reset32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type Digest32 = unsafe extern "C" fn(*const c_void) -> u32;
        type Reset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
        type Digest64 = unsafe extern "C" fn(*const c_void) -> u64;

        for bits in [32, 64] {
            let create_name = if bits == 32 {
                b"ZSTD_XXH32_createState\0".as_slice()
            } else {
                b"ZSTD_XXH64_createState\0"
            };
            let free_name = if bits == 32 {
                b"ZSTD_XXH32_freeState\0".as_slice()
            } else {
                b"ZSTD_XXH64_freeState\0"
            };
            let update_name = if bits == 32 {
                b"ZSTD_XXH32_update\0".as_slice()
            } else {
                b"ZSTD_XXH64_update\0"
            };
            let create_c: Create = c.symbol(create_name);
            let create_r: Create = rust.symbol(create_name);
            let free_c: Free = c.symbol(free_name);
            let free_r: Free = rust.symbol(free_name);
            let update_c: Update = c.symbol(update_name);
            let update_r: Update = rust.symbol(update_name);
            let cs = create_c();
            let rs = create_r();
            assert!(!cs.is_null() && !rs.is_null());
            if bits == 32 {
                let reset_c: Reset32 = c.symbol(b"ZSTD_XXH32_reset\0");
                let reset_r: Reset32 = rust.symbol(b"ZSTD_XXH32_reset\0");
                assert_eq!(reset_c(cs, 0x89ab_cdef), reset_r(rs, 0x89ab_cdef));
            } else {
                let reset_c: Reset64 = c.symbol(b"ZSTD_XXH64_reset\0");
                let reset_r: Reset64 = rust.symbol(b"ZSTD_XXH64_reset\0");
                assert_eq!(
                    reset_c(cs, 0x0123_4567_89ab_cdef),
                    reset_r(rs, 0x0123_4567_89ab_cdef)
                );
            }
            let input = random_bytes(&mut seed, 8193);
            for chunk in input.chunks(127) {
                assert_eq!(
                    update_c(cs, chunk.as_ptr().cast(), chunk.len()),
                    update_r(rs, chunk.as_ptr().cast(), chunk.len())
                );
            }
            if bits == 32 {
                let digest_c: Digest32 = c.symbol(b"ZSTD_XXH32_digest\0");
                let digest_r: Digest32 = rust.symbol(b"ZSTD_XXH32_digest\0");
                assert_eq!(digest_c(cs), digest_r(rs));
            } else {
                let digest_c: Digest64 = c.symbol(b"ZSTD_XXH64_digest\0");
                let digest_r: Digest64 = rust.symbol(b"ZSTD_XXH64_digest\0");
                assert_eq!(digest_c(cs), digest_r(rs));
            }
            assert_eq!(free_c(cs), free_r(rs));
        }
    }
}

#[test]
fn entropy_primitive_boundaries_match() {
    unsafe {
        let (c, rust) = libraries();
        let version_c: unsafe extern "C" fn() -> c_uint = c.symbol(b"FSE_versionNumber\0");
        let version_r: unsafe extern "C" fn() -> c_uint = rust.symbol(b"FSE_versionNumber\0");
        assert_eq!(version_c(), version_r());
        for name in [
            b"FSE_isError\0".as_slice(),
            b"HUF_isError\0",
            b"HIST_isError\0",
        ] {
            let left: unsafe extern "C" fn(usize) -> c_uint = c.symbol(name);
            let right: unsafe extern "C" fn(usize) -> c_uint = rust.symbol(name);
            for value in [0, 1, 255, usize::MAX, usize::MAX - 120] {
                assert_eq!(left(value), right(value), "{name:?}({value})");
            }
        }
        for name in [b"FSE_compressBound\0".as_slice(), b"HUF_compressBound\0"] {
            let left: unsafe extern "C" fn(usize) -> usize = c.symbol(name);
            let right: unsafe extern "C" fn(usize) -> usize = rust.symbol(name);
            for size in [0, 1, 2, 255, 256, 65_535, usize::MAX] {
                assert_eq!(left(size), right(size), "{name:?}({size})");
            }
        }
        let optimal_c: unsafe extern "C" fn(c_uint, usize, c_uint) -> c_uint =
            c.symbol(b"FSE_optimalTableLog\0");
        let optimal_r: unsafe extern "C" fn(c_uint, usize, c_uint) -> c_uint =
            rust.symbol(b"FSE_optimalTableLog\0");
        let ncount_c: unsafe extern "C" fn(c_uint, c_uint) -> usize =
            c.symbol(b"FSE_NCountWriteBound\0");
        let ncount_r: unsafe extern "C" fn(c_uint, c_uint) -> usize =
            rust.symbol(b"FSE_NCountWriteBound\0");
        for table_log in [0, 1, 5, 6, 12, 15, c_uint::MAX] {
            for size in [0, 1, 2, 255, 4096, usize::MAX] {
                for symbol in [0, 1, 15, 255, c_uint::MAX] {
                    assert_eq!(
                        optimal_c(table_log, size, symbol),
                        optimal_r(table_log, size, symbol)
                    );
                    assert_eq!(ncount_c(symbol, table_log), ncount_r(symbol, table_log));
                }
            }
        }
    }
}

#[test]
fn randomized_one_shot_frames_are_byte_identical() {
    unsafe {
        let (c, rust) = libraries();
        let c_bound: unsafe extern "C" fn(usize) -> usize = c.symbol(b"ZSTD_compressBound\0");
        let mut seed = 0x4d59_5df4_d0f3_3173;
        let levels = [-100, -7, -1, 0, 1, 3, 9, 16, 19, 22, 23, c_int::MAX];
        let sizes = [0, 1, 2, 3, 7, 31, 128, 1024, 4096, 65_535, 131_072];

        for level in levels {
            for size in sizes {
                let input = random_bytes(&mut seed, size);
                let capacity = c_bound(size);
                let (c_size, c_frame) = compress(&c, &input, level, capacity);
                let (r_size, r_frame) = compress(&rust, &input, level, capacity);
                assert_eq!(c_size, r_size, "level={level}, size={size}");
                assert_eq!(c_frame, r_frame, "level={level}, size={size}");
                if !c.is_error(c_size) {
                    let (c_decoded, c_output) = decompress(&c, &c_frame, size);
                    let (r_decoded, r_output) = decompress(&rust, &r_frame, size);
                    assert_eq!(c_decoded, r_decoded);
                    assert_eq!(c_output, r_output);
                    assert_eq!(c_output, input);
                }

                for small_capacity in [0, 1, c_size.saturating_sub(1).min(capacity)] {
                    let (c_error, _) = compress(&c, &input, level, small_capacity);
                    let (r_error, _) = compress(&rust, &input, level, small_capacity);
                    assert_eq!(c_error, r_error, "small dst, level={level}, size={size}");
                }
            }
        }
    }
}

unsafe fn staged_round_trip(
    api: &Api,
    input: &[u8],
    level: c_int,
) -> (Vec<usize>, Vec<u8>, Vec<usize>, Vec<u8>) {
    let bound: unsafe extern "C" fn(usize) -> usize = api.symbol(b"ZSTD_compressBound\0");
    let create_c: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createCCtx\0");
    let free_c: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCCtx\0");
    let begin_c: unsafe extern "C" fn(*mut c_void, c_int) -> usize =
        api.symbol(b"ZSTD_compressBegin\0");
    let continue_c: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize = api.symbol(b"ZSTD_compressContinue\0");
    let end_c: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize = api.symbol(b"ZSTD_compressEnd\0");
    let cctx = create_c();
    let mut compression_trace = vec![begin_c(cctx, level)];
    let mut frame = vec![0; bound(input.len()) + 64];
    let first = continue_c(
        cctx,
        frame.as_mut_ptr().cast(),
        frame.len(),
        input.as_ptr().cast(),
        input.len(),
    );
    compression_trace.push(first);
    assert!(!api.is_error(first));
    let last = end_c(
        cctx,
        frame[first..].as_mut_ptr().cast(),
        frame.len() - first,
        ptr::null(),
        0,
    );
    compression_trace.push(last);
    assert!(!api.is_error(last));
    frame.truncate(first + last);
    assert_eq!(free_c(cctx), 0);

    let create_d: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createDCtx\0");
    let free_d: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeDCtx\0");
    let begin_d: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_decompressBegin\0");
    let next_d: unsafe extern "C" fn(*mut c_void) -> usize =
        api.symbol(b"ZSTD_nextSrcSizeToDecompress\0");
    let continue_d: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize = api.symbol(b"ZSTD_decompressContinue\0");
    let dctx = create_d();
    let mut decompression_trace = vec![begin_d(dctx)];
    let mut source_offset = 0;
    let mut output = vec![0; input.len().max(1)];
    let mut output_offset = 0;
    loop {
        let required = next_d(dctx);
        decompression_trace.push(required);
        if required == 0 {
            break;
        }
        assert!(source_offset + required <= frame.len());
        let decoded = continue_d(
            dctx,
            output[output_offset..].as_mut_ptr().cast(),
            input.len() - output_offset,
            frame[source_offset..].as_ptr().cast(),
            required,
        );
        decompression_trace.push(decoded);
        assert!(!api.is_error(decoded));
        source_offset += required;
        output_offset += decoded;
    }
    output.truncate(output_offset);
    assert_eq!(source_offset, frame.len());
    assert_eq!(free_d(dctx), 0);
    (compression_trace, frame, decompression_trace, output)
}

#[test]
fn staged_low_level_frame_protocol_matches() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0x7654_3210_fedc_ba98;
        for size in [0, 1, 127, 4096, 131_073] {
            let input = random_bytes(&mut seed, size);
            for level in [-5, 1, 3, 15, 22] {
                let left = staged_round_trip(&c, &input, level);
                let right = staged_round_trip(&rust, &input, level);
                assert_eq!(left, right, "size={size}, level={level}");
                assert_eq!(left.3, input);
            }
        }
    }
}

unsafe fn context_compress(
    api: &Api,
    input: &[u8],
    settings: &[(c_int, c_int)],
) -> (Vec<usize>, usize, Vec<u8>) {
    let create: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createCCtx\0");
    let free: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCCtx\0");
    let set: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize =
        api.symbol(b"ZSTD_CCtx_setParameter\0");
    let bound: unsafe extern "C" fn(usize) -> usize = api.symbol(b"ZSTD_compressBound\0");
    let compress2: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
    ) -> usize = api.symbol(b"ZSTD_compress2\0");
    let context = create();
    assert!(!context.is_null());
    let results = settings
        .iter()
        .map(|&(parameter, value)| set(context, parameter, value))
        .collect();
    let mut output = vec![0; bound(input.len())];
    let size = compress2(
        context,
        output.as_mut_ptr().cast(),
        output.len(),
        input.as_ptr().cast(),
        input.len(),
    );
    if !api.is_error(size) {
        output.truncate(size);
    }
    assert_eq!(free(context), 0);
    (results, size, output)
}

#[test]
fn context_parameter_cross_product_matches() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0xfedc_ba98_7654_3210;
        for level in [-5, 0, 3, 12, 19] {
            for strategy in 0..=9 {
                for flags in 0..8 {
                    let input = random_bytes(&mut seed, 8192 + (flags * 17) as usize);
                    let settings = [
                        (100, level),
                        (107, strategy),
                        (200, flags & 1),
                        (201, (flags >> 1) & 1),
                        (202, (flags >> 2) & 1),
                    ];
                    let left = context_compress(&c, &input, &settings);
                    let right = context_compress(&rust, &input, &settings);
                    assert_eq!(
                        left, right,
                        "level={level}, strategy={strategy}, flags={flags}"
                    );
                }
            }
        }
    }
}

#[test]
fn parameter_objects_and_generated_parameters_match() {
    unsafe {
        let (c, rust) = libraries();
        let create_c: unsafe extern "C" fn() -> *mut c_void = c.symbol(b"ZSTD_createCCtxParams\0");
        let create_r: unsafe extern "C" fn() -> *mut c_void =
            rust.symbol(b"ZSTD_createCCtxParams\0");
        let free_c: unsafe extern "C" fn(*mut c_void) -> usize = c.symbol(b"ZSTD_freeCCtxParams\0");
        let free_r: unsafe extern "C" fn(*mut c_void) -> usize =
            rust.symbol(b"ZSTD_freeCCtxParams\0");
        let set_c: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize =
            c.symbol(b"ZSTD_CCtxParams_setParameter\0");
        let set_r: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize =
            rust.symbol(b"ZSTD_CCtxParams_setParameter\0");
        let get_c: unsafe extern "C" fn(*const c_void, c_int, *mut c_int) -> usize =
            c.symbol(b"ZSTD_CCtxParams_getParameter\0");
        let get_r: unsafe extern "C" fn(*const c_void, c_int, *mut c_int) -> usize =
            rust.symbol(b"ZSTD_CCtxParams_getParameter\0");
        let reset_c: unsafe extern "C" fn(*mut c_void) -> usize =
            c.symbol(b"ZSTD_CCtxParams_reset\0");
        let reset_r: unsafe extern "C" fn(*mut c_void) -> usize =
            rust.symbol(b"ZSTD_CCtxParams_reset\0");
        let init_c: unsafe extern "C" fn(*mut c_void, c_int) -> usize =
            c.symbol(b"ZSTD_CCtxParams_init\0");
        let init_r: unsafe extern "C" fn(*mut c_void, c_int) -> usize =
            rust.symbol(b"ZSTD_CCtxParams_init\0");
        assert_eq!(free_c(ptr::null_mut()), free_r(ptr::null_mut()));
        for &(parameter, value) in &[
            (100, -5),
            (100, 22),
            (101, 10),
            (107, 9),
            (200, 0),
            (200, 1),
            (201, 1),
            (400, 0),
            (-1, 0),
            (c_int::MAX, c_int::MAX),
        ] {
            let cp = create_c();
            let rp = create_r();
            assert!(!cp.is_null() && !rp.is_null());
            assert_eq!(init_c(cp, 3), init_r(rp, 3));
            assert_eq!(set_c(cp, parameter, value), set_r(rp, parameter, value));
            let mut cv = 0x1234_5678;
            let mut rv = 0x1234_5678;
            assert_eq!(get_c(cp, parameter, &mut cv), get_r(rp, parameter, &mut rv));
            assert_eq!(cv, rv);
            assert_eq!(reset_c(cp), reset_r(rp));
            assert_eq!(free_c(cp), free_r(rp));
        }

        let get_params_c: unsafe extern "C" fn(c_int, u64, usize) -> CompressionParameters =
            c.symbol(b"ZSTD_getCParams\0");
        let get_params_r: unsafe extern "C" fn(c_int, u64, usize) -> CompressionParameters =
            rust.symbol(b"ZSTD_getCParams\0");
        let check_c: unsafe extern "C" fn(CompressionParameters) -> usize =
            c.symbol(b"ZSTD_checkCParams\0");
        let check_r: unsafe extern "C" fn(CompressionParameters) -> usize =
            rust.symbol(b"ZSTD_checkCParams\0");
        let adjust_c: unsafe extern "C" fn(
            CompressionParameters,
            u64,
            usize,
        ) -> CompressionParameters = c.symbol(b"ZSTD_adjustCParams\0");
        let adjust_r: unsafe extern "C" fn(
            CompressionParameters,
            u64,
            usize,
        ) -> CompressionParameters = rust.symbol(b"ZSTD_adjustCParams\0");
        let estimate_c: unsafe extern "C" fn(CompressionParameters) -> usize =
            c.symbol(b"ZSTD_estimateCCtxSize_usingCParams\0");
        let estimate_r: unsafe extern "C" fn(CompressionParameters) -> usize =
            rust.symbol(b"ZSTD_estimateCCtxSize_usingCParams\0");
        for level in [-100, -5, 0, 1, 3, 9, 19, 22, 23, c_int::MAX] {
            for source_size in [0, 1, 1024, 131_072, u64::MAX] {
                for dictionary_size in [0, 1, 8, 1024, usize::MAX] {
                    let cp = get_params_c(level, source_size, dictionary_size);
                    let rp = get_params_r(level, source_size, dictionary_size);
                    assert_eq!(cp, rp);
                    assert_eq!(check_c(cp), check_r(rp));
                    assert_eq!(
                        adjust_c(cp, source_size, dictionary_size),
                        adjust_r(rp, source_size, dictionary_size)
                    );
                    assert_eq!(estimate_c(cp), estimate_r(rp));
                }
            }
        }
        for invalid in [
            CompressionParameters {
                window_log: 0,
                chain_log: 0,
                hash_log: 0,
                search_log: 0,
                min_match: 0,
                target_length: 0,
                strategy: 0,
            },
            CompressionParameters {
                window_log: c_uint::MAX,
                chain_log: c_uint::MAX,
                hash_log: c_uint::MAX,
                search_log: c_uint::MAX,
                min_match: c_uint::MAX,
                target_length: c_uint::MAX,
                strategy: c_uint::MAX,
            },
        ] {
            assert_eq!(check_c(invalid), check_r(invalid));
        }
    }
}

#[test]
fn invalid_parameters_and_generic_boundaries_match() {
    unsafe {
        let (c, rust) = libraries();
        let create_c: unsafe extern "C" fn() -> *mut c_void = c.symbol(b"ZSTD_createCCtx\0");
        let create_r: unsafe extern "C" fn() -> *mut c_void = rust.symbol(b"ZSTD_createCCtx\0");
        let free_c: unsafe extern "C" fn(*mut c_void) -> usize = c.symbol(b"ZSTD_freeCCtx\0");
        let free_r: unsafe extern "C" fn(*mut c_void) -> usize = rust.symbol(b"ZSTD_freeCCtx\0");
        let set_c: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize =
            c.symbol(b"ZSTD_CCtx_setParameter\0");
        let set_r: unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize =
            rust.symbol(b"ZSTD_CCtx_setParameter\0");

        assert_eq!(free_c(ptr::null_mut()), free_r(ptr::null_mut()));
        let cc = create_c();
        let rc = create_r();
        for parameter in [c_int::MIN, -1, 0, 99, 108, 165, 203, 399, 403, c_int::MAX] {
            for value in [c_int::MIN, -1, 0, 1, c_int::MAX] {
                assert_eq!(
                    set_c(cc, parameter, value),
                    set_r(rc, parameter, value),
                    "parameter={parameter}, value={value}"
                );
            }
        }
        for (parameter, values) in [
            (101, vec![-1, 0, 9, 10, 31, 32, c_int::MAX]),
            (107, vec![-1, 0, 1, 9, 10, c_int::MAX]),
            (200, vec![-1, 0, 1, 2, c_int::MAX]),
            (400, vec![-1, 0, 1, c_int::MAX]),
            (402, vec![-1, 0, 1, 9, 10, c_int::MAX]),
        ] {
            for value in values {
                assert_eq!(
                    set_c(cc, parameter, value),
                    set_r(rc, parameter, value),
                    "parameter={parameter}, value={value}"
                );
            }
        }
        assert_eq!(free_c(cc), free_r(rc));

        let bad_inputs = [
            vec![],
            vec![0],
            vec![0x28, 0xb5, 0x2f],
            vec![0x28, 0xb5, 0x2f, 0xfd],
            vec![0xff; 64],
        ];
        for input in bad_inputs {
            for capacity in [0, 1, 8, 1024] {
                let (left, _) = decompress(&c, &input, capacity);
                let (right, _) = decompress(&rust, &input, capacity);
                assert_eq!(left, right, "input={input:x?}, capacity={capacity}");
            }
        }
    }
}

unsafe fn dictionary_compress(
    api: &Api,
    dictionary: &[u8],
    input: &[u8],
    level: c_int,
) -> (usize, Vec<u8>) {
    let create: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createCCtx\0");
    let free: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCCtx\0");
    let bound: unsafe extern "C" fn(usize) -> usize = api.symbol(b"ZSTD_compressBound\0");
    let function: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        usize,
        c_int,
    ) -> usize = api.symbol(b"ZSTD_compress_usingDict\0");
    let context = create();
    let mut output = vec![0; bound(input.len())];
    let size = function(
        context,
        output.as_mut_ptr().cast(),
        output.len(),
        input.as_ptr().cast(),
        input.len(),
        dictionary.as_ptr().cast(),
        dictionary.len(),
        level,
    );
    if !api.is_error(size) {
        output.truncate(size);
    }
    assert_eq!(free(context), 0);
    (size, output)
}

#[test]
fn dictionary_workflows_match() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0x0123_4567_89ab_cdef;
        for dict_size in [0, 1, 7, 8, 64, 1024, 8192] {
            let dictionary = random_bytes(&mut seed, dict_size);
            for input_size in [0, 1, 31, 1024, 16_384] {
                let mut input = random_bytes(&mut seed, input_size);
                if !dictionary.is_empty() {
                    for (index, byte) in input.iter_mut().enumerate() {
                        *byte = dictionary[index % dictionary.len()];
                    }
                }
                for level in [-3, 1, 5, 15] {
                    assert_eq!(
                        dictionary_compress(&c, &dictionary, &input, level),
                        dictionary_compress(&rust, &dictionary, &input, level),
                        "dict={dict_size}, input={input_size}, level={level}"
                    );
                }
            }
        }
    }
}

unsafe fn prepared_dictionary_round_trip(
    api: &Api,
    dictionary: &[u8],
    input: &[u8],
) -> (usize, Vec<u8>, usize, Vec<u8>) {
    let bound: unsafe extern "C" fn(usize) -> usize = api.symbol(b"ZSTD_compressBound\0");
    let create_cctx: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createCCtx\0");
    let free_cctx: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCCtx\0");
    let create_dctx: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createDCtx\0");
    let free_dctx: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeDCtx\0");
    let create_cdict: unsafe extern "C" fn(*const c_void, usize, c_int) -> *mut c_void =
        api.symbol(b"ZSTD_createCDict\0");
    let free_cdict: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCDict\0");
    let create_ddict: unsafe extern "C" fn(*const c_void, usize) -> *mut c_void =
        api.symbol(b"ZSTD_createDDict\0");
    let free_ddict: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeDDict\0");
    let compress: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
    ) -> usize = api.symbol(b"ZSTD_compress_usingCDict\0");
    let decompress: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
    ) -> usize = api.symbol(b"ZSTD_decompress_usingDDict\0");

    let cdict = create_cdict(dictionary.as_ptr().cast(), dictionary.len(), 5);
    let ddict = create_ddict(dictionary.as_ptr().cast(), dictionary.len());
    let cctx = create_cctx();
    let dctx = create_dctx();
    assert!(!cdict.is_null() && !ddict.is_null() && !cctx.is_null() && !dctx.is_null());

    let mut frame = vec![0; bound(input.len())];
    let compressed = compress(
        cctx,
        frame.as_mut_ptr().cast(),
        frame.len(),
        input.as_ptr().cast(),
        input.len(),
        cdict,
    );
    if !api.is_error(compressed) {
        frame.truncate(compressed);
    }
    let mut output = vec![0; input.len().max(1)];
    let decoded = decompress(
        dctx,
        output.as_mut_ptr().cast(),
        input.len(),
        frame.as_ptr().cast(),
        if api.is_error(compressed) {
            0
        } else {
            frame.len()
        },
        ddict,
    );
    if !api.is_error(decoded) {
        output.truncate(decoded);
    }

    assert_eq!(free_cctx(cctx), 0);
    assert_eq!(free_dctx(dctx), 0);
    assert_eq!(free_cdict(cdict), 0);
    assert_eq!(free_ddict(ddict), 0);
    (compressed, frame, decoded, output)
}

#[test]
fn prepared_dictionary_workflows_match() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0x9988_7766_5544_3322;
        for dict_size in [8, 64, 1024, 8192] {
            let dictionary = random_bytes(&mut seed, dict_size);
            for input_size in [0, 1, 4096, 65_537] {
                let mut input = random_bytes(&mut seed, input_size);
                for (index, byte) in input.iter_mut().enumerate() {
                    *byte = dictionary[index % dictionary.len()];
                }
                let left = prepared_dictionary_round_trip(&c, &dictionary, &input);
                let right = prepared_dictionary_round_trip(&rust, &dictionary, &input);
                assert_eq!(left, right, "dict={dict_size}, input={input_size}");
                assert_eq!(left.3, input);
            }
        }
    }
}

#[test]
fn dictionary_training_and_header_queries_match() {
    unsafe {
        let (c, rust) = libraries();
        type Train =
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, *const usize, c_uint) -> usize;
        let train_c: Train = c.symbol(b"ZDICT_trainFromBuffer\0");
        let train_r: Train = rust.symbol(b"ZDICT_trainFromBuffer\0");
        let id_c: unsafe extern "C" fn(*const c_void, usize) -> c_uint =
            c.symbol(b"ZDICT_getDictID\0");
        let id_r: unsafe extern "C" fn(*const c_void, usize) -> c_uint =
            rust.symbol(b"ZDICT_getDictID\0");
        let header_c: unsafe extern "C" fn(*const c_void, usize) -> usize =
            c.symbol(b"ZDICT_getDictHeaderSize\0");
        let header_r: unsafe extern "C" fn(*const c_void, usize) -> usize =
            rust.symbol(b"ZDICT_getDictHeaderSize\0");
        let mut seed = 0xc001_d00d_abcdef01;
        let sample_sizes = vec![256usize; 64];
        let mut samples = Vec::with_capacity(sample_sizes.iter().sum());
        let basis = random_bytes(&mut seed, 1024);
        for sample in 0..sample_sizes.len() {
            for index in 0..sample_sizes[sample] {
                samples.push(basis[(index + sample * 13) % basis.len()] ^ (sample as u8 & 7));
            }
        }
        for capacity in [0, 1, 255, 256, 512, 1024, 4096] {
            let mut left = vec![0xa5; capacity.max(1)];
            let mut right = vec![0xa5; capacity.max(1)];
            let lc = train_c(
                left.as_mut_ptr().cast(),
                capacity,
                samples.as_ptr().cast(),
                sample_sizes.as_ptr(),
                sample_sizes.len() as c_uint,
            );
            let rc = train_r(
                right.as_mut_ptr().cast(),
                capacity,
                samples.as_ptr().cast(),
                sample_sizes.as_ptr(),
                sample_sizes.len() as c_uint,
            );
            assert_eq!(lc, rc, "capacity={capacity}");
            if !c.is_error(lc) {
                assert_eq!(&left[..lc], &right[..rc]);
                for size in [0, 1, 7, 8, lc.saturating_sub(1), lc] {
                    assert_eq!(
                        id_c(left.as_ptr().cast(), size),
                        id_r(right.as_ptr().cast(), size)
                    );
                    assert_eq!(
                        header_c(left.as_ptr().cast(), size),
                        header_r(right.as_ptr().cast(), size)
                    );
                }
            }
        }
        let mut left = [0u8; 256];
        let mut right = [0u8; 256];
        assert_eq!(
            train_c(
                left.as_mut_ptr().cast(),
                left.len(),
                ptr::null(),
                ptr::null(),
                0
            ),
            train_r(
                right.as_mut_ptr().cast(),
                right.len(),
                ptr::null(),
                ptr::null(),
                0
            )
        );
    }
}

unsafe fn stream_compress(api: &Api, input: &[u8], chunk: usize, output_chunk: usize) -> Vec<u8> {
    let create: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createCStream\0");
    let free: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeCStream\0");
    let init: unsafe extern "C" fn(*mut c_void, c_int) -> usize = api.symbol(b"ZSTD_initCStream\0");
    let call: unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize =
        api.symbol(b"ZSTD_compressStream2\0");
    let stream = create();
    assert!(!stream.is_null());
    assert!(!api.is_error(init(stream, 3)));
    let mut result = Vec::new();
    let mut offset = 0;
    let mut calls = 0;
    loop {
        let end = (offset + chunk).min(input.len());
        let directive = if end == input.len() { 2 } else { 0 };
        let mut in_buffer = InBuffer {
            src: input[offset..end].as_ptr().cast(),
            size: end - offset,
            pos: 0,
        };
        loop {
            calls += 1;
            assert!(calls < 100_000, "compression stream made no progress");
            let mut storage = vec![0; output_chunk.max(1)];
            let mut out_buffer = OutBuffer {
                dst: storage.as_mut_ptr().cast(),
                size: output_chunk,
                pos: 0,
            };
            let remaining = call(stream, &mut out_buffer, &mut in_buffer, directive);
            assert!(!api.is_error(remaining));
            result.extend_from_slice(&storage[..out_buffer.pos]);
            if in_buffer.pos == in_buffer.size && (directive != 2 || remaining == 0) {
                break;
            }
        }
        offset = end;
        if offset == input.len() {
            break;
        }
    }
    assert_eq!(free(stream), 0);
    result
}

unsafe fn stream_decompress(
    api: &Api,
    frame: &[u8],
    input_chunk: usize,
    output_chunk: usize,
) -> Vec<u8> {
    let create: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZSTD_createDStream\0");
    let free: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_freeDStream\0");
    let init: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZSTD_initDStream\0");
    let call: unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer) -> usize =
        api.symbol(b"ZSTD_decompressStream\0");
    let stream = create();
    assert!(!stream.is_null());
    assert!(!api.is_error(init(stream)));
    let mut result = Vec::new();
    let mut offset = 0;
    let mut remaining = 1;
    let mut calls = 0;
    while offset < frame.len() || remaining != 0 {
        let end = (offset + input_chunk).min(frame.len());
        let mut in_buffer = InBuffer {
            src: frame[offset..end].as_ptr().cast(),
            size: end - offset,
            pos: 0,
        };
        loop {
            calls += 1;
            assert!(calls < 100_000, "decompression stream made no progress");
            let mut storage = vec![0; output_chunk.max(1)];
            let mut out_buffer = OutBuffer {
                dst: storage.as_mut_ptr().cast(),
                size: output_chunk,
                pos: 0,
            };
            remaining = call(stream, &mut out_buffer, &mut in_buffer);
            assert!(!api.is_error(remaining));
            result.extend_from_slice(&storage[..out_buffer.pos]);
            if in_buffer.pos == in_buffer.size
                && (remaining == 0 || out_buffer.pos < out_buffer.size)
            {
                break;
            }
        }
        offset = end;
    }
    assert_eq!(free(stream), 0);
    result
}

#[test]
fn streaming_chunk_shapes_match() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0xa55a_33cc_77ee_1199;
        for size in [0, 1, 127, 4096, 131_073] {
            let input = random_bytes(&mut seed, size);
            let chunks: &[usize] = if size > 127 {
                &[1024, 131_072]
            } else {
                &[1, 7, 1024, 131_072]
            };
            let output_chunks: &[usize] = if size > 127 {
                &[257, 131_591]
            } else {
                &[1, 13, 257, 131_591]
            };
            for &chunk in chunks {
                for &output_chunk in output_chunks {
                    let left = stream_compress(&c, &input, chunk, output_chunk);
                    let right = stream_compress(&rust, &input, chunk, output_chunk);
                    assert_eq!(
                        left, right,
                        "size={size}, chunk={chunk}, out={output_chunk}"
                    );
                    let c_output = stream_decompress(&c, &left, chunk, output_chunk);
                    let rust_output = stream_decompress(&rust, &right, chunk, output_chunk);
                    assert_eq!(c_output, rust_output);
                    assert_eq!(c_output, input);
                }
            }
        }
    }
}

#[test]
fn invalid_stream_states_and_enum_values_match() {
    unsafe {
        let (c, rust) = libraries();
        let create_c: unsafe extern "C" fn() -> *mut c_void = c.symbol(b"ZSTD_createCStream\0");
        let create_r: unsafe extern "C" fn() -> *mut c_void = rust.symbol(b"ZSTD_createCStream\0");
        let free_c: unsafe extern "C" fn(*mut c_void) -> usize = c.symbol(b"ZSTD_freeCStream\0");
        let free_r: unsafe extern "C" fn(*mut c_void) -> usize = rust.symbol(b"ZSTD_freeCStream\0");
        let call_c: unsafe extern "C" fn(
            *mut c_void,
            *mut OutBuffer,
            *mut InBuffer,
            c_int,
        ) -> usize = c.symbol(b"ZSTD_compressStream2\0");
        let call_r: unsafe extern "C" fn(
            *mut c_void,
            *mut OutBuffer,
            *mut InBuffer,
            c_int,
        ) -> usize = rust.symbol(b"ZSTD_compressStream2\0");

        for (input_size, input_pos, output_size, output_pos, directive) in [
            (0, 1, 8, 0, 0),
            (8, 9, 8, 0, 0),
            (0, 0, 0, 1, 0),
            (0, 0, 8, 9, 0),
            (0, 0, 8, 0, -1),
            (0, 0, 8, 0, 3),
            (0, 0, 8, 0, c_int::MAX),
        ] {
            let cc = create_c();
            let rc = create_r();
            let input = [0u8; 8];
            let mut c_output = [0u8; 8];
            let mut r_output = [0u8; 8];
            let mut ci = InBuffer {
                src: input.as_ptr().cast(),
                size: input_size,
                pos: input_pos,
            };
            let mut ri = ci;
            let mut co = OutBuffer {
                dst: c_output.as_mut_ptr().cast(),
                size: output_size,
                pos: output_pos,
            };
            let mut ro = OutBuffer {
                dst: r_output.as_mut_ptr().cast(),
                size: output_size,
                pos: output_pos,
            };
            let left = call_c(cc, &mut co, &mut ci, directive);
            let right = call_r(rc, &mut ro, &mut ri, directive);
            assert_eq!(left, right);
            assert_eq!(
                (ci.size, ci.pos, co.size, co.pos),
                (ri.size, ri.pos, ro.size, ro.pos)
            );
            assert_eq!(free_c(cc), free_r(rc));
        }
    }
}

unsafe fn zbuff_round_trip(api: &Api, input: &[u8]) -> (Vec<usize>, Vec<u8>, Vec<usize>, Vec<u8>) {
    let create_c: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZBUFF_createCCtx\0");
    let free_c: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZBUFF_freeCCtx\0");
    let init_c: unsafe extern "C" fn(*mut c_void, c_int) -> usize =
        api.symbol(b"ZBUFF_compressInit\0");
    let continue_c: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
    ) -> usize = api.symbol(b"ZBUFF_compressContinue\0");
    let end_c: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut usize) -> usize =
        api.symbol(b"ZBUFF_compressEnd\0");
    let create_d: unsafe extern "C" fn() -> *mut c_void = api.symbol(b"ZBUFF_createDCtx\0");
    let free_d: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZBUFF_freeDCtx\0");
    let init_d: unsafe extern "C" fn(*mut c_void) -> usize = api.symbol(b"ZBUFF_decompressInit\0");
    let continue_d: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
    ) -> usize = api.symbol(b"ZBUFF_decompressContinue\0");

    let cctx = create_c();
    assert!(!cctx.is_null());
    let mut compression_trace = vec![init_c(cctx, 3)];
    let mut frame = vec![0; input.len() + input.len() / 8 + 1024];
    let mut source_size = input.len();
    let mut destination_size = frame.len();
    compression_trace.push(continue_c(
        cctx,
        frame.as_mut_ptr().cast(),
        &mut destination_size,
        input.as_ptr().cast(),
        &mut source_size,
    ));
    assert_eq!(source_size, input.len());
    let mut written = destination_size;
    loop {
        let mut available = frame.len() - written;
        let remaining = end_c(cctx, frame[written..].as_mut_ptr().cast(), &mut available);
        compression_trace.push(remaining);
        written += available;
        if remaining == 0 {
            break;
        }
        assert!(!api.is_error(remaining));
    }
    frame.truncate(written);
    assert_eq!(free_c(cctx), 0);

    let dctx = create_d();
    assert!(!dctx.is_null());
    let mut decompression_trace = vec![init_d(dctx)];
    let mut output = vec![0; input.len().max(1)];
    let mut input_offset = 0;
    let mut output_offset = 0;
    loop {
        let mut source_size = frame.len() - input_offset;
        let mut destination_size = input.len() - output_offset;
        let remaining = continue_d(
            dctx,
            output[output_offset..].as_mut_ptr().cast(),
            &mut destination_size,
            frame[input_offset..].as_ptr().cast(),
            &mut source_size,
        );
        decompression_trace.push(remaining);
        input_offset += source_size;
        output_offset += destination_size;
        if remaining == 0 {
            break;
        }
        assert!(!api.is_error(remaining));
        assert!(source_size != 0 || destination_size != 0);
    }
    output.truncate(output_offset);
    assert_eq!(free_d(dctx), 0);
    (compression_trace, frame, decompression_trace, output)
}

#[test]
fn deprecated_buffered_streaming_matches() {
    unsafe {
        let (c, rust) = libraries();
        for name in [
            b"ZBUFF_recommendedCInSize\0".as_slice(),
            b"ZBUFF_recommendedCOutSize\0",
            b"ZBUFF_recommendedDInSize\0",
            b"ZBUFF_recommendedDOutSize\0",
        ] {
            let left: unsafe extern "C" fn() -> usize = c.symbol(name);
            let right: unsafe extern "C" fn() -> usize = rust.symbol(name);
            assert_eq!(left(), right());
        }
        let mut seed = 0x2468_ace0_1357_9bdf;
        for size in [0, 1, 127, 4096, 65_537] {
            let input = random_bytes(&mut seed, size);
            let left = zbuff_round_trip(&c, &input);
            let right = zbuff_round_trip(&rust, &input);
            assert_eq!(left, right, "size={size}");
            assert_eq!(left.3, input);
        }
    }
}

#[test]
fn frame_query_boundaries_match() {
    unsafe {
        let (c, rust) = libraries();
        let mut seed = 0xdead_beef_1029_3847;
        let content_size_c: unsafe extern "C" fn(*const c_void, usize) -> u64 =
            c.symbol(b"ZSTD_getFrameContentSize\0");
        let content_size_r: unsafe extern "C" fn(*const c_void, usize) -> u64 =
            rust.symbol(b"ZSTD_getFrameContentSize\0");
        let decompressed_size_c: unsafe extern "C" fn(*const c_void, usize) -> u64 =
            c.symbol(b"ZSTD_findDecompressedSize\0");
        let decompressed_size_r: unsafe extern "C" fn(*const c_void, usize) -> u64 =
            rust.symbol(b"ZSTD_findDecompressedSize\0");
        let compressed_size_c: unsafe extern "C" fn(*const c_void, usize) -> usize =
            c.symbol(b"ZSTD_findFrameCompressedSize\0");
        let compressed_size_r: unsafe extern "C" fn(*const c_void, usize) -> usize =
            rust.symbol(b"ZSTD_findFrameCompressedSize\0");
        let header_size_c: unsafe extern "C" fn(*const c_void, usize) -> usize =
            c.symbol(b"ZSTD_frameHeaderSize\0");
        let header_size_r: unsafe extern "C" fn(*const c_void, usize) -> usize =
            rust.symbol(b"ZSTD_frameHeaderSize\0");
        let is_frame_c: unsafe extern "C" fn(*const c_void, usize) -> c_uint =
            c.symbol(b"ZSTD_isFrame\0");
        let is_frame_r: unsafe extern "C" fn(*const c_void, usize) -> c_uint =
            rust.symbol(b"ZSTD_isFrame\0");

        for size in [0, 1, 255, 4096, 131_073] {
            let input = random_bytes(&mut seed, size);
            let bound: unsafe extern "C" fn(usize) -> usize = c.symbol(b"ZSTD_compressBound\0");
            let (_, frame) = compress(&c, &input, 3, bound(size));
            let mut prefixes: Vec<usize> = (0..=frame.len().min(32)).collect();
            prefixes.extend([frame.len() / 2, frame.len().saturating_sub(1), frame.len()]);
            prefixes.sort_unstable();
            prefixes.dedup();
            for prefix in prefixes {
                let pointer = frame.as_ptr().cast();
                assert_eq!(
                    content_size_c(pointer, prefix),
                    content_size_r(pointer, prefix),
                    "content size={size}, prefix={prefix}"
                );
                assert_eq!(
                    decompressed_size_c(pointer, prefix),
                    decompressed_size_r(pointer, prefix)
                );
                assert_eq!(
                    compressed_size_c(pointer, prefix),
                    compressed_size_r(pointer, prefix)
                );
                assert_eq!(
                    header_size_c(pointer, prefix),
                    header_size_r(pointer, prefix)
                );
                assert_eq!(is_frame_c(pointer, prefix), is_frame_r(pointer, prefix));
            }
        }
    }
}

#[test]
fn skippable_frames_and_frame_queries_match() {
    unsafe {
        let (c, rust) = libraries();
        type Write =
            unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_uint) -> usize;
        type Read =
            unsafe extern "C" fn(*mut c_void, usize, *mut c_uint, *const c_void, usize) -> usize;
        let write_c: Write = c.symbol(b"ZSTD_writeSkippableFrame\0");
        let write_r: Write = rust.symbol(b"ZSTD_writeSkippableFrame\0");
        let read_c: Read = c.symbol(b"ZSTD_readSkippableFrame\0");
        let read_r: Read = rust.symbol(b"ZSTD_readSkippableFrame\0");
        let mut seed = 0x1020_3040_5060_7080;
        for payload_size in [0, 1, 8, 255, 4096] {
            let payload = random_bytes(&mut seed, payload_size);
            for variant in [0, 1, 7, 15, 16, c_uint::MAX] {
                for capacity in [0, 7, 8, payload_size + 7, payload_size + 8] {
                    let mut left = vec![0; capacity.max(1)];
                    let mut right = vec![0; capacity.max(1)];
                    let lc = write_c(
                        left.as_mut_ptr().cast(),
                        capacity,
                        payload.as_ptr().cast(),
                        payload.len(),
                        variant,
                    );
                    let rc = write_r(
                        right.as_mut_ptr().cast(),
                        capacity,
                        payload.as_ptr().cast(),
                        payload.len(),
                        variant,
                    );
                    assert_eq!(lc, rc);
                    if !c.is_error(lc) {
                        assert_eq!(&left[..lc], &right[..rc]);
                        let mut lout = vec![0; payload_size.max(1)];
                        let mut rout = vec![0; payload_size.max(1)];
                        let mut lv = 99;
                        let mut rv = 99;
                        let lr = read_c(
                            lout.as_mut_ptr().cast(),
                            payload_size,
                            &mut lv,
                            left.as_ptr().cast(),
                            lc,
                        );
                        let rr = read_r(
                            rout.as_mut_ptr().cast(),
                            payload_size,
                            &mut rv,
                            right.as_ptr().cast(),
                            rc,
                        );
                        assert_eq!((lr, lv, lout), (rr, rv, rout));
                    }
                }
            }
        }
    }
}
