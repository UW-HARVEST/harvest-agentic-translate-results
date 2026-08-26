mod common;

use common::*;
use std::ffi::{c_int, c_uint, c_void};
use std::ptr;

#[test]
fn core_range_initialization_and_malformed_block_errors_match_exactly() {
    unsafe {
        let libs = Libraries::load();
        let (bound_c, bound_r) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        for value in [-1, LZ4_MAX_INPUT_SIZE + 1, c_int::MAX, c_int::MIN] {
            assert_eq!(bound_c(value), bound_r(value));
            assert_eq!(bound_c(value), 0);
        }

        let (compress_c, compress_r) = libs.pair::<Compress>(b"LZ4_compress_default\0");
        let mut c_output = [0u8; 32];
        let mut r_output = [0u8; 32];
        let source = [0u8; 1];
        for size in [-1, LZ4_MAX_INPUT_SIZE + 1] {
            assert_eq!(
                compress_c(
                    source.as_ptr().cast(),
                    c_output.as_mut_ptr().cast(),
                    size,
                    c_output.len() as c_int,
                ),
                compress_r(
                    source.as_ptr().cast(),
                    r_output.as_mut_ptr().cast(),
                    size,
                    r_output.len() as c_int,
                )
            );
        }
        assert_eq!(
            compress_c(source.as_ptr().cast(), c_output.as_mut_ptr().cast(), 0, 0),
            compress_r(source.as_ptr().cast(), r_output.as_mut_ptr().cast(), 0, 0)
        );
        let input = patterned(4096);
        assert_eq!(
            compress_c(
                input.as_ptr().cast(),
                c_output.as_mut_ptr().cast(),
                input.len() as c_int,
                1,
            ),
            compress_r(
                input.as_ptr().cast(),
                r_output.as_mut_ptr().cast(),
                input.len() as c_int,
                1,
            )
        );

        type Init = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
        let (size_c, size_r) = libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofState\0");
        let (init_c, init_r) = libs.pair::<Init>(b"LZ4_initStream\0");
        assert_eq!(size_c(), size_r());
        assert_eq!(init_c(ptr::null_mut(), size_c() as usize), ptr::null_mut());
        assert_eq!(init_r(ptr::null_mut(), size_r() as usize), ptr::null_mut());
        let mut c_state = vec![0u64; size_c() as usize / 8 + 2];
        let mut r_state = vec![0u64; size_r() as usize / 8 + 2];
        assert_eq!(
            init_c(c_state.as_mut_ptr().cast(), size_c() as usize - 1).is_null(),
            init_r(r_state.as_mut_ptr().cast(), size_r() as usize - 1).is_null()
        );
        assert_eq!(
            init_c(
                c_state.as_mut_ptr().cast::<u8>().add(1).cast(),
                size_c() as usize
            )
            .is_null(),
            init_r(
                r_state.as_mut_ptr().cast::<u8>().add(1).cast(),
                size_r() as usize
            )
            .is_null()
        );

        let (hc_size_c, hc_size_r) =
            libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofStateHC\0");
        let (hc_init_c, hc_init_r) = libs.pair::<Init>(b"LZ4_initStreamHC\0");
        assert_eq!(hc_size_c(), hc_size_r());
        assert_eq!(
            hc_init_c(ptr::null_mut(), hc_size_c() as usize).is_null(),
            hc_init_r(ptr::null_mut(), hc_size_r() as usize).is_null()
        );
        let mut c_hc = vec![0u64; hc_size_c() as usize / 8 + 2];
        let mut r_hc = vec![0u64; hc_size_r() as usize / 8 + 2];
        assert_eq!(
            hc_init_c(c_hc.as_mut_ptr().cast(), hc_size_c() as usize - 1).is_null(),
            hc_init_r(r_hc.as_mut_ptr().cast(), hc_size_r() as usize - 1).is_null()
        );

        let (ring_c, ring_r) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_decoderRingBufferSize\0");
        for value in [-1, LZ4_MAX_INPUT_SIZE + 1, c_int::MAX] {
            assert_eq!(ring_c(value), ring_r(value));
            assert_eq!(ring_c(value), 0);
        }

        let (decompress_c, decompress_r) = libs.pair::<Decompress>(b"LZ4_decompress_safe\0");
        for (bytes, compressed_size, capacity) in [
            (vec![0u8], 0, 16),
            (vec![0xff], 1, 16),
            (vec![0x10, 0], 2, 0),
            (vec![0, 0, 0], 3, 32),
            (vec![0xf0, 255, 255, 255], 4, 8),
        ] {
            let mut c_dst = vec![0u8; capacity.max(1) as usize];
            let mut r_dst = vec![0u8; capacity.max(1) as usize];
            let c_result = decompress_c(
                bytes.as_ptr().cast(),
                c_dst.as_mut_ptr().cast(),
                compressed_size,
                capacity,
            );
            let r_result = decompress_r(
                bytes.as_ptr().cast(),
                r_dst.as_mut_ptr().cast(),
                compressed_size,
                capacity,
            );
            assert_eq!(c_result, r_result);
        }
        assert_eq!(
            decompress_c(ptr::null(), c_output.as_mut_ptr().cast(), 1, 16),
            decompress_r(ptr::null(), r_output.as_mut_ptr().cast(), 1, 16)
        );
        assert_eq!(
            decompress_c(source.as_ptr().cast(), c_output.as_mut_ptr().cast(), 1, -1),
            decompress_r(source.as_ptr().cast(), r_output.as_mut_ptr().cast(), 1, -1)
        );
    }
}

