mod common;

use common::{Libraries, Rng, mut_ptr_or_dangling, ptr_or_dangling};
use std::ffi::{c_int, c_uint, c_void};

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
    reserved1: c_uint,
    reserved0: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CustomMem {
    alloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    calloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    opaque: *mut c_void,
}

fn preference_matrix() -> Vec<Preferences> {
    let mut values = vec![Preferences::default()];
    for block_size in [4, 5, 6, 7] {
        values.push(Preferences {
            frame_info: FrameInfo {
                block_size_id: block_size,
                block_mode: (block_size & 1),
                content_checksum_flag: ((block_size >> 1) & 1),
                content_size: if block_size & 1 == 0 { 0 } else { 1 },
                dict_id: if block_size == 7 { 0x1234_5678 } else { 0 },
                block_checksum_flag: ((block_size + 1) & 1),
                ..FrameInfo::default()
            },
            compression_level: [-3, 0, 9, 10][(block_size - 4) as usize],
            auto_flush: (block_size & 1) as c_uint,
            favor_dec_speed: (block_size >= 6) as c_uint,
            reserved: [0; 3],
        });
    }
    values.extend([
        Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 0,
                content_checksum_flag: 1,
                block_checksum_flag: 1,
                ..FrameInfo::default()
            },
            compression_level: 12,
            auto_flush: 1,
            favor_dec_speed: 1,
            reserved: [0; 3],
        },
        Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 1,
                ..FrameInfo::default()
            },
            compression_level: -65_537,
            auto_flush: 0,
            favor_dec_speed: 0,
            reserved: [0; 3],
        },
    ]);
    values
}

#[test]
fn frame_one_shot_preferences_and_decompression_match() {
    unsafe {
        let libs = Libraries::load();
        let (c_bound, r_bound) = libs
            .pair::<unsafe extern "C" fn(usize, *const Preferences) -> usize>(
                b"LZ4F_compressFrameBound\0",
            );
        let (c_compress, r_compress) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const Preferences,
        ) -> usize>(b"LZ4F_compressFrame\0");
        let (c_is_error, r_is_error) =
            libs.pair::<unsafe extern "C" fn(usize) -> c_uint>(b"LZ4F_isError\0");
        let mut rng = Rng::new(0xc0ac_29b7_c97c_50dd);
        let mut inputs = vec![
            vec![],
            vec![0],
            vec![7; 13],
            vec![0; 65_535],
            vec![0; 65_536],
            vec![0; 65_537],
        ];
        for _ in 0..25 {
            let len = (rng.next_u64() as usize) % 180_000;
            let mut input = rng.bytes(len);
            if rng.next_u64() & 1 == 0 {
                for byte in &mut input {
                    *byte &= 15;
                }
            }
            inputs.push(input);
        }

        for prefs in preference_matrix() {
            for input in &inputs {
                let mut effective = prefs;
                if effective.frame_info.content_size != 0 {
                    effective.frame_info.content_size = input.len() as u64;
                }
                let cb = c_bound(input.len(), &effective);
                let rb = r_bound(input.len(), &effective);
                assert_eq!(cb, rb);
                let mut co = vec![0u8; cb.max(1)];
                let mut ro = vec![0u8; rb.max(1)];
                let cn = c_compress(
                    co.as_mut_ptr().cast(),
                    cb,
                    ptr_or_dangling(input),
                    input.len(),
                    &effective,
                );
                let rn = r_compress(
                    ro.as_mut_ptr().cast(),
                    rb,
                    ptr_or_dangling(input),
                    input.len(),
                    &effective,
                );
                assert_eq!(c_is_error(cn), r_is_error(rn));
                assert_eq!(cn, rn, "prefs={effective:?}, len={}", input.len());
                assert_eq!(&co[..cn], &ro[..rn]);
                let cdecoded = decompress_frame(&libs.c, &co[..cn], input.len(), false);
                let rdecoded = decompress_frame(&libs.rust, &ro[..rn], input.len(), false);
                assert_eq!(cdecoded, rdecoded);
                assert_eq!(cdecoded, *input);
            }
        }

        for input in inputs.iter().take(12) {
            let cb = c_bound(input.len(), std::ptr::null());
            let rb = r_bound(input.len(), std::ptr::null());
            assert_eq!(cb, rb);
            let mut co = vec![0u8; cb];
            let mut ro = vec![0u8; rb];
            let cn = c_compress(
                co.as_mut_ptr().cast(),
                cb,
                ptr_or_dangling(input),
                input.len(),
                std::ptr::null(),
            );
            let rn = r_compress(
                ro.as_mut_ptr().cast(),
                rb,
                ptr_or_dangling(input),
                input.len(),
                std::ptr::null(),
            );
            assert_eq!(cn, rn);
            assert_eq!(&co[..cn], &ro[..rn]);
        }
    }
}

