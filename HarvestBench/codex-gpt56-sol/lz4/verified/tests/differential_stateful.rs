mod common;

use common::*;
use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::ptr;

#[test]
fn external_state_destination_size_and_streaming_dictionary_paths_match() {
    unsafe {
        let libs = Libraries::load();
        exercise_external_states(&libs);
        exercise_core_streams(&libs);
        exercise_hc_streams(&libs);
    }
}

unsafe fn exercise_external_states(libs: &Libraries) {
    type Size = unsafe extern "C" fn() -> c_int;
    type Ext =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    type Dest = unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
        *mut c_char,
        *mut c_int,
        c_int,
        c_int,
    ) -> c_int;
    let (size_c, size_r) = unsafe { libs.pair::<Size>(b"LZ4_sizeofState\0") };
    let state_size = unsafe { size_c() };
    assert_eq!(state_size, unsafe { size_r() });
    let (ext_c, ext_r) = unsafe { libs.pair::<Ext>(b"LZ4_compress_fast_extState\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Ext>(b"LZ4_compress_fast_extState_fastReset\0") };
    let (dest_c, dest_r) = unsafe { libs.pair::<Dest>(b"LZ4_compress_destSize_extState\0") };
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for size in [0usize, 1, 15, 16, 4096, 65535, 65536, 90000] {
        let generated = rng.bytes(size);
        let storage = if generated.is_empty() {
            vec![0]
        } else {
            generated
        };
        let input = &storage[..size];
        let bound = size + size / 255 + 16;
        let words = state_size as usize / 8 + 2;
        let mut c_state = vec![0u64; words];
        let mut r_state = vec![0u64; words];
        for (index, function) in [(&ext_c, &ext_r), (&reset_c, &reset_r)]
            .into_iter()
            .enumerate()
        {
            let mut c_out = vec![0u8; bound];
            let mut r_out = vec![0u8; bound];
            let c_size = unsafe {
                function.0(
                    c_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound as c_int,
                    3,
                )
            };
            let r_size = unsafe {
                function.1(
                    r_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    size as c_int,
                    bound as c_int,
                    3,
                )
            };
            assert_eq!(c_size, r_size, "external state variant {index}");
            assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        }
        for target in [1usize, 8, (bound / 2).max(1), bound] {
            let mut c_source_size = size as c_int;
            let mut r_source_size = size as c_int;
            let mut c_out = vec![0u8; target];
            let mut r_out = vec![0u8; target];
            let c_size = unsafe {
                dest_c(
                    c_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    c_out.as_mut_ptr().cast(),
                    &mut c_source_size,
                    target as c_int,
                    5,
                )
            };
            let r_size = unsafe {
                dest_r(
                    r_state.as_mut_ptr().cast(),
                    input.as_ptr().cast(),
                    r_out.as_mut_ptr().cast(),
                    &mut r_source_size,
                    target as c_int,
                    5,
                )
            };
            assert_eq!((c_size, c_source_size), (r_size, r_source_size));
            assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        }
    }
}

unsafe fn exercise_core_streams(libs: &Libraries) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Attach = unsafe extern "C" fn(*mut c_void, *const c_void);
    type Continue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    type Save = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void);
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStream\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStream\0") };
    let (load_c, load_r) = unsafe { libs.pair::<Load>(b"LZ4_loadDict\0") };
    let (load_slow_c, load_slow_r) = unsafe { libs.pair::<Load>(b"LZ4_loadDictSlow\0") };
    let (attach_c, attach_r) = unsafe { libs.pair::<Attach>(b"LZ4_attach_dictionary\0") };
    let (continue_c, continue_r) =
        unsafe { libs.pair::<Continue>(b"LZ4_compress_fast_continue\0") };
    let (save_c, save_r) = unsafe { libs.pair::<Save>(b"LZ4_saveDict\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_resetStream_fast\0") };
    let dictionary = patterned(70_000);
    let dict_c = unsafe { create_c() };
    let dict_r = unsafe { create_r() };
    let stream_c = unsafe { create_c() };
    let stream_r = unsafe { create_r() };
    assert!(!dict_c.is_null() && !dict_r.is_null() && !stream_c.is_null() && !stream_r.is_null());
    assert_eq!(
        unsafe {
            load_c(
                dict_c,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        },
        unsafe {
            load_r(
                dict_r,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        }
    );
    unsafe {
        reset_c(stream_c);
        reset_r(stream_r);
        attach_c(stream_c, dict_c);
        attach_r(stream_r, dict_r);
    }
    let blocks = [patterned(8192), patterned(65536), patterned(1777)];
    for block in &blocks {
        let bound = block.len() + block.len() / 255 + 16;
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            continue_c(
                stream_c,
                block.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                block.len() as c_int,
                bound as c_int,
                1,
            )
        };
        let r_size = unsafe {
            continue_r(
                stream_r,
                block.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                block.len() as c_int,
                bound as c_int,
                1,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
    }
    let mut c_saved = vec![0u8; 65536];
    let mut r_saved = vec![0u8; 65536];
    let c_saved_size = unsafe { save_c(stream_c, c_saved.as_mut_ptr().cast(), 65536) };
    let r_saved_size = unsafe { save_r(stream_r, r_saved.as_mut_ptr().cast(), 65536) };
    assert_eq!(c_saved_size, r_saved_size);
    assert_eq!(
        &c_saved[..c_saved_size as usize],
        &r_saved[..r_saved_size as usize]
    );
    assert_eq!(
        unsafe {
            load_slow_c(
                stream_c,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        },
        unsafe {
            load_slow_r(
                stream_r,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        }
    );
    assert_eq!(unsafe { free_c(dict_c) }, unsafe { free_r(dict_r) });
    assert_eq!(unsafe { free_c(stream_c) }, unsafe { free_r(stream_r) });

    unsafe { exercise_decode_stream(libs) };
}

unsafe fn exercise_decode_stream(libs: &Libraries) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Set = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Continue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
    let (compress_c, compress_r) = unsafe { libs.pair::<Compress>(b"LZ4_compress_default\0") };
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStreamDecode\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStreamDecode\0") };
    let (set_c, set_r) = unsafe { libs.pair::<Set>(b"LZ4_setStreamDecode\0") };
    let (continue_c, continue_r) =
        unsafe { libs.pair::<Continue>(b"LZ4_decompress_safe_continue\0") };
    let state_c = unsafe { create_c() };
    let state_r = unsafe { create_r() };
    assert_eq!(unsafe { set_c(state_c, ptr::null(), 0) }, unsafe {
        set_r(state_r, ptr::null(), 0)
    });
    for input in [patterned(4096), patterned(70_000)] {
        let bound = input.len() + input.len() / 255 + 16;
        let (c_size, c_compressed) = unsafe { compress_with(&compress_c, &input, bound) };
        let (r_size, r_compressed) = unsafe { compress_with(&compress_r, &input, bound) };
        assert_eq!(c_size, r_size);
        let mut c_out = vec![0u8; input.len()];
        let mut r_out = vec![0u8; input.len()];
        let c_dec = unsafe {
            continue_c(
                state_c,
                c_compressed.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                c_size,
                input.len() as c_int,
            )
        };
        let r_dec = unsafe {
            continue_r(
                state_r,
                r_compressed.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                r_size,
                input.len() as c_int,
            )
        };
        assert_eq!(c_dec, r_dec);
        assert_eq!(&c_out[..c_dec as usize], &r_out[..r_dec as usize]);
    }
    assert_eq!(unsafe { free_c(state_c) }, unsafe { free_r(state_r) });
}

unsafe fn exercise_hc_streams(libs: &Libraries) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void, c_int);
    type SetLevel = unsafe extern "C" fn(*mut c_void, c_int);
    type Favor = unsafe extern "C" fn(*mut c_void, c_int);
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Attach = unsafe extern "C" fn(*mut c_void, *const c_void);
    type Continue =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
    type Save = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStreamHC\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStreamHC\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_resetStreamHC_fast\0") };
    let (level_c, level_r) = unsafe { libs.pair::<SetLevel>(b"LZ4_setCompressionLevel\0") };
    let (favor_c, favor_r) = unsafe { libs.pair::<Favor>(b"LZ4_favorDecompressionSpeed\0") };
    let (load_c, load_r) = unsafe { libs.pair::<Load>(b"LZ4_loadDictHC\0") };
    let (attach_c, attach_r) = unsafe { libs.pair::<Attach>(b"LZ4_attach_HC_dictionary\0") };
    let (continue_c, continue_r) = unsafe { libs.pair::<Continue>(b"LZ4_compress_HC_continue\0") };
    let (save_c, save_r) = unsafe { libs.pair::<Save>(b"LZ4_saveDictHC\0") };
    let dict_c = unsafe { create_c() };
    let dict_r = unsafe { create_r() };
    let stream_c = unsafe { create_c() };
    let stream_r = unsafe { create_r() };
    let dictionary = patterned(70_000);
    unsafe {
        reset_c(dict_c, 10);
        reset_r(dict_r, 10);
    }
    assert_eq!(
        unsafe {
            load_c(
                dict_c,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        },
        unsafe {
            load_r(
                dict_r,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        }
    );
    unsafe {
        reset_c(stream_c, 10);
        reset_r(stream_r, 10);
        level_c(stream_c, 11);
        level_r(stream_r, 11);
        favor_c(stream_c, 1);
        favor_r(stream_r, 1);
        attach_c(stream_c, dict_c);
        attach_r(stream_r, dict_r);
    }
    let blocks = [patterned(2048), patterned(70_000)];
    for input in &blocks {
        let bound = input.len() + input.len() / 255 + 16;
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            continue_c(
                stream_c,
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
            )
        };
        let r_size = unsafe {
            continue_r(
                stream_r,
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
    }
    let mut c_saved = vec![0u8; 65536];
    let mut r_saved = vec![0u8; 65536];
    let c_saved_size = unsafe { save_c(stream_c, c_saved.as_mut_ptr().cast(), 65536) };
    let r_saved_size = unsafe { save_r(stream_r, r_saved.as_mut_ptr().cast(), 65536) };
    assert_eq!(c_saved_size, r_saved_size);
    assert_eq!(
        &c_saved[..c_saved_size as usize],
        &r_saved[..r_saved_size as usize]
    );
    assert_eq!(unsafe { free_c(dict_c) }, unsafe { free_r(dict_r) });
    assert_eq!(unsafe { free_c(stream_c) }, unsafe { free_r(stream_r) });
}

#[test]
fn frame_one_shot_preferences_info_and_decompression_match() {
    unsafe {
        let libs = Libraries::load();
        type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
        type CompressFrame = unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const Preferences,
        ) -> usize;
        let (bound_c, bound_r) = libs.pair::<Bound>(b"LZ4F_compressFrameBound\0");
        let (compress_c, compress_r) = libs.pair::<CompressFrame>(b"LZ4F_compressFrame\0");
        let mut rng = Rng::new(0x91c4_c06d_5eed_1234);
        for iteration in 0..72 {
            let size = match iteration {
                0 => 0,
                1 => 1,
                2 => 65535,
                3 => 65536,
                _ => (rng.next_u64() as usize) % 180_000,
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
            let prefs = Preferences {
                frame_info: FrameInfo {
                    block_size_id: [0, 4, 5, 6, 7][iteration % 5],
                    block_mode: (iteration / 5 % 2) as c_int,
                    content_checksum_flag: (iteration / 10 % 2) as c_int,
                    frame_type: 0,
                    content_size: if iteration % 3 == 0 { size as u64 } else { 0 },
                    dict_id: if iteration % 4 == 0 { 0x1234_5678 } else { 0 },
                    block_checksum_flag: (iteration / 20 % 2) as c_int,
                },
                compression_level: [-3, 0, 1, 9, 10, 12, 13][iteration % 7],
                auto_flush: (iteration / 7 % 2) as c_uint,
                favor_dec_speed: (iteration / 14 % 2) as c_uint,
                reserved: [0; 3],
            };
            let c_bound = bound_c(size, &prefs);
            let r_bound = bound_r(size, &prefs);
            assert_eq!(c_bound, r_bound);
            let mut c_frame = vec![0u8; c_bound];
            let mut r_frame = vec![0u8; r_bound];
            let c_size = compress_c(
                c_frame.as_mut_ptr().cast(),
                c_bound,
                input.as_ptr().cast(),
                size,
                &prefs,
            );
            let r_size = compress_r(
                r_frame.as_mut_ptr().cast(),
                r_bound,
                input.as_ptr().cast(),
                size,
                &prefs,
            );
            assert_eq!(c_size, r_size);
            assert_eq!(&c_frame[..c_size], &r_frame[..r_size]);
            exercise_frame_info_and_decode(&libs, &c_frame[..c_size], &r_frame[..r_size], input);
        }
    }
}

unsafe fn exercise_frame_info_and_decode(
    libs: &Libraries,
    c_frame: &[u8],
    r_frame: &[u8],
    expected: &[u8],
) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Header = unsafe extern "C" fn(*const c_void, usize) -> usize;
    type Info =
        unsafe extern "C" fn(*mut c_void, *mut FrameInfo, *const c_void, *mut usize) -> usize;
    type Decompress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
        *const DecompressOptions,
    ) -> usize;
    let (header_c, header_r) = unsafe { libs.pair::<Header>(b"LZ4F_headerSize\0") };
    assert_eq!(
        unsafe { header_c(c_frame.as_ptr().cast(), c_frame.len()) },
        unsafe { header_r(r_frame.as_ptr().cast(), r_frame.len()) }
    );
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createDecompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeDecompressionContext\0") };
    let (info_c, info_r) = unsafe { libs.pair::<Info>(b"LZ4F_getFrameInfo\0") };
    let (decompress_c, decompress_r) = unsafe { libs.pair::<Decompress>(b"LZ4F_decompress\0") };
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_ctx, LZ4F_VERSION)
    });
    let mut c_info = FrameInfo::default();
    let mut r_info = FrameInfo::default();
    let mut c_header_size = c_frame.len();
    let mut r_header_size = r_frame.len();
    assert_eq!(
        unsafe {
            info_c(
                c_ctx,
                &mut c_info,
                c_frame.as_ptr().cast(),
                &mut c_header_size,
            )
        },
        unsafe {
            info_r(
                r_ctx,
                &mut r_info,
                r_frame.as_ptr().cast(),
                &mut r_header_size,
            )
        }
    );
    assert_eq!(c_info, r_info);
    assert_eq!(c_header_size, r_header_size);
    let mut c_output = vec![0u8; expected.len().max(1)];
    let mut r_output = vec![0u8; expected.len().max(1)];
    let mut c_dst_size = expected.len();
    let mut r_dst_size = expected.len();
    let mut c_src_size = c_frame.len() - c_header_size;
    let mut r_src_size = r_frame.len() - r_header_size;
    let options = DecompressOptions {
        stable_dst: 1,
        skip_checksums: 0,
        ..DecompressOptions::default()
    };
    let c_hint = unsafe {
        decompress_c(
            c_ctx,
            c_output.as_mut_ptr().cast(),
            &mut c_dst_size,
            c_frame[c_header_size..].as_ptr().cast(),
            &mut c_src_size,
            &options,
        )
    };
    let r_hint = unsafe {
        decompress_r(
            r_ctx,
            r_output.as_mut_ptr().cast(),
            &mut r_dst_size,
            r_frame[r_header_size..].as_ptr().cast(),
            &mut r_src_size,
            &options,
        )
    };
    assert_eq!(
        (c_hint, c_dst_size, c_src_size),
        (r_hint, r_dst_size, r_src_size)
    );
    assert_eq!(&c_output[..c_dst_size], &r_output[..r_dst_size]);
    assert_eq!(&c_output[..c_dst_size], expected);
    assert_eq!(unsafe { free_c(c_ctx) }, unsafe { free_r(r_ctx) });
}

