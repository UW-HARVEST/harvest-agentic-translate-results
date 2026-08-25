mod common;

use common::*;
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

#[test]
fn core_deprecated_external_state_and_dictionary_aliases_match() {
    unsafe {
        let libs = Libraries::load();
        let input = patterned(32_000);
        let bound = input.len() + input.len() / 255 + 16;
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];

        type Compress3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        let (compress_c, compress_r) = libs.pair::<Compress3>(b"LZ4_compress\0");
        let c_size = compress_c(
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        let r_size = compress_r(
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
        );
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        let compressed = c_out[..c_size as usize].to_vec();

        for name in [
            b"LZ4_compress_limitedOutput\0".as_slice(),
            b"LZ4_compress_default\0",
        ] {
            let (c, r) = libs.pair::<Compress>(name);
            let (c_size, c_bytes) = compress_with(&c, &input, bound);
            let (r_size, r_bytes) = compress_with(&r, &input, bound);
            assert_eq!((c_size, c_bytes), (r_size, r_bytes));
        }

        type State4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type State5 =
            unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        let (state_size_c, state_size_r) =
            libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofStreamState\0");
        assert_eq!(state_size_c(), state_size_r());
        let words = state_size_c() as usize / 8 + 2;
        for name in [
            b"LZ4_compress_withState\0".as_slice(),
            b"LZ4_compress_continue\0",
        ] {
            let (c, r) = libs.pair::<State4>(name);
            let mut c_state = vec![0u64; words];
            let mut r_state = vec![0u64; words];
            let mut c_dst = vec![0u8; bound];
            let mut r_dst = vec![0u8; bound];
            let c_size = c(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_dst.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            let r_size = r(
                r_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                r_dst.as_mut_ptr().cast(),
                input.len() as c_int,
            );
            assert_eq!(c_size, r_size);
            assert_eq!(&c_dst[..c_size as usize], &r_dst[..r_size as usize]);
        }
        for name in [
            b"LZ4_compress_limitedOutput_withState\0".as_slice(),
            b"LZ4_compress_limitedOutput_continue\0",
        ] {
            let (c, r) = libs.pair::<State5>(name);
            let mut c_state = vec![0u64; words];
            let mut r_state = vec![0u64; words];
            let mut c_dst = vec![0u8; bound];
            let mut r_dst = vec![0u8; bound];
            let c_size = c(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_dst.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
            );
            let r_size = r(
                r_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                r_dst.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
            );
            assert_eq!(c_size, r_size);
            assert_eq!(&c_dst[..c_size as usize], &r_dst[..r_size as usize]);
        }

        type Dest = unsafe extern "C" fn(*const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
        let (dest_c, dest_r) = libs.pair::<Dest>(b"LZ4_compress_destSize\0");
        let mut c_source_size = input.len() as c_int;
        let mut r_source_size = input.len() as c_int;
        let c_size = dest_c(
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            &mut c_source_size,
            bound as c_int,
        );
        let r_size = dest_r(
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            &mut r_source_size,
            bound as c_int,
        );
        assert_eq!((c_size, c_source_size), (r_size, r_source_size));
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);

        exercise_core_decompression_aliases(&libs, &compressed, &input);
        exercise_deprecated_stream_state(&libs, &input);
    }
}

unsafe fn exercise_core_decompression_aliases(
    libs: &Libraries,
    compressed: &[u8],
    expected: &[u8],
) {
    type Fast3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
    for name in [b"LZ4_decompress_fast\0".as_slice(), b"LZ4_uncompress\0"] {
        let (c, r) = unsafe { libs.pair::<Fast3>(name) };
        let mut c_out = vec![0u8; expected.len()];
        let mut r_out = vec![0u8; expected.len()];
        let c_size = unsafe {
            c(
                compressed.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                expected.len() as c_int,
            )
        };
        let r_size = unsafe {
            r(
                compressed.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                expected.len() as c_int,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(c_out, r_out);
    }
    let (unknown_c, unknown_r) =
        unsafe { libs.pair::<Decompress>(b"LZ4_uncompress_unknownOutputSize\0") };
    let (c_size, c_out) = unsafe { decompress_with(&unknown_c, compressed, expected.len()) };
    let (r_size, r_out) = unsafe { decompress_with(&unknown_r, compressed, expected.len()) };
    assert_eq!((c_size, c_out), (r_size, r_out));

    type SafeDict = unsafe extern "C" fn(
        *const c_char,
        *mut c_char,
        c_int,
        c_int,
        *const c_char,
        c_int,
    ) -> c_int;
    let (safe_c, safe_r) = unsafe { libs.pair::<SafeDict>(b"LZ4_decompress_safe_usingDict\0") };
    let mut c_out = vec![0u8; expected.len()];
    let mut r_out = vec![0u8; expected.len()];
    let c_size = unsafe {
        safe_c(
            compressed.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            compressed.len() as c_int,
            expected.len() as c_int,
            ptr::null(),
            0,
        )
    };
    let r_size = unsafe {
        safe_r(
            compressed.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            compressed.len() as c_int,
            expected.len() as c_int,
            ptr::null(),
            0,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_out, r_out);

    type FastDict =
        unsafe extern "C" fn(*const c_char, *mut c_char, c_int, *const c_char, c_int) -> c_int;
    let (fast_c, fast_r) = unsafe { libs.pair::<FastDict>(b"LZ4_decompress_fast_usingDict\0") };
    c_out.fill(0);
    r_out.fill(0);
    let c_size = unsafe {
        fast_c(
            compressed.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            expected.len() as c_int,
            ptr::null(),
            0,
        )
    };
    let r_size = unsafe {
        fast_r(
            compressed.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            expected.len() as c_int,
            ptr::null(),
            0,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_out, r_out);

    type Force = unsafe extern "C" fn(
        *const c_char,
        *mut c_char,
        c_int,
        c_int,
        *const c_void,
        usize,
    ) -> c_int;
    let dictionary = patterned(4096);
    let (force_c, force_r) = unsafe { libs.pair::<Force>(b"LZ4_decompress_safe_forceExtDict\0") };
    let mut c_out = vec![0u8; expected.len()];
    let mut r_out = vec![0u8; expected.len()];
    assert_eq!(
        unsafe {
            force_c(
                compressed.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                compressed.len() as c_int,
                expected.len() as c_int,
                dictionary.as_ptr().cast(),
                dictionary.len(),
            )
        },
        unsafe {
            force_r(
                compressed.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                compressed.len() as c_int,
                expected.len() as c_int,
                dictionary.as_ptr().cast(),
                dictionary.len(),
            )
        }
    );
    assert_eq!(c_out, r_out);

    type PartialDict = unsafe extern "C" fn(
        *const c_char,
        *mut c_char,
        c_int,
        c_int,
        c_int,
        *const c_char,
        c_int,
    ) -> c_int;
    let (partial_c, partial_r) =
        unsafe { libs.pair::<PartialDict>(b"LZ4_decompress_safe_partial_usingDict\0") };
    let mut c_partial = vec![0u8; expected.len() / 2];
    let mut r_partial = vec![0u8; expected.len() / 2];
    assert_eq!(
        unsafe {
            partial_c(
                compressed.as_ptr().cast(),
                c_partial.as_mut_ptr().cast(),
                compressed.len() as c_int,
                c_partial.len() as c_int,
                c_partial.len() as c_int,
                ptr::null(),
                0,
            )
        },
        unsafe {
            partial_r(
                compressed.as_ptr().cast(),
                r_partial.as_mut_ptr().cast(),
                compressed.len() as c_int,
                r_partial.len() as c_int,
                r_partial.len() as c_int,
                ptr::null(),
                0,
            )
        }
    );
    assert_eq!(c_partial, r_partial);
}

unsafe fn exercise_deprecated_stream_state(libs: &Libraries, input: &[u8]) {
    type Create = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
    type Reset = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
    type Slide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_create\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_resetStreamState\0") };
    let (slide_c, slide_r) = unsafe { libs.pair::<Slide>(b"LZ4_slideInputBuffer\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStream\0") };
    let c_state = unsafe { create_c(input.as_ptr().cast_mut().cast()) };
    let r_state = unsafe { create_r(input.as_ptr().cast_mut().cast()) };
    assert_eq!(
        unsafe { reset_c(c_state, input.as_ptr().cast_mut().cast()) },
        unsafe { reset_r(r_state, input.as_ptr().cast_mut().cast()) }
    );
    assert_eq!(
        unsafe { slide_c(c_state) }.is_null(),
        unsafe { slide_r(r_state) }.is_null()
    );
    assert_eq!(unsafe { free_c(c_state) }, unsafe { free_r(r_state) });
}

#[test]
fn high_compression_aliases_and_destination_size_paths_match() {
    unsafe {
        let libs = Libraries::load();
        let input = patterned(24_000);
        let bound = input.len() + input.len() / 255 + 16;
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        type Hc3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        type Hc4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type Hc5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let (hc_c, hc_r) = libs.pair::<Hc3>(b"LZ4_compressHC\0");
        assert_eq!(
            hc_c(
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
            ),
            hc_r(
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
            )
        );
        let (limited_c, limited_r) = libs.pair::<Hc4>(b"LZ4_compressHC_limitedOutput\0");
        let c_size = limited_c(
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
        );
        let r_size = limited_r(
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
        );
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        let (hc2_c, hc2_r) = libs.pair::<Hc4>(b"LZ4_compressHC2\0");
        assert_eq!(
            hc2_c(
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
                10,
            ),
            hc2_r(
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
                10,
            )
        );
        let (hc2_limit_c, hc2_limit_r) = libs.pair::<Hc5>(b"LZ4_compressHC2_limitedOutput\0");
        assert_eq!(
            hc2_limit_c(
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
                10,
            ),
            hc2_limit_r(
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
                10,
            )
        );

        exercise_hc_external_aliases(&libs, &input, bound);
        exercise_hc_legacy_stream(&libs, &input, bound);
    }
}

unsafe fn exercise_hc_external_aliases(libs: &Libraries, input: &[u8], bound: usize) {
    type Size = unsafe extern "C" fn() -> c_int;
    type State4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
    type State5 =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
    type State6 =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    let (size_c, size_r) = unsafe { libs.pair::<Size>(b"LZ4_sizeofStreamStateHC\0") };
    assert_eq!(unsafe { size_c() }, unsafe { size_r() });
    let words = unsafe { size_c() } as usize / 8 + 2;
    let mut c_state = vec![0u64; words];
    let mut r_state = vec![0u64; words];
    let mut c_out = vec![0u8; bound];
    let mut r_out = vec![0u8; bound];
    let (state_c, state_r) = unsafe { libs.pair::<State4>(b"LZ4_compressHC_withStateHC\0") };
    let c_size = unsafe {
        state_c(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
        )
    };
    let r_size = unsafe {
        state_r(
            r_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);

    c_state.fill(0);
    r_state.fill(0);
    let (limited_c, limited_r) =
        unsafe { libs.pair::<State5>(b"LZ4_compressHC_limitedOutput_withStateHC\0") };
    let c_size = unsafe {
        limited_c(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
        )
    };
    let r_size = unsafe {
        limited_r(
            r_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);

    for name in [
        b"LZ4_compress_HC_extStateHC\0".as_slice(),
        b"LZ4_compress_HC_extStateHC_fastReset\0",
        b"LZ4_compressHC2_limitedOutput_withStateHC\0",
    ] {
        let (c, r) = unsafe { libs.pair::<State6>(name) };
        let mut c_state = vec![0u64; words];
        let mut r_state = vec![0u64; words];
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            c(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
                10,
            )
        };
        let r_size = unsafe {
            r(
                r_state.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
                bound as c_int,
                10,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
    }
    let (hc2_c, hc2_r) = unsafe { libs.pair::<State5>(b"LZ4_compressHC2_withStateHC\0") };
    c_state.fill(0);
    r_state.fill(0);
    let c_size = unsafe {
        hc2_c(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
            10,
        )
    };
    let r_size = unsafe {
        hc2_r(
            r_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
            10,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);

    type Dest = unsafe extern "C" fn(
        *mut c_void,
        *const c_char,
        *mut c_char,
        *mut c_int,
        c_int,
        c_int,
    ) -> c_int;
    let (dest_c, dest_r) = unsafe { libs.pair::<Dest>(b"LZ4_compress_HC_destSize\0") };
    let mut c_state = vec![0u64; words];
    let mut r_state = vec![0u64; words];
    let mut c_out = vec![0u8; bound];
    let mut r_out = vec![0u8; bound];
    let mut c_source = input.len() as c_int;
    let mut r_source = input.len() as c_int;
    let c_size = unsafe {
        dest_c(
            c_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            &mut c_source,
            bound as c_int,
            10,
        )
    };
    let r_size = unsafe {
        dest_r(
            r_state.as_mut_ptr().cast(),
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            &mut r_source,
            bound as c_int,
            10,
        )
    };
    assert_eq!((c_size, c_source), (r_size, r_source));
}

unsafe fn exercise_hc_legacy_stream(libs: &Libraries, input: &[u8], bound: usize) {
    type Create = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Continue4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
    type Continue5 =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
    type Continue6 =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createHC\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeHC\0") };
    {
        let (c, r) = unsafe { libs.pair::<Continue4>(b"LZ4_compressHC_continue\0") };
        let c_state = unsafe { create_c(input.as_ptr().cast()) };
        let r_state = unsafe { create_r(input.as_ptr().cast()) };
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            c(
                c_state,
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
            )
        };
        let r_size = unsafe {
            r(
                r_state,
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        assert_eq!(unsafe { free_c(c_state) }, unsafe { free_r(r_state) });
    }

    for (name, level) in [
        (b"LZ4_compressHC2_continue\0".as_slice(), 10),
        (
            b"LZ4_compressHC_limitedOutput_continue\0".as_slice(),
            bound as c_int,
        ),
    ] {
        let (c, r) = unsafe { libs.pair::<Continue5>(name) };
        let c_state = unsafe { create_c(input.as_ptr().cast()) };
        let r_state = unsafe { create_r(input.as_ptr().cast()) };
        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            c(
                c_state,
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
                level,
            )
        };
        let r_size = unsafe {
            r(
                r_state,
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
                level,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        assert_eq!(unsafe { free_c(c_state) }, unsafe { free_r(r_state) });
    }

    let (c, r) = unsafe { libs.pair::<Continue6>(b"LZ4_compressHC2_limitedOutput_continue\0") };
    let c_state = unsafe { create_c(input.as_ptr().cast()) };
    let r_state = unsafe { create_r(input.as_ptr().cast()) };
    let mut c_out = vec![0u8; bound];
    let mut r_out = vec![0u8; bound];
    let c_size = unsafe {
        c(
            c_state,
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
            10,
        )
    };
    let r_size = unsafe {
        r(
            r_state,
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
            bound as c_int,
            10,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
    assert_eq!(unsafe { free_c(c_state) }, unsafe { free_r(r_state) });
}

#[test]
fn frame_dictionary_begin_decompress_and_reset_variants_match() {
    unsafe {
        let libs = Libraries::load();
        type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
        type Free = unsafe extern "C" fn(*mut c_void) -> usize;
        type BeginDict = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            usize,
            *const c_void,
            usize,
            *const Preferences,
        ) -> usize;
        let (create_c, create_r) = libs.pair::<Create>(b"LZ4F_createCompressionContext\0");
        let (free_c, free_r) = libs.pair::<Free>(b"LZ4F_freeCompressionContext\0");
        let dictionary = patterned(4096);
        let prefs = Preferences::default();
        for name in [
            b"LZ4F_compressBegin_usingDict\0".as_slice(),
            b"LZ4F_compressBegin_usingDictOnce\0",
        ] {
            let (begin_c, begin_r) = libs.pair::<BeginDict>(name);
            let mut c_ctx = ptr::null_mut();
            let mut r_ctx = ptr::null_mut();
            assert_eq!(
                create_c(&mut c_ctx, LZ4F_VERSION),
                create_r(&mut r_ctx, LZ4F_VERSION)
            );
            let mut c_header = [0u8; 19];
            let mut r_header = [0u8; 19];
            let c_size = begin_c(
                c_ctx,
                c_header.as_mut_ptr().cast(),
                c_header.len(),
                dictionary.as_ptr().cast(),
                dictionary.len(),
                &prefs,
            );
            let r_size = begin_r(
                r_ctx,
                r_header.as_mut_ptr().cast(),
                r_header.len(),
                dictionary.as_ptr().cast(),
                dictionary.len(),
                &prefs,
            );
            assert_eq!(c_size, r_size);
            assert_eq!(&c_header[..c_size], &r_header[..r_size]);
            assert_eq!(free_c(c_ctx), free_r(r_ctx));
        }

        exercise_frame_cdict_begin(&libs, &dictionary, &prefs);
        exercise_frame_dictionary_decode_and_reset(&libs, &dictionary);
    }
}

unsafe fn exercise_frame_cdict_begin(libs: &Libraries, dictionary: &[u8], prefs: &Preferences) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type CreateDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
    type FreeDict = unsafe extern "C" fn(*mut c_void);
    type Begin = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        *const Preferences,
    ) -> usize;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createCompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeCompressionContext\0") };
    let (dict_c, dict_r) = unsafe { libs.pair::<CreateDict>(b"LZ4F_createCDict\0") };
    let (free_dict_c, free_dict_r) = unsafe { libs.pair::<FreeDict>(b"LZ4F_freeCDict\0") };
    let (begin_c, begin_r) = unsafe { libs.pair::<Begin>(b"LZ4F_compressBegin_usingCDict\0") };
    let c_dict = unsafe { dict_c(dictionary.as_ptr().cast(), dictionary.len()) };
    let r_dict = unsafe { dict_r(dictionary.as_ptr().cast(), dictionary.len()) };
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_ctx, LZ4F_VERSION)
    });
    let mut c_header = [0u8; 19];
    let mut r_header = [0u8; 19];
    let c_size = unsafe {
        begin_c(
            c_ctx,
            c_header.as_mut_ptr().cast(),
            c_header.len(),
            c_dict,
            prefs,
        )
    };
    let r_size = unsafe {
        begin_r(
            r_ctx,
            r_header.as_mut_ptr().cast(),
            r_header.len(),
            r_dict,
            prefs,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_header[..c_size], &r_header[..r_size]);
    unsafe {
        free_dict_c(c_dict);
        free_dict_r(r_dict);
    }
    assert_eq!(unsafe { free_c(c_ctx) }, unsafe { free_r(r_ctx) });
}

unsafe fn exercise_frame_dictionary_decode_and_reset(libs: &Libraries, dictionary: &[u8]) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Reset = unsafe extern "C" fn(*mut c_void);
    type Decompress = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut usize,
        *const c_void,
        *mut usize,
        *const c_void,
        usize,
        *const DecompressOptions,
    ) -> usize;
    let input = patterned(8192);
    let (c_frame, r_frame) = unsafe { compress_frame_pair(libs, &input, &Preferences::default()) };
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createDecompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeDecompressionContext\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4F_resetDecompressionContext\0") };
    let (decompress_c, decompress_r) =
        unsafe { libs.pair::<Decompress>(b"LZ4F_decompress_usingDict\0") };
    let mut c_ctx = ptr::null_mut();
    let mut r_ctx = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_ctx, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_ctx, LZ4F_VERSION)
    });
    unsafe {
        reset_c(c_ctx);
        reset_r(r_ctx);
    }
    let mut c_out = vec![0u8; input.len()];
    let mut r_out = vec![0u8; input.len()];
    let mut c_dst = input.len();
    let mut r_dst = input.len();
    let mut c_src = c_frame.len();
    let mut r_src = r_frame.len();
    assert_eq!(
        unsafe {
            decompress_c(
                c_ctx,
                c_out.as_mut_ptr().cast(),
                &mut c_dst,
                c_frame.as_ptr().cast(),
                &mut c_src,
                dictionary.as_ptr().cast(),
                dictionary.len(),
                ptr::null(),
            )
        },
        unsafe {
            decompress_r(
                r_ctx,
                r_out.as_mut_ptr().cast(),
                &mut r_dst,
                r_frame.as_ptr().cast(),
                &mut r_src,
                dictionary.as_ptr().cast(),
                dictionary.len(),
                ptr::null(),
            )
        }
    );
    assert_eq!(&c_out[..c_dst], &r_out[..r_dst]);
    assert_eq!(unsafe { free_c(c_ctx) }, unsafe { free_r(r_ctx) });
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
    let bound = unsafe { bound_c(input.len(), prefs) };
    assert_eq!(bound, unsafe { bound_r(input.len(), prefs) });
    let mut c_frame = vec![0u8; bound];
    let mut r_frame = vec![0u8; bound];
    let c_size = unsafe {
        compress_c(
            c_frame.as_mut_ptr().cast(),
            bound,
            input.as_ptr().cast(),
            input.len(),
            prefs,
        )
    };
    let r_size = unsafe {
        compress_r(
            r_frame.as_mut_ptr().cast(),
            bound,
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

#[test]
fn internal_stream_and_prefix_exports_match() {
    unsafe {
        let libs = Libraries::load();
        let input = patterned(16_384);
        let dictionary = patterned(8192);
        let bound = input.len() + input.len() / 255 + 16;

        exercise_internal_core_streams(&libs, &input, &dictionary, bound);
        exercise_prefix_and_stream_decoders(&libs, &input, &dictionary, bound);
        exercise_internal_hc_streams(&libs, &input, &dictionary, bound);
        exercise_internal_frame_begin(&libs);
    }
}

unsafe fn exercise_internal_core_streams(
    libs: &Libraries,
    input: &[u8],
    dictionary: &[u8],
    bound: usize,
) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void);
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, c_int) -> c_int;
    type Force = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;

    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStream\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStream\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_resetStream\0") };
    let (load_c, load_r) = unsafe { libs.pair::<Load>(b"LZ4_loadDict_internal\0") };
    let (force_c, force_r) = unsafe { libs.pair::<Force>(b"LZ4_compress_forceExtDict\0") };

    for mode in [0, 1] {
        let c_stream = unsafe { create_c() };
        let r_stream = unsafe { create_r() };
        assert!(!c_stream.is_null() && !r_stream.is_null());
        unsafe {
            reset_c(c_stream);
            reset_r(r_stream);
        }
        assert_eq!(
            unsafe {
                load_c(
                    c_stream,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode,
                )
            },
            unsafe {
                load_r(
                    r_stream,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode,
                )
            }
        );

        let mut c_out = vec![0u8; bound];
        let mut r_out = vec![0u8; bound];
        let c_size = unsafe {
            force_c(
                c_stream,
                input.as_ptr().cast(),
                c_out.as_mut_ptr().cast(),
                input.len() as c_int,
            )
        };
        let r_size = unsafe {
            force_r(
                r_stream,
                input.as_ptr().cast(),
                r_out.as_mut_ptr().cast(),
                input.len() as c_int,
            )
        };
        assert_eq!(c_size, r_size);
        assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
        assert_eq!(unsafe { free_c(c_stream) }, unsafe { free_r(r_stream) });
    }
}

unsafe fn exercise_prefix_and_stream_decoders(
    libs: &Libraries,
    input: &[u8],
    dictionary: &[u8],
    bound: usize,
) {
    let (compress_c, _) = unsafe { libs.pair::<Compress>(b"LZ4_compress_default\0") };
    let (compressed_size, mut compressed) = unsafe { compress_with(&compress_c, input, bound) };
    assert!(compressed_size > 0);
    compressed.truncate(compressed_size as usize);

    type PrefixSafe = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
    type PrefixFast = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
    let (safe_c, safe_r) =
        unsafe { libs.pair::<PrefixSafe>(b"LZ4_decompress_safe_withPrefix64k\0") };
    let (fast_c, fast_r) =
        unsafe { libs.pair::<PrefixFast>(b"LZ4_decompress_fast_withPrefix64k\0") };
    let prefix_size = 64 * 1024;
    let mut c_buffer = vec![0u8; prefix_size + input.len()];
    let mut r_buffer = vec![0u8; prefix_size + input.len()];
    c_buffer[..prefix_size].copy_from_slice(&patterned(prefix_size));
    r_buffer[..prefix_size].copy_from_slice(&patterned(prefix_size));
    let c_dst = unsafe { c_buffer.as_mut_ptr().add(prefix_size) };
    let r_dst = unsafe { r_buffer.as_mut_ptr().add(prefix_size) };
    let c_size = unsafe {
        safe_c(
            compressed.as_ptr().cast(),
            c_dst.cast(),
            compressed.len() as c_int,
            input.len() as c_int,
        )
    };
    let r_size = unsafe {
        safe_r(
            compressed.as_ptr().cast(),
            r_dst.cast(),
            compressed.len() as c_int,
            input.len() as c_int,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_buffer, r_buffer);

    c_buffer[prefix_size..].fill(0);
    r_buffer[prefix_size..].fill(0);
    let c_size = unsafe {
        fast_c(
            compressed.as_ptr().cast(),
            c_dst.cast(),
            input.len() as c_int,
        )
    };
    let r_size = unsafe {
        fast_r(
            compressed.as_ptr().cast(),
            r_dst.cast(),
            input.len() as c_int,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_buffer, r_buffer);

    type Partial = unsafe extern "C" fn(
        *const c_char,
        *mut c_char,
        c_int,
        c_int,
        c_int,
        *const c_void,
        usize,
    ) -> c_int;
    let (partial_c, partial_r) =
        unsafe { libs.pair::<Partial>(b"LZ4_decompress_safe_partial_forceExtDict\0") };
    let target = input.len() / 2;
    let mut c_partial = vec![0u8; target];
    let mut r_partial = vec![0u8; target];
    let c_size = unsafe {
        partial_c(
            compressed.as_ptr().cast(),
            c_partial.as_mut_ptr().cast(),
            compressed.len() as c_int,
            target as c_int,
            target as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len(),
        )
    };
    let r_size = unsafe {
        partial_r(
            compressed.as_ptr().cast(),
            r_partial.as_mut_ptr().cast(),
            compressed.len() as c_int,
            target as c_int,
            target as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len(),
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_partial, r_partial);

    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Set = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Continue = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStreamDecode\0") };
    let (set_c, set_r) = unsafe { libs.pair::<Set>(b"LZ4_setStreamDecode\0") };
    let (continue_c, continue_r) =
        unsafe { libs.pair::<Continue>(b"LZ4_decompress_fast_continue\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStreamDecode\0") };
    let c_stream = unsafe { create_c() };
    let r_stream = unsafe { create_r() };
    assert_eq!(unsafe { set_c(c_stream, ptr::null(), 0) }, unsafe {
        set_r(r_stream, ptr::null(), 0)
    });
    let mut c_out = vec![0u8; input.len()];
    let mut r_out = vec![0u8; input.len()];
    let c_size = unsafe {
        continue_c(
            c_stream,
            compressed.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            input.len() as c_int,
        )
    };
    let r_size = unsafe {
        continue_r(
            r_stream,
            compressed.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            input.len() as c_int,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(c_out, r_out);
    assert_eq!(unsafe { free_c(c_stream) }, unsafe { free_r(r_stream) });
}

unsafe fn exercise_internal_hc_streams(
    libs: &Libraries,
    input: &[u8],
    dictionary: &[u8],
    bound: usize,
) {
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Reset = unsafe extern "C" fn(*mut c_void, c_int);
    type ContinueDest =
        unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int) -> c_int;
    type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
    type Search = unsafe extern "C" fn(
        *const u8,
        c_uint,
        *const u8,
        *const u8,
        *const c_void,
        c_uint,
        c_int,
        c_int,
    ) -> HcMatch;

    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4_createStreamHC\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4_freeStreamHC\0") };
    let (reset_c, reset_r) = unsafe { libs.pair::<Reset>(b"LZ4_resetStreamHC\0") };
    let (continue_c, continue_r) =
        unsafe { libs.pair::<ContinueDest>(b"LZ4_compress_HC_continue_destSize\0") };
    let c_stream = unsafe { create_c() };
    let r_stream = unsafe { create_r() };
    unsafe {
        reset_c(c_stream, 10);
        reset_r(r_stream, 10);
    }
    let mut c_source_size = input.len() as c_int;
    let mut r_source_size = input.len() as c_int;
    let mut c_out = vec![0u8; bound];
    let mut r_out = vec![0u8; bound];
    let target = bound / 2;
    let c_size = unsafe {
        continue_c(
            c_stream,
            input.as_ptr().cast(),
            c_out.as_mut_ptr().cast(),
            &mut c_source_size,
            target as c_int,
        )
    };
    let r_size = unsafe {
        continue_r(
            r_stream,
            input.as_ptr().cast(),
            r_out.as_mut_ptr().cast(),
            &mut r_source_size,
            target as c_int,
        )
    };
    assert_eq!((c_size, c_source_size), (r_size, r_source_size));
    assert_eq!(&c_out[..c_size as usize], &r_out[..r_size as usize]);
    assert_eq!(unsafe { free_c(c_stream) }, unsafe { free_r(r_stream) });

    type Size = unsafe extern "C" fn() -> c_int;
    type ResetState = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
    type Slide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
    let (size_c, size_r) = unsafe { libs.pair::<Size>(b"LZ4_sizeofStreamStateHC\0") };
    assert_eq!(unsafe { size_c() }, unsafe { size_r() });
    let words = unsafe { size_c() } as usize / 8 + 2;
    let mut c_state = vec![0u64; words];
    let mut r_state = vec![0u64; words];
    let (reset_state_c, reset_state_r) =
        unsafe { libs.pair::<ResetState>(b"LZ4_resetStreamStateHC\0") };
    let (slide_c, slide_r) = unsafe { libs.pair::<Slide>(b"LZ4_slideInputBufferHC\0") };
    assert_eq!(
        unsafe {
            reset_state_c(
                c_state.as_mut_ptr().cast(),
                input.as_ptr().cast_mut().cast(),
            )
        },
        unsafe {
            reset_state_r(
                r_state.as_mut_ptr().cast(),
                input.as_ptr().cast_mut().cast(),
            )
        }
    );
    assert_eq!(unsafe { slide_c(c_state.as_mut_ptr().cast()) }, unsafe {
        slide_r(r_state.as_mut_ptr().cast())
    });

    let c_dict_stream = unsafe { create_c() };
    let r_dict_stream = unsafe { create_r() };
    let (load_c, load_r) = unsafe { libs.pair::<Load>(b"LZ4_loadDictHC\0") };
    assert_eq!(
        unsafe {
            load_c(
                c_dict_stream,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        },
        unsafe {
            load_r(
                r_dict_stream,
                dictionary.as_ptr().cast(),
                dictionary.len() as c_int,
            )
        }
    );
    let (search_c, search_r) = unsafe { libs.pair::<Search>(b"LZ4HC_searchExtDict\0") };
    let no_match = vec![0xa7; 128];
    let mut short_match = vec![0x5a; 128];
    short_match[..4].copy_from_slice(b"abcd");
    let long_match = patterned(128);
    for probe in [&no_match, &short_match, &long_match] {
        for attempts in [1, 32] {
            let c_match = unsafe {
                search_c(
                    probe.as_ptr(),
                    73_728,
                    probe.as_ptr(),
                    probe.as_ptr().add(probe.len()),
                    c_dict_stream,
                    73_728,
                    3,
                    attempts,
                )
            };
            let r_match = unsafe {
                search_r(
                    probe.as_ptr(),
                    73_728,
                    probe.as_ptr(),
                    probe.as_ptr().add(probe.len()),
                    r_dict_stream,
                    73_728,
                    3,
                    attempts,
                )
            };
            assert_eq!(c_match, r_match);
        }
    }
    assert_eq!(unsafe { free_c(c_dict_stream) }, unsafe {
        free_r(r_dict_stream)
    });
}

unsafe fn exercise_internal_frame_begin(libs: &Libraries) {
    type Create = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
    type Free = unsafe extern "C" fn(*mut c_void) -> usize;
    type Begin = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        usize,
        *const c_void,
        usize,
        *const c_void,
        *const Preferences,
    ) -> usize;
    let (create_c, create_r) = unsafe { libs.pair::<Create>(b"LZ4F_createCompressionContext\0") };
    let (free_c, free_r) = unsafe { libs.pair::<Free>(b"LZ4F_freeCompressionContext\0") };
    let (begin_c, begin_r) = unsafe { libs.pair::<Begin>(b"LZ4F_compressBegin_internal\0") };
    let mut c_context = ptr::null_mut();
    let mut r_context = ptr::null_mut();
    assert_eq!(unsafe { create_c(&mut c_context, LZ4F_VERSION) }, unsafe {
        create_r(&mut r_context, LZ4F_VERSION)
    });
    let prefs = Preferences::default();
    let mut c_header = [0u8; 19];
    let mut r_header = [0u8; 19];
    let c_size = unsafe {
        begin_c(
            c_context,
            c_header.as_mut_ptr().cast(),
            c_header.len(),
            ptr::null(),
            0,
            ptr::null(),
            &prefs,
        )
    };
    let r_size = unsafe {
        begin_r(
            r_context,
            r_header.as_mut_ptr().cast(),
            r_header.len(),
            ptr::null(),
            0,
            ptr::null(),
            &prefs,
        )
    };
    assert_eq!(c_size, r_size);
    assert_eq!(&c_header[..c_size], &r_header[..r_size]);
    assert_eq!(unsafe { free_c(c_context) }, unsafe { free_r(r_context) });
}