unsafe fn decompress_frame(
    lib: &libloading::Library,
    compressed: &[u8],
    expected_size: usize,
    skip_checksums: bool,
) -> Vec<u8> {
    let create = unsafe {
        lib.get::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
            b"LZ4F_createDecompressionContext\0",
        )
        .unwrap()
    };
    let decompress = unsafe {
        lib.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
            *const DecompressOptions,
        ) -> usize>(b"LZ4F_decompress\0")
            .unwrap()
    };
    let free = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeDecompressionContext\0")
            .unwrap()
    };
    let is_error = unsafe {
        lib.get::<unsafe extern "C" fn(usize) -> c_uint>(b"LZ4F_isError\0")
            .unwrap()
    };
    let mut context = std::ptr::null_mut();
    let create_result = unsafe { create(&mut context, 100) };
    assert_eq!(unsafe { is_error(create_result) }, 0);
    let mut output = vec![0u8; expected_size.max(1)];
    let mut output_size = expected_size;
    let mut input_size = compressed.len();
    let options = DecompressOptions {
        stable_dst: 0,
        skip_checksums: skip_checksums as c_uint,
        reserved1: 0,
        reserved0: 0,
    };
    let hint = unsafe {
        decompress(
            context,
            mut_ptr_or_dangling(&mut output),
            &mut output_size,
            ptr_or_dangling(compressed),
            &mut input_size,
            &options,
        )
    };
    assert_eq!(unsafe { is_error(hint) }, 0);
    assert_eq!(hint, 0);
    assert_eq!(input_size, compressed.len());
    output.truncate(output_size);
    unsafe { free(context) };
    output
}

#[test]
fn frame_streaming_compression_flush_uncompressed_and_info_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Rng::new(0x3f84_d5b5_b547_0917);
        for (iteration, mut prefs) in preference_matrix().into_iter().enumerate() {
            let len = 20_000 + (rng.next_u64() as usize % 100_000);
            let mut input = rng.bytes(len);
            if iteration & 1 == 0 {
                for byte in &mut input {
                    *byte &= 7;
                }
            }
            prefs.frame_info.content_size = len as u64;
            let use_uncompressed = iteration % 2 == 0;
            if iteration % 3 == 0 || use_uncompressed {
                prefs.frame_info.block_mode = 1;
            }
            let c_frame = stream_compress(&libs.c, &input, &prefs, use_uncompressed);
            let r_frame = stream_compress(&libs.rust, &input, &prefs, use_uncompressed);
            assert_eq!(c_frame, r_frame);
            assert_eq!(decompress_frame(&libs.c, &c_frame, len, false), input);
            assert_eq!(decompress_frame(&libs.rust, &r_frame, len, false), input);

            compare_header_and_info(&libs, &c_frame);
        }
    }
}