#[test]
fn frame_streaming_dictionary_custom_memory_and_file_apis_match() {
    unsafe {
        let libs = Libraries::load();
        exercise_frame_streaming(&libs);
        exercise_frame_dictionary(&libs);
        exercise_custom_memory(&libs);
        exercise_file_api(&libs);
    }
}

unsafe fn exercise_frame_streaming(libs: &Libraries) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Begin = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const Preferences) -> usize;
    type Bound = unsafe extern "C" fn(usize, *const Preferences) -> usize;
    type Update = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const CompressOptions,
    ) -> usize;
    type Flush =
        unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const CompressOptions) -> usize;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createCompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeCompressionContext\0") };
    let (begin_c, begin_r) = unsafe { libs.pair::<Begin>(b"LZ4F_compressBegin\0") };
    let (bound_c, bound_r) = unsafe { libs.pair::<Bound>(b"LZ4F_compressBound\0") };
    let (update_c, update_r) = unsafe { libs.pair::<Update>(b"LZ4F_compressUpdate\0") };
    let (raw_c, raw_r) = unsafe { libs.pair::<Update>(b"LZ4F_uncompressedUpdate\0") };
    let (flush_c, flush_r) = unsafe { libs.pair::<Flush>(b"LZ4F_flush\0") };
    let (end_c, end_r) = unsafe { libs.pair::<Flush>(b"LZ4F_compressEnd\0") };
    let prefs = Preferences {
        frame_info: FrameInfo {
            block_size_id: 4,
            block_mode: 0,
            content_checksum_flag: 1,
            block_checksum_flag: 1,
            ..FrameInfo::default()
        },
        compression_level: 10,
        auto_flush: 0,
        favor_dec_speed: 1,
        reserved: [0; 3],
    };
    let options = CompressOptions {
        stable_src: 1,
        reserved: [0; 3],
    };
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_ctx, LZ4F_VERSION)
    });
    let mut c_total = Vec::new();
    let mut r_total = Vec::new();
    let mut c_buffer = vec![0u8; 200_000];
    let mut r_buffer = vec![0u8; 200_000];
    let c_size = unsafe { begin_c(c_ctx, c_buffer.as_mut_ptr().cast(), 19, &prefs) };
    let r_size = unsafe { begin_r(r_ctx, r_buffer.as_mut_ptr().cast(), 19, &prefs) };
    assert_eq!(c_size, r_size);
    c_total.extend_from_slice(&c_buffer[..c_size]);
    r_total.extend_from_slice(&r_buffer[..r_size]);
    for (index, input) in [patterned(31), patterned(70_000), patterned(3333)]
        .into_iter()
        .enumerate()
    {
        let capacity = unsafe { bound_c(input.len(), &prefs) };
        assert_eq!(capacity, unsafe { bound_r(input.len(), &prefs) });
        let (c_function, r_function) = if index == 1 {
            (&raw_c, &raw_r)
        } else {
            (&update_c, &update_r)
        };
        let c_size = unsafe {
            c_function(
                c_ctx,
                c_buffer.as_mut_ptr().cast(),
                capacity,
                input.as_ptr().cast(),
                input.len(),
                &options,
            )
        };
        let r_size = unsafe {
            r_function(
                r_ctx,
                r_buffer.as_mut_ptr().cast(),
                capacity,
                input.as_ptr().cast(),
                input.len(),
                &options,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_buffer[..c_size], &r_buffer[..r_size]);
        c_total.extend_from_slice(&c_buffer[..c_size]);
        r_total.extend_from_slice(&r_buffer[..r_size]);
    }
    let capacity = unsafe { bound_c(0, &prefs) };
    let c_size = unsafe { flush_c(c_ctx, c_buffer.as_mut_ptr().cast(), capacity, &options) };
    let r_size = unsafe { flush_r(r_ctx, r_buffer.as_mut_ptr().cast(), capacity, &options) };
    assert_eq!(c_size, r_size);
    c_total.extend_from_slice(&c_buffer[..c_size]);
    r_total.extend_from_slice(&r_buffer[..r_size]);
    let c_size = unsafe { end_c(c_ctx, c_buffer.as_mut_ptr().cast(), capacity, &options) };
    let r_size = unsafe { end_r(r_ctx, r_buffer.as_mut_ptr().cast(), capacity, &options) };
    assert_eq!(c_size, r_size);
    c_total.extend_from_slice(&c_buffer[..c_size]);
    r_total.extend_from_slice(&r_buffer[..r_size]);
    assert_eq!(c_total, r_total);
    assert_eq!(unsafe { free_c(c_ctx) }, unsafe { free_r(r_ctx) });
}