#[test]
fn xxhash_null_update_and_frame_scalar_errors_match_exactly() {
    unsafe {
        let libs = Libraries::load();
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Reset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
        type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        for width in [32, 64] {
            let create_name = if width == 32 {
                b"LZ4_XXH32_createState\0".as_slice()
            } else {
                b"LZ4_XXH64_createState\0".as_slice()
            };
            let free_name = if width == 32 {
                b"LZ4_XXH32_freeState\0".as_slice()
            } else {
                b"LZ4_XXH64_freeState\0".as_slice()
            };
            let update_name = if width == 32 {
                b"LZ4_XXH32_update\0".as_slice()
            } else {
                b"LZ4_XXH64_update\0".as_slice()
            };
            let (create_c, create_r) = libs.pair::<Create>(create_name);
            let (free_c, free_r) = libs.pair::<Free>(free_name);
            let (update_c, update_r) = libs.pair::<Update>(update_name);
            let c_state = create_c();
            let r_state = create_r();
            if width == 32 {
                let (reset_c, reset_r) = libs.pair::<Reset32>(b"LZ4_XXH32_reset\0");
                assert_eq!(reset_c(c_state, 0), reset_r(r_state, 0));
            } else {
                let (reset_c, reset_r) = libs
                    .pair::<unsafe extern "C" fn(*mut c_void, u64) -> c_int>(b"LZ4_XXH64_reset\0");
                assert_eq!(reset_c(c_state, 0), reset_r(r_state, 0));
            }
            for length in [0, 1, usize::MAX] {
                assert_eq!(
                    update_c(c_state, ptr::null(), length),
                    update_r(r_state, ptr::null(), length)
                );
                assert_eq!(update_c(c_state, ptr::null(), length), 1);
            }
            assert_eq!(free_c(c_state), free_r(r_state));
        }

        let (block_c, block_r) =
            libs.pair::<unsafe extern "C" fn(c_int) -> usize>(b"LZ4F_getBlockSize\0");
        for value in [c_int::MIN, -1, 1, 2, 3, 8, c_int::MAX] {
            assert_eq!(block_c(value), block_r(value));
            assert!(is_frame_error(block_c(value)));
        }

        type CreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        let (create_c_c, create_c_r) = libs.pair::<CreateCtx>(b"LZ4F_createCompressionContext\0");
        let (create_d_c, create_d_r) = libs.pair::<CreateCtx>(b"LZ4F_createDecompressionContext\0");
        assert_eq!(
            create_c_c(ptr::null_mut(), LZ4F_VERSION),
            create_c_r(ptr::null_mut(), LZ4F_VERSION)
        );
        assert_eq!(
            create_d_c(ptr::null_mut(), LZ4F_VERSION),
            create_d_r(ptr::null_mut(), LZ4F_VERSION)
        );
        assert_eq!(create_c_c(ptr::null_mut(), LZ4F_VERSION), frame_error(21));
        assert_eq!(create_d_c(ptr::null_mut(), LZ4F_VERSION), frame_error(21));
    }
}