unsafe fn stream_compress(
    lib: &libloading::Library,
    input: &[u8],
    prefs: &Preferences,
    use_uncompressed: bool,
) -> Vec<u8> {
    let create = unsafe {
        lib.get::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
            b"LZ4F_createCompressionContext\0",
        )
        .unwrap()
    };
    let begin = unsafe {
        lib.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const Preferences,
        ) -> usize>(b"LZ4F_compressBegin\0")
        .unwrap()
    };
    let bound = unsafe {
        lib.get::<unsafe extern "C" fn(usize, *const Preferences) -> usize>(b"LZ4F_compressBound\0")
            .unwrap()
    };
    let update_name = if use_uncompressed {
        b"LZ4F_uncompressedUpdate\0".as_slice()
    } else {
        b"LZ4F_compressUpdate\0".as_slice()
    };
    let update = unsafe {
        lib.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize>(update_name)
            .unwrap()
    };
    let flush = unsafe {
        lib.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const CompressOptions,
        ) -> usize>(b"LZ4F_flush\0")
        .unwrap()
    };
    let end = unsafe {
        lib.get::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const CompressOptions,
        ) -> usize>(b"LZ4F_compressEnd\0")
        .unwrap()
    };
    let free = unsafe {
        lib.get::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeCompressionContext\0")
            .unwrap()
    };
    let is_error = unsafe {
        lib.get::<unsafe extern "C" fn(usize) -> c_uint>(b"LZ4F_isError\0")
            .unwrap()
    };

    let mut context = std::ptr::null_mut();
    assert_eq!(unsafe { is_error(create(&mut context, 100)) }, 0);
    let mut result = Vec::new();
    let mut buffer = vec![0u8; unsafe { bound(input.len(), prefs) }.max(64) + 64];
    let header = unsafe { begin(context, buffer.as_mut_ptr().cast(), buffer.len(), prefs) };
    assert_eq!(unsafe { is_error(header) }, 0);
    result.extend_from_slice(&buffer[..header]);
    let options = CompressOptions {
        stable_src: 1,
        reserved: [0; 3],
    };
    for (index, chunk) in input.chunks(4093).enumerate() {
        let needed = unsafe { bound(chunk.len(), prefs) }.max(chunk.len() + 8);
        if buffer.len() < needed {
            buffer.resize(needed, 0);
        }
        let written = unsafe {
            update(
                context,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                ptr_or_dangling(chunk),
                chunk.len(),
                &options,
            )
        };
        assert_eq!(unsafe { is_error(written) }, 0);
        result.extend_from_slice(&buffer[..written]);
        if index % 4 == 2 {
            let flushed =
                unsafe { flush(context, buffer.as_mut_ptr().cast(), buffer.len(), &options) };
            assert_eq!(unsafe { is_error(flushed) }, 0);
            result.extend_from_slice(&buffer[..flushed]);
        }
    }
    let ended = unsafe { end(context, buffer.as_mut_ptr().cast(), buffer.len(), &options) };
    assert_eq!(unsafe { is_error(ended) }, 0);
    result.extend_from_slice(&buffer[..ended]);
    unsafe { free(context) };
    result
}

unsafe fn compare_header_and_info(libs: &Libraries, frame: &[u8]) {
    let (c_header, r_header) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_void, usize) -> usize>(b"LZ4F_headerSize\0")
    };
    for size in [0, 1, 4, 5, 7, 19, frame.len()] {
        let available = size.min(frame.len());
        assert_eq!(
            unsafe { c_header(ptr_or_dangling(&frame[..available]), available) },
            unsafe { r_header(ptr_or_dangling(&frame[..available]), available) }
        );
    }

    let (c_create, r_create) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
            b"LZ4F_createDecompressionContext\0",
        )
    };
    let (c_info, r_info) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *mut FrameInfo,
            *const c_void,
            *mut usize,
        ) -> usize>(b"LZ4F_getFrameInfo\0")
    };
    let (c_free, r_free) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeDecompressionContext\0")
    };
    let mut cc = std::ptr::null_mut();
    let mut rc = std::ptr::null_mut();
    assert_eq!(unsafe { c_create(&mut cc, 100) }, unsafe {
        r_create(&mut rc, 100)
    });
    let mut ci = FrameInfo::default();
    let mut ri = FrameInfo::default();
    let mut cs = frame.len();
    let mut rs = frame.len();
    let cr = unsafe { c_info(cc, &mut ci, frame.as_ptr().cast(), &mut cs) };
    let rr = unsafe { r_info(rc, &mut ri, frame.as_ptr().cast(), &mut rs) };
    assert_eq!((cr, cs), (rr, rs));
    assert_eq!(
        (
            ci.block_size_id,
            ci.block_mode,
            ci.content_checksum_flag,
            ci.frame_type,
            ci.content_size,
            ci.dict_id,
            ci.block_checksum_flag,
        ),
        (
            ri.block_size_id,
            ri.block_mode,
            ri.content_checksum_flag,
            ri.frame_type,
            ri.content_size,
            ri.dict_id,
            ri.block_checksum_flag,
        )
    );
    assert_eq!(unsafe { c_free(cc) }, unsafe { r_free(rc) });
}