unsafe fn exercise_frame_dictionary(libs: &Libraries) {
    type CreateDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
    type FreeDict = unsafe extern "C" fn(*mut c_void);
    type CreateCtx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type FreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
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
    let (dict_c, dict_r) = unsafe { libs.pair::<CreateDict>(b"LZ4F_createCDict\0") };
    let (free_dict_c, free_dict_r) = unsafe { libs.pair::<FreeDict>(b"LZ4F_freeCDict\0") };
    let (ctx_c, ctx_r) = unsafe { libs.pair::<CreateCtx>(b"LZ4F_createCompressionContext\0") };
    let (free_ctx_c, free_ctx_r) =
        unsafe { libs.pair::<FreeCtx>(b"LZ4F_freeCompressionContext\0") };
    let (bound_c, bound_r) = unsafe { libs.pair::<Bound>(b"LZ4F_compressFrameBound\0") };
    let (compress_c, compress_r) =
        unsafe { libs.pair::<Compress>(b"LZ4F_compressFrame_usingCDict\0") };
    let dictionary = patterned(80_000);
    let c_dict = unsafe { dict_c(dictionary.as_ptr().cast(), dictionary.len()) };
    let r_dict = unsafe { dict_r(dictionary.as_ptr().cast(), dictionary.len()) };
    assert_eq!(c_dict.is_null(), r_dict.is_null());
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { ctx_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        ctx_r(&mut r_ctx, LZ4F_VERSION)
    });
    let prefs = Preferences {
        frame_info: FrameInfo {
            block_size_id: 4,
            block_mode: 1,
            dict_id: 77,
            ..FrameInfo::default()
        },
        compression_level: 10,
        ..Preferences::default()
    };
    let input = patterned(120_000);
    let bound = unsafe { bound_c(input.len(), &prefs) };
    assert_eq!(bound, unsafe { bound_r(input.len(), &prefs) });
    let mut c_out = vec![0u8; bound];
    let mut r_out = vec![0u8; bound];
    let c_size = unsafe {
        compress_c(
            c_ctx,
            c_out.as_mut_ptr().cast(),
            bound,
            input.as_ptr().cast(),
            input.len(),
            c_dict,
            &prefs,
        )
    };
    let r_size = unsafe {
        compress_r(
            r_ctx,
            r_out.as_mut_ptr().cast(),
            bound,
            input.as_ptr().cast(),
            input.len(),
            r_dict,
            &prefs,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_out[..c_size], &r_out[..r_size]);
    unsafe {
        free_dict_c(c_dict);
        free_dict_r(r_dict);
    }
    assert_eq!(unsafe { free_ctx_c(c_ctx) }, unsafe { free_ctx_r(r_ctx) });
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn tmpfile() -> *mut c_void;
    fn fflush(file: *mut c_void) -> c_int;
    fn fseek(file: *mut c_void, offset: c_long, origin: c_int) -> c_int;
    fn ftell(file: *mut c_void) -> c_long;
    fn fread(buffer: *mut c_void, size: usize, count: usize, file: *mut c_void) -> usize;
    fn fwrite(buffer: *const c_void, size: usize, count: usize, file: *mut c_void) -> usize;
    fn fclose(file: *mut c_void) -> c_int;
}