#[test]
fn frame_header_state_checksum_and_allocator_errors_match_exactly() {
    unsafe {
        let libs = Libraries::load();
        let input = patterned(80_000);
        let prefs = Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 0,
                content_checksum_flag: 1,
                content_size: input.len() as u64,
                block_checksum_flag: 1,
                ..FrameInfo::default()
            },
            ..Preferences::default()
        };
        let (c_frame, r_frame) = compress_frame_pair(&libs, &input, &prefs);
        assert_eq!(c_frame, r_frame);

        type Header = unsafe extern "C" fn(*const c_void, usize) -> usize;
        let (header_c, header_r) = libs.pair::<Header>(b"LZ4F_headerSize\0");
        assert_eq!(header_c(ptr::null(), 5), header_r(ptr::null(), 5));
        assert_eq!(header_c(ptr::null(), 5), frame_error(15));
        for size in 0..5 {
            assert_eq!(
                header_c(c_frame.as_ptr().cast(), size),
                header_r(r_frame.as_ptr().cast(), size)
            );
            assert_eq!(header_c(c_frame.as_ptr().cast(), size), frame_error(12));
        }
        let unknown = [0u8; 5];
        assert_eq!(
            header_c(unknown.as_ptr().cast(), unknown.len()),
            header_r(unknown.as_ptr().cast(), unknown.len())
        );
        assert_eq!(
            header_c(unknown.as_ptr().cast(), unknown.len()),
            frame_error(13)
        );

        for mutation in [
            HeaderMutation::FlgReserved,
            HeaderMutation::Version,
            HeaderMutation::BdReservedHigh,
            HeaderMutation::BdReservedLow,
            HeaderMutation::BlockSize,
            HeaderMutation::Checksum,
        ] {
            let c_mutated = mutate_header(c_frame.clone(), mutation);
            let r_mutated = mutate_header(r_frame.clone(), mutation);
            let c_result = decode_frame(&libs.c, &c_mutated, input.len());
            let r_result = decode_frame(&libs.rust, &r_mutated, input.len());
            assert_eq!(c_result, r_result, "{mutation:?}");
            assert!(is_frame_error(c_result), "{mutation:?}: {c_result}");
        }

        let mut c_block_crc = c_frame.clone();
        let mut r_block_crc = r_frame.clone();
        corrupt_block_checksum(&mut c_block_crc);
        corrupt_block_checksum(&mut r_block_crc);
        assert_eq!(
            decode_frame(&libs.c, &c_block_crc, input.len()),
            decode_frame(&libs.rust, &r_block_crc, input.len())
        );
        assert_eq!(
            decode_frame(&libs.c, &c_block_crc, input.len()),
            frame_error(7)
        );

        let mut c_content_crc = c_frame.clone();
        let mut r_content_crc = r_frame.clone();
        *c_content_crc.last_mut().unwrap() ^= 1;
        *r_content_crc.last_mut().unwrap() ^= 1;
        assert_eq!(
            decode_frame(&libs.c, &c_content_crc, input.len()),
            decode_frame(&libs.rust, &r_content_crc, input.len())
        );
        assert_eq!(
            decode_frame(&libs.c, &c_content_crc, input.len()),
            frame_error(18)
        );

        exercise_frame_state_errors(&libs, &input, &prefs);
        exercise_failing_custom_allocators(&libs);
    }
}