#[test]
fn frame_dictionary_custom_allocator_and_error_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let default_mem = CustomMem {
            alloc: None,
            calloc: None,
            free: None,
            opaque: std::ptr::null_mut(),
        };
        let (c_adv_c, r_adv_c) = libs
            .pair::<unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void>(
                b"LZ4F_createCompressionContext_advanced\0",
            );
        let (c_free_c, r_free_c) = libs
            .pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeCompressionContext\0");
        let cc = c_adv_c(default_mem, 100);
        let rc = r_adv_c(default_mem, 100);
        assert_eq!(cc.is_null(), rc.is_null());
        assert_eq!(c_free_c(cc), r_free_c(rc));

        let (c_adv_d, r_adv_d) = libs
            .pair::<unsafe extern "C" fn(CustomMem, c_uint) -> *mut c_void>(
                b"LZ4F_createDecompressionContext_advanced\0",
            );
        let (c_free_d, r_free_d) = libs
            .pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeDecompressionContext\0");
        let cd = c_adv_d(default_mem, 100);
        let rd = r_adv_d(default_mem, 100);
        assert_eq!(cd.is_null(), rd.is_null());
        assert_eq!(c_free_d(cd), r_free_d(rd));

        let dictionary = vec![3u8; 70_000];
        let (c_cdict, r_cdict) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize) -> *mut c_void>(
                b"LZ4F_createCDict\0",
            );
        let (c_free_cdict, r_free_cdict) =
            libs.pair::<unsafe extern "C" fn(*mut c_void)>(b"LZ4F_freeCDict\0");
        let cdict = c_cdict(dictionary.as_ptr().cast(), dictionary.len());
        let rdict = r_cdict(dictionary.as_ptr().cast(), dictionary.len());
        assert_eq!(cdict.is_null(), rdict.is_null());
        c_free_cdict(cdict);
        r_free_cdict(rdict);
        c_free_cdict(std::ptr::null_mut());
        r_free_cdict(std::ptr::null_mut());

        let (c_create, r_create) = libs
            .pair::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
                b"LZ4F_createCompressionContext\0",
            );
        let (c_update, r_update) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const CompressOptions,
        ) -> usize>(b"LZ4F_compressUpdate\0");
        let mut cctx = std::ptr::null_mut();
        let mut rctx = std::ptr::null_mut();
        assert_eq!(c_create(&mut cctx, 100), r_create(&mut rctx, 100));
        let input = [1u8; 16];
        let mut co = [0u8; 64];
        let mut ro = [0u8; 64];
        assert_eq!(
            c_update(
                cctx,
                co.as_mut_ptr().cast(),
                co.len(),
                input.as_ptr().cast(),
                input.len(),
                std::ptr::null(),
            ),
            r_update(
                rctx,
                ro.as_mut_ptr().cast(),
                ro.len(),
                input.as_ptr().cast(),
                input.len(),
                std::ptr::null(),
            )
        );
        assert_eq!(c_free_c(cctx), r_free_c(rctx));

        let (c_header, r_header) =
            libs.pair::<unsafe extern "C" fn(*const c_void, usize) -> usize>(b"LZ4F_headerSize\0");
        let malformed = [0u8; 32];
        for size in [0, 1, 4, 5, 6, 7, 19, 32] {
            assert_eq!(
                c_header(malformed.as_ptr().cast(), size),
                r_header(malformed.as_ptr().cast(), size)
            );
        }

        let mut invalid = Preferences::default();
        let (c_bound, r_bound) = libs
            .pair::<unsafe extern "C" fn(usize, *const Preferences) -> usize>(
                b"LZ4F_compressFrameBound\0",
            );
        for block_size in [-1, 1, 3, 8, c_int::MAX] {
            invalid.frame_info.block_size_id = block_size;
            assert_eq!(c_bound(100, &invalid), r_bound(100, &invalid));
        }
    }
}