unsafe extern "C" fn custom_alloc(_: *mut c_void, size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

unsafe extern "C" fn custom_calloc(_: *mut c_void, size: usize) -> *mut c_void {
    unsafe { calloc(1, size) }
}

unsafe extern "C" fn custom_free(_: *mut c_void, pointer: *mut c_void) {
    unsafe { free(pointer) }
}

unsafe fn exercise_custom_memory(libs: &Libraries) {
    type CreateC = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
    type CreateD = unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void;
    type CreateDict = unsafe extern "C" fn(CustomMem, *const c_void, usize) -> *mut c_void;
    type FreeCtx = unsafe extern "C" fn(*mut c_void) -> usize;
    type FreeDict = unsafe extern "C" fn(*mut c_void);
    let memory = CustomMem {
        custom_alloc: custom_alloc as *const () as *mut c_void,
        custom_calloc: custom_calloc as *const () as *mut c_void,
        custom_free: custom_free as *const () as *mut c_void,
        opaque_state: ptr::null_mut(),
    };
    let (create_c_c, create_c_r) =
        unsafe { libs.pair::<CreateC>(b"LZ4F_createCompressionContext_advanced\0") };
    let (create_d_c, create_d_r) =
        unsafe { libs.pair::<CreateD>(b"LZ4F_createDecompressionContext_advanced\0") };
    let (create_dict_c, create_dict_r) =
        unsafe { libs.pair::<CreateDict>(b"LZ4F_createCDict_advanced\0") };
    let (free_c_c, free_c_r) = unsafe { libs.pair::<FreeCtx>(b"LZ4F_freeCompressionContext\0") };
    let (free_d_c, free_d_r) = unsafe { libs.pair::<FreeCtx>(b"LZ4F_freeDecompressionContext\0") };
    let (free_dict_c, free_dict_r) = unsafe { libs.pair::<FreeDict>(b"LZ4F_freeCDict\0") };
    let c_cctx = unsafe { create_c_c(memory, 77) };
    let r_cctx = unsafe { create_c_r(memory, 77) };
    let c_dctx = unsafe { create_d_c(memory, 77) };
    let r_dctx = unsafe { create_d_r(memory, 77) };
    assert_eq!(c_cctx.is_null(), r_cctx.is_null());
    assert_eq!(c_dctx.is_null(), r_dctx.is_null());
    let dictionary = patterned(4096);
    let c_dict = unsafe { create_dict_c(memory, dictionary.as_ptr().cast(), dictionary.len()) };
    let r_dict = unsafe { create_dict_r(memory, dictionary.as_ptr().cast(), dictionary.len()) };
    assert_eq!(c_dict.is_null(), r_dict.is_null());
    assert_eq!(unsafe { free_c_c(c_cctx) }, unsafe { free_c_r(r_cctx) });
    assert_eq!(unsafe { free_d_c(c_dctx) }, unsafe { free_d_r(r_dctx) });
    unsafe {
        free_dict_c(c_dict);
        free_dict_r(r_dict);
    }
}

unsafe fn exercise_file_api(libs: &Libraries) {
    type OpenWrite =
        unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const Preferences) -> usize;
    type Write = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> usize;
    type Close = unsafe extern "C" fn(*mut c_void) -> usize;
    type OpenRead = unsafe extern "C" fn(*mut *mut c_void, *mut c_void) -> usize;
    type Read = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize;
    let (open_write_c, open_write_r) = unsafe { libs.pair::<OpenWrite>(b"LZ4F_writeOpen\0") };
    let (write_c, write_r) = unsafe { libs.pair::<Write>(b"LZ4F_write\0") };
    let (close_write_c, close_write_r) = unsafe { libs.pair::<Close>(b"LZ4F_writeClose\0") };
    let (open_read_c, open_read_r) = unsafe { libs.pair::<OpenRead>(b"LZ4F_readOpen\0") };
    let (read_c, read_r) = unsafe { libs.pair::<Read>(b"LZ4F_read\0") };
    let (close_read_c, close_read_r) = unsafe { libs.pair::<Close>(b"LZ4F_readClose\0") };
    let input = patterned(150_000);
    let prefs = Preferences {
        frame_info: FrameInfo {
            block_size_id: 5,
            content_checksum_flag: 1,
            ..FrameInfo::default()
        },
        ..Preferences::default()
    };
    let c_file = unsafe { tmpfile() };
    let r_file = unsafe { tmpfile() };
    assert!(!c_file.is_null() && !r_file.is_null());
    let mut c_writer = ptr::null_mut();
    let mut r_writer = ptr::null_mut();
    assert_eq!(
        unsafe { open_write_c(&mut c_writer, c_file, &prefs) },
        unsafe { open_write_r(&mut r_writer, r_file, &prefs) }
    );
    for chunk in input.chunks(17_777) {
        assert_eq!(
            unsafe { write_c(c_writer, chunk.as_ptr().cast(), chunk.len()) },
            unsafe { write_r(r_writer, chunk.as_ptr().cast(), chunk.len()) }
        );
    }
    assert_eq!(unsafe { close_write_c(c_writer) }, unsafe {
        close_write_r(r_writer)
    });
    let c_bytes = unsafe { file_bytes(c_file) };
    let r_bytes = unsafe { file_bytes(r_file) };
    assert_eq!(c_bytes, r_bytes);

    let c_read_file = unsafe { file_from_bytes(&c_bytes) };
    let r_read_file = unsafe { file_from_bytes(&r_bytes) };
    let mut c_reader = ptr::null_mut();
    let mut r_reader = ptr::null_mut();
    assert_eq!(unsafe { open_read_c(&mut c_reader, c_read_file) }, unsafe {
        open_read_r(&mut r_reader, r_read_file)
    });
    let mut c_output = vec![0u8; input.len()];
    let mut r_output = vec![0u8; input.len()];
    let c_size = unsafe { read_c(c_reader, c_output.as_mut_ptr().cast(), c_output.len()) };
    let r_size = unsafe { read_r(r_reader, r_output.as_mut_ptr().cast(), r_output.len()) };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_output[..c_size], &r_output[..r_size]);
    assert_eq!(&c_output[..c_size], input);
    assert_eq!(unsafe { close_read_c(c_reader) }, unsafe {
        close_read_r(r_reader)
    });
    unsafe {
        fclose(c_file);
        fclose(r_file);
        fclose(c_read_file);
        fclose(r_read_file);
    }
}

unsafe fn file_bytes(file: *mut c_void) -> Vec<u8> {
    unsafe {
        fflush(file);
        assert_eq!(fseek(file, 0, 2), 0);
        let length = ftell(file);
        assert!(length >= 0);
        assert_eq!(fseek(file, 0, 0), 0);
        let mut bytes = vec![0u8; length as usize];
        assert_eq!(
            fread(bytes.as_mut_ptr().cast(), 1, bytes.len(), file),
            bytes.len()
        );
        bytes
    }
}

unsafe fn file_from_bytes(bytes: &[u8]) -> *mut c_void {
    unsafe {
        let file = tmpfile();
        assert!(!file.is_null());
        assert_eq!(
            fwrite(bytes.as_ptr().cast(), 1, bytes.len(), file),
            bytes.len()
        );
        assert_eq!(fflush(file), 0);
        assert_eq!(fseek(file, 0, 0), 0);
        file
    }
}