#[derive(Clone, Copy, Debug)]
enum HeaderMutation {
    FlgReserved,
    Version,
    BdReservedHigh,
    BdReservedLow,
    BlockSize,
    Checksum,
}

fn mutate_header(mut frame: Vec<u8>, mutation: HeaderMutation) -> Vec<u8> {
    match mutation {
        HeaderMutation::FlgReserved => frame[4] |= 0x02,
        HeaderMutation::Version => frame[4] &= 0x3f,
        HeaderMutation::BdReservedHigh => frame[5] |= 0x80,
        HeaderMutation::BdReservedLow => frame[5] |= 0x01,
        HeaderMutation::BlockSize => frame[5] &= 0x0f,
        HeaderMutation::Checksum => {
            let header_size = frame_header_size(&frame);
            frame[header_size - 1] ^= 1;
        }
    }
    frame
}

fn frame_header_size(frame: &[u8]) -> usize {
    7 + if frame[4] & 0x08 != 0 { 8 } else { 0 } + if frame[4] & 1 != 0 { 4 } else { 0 }
}

fn corrupt_block_checksum(frame: &mut [u8]) {
    let header_size = frame_header_size(frame);
    let block_header = u32::from_le_bytes(frame[header_size..header_size + 4].try_into().unwrap());
    let block_size = (block_header & 0x7fff_ffff) as usize;
    frame[header_size + 4 + block_size] ^= 1;
}

unsafe fn compress_frame_pair(
    libs: &Libraries,
    input: &[u8],
    prefs: &Preferences,
) -> (Vec<u8>, Vec<u8>) {
    type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
    type Compress =
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
    let (bound_c, bound_r) = unsafe { libs.pair::<Bound>(b"LZ4F_compressFrameBound\0") };
    let (compress_c, compress_r) = unsafe { libs.pair::<Compress>(b"LZ4F_compressFrame\0") };
    let c_bound = unsafe { bound_c(input.len(), prefs) };
    let r_bound = unsafe { bound_r(input.len(), prefs) };
    assert_eq!(c_bound, r_bound);
    let mut c_frame = vec![0u8; c_bound];
    let mut r_frame = vec![0u8; r_bound];
    let c_size = unsafe {
        compress_c(
            c_frame.as_mut_ptr().cast(),
            c_bound,
            input.as_ptr().cast(),
            input.len(),
            prefs,
        )
    };
    let r_size = unsafe {
        compress_r(
            r_frame.as_mut_ptr().cast(),
            r_bound,
            input.as_ptr().cast(),
            input.len(),
            prefs,
        )
    };
    assert_eq!(c_size, r_size);
    c_frame.truncate(c_size);
    r_frame.truncate(r_size);
    (c_frame, r_frame)
}

unsafe fn decode_frame(library: &libloading::Library, frame: &[u8], output_size: usize) -> usize {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Decompress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
        *const DecompressOptions,
    ) -> usize;
    let create = unsafe {
        library
            .get::<Create>(b"LZ4F_createDecompressionContext\0")
            .unwrap()
    };
    let free = unsafe {
        library
            .get::<Free>(b"LZ4F_freeDecompressionContext\0")
            .unwrap()
    };
    let decompress = unsafe { library.get::<Decompress>(b"LZ4F_decompress\0").unwrap() };
    let mut context = ptr::null_mut();
    assert_eq!(unsafe { create(&mut context, LZ4F_VERSION) }, 0);
    let mut output = vec![0u8; output_size.max(1)];
    let mut dst_size = output_size;
    let mut src_size = frame.len();
    let result = unsafe {
        decompress(
            context,
            output.as_mut_ptr().cast(),
            &mut dst_size,
            frame.as_ptr().cast(),
            &mut src_size,
            ptr::null(),
        )
    };
    unsafe { free(context) };
    result
}