#[test]
fn frame_raw_and_digested_dictionary_entry_points_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Rng::new(0xba7c_9045_f12c_7f99);
        let dictionary = rng.bytes(70_000);
        let mut input = Vec::new();
        for _ in 0..1000 {
            let start = rng.next_u64() as usize % (dictionary.len() - 64);
            input.extend_from_slice(&dictionary[start..start + 64]);
        }
        let prefs = Preferences {
            frame_info: FrameInfo {
                block_size_id: 4,
                block_mode: 1,
                content_checksum_flag: 1,
                content_size: input.len() as u64,
                dict_id: 0xdead_beef,
                block_checksum_flag: 1,
                ..FrameInfo::default()
            },
            compression_level: 10,
            auto_flush: 1,
            favor_dec_speed: 1,
            reserved: [0; 3],
        };

        let (c_create_cdict, r_create_cdict) = libs
            .pair::<unsafe extern "C" fn(*const c_void, usize) -> *mut c_void>(
                b"LZ4F_createCDict\0",
            );
        let cdict = c_create_cdict(dictionary.as_ptr().cast(), dictionary.len());
        let rdict = r_create_cdict(dictionary.as_ptr().cast(), dictionary.len());
        assert!(!cdict.is_null() && !rdict.is_null());
        let (c_create_ctx, r_create_ctx) =
            libs.pair::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
                b"LZ4F_createCompressionContext\0",
            );
        let mut cc = std::ptr::null_mut();
        let mut rc = std::ptr::null_mut();
        assert_eq!(c_create_ctx(&mut cc, 100), r_create_ctx(&mut rc, 100));
        let (c_bound, r_bound) = libs
            .pair::<unsafe extern "C" fn(usize, *const Preferences) -> usize>(
                b"LZ4F_compressFrameBound\0",
            );
        let bound = c_bound(input.len(), &prefs);
        assert_eq!(bound, r_bound(input.len(), &prefs));
        let (c_compress, r_compress) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const c_void,
            *const Preferences,
        ) -> usize>(b"LZ4F_compressFrame_usingCDict\0");
        let mut co = vec![0u8; bound];
        let mut ro = vec![0u8; bound];
        let cn = c_compress(
            cc,
            co.as_mut_ptr().cast(),
            co.len(),
            input.as_ptr().cast(),
            input.len(),
            cdict,
            &prefs,
        );
        let rn = r_compress(
            rc,
            ro.as_mut_ptr().cast(),
            ro.len(),
            input.as_ptr().cast(),
            input.len(),
            rdict,
            &prefs,
        );
        assert_eq!(cn, rn);
        assert_eq!(&co[..cn], &ro[..rn]);
        compare_dictionary_decompression(&libs, &co[..cn], &ro[..rn], &input, &dictionary);

        let (c_free_ctx, r_free_ctx) = libs
            .pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeCompressionContext\0");
        assert_eq!(c_free_ctx(cc), r_free_ctx(rc));

        for variant in 0..4 {
            let mut cc = std::ptr::null_mut();
            let mut rc = std::ptr::null_mut();
            assert_eq!(c_create_ctx(&mut cc, 100), r_create_ctx(&mut rc, 100));
            let mut ch = [0u8; 19];
            let mut rh = [0u8; 19];
            let (cr, rr) = match variant {
                0 => {
                    let (c, r) =
                        libs.pair::<unsafe extern "C" fn(
                            *mut c_void,
                            *mut c_void,
                            usize,
                            *const c_void,
                            usize,
                            *const Preferences,
                        ) -> usize>(b"LZ4F_compressBegin_usingDict\0");
                    (
                        c(
                            cc,
                            ch.as_mut_ptr().cast(),
                            ch.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            &prefs,
                        ),
                        r(
                            rc,
                            rh.as_mut_ptr().cast(),
                            rh.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            &prefs,
                        ),
                    )
                }
                1 => {
                    let (c, r) = libs.pair::<unsafe extern "C" fn(
                        *mut c_void,
                        *mut c_void,
                        usize,
                        *const c_void,
                        usize,
                        *const Preferences,
                    ) -> usize>(
                        b"LZ4F_compressBegin_usingDictOnce\0"
                    );
                    (
                        c(
                            cc,
                            ch.as_mut_ptr().cast(),
                            ch.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            &prefs,
                        ),
                        r(
                            rc,
                            rh.as_mut_ptr().cast(),
                            rh.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            &prefs,
                        ),
                    )
                }
                2 => {
                    let (c, r) =
                        libs.pair::<unsafe extern "C" fn(
                            *mut c_void,
                            *mut c_void,
                            usize,
                            *const c_void,
                            *const Preferences,
                        ) -> usize>(b"LZ4F_compressBegin_usingCDict\0");
                    (
                        c(cc, ch.as_mut_ptr().cast(), ch.len(), cdict, &prefs),
                        r(rc, rh.as_mut_ptr().cast(), rh.len(), rdict, &prefs),
                    )
                }
                _ => {
                    let (c, r) =
                        libs.pair::<unsafe extern "C" fn(
                            *mut c_void,
                            *mut c_void,
                            usize,
                            *const c_void,
                            usize,
                            *const c_void,
                            *const Preferences,
                        ) -> usize>(b"LZ4F_compressBegin_internal\0");
                    (
                        c(
                            cc,
                            ch.as_mut_ptr().cast(),
                            ch.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            std::ptr::null(),
                            &prefs,
                        ),
                        r(
                            rc,
                            rh.as_mut_ptr().cast(),
                            rh.len(),
                            dictionary.as_ptr().cast(),
                            dictionary.len(),
                            std::ptr::null(),
                            &prefs,
                        ),
                    )
                }
            };
            assert_eq!(cr, rr);
            assert_eq!(&ch[..cr], &rh[..rr]);
            assert_eq!(c_free_ctx(cc), r_free_ctx(rc));
        }

        let (c_free_cdict, r_free_cdict) =
            libs.pair::<unsafe extern "C" fn(*mut c_void)>(b"LZ4F_freeCDict\0");
        c_free_cdict(cdict);
        r_free_cdict(rdict);
    }
}

unsafe fn compare_dictionary_decompression(
    libs: &Libraries,
    c_frame: &[u8],
    r_frame: &[u8],
    original: &[u8],
    dictionary: &[u8],
) {
    let (c_create, r_create) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize>(
            b"LZ4F_createDecompressionContext\0",
        )
    };
    let (c_decompress, r_decompress) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut usize,
            *const c_void,
            *mut usize,
            *const c_void,
            usize,
            *const DecompressOptions,
        ) -> usize>(b"LZ4F_decompress_usingDict\0")
    };
    let (c_reset, r_reset) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void)>(b"LZ4F_resetDecompressionContext\0")
    };
    let (c_free, r_free) = unsafe {
        libs.pair::<unsafe extern "C" fn(*mut c_void) -> usize>(b"LZ4F_freeDecompressionContext\0")
    };
    let mut cc = std::ptr::null_mut();
    let mut rc = std::ptr::null_mut();
    assert_eq!(unsafe { c_create(&mut cc, 100) }, unsafe {
        r_create(&mut rc, 100)
    });
    let mut co = vec![0u8; original.len()];
    let mut ro = vec![0u8; original.len()];
    let mut cos = co.len();
    let mut ros = ro.len();
    let mut cis = c_frame.len();
    let mut ris = r_frame.len();
    let options = DecompressOptions::default();
    let cr = unsafe {
        c_decompress(
            cc,
            co.as_mut_ptr().cast(),
            &mut cos,
            c_frame.as_ptr().cast(),
            &mut cis,
            dictionary.as_ptr().cast(),
            dictionary.len(),
            &options,
        )
    };
    let rr = unsafe {
        r_decompress(
            rc,
            ro.as_mut_ptr().cast(),
            &mut ros,
            r_frame.as_ptr().cast(),
            &mut ris,
            dictionary.as_ptr().cast(),
            dictionary.len(),
            &options,
        )
    };
    assert_eq!((cr, cos, cis), (rr, ros, ris));
    assert_eq!(&co[..cos], &ro[..ros]);
    assert_eq!(&co[..cos], original);
    unsafe {
        c_reset(cc);
        r_reset(rc);
    }
    assert_eq!(unsafe { c_free(cc) }, unsafe { r_free(rc) });
}