unsafe fn exercise_frame_state_errors(libs: &Libraries, input: &[u8], prefs: &Preferences) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
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
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createCompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeCompressionContext\0") };
    let (begin_c, begin_r) = unsafe { libs.pair::<Begin>(b"LZ4F_compressBegin\0") };
    let (update_c, update_r) = unsafe { libs.pair::<Update>(b"LZ4F_compressUpdate\0") };
    let (raw_c, raw_r) = unsafe { libs.pair::<Update>(b"LZ4F_uncompressedUpdate\0") };
    let (end_c, end_r) = unsafe { libs.pair::<End>(b"LZ4F_compressEnd\0") };
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_ctx, LZ4F_VERSION)
    });
    let mut c_out = vec![0u8; 200_000];
    let mut r_out = vec![0u8; 200_000];
    assert_eq!(
        unsafe {
            update_c(
                c_ctx,
                c_out.as_mut_ptr().cast(),
                c_out.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        },
        unsafe {
            update_r(
                r_ctx,
                r_out.as_mut_ptr().cast(),
                r_out.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        }
    );
    assert_eq!(
        unsafe {
            raw_c(
                c_ctx,
                c_out.as_mut_ptr().cast(),
                c_out.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        },
        unsafe {
            raw_r(
                r_ctx,
                r_out.as_mut_ptr().cast(),
                r_out.len(),
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        }
    );
    assert_eq!(
        unsafe { begin_c(c_ctx, c_out.as_mut_ptr().cast(), 18, prefs) },
        unsafe { begin_r(r_ctx, r_out.as_mut_ptr().cast(), 18, prefs) }
    );
    let c_header = unsafe { begin_c(c_ctx, c_out.as_mut_ptr().cast(), 19, prefs) };
    let r_header = unsafe { begin_r(r_ctx, r_out.as_mut_ptr().cast(), 19, prefs) };
    assert_eq!(c_header, r_header);
    assert_eq!(
        unsafe {
            update_c(
                c_ctx,
                c_out.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        },
        unsafe {
            update_r(
                r_ctx,
                r_out.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                ptr::null(),
            )
        }
    );
    assert_eq!(
        unsafe { end_c(c_ctx, c_out.as_mut_ptr().cast(), 3, ptr::null()) },
        unsafe { end_r(r_ctx, r_out.as_mut_ptr().cast(), 3, ptr::null()) }
    );
    assert_eq!(unsafe { free_c(c_ctx) }, unsafe { free_r(r_ctx) });

    type CompressFrame =
        unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, *const Preferences) -> usize;
    let (frame_c, frame_r) = unsafe { libs.pair::<CompressFrame>(b"LZ4F_compressFrame\0") };
    assert_eq!(
        unsafe {
            frame_c(
                c_out.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                prefs,
            )
        },
        unsafe {
            frame_r(
                r_out.as_mut_ptr().cast(),
                1,
                input.as_ptr().cast(),
                input.len(),
                prefs,
            )
        }
    );
}

unsafe extern "C" fn fail_alloc(_: *mut c_void, _: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn fail_free(_: *mut c_void, _: *mut c_void) {}

unsafe fn exercise_failing_custom_allocators(libs: &Libraries) {
    let memory = CustomMem {
        custom_alloc: fail_alloc as *const () as *mut c_void,
        custom_calloc: fail_alloc as *const () as *mut c_void,
        custom_free: fail_free as *const () as *mut c_void,
        opaque_state: ptr::null_mut(),
    };
    type Create = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
    type Dict = unsafe extern "C" fn(CustomMem, *const c_void, usize) -> *mut c_void;
    for name in [
        b"LZ4F_createCompressionContext_advanced\0".as_slice(),
        b"LZ4F_createDecompressionContext_advanced\0".as_slice(),
    ] {
        let (c, r) = unsafe { libs.pair::<Create>(name) };
        let c_result = unsafe { c(memory, LZ4F_VERSION) };
        let r_result = unsafe { r(memory, LZ4F_VERSION) };
        assert_eq!(c_result.is_null(), r_result.is_null());
        assert!(c_result.is_null());
    }
    let (dict_c, dict_r) = unsafe { libs.pair::<Dict>(b"LZ4F_createCDict_advanced\0") };
    let byte = 0u8;
    assert_eq!(
        unsafe { dict_c(memory, (&byte as *const u8).cast(), 1) }.is_null(),
        unsafe { dict_r(memory, (&byte as *const u8).cast(), 1) }.is_null()
    );
}

#[test]
fn file_api_null_short_read_invalid_enum_and_write_failures_match() {
    unsafe {
        let libs = Libraries::load();
        type OpenRead = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
        type Read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
        type Close = unsafe extern "C" fn(*mut c_void) -> usize;
        type OpenWrite =
            unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
        type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
        let (open_read_c, open_read_r) = libs.pair::<OpenRead>(b"LZ4F_readOpen\0");
        let (read_c, read_r) = libs.pair::<Read>(b"LZ4F_read\0");
        let (close_read_c, close_read_r) = libs.pair::<Close>(b"LZ4F_readClose\0");
        let (open_write_c, open_write_r) = libs.pair::<OpenWrite>(b"LZ4F_writeOpen\0");
        let (write_c, write_r) = libs.pair::<Write>(b"LZ4F_write\0");
        let (close_write_c, close_write_r) = libs.pair::<Close>(b"LZ4F_writeClose\0");
        assert_eq!(
            open_read_c(ptr::null_mut(), ptr::null_mut()),
            open_read_r(ptr::null_mut(), ptr::null_mut())
        );
        assert_eq!(
            read_c(ptr::null_mut(), ptr::null_mut(), 0),
            read_r(ptr::null_mut(), ptr::null_mut(), 0)
        );
        assert_eq!(close_read_c(ptr::null_mut()), close_read_r(ptr::null_mut()));
        assert_eq!(
            open_write_c(ptr::null_mut(), ptr::null_mut(), ptr::null()),
            open_write_r(ptr::null_mut(), ptr::null_mut(), ptr::null())
        );
        assert_eq!(
            write_c(ptr::null_mut(), ptr::null(), 0),
            write_r(ptr::null_mut(), ptr::null(), 0)
        );
        assert_eq!(
            close_write_c(ptr::null_mut()),
            close_write_r(ptr::null_mut())
        );

        let c_file = tmpfile();
        let r_file = tmpfile();
        let mut c_reader = ptr::null_mut();
        let mut r_reader = ptr::null_mut();
        let c_result = open_read_c(&mut c_reader, c_file);
        let r_result = open_read_r(&mut r_reader, r_file);
        assert_eq!(c_result, r_result);
        assert_eq!(c_result, frame_error(23));
        fclose(c_file);
        fclose(r_file);

        let bad_prefs = Preferences {
            frame_info: FrameInfo {
                block_size_id: c_int::MAX,
                ..FrameInfo::default()
            },
            ..Preferences::default()
        };
        let c_file = tmpfile();
        let r_file = tmpfile();
        let mut c_writer = ptr::null_mut();
        let mut r_writer = ptr::null_mut();
        assert_eq!(
            open_write_c(&mut c_writer, c_file, &bad_prefs),
            open_write_r(&mut r_writer, r_file, &bad_prefs)
        );
        fclose(c_file);
        fclose(r_file);
    }
}

unsafe extern "C" {
    fn tmpfile() -> *mut c_void;
    fn fclose(file: *mut c_void) -> c_int;
}

fn frame_error(code: usize) -> usize {
    0usize.wrapping_sub(code)
}

fn is_frame_error(code: usize) -> bool {
    code > 0usize.wrapping_sub(24)
}
