mod common;

use common::{Libraries, Rng, ptr_or_dangling};
use std::ffi::{c_char, c_int, c_uint, c_void};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HcMatch {
    off: c_int,
    len: c_int,
    back: c_int,
}

#[test]
fn deprecated_and_external_state_compressors_match() {
    unsafe {
        let libs = Libraries::load();
        let (c_bound, _) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let (c_size, r_size) = libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofState\0");
        let (c_hc_size, r_hc_size) =
            libs.pair::<unsafe extern "C" fn() -> c_int>(b"LZ4_sizeofStateHC\0");
        let mut rng = Rng::new(0xd131_0ba6_98df_b5ac);

        for _ in 0..30 {
            let len = (rng.next_u64() as usize) % 16_000;
            let mut input = rng.bytes(len);
            if rng.next_u64() & 1 == 0 {
                for byte in &mut input {
                    *byte &= 15;
                }
            }
            let bound = c_bound(len as c_int);
            for name in [
                b"LZ4_compressHC_limitedOutput\0".as_slice(),
                b"LZ4_compress_limitedOutput\0".as_slice(),
            ] {
                compare4(&libs, name, &input, bound);
                compare4(&libs, name, &input, (bound / 2).max(1));
            }
            for name in [b"LZ4_compressHC2\0".as_slice()] {
                compare4(&libs, name, &input, 10);
            }
            for name in [b"LZ4_compressHC2_limitedOutput\0".as_slice()] {
                compare5(&libs, name, &input, bound, 10);
                compare5(&libs, name, &input, (bound / 2).max(1), 12);
            }

            let mut cs = vec![0u64; (c_size() as usize + 7) / 8];
            let mut rs = vec![0u64; (r_size() as usize + 7) / 8];
            for name in [b"LZ4_compress_limitedOutput_withState\0".as_slice()] {
                compare_state5(&libs, name, &mut cs, &mut rs, &input, bound);
            }
            let (c_with, r_with) = libs.pair::<unsafe extern "C" fn(
                *mut c_void,
                *const c_char,
                *mut c_char,
                c_int,
            ) -> c_int>(b"LZ4_compress_withState\0");
            let mut co = vec![0u8; bound as usize];
            let mut ro = vec![0u8; bound as usize];
            let cn = c_with(
                cs.as_mut_ptr().cast(),
                ptr_or_dangling(&input).cast(),
                co.as_mut_ptr().cast(),
                len as c_int,
            );
            let rn = r_with(
                rs.as_mut_ptr().cast(),
                ptr_or_dangling(&input).cast(),
                ro.as_mut_ptr().cast(),
                len as c_int,
            );
            assert_eq!(cn, rn);
            assert_eq!(&co[..cn as usize], &ro[..rn as usize]);

            let mut ch = vec![0u64; (c_hc_size() as usize + 7) / 8];
            let mut rh = vec![0u64; (r_hc_size() as usize + 7) / 8];
            for name in [b"LZ4_compressHC_limitedOutput_withStateHC\0".as_slice()] {
                compare_state5(&libs, name, &mut ch, &mut rh, &input, bound);
            }
            for name in [b"LZ4_compressHC2_withStateHC\0".as_slice()] {
                compare_state5(&libs, name, &mut ch, &mut rh, &input, 10);
            }
            let (c_hc6, r_hc6) =
                libs.pair::<unsafe extern "C" fn(
                    *mut c_void,
                    *const c_char,
                    *mut c_char,
                    c_int,
                    c_int,
                    c_int,
                ) -> c_int>(b"LZ4_compressHC2_limitedOutput_withStateHC\0");
            let mut co = vec![0u8; bound as usize];
            let mut ro = vec![0u8; bound as usize];
            let cn = c_hc6(
                ch.as_mut_ptr().cast(),
                ptr_or_dangling(&input).cast(),
                co.as_mut_ptr().cast(),
                len as c_int,
                bound,
                12,
            );
            let rn = r_hc6(
                rh.as_mut_ptr().cast(),
                ptr_or_dangling(&input).cast(),
                ro.as_mut_ptr().cast(),
                len as c_int,
                bound,
                12,
            );
            assert_eq!(cn, rn);
            assert_eq!(&co[..cn as usize], &ro[..rn as usize]);

            let (c_fast_reset, r_fast_reset) =
                libs.pair::<unsafe extern "C" fn(
                    *mut c_void,
                    *const c_char,
                    *mut c_char,
                    c_int,
                    c_int,
                    c_int,
                ) -> c_int>(b"LZ4_compress_fast_extState_fastReset\0");
            for acceleration in [1, 7] {
                let mut co = vec![0u8; bound as usize];
                let mut ro = vec![0u8; bound as usize];
                let cn = c_fast_reset(
                    cs.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    len as c_int,
                    bound,
                    acceleration,
                );
                let rn = r_fast_reset(
                    rs.as_mut_ptr().cast(),
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    len as c_int,
                    bound,
                    acceleration,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
            }
        }
    }
}

unsafe fn compare4(libs: &Libraries, name: &[u8], input: &[u8], option: c_int) {
    let (c, r) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int>(name)
    };
    let capacity = if name == b"LZ4_compressHC2\0" {
        unsafe {
            libs.c
                .get::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0")
                .unwrap()(input.len() as c_int)
        }
    } else {
        option
    };
    let mut co = vec![0u8; capacity.max(1) as usize];
    let mut ro = vec![0u8; capacity.max(1) as usize];
    let cn = unsafe {
        c(
            ptr_or_dangling(input).cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            option,
        )
    };
    let rn = unsafe {
        r(
            ptr_or_dangling(input).cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            option,
        )
    };
    assert_eq!(cn, rn, "{name:?}");
    if cn > 0 {
        assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
    }
}

unsafe fn compare5(libs: &Libraries, name: &[u8], input: &[u8], capacity: c_int, level: c_int) {
    let (c, r) = unsafe {
        libs.pair::<unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int>(
            name,
        )
    };
    let mut co = vec![0u8; capacity.max(1) as usize];
    let mut ro = vec![0u8; capacity.max(1) as usize];
    let cn = unsafe {
        c(
            ptr_or_dangling(input).cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
            level,
        )
    };
    let rn = unsafe {
        r(
            ptr_or_dangling(input).cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            capacity,
            level,
        )
    };
    assert_eq!(cn, rn);
    if cn > 0 {
        assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
    }
}

unsafe fn compare_state5(
    libs: &Libraries,
    name: &[u8],
    cs: &mut [u64],
    rs: &mut [u64],
    input: &[u8],
    option: c_int,
) {
    let (c, r) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int>(name)
    };
    let capacity = unsafe {
        libs.c
            .get::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0")
            .unwrap()(input.len() as c_int)
    };
    let output_capacity = if name == b"LZ4_compressHC2_withStateHC\0" {
        capacity
    } else {
        option
    };
    let mut co = vec![0u8; output_capacity.max(1) as usize];
    let mut ro = vec![0u8; output_capacity.max(1) as usize];
    let cn = unsafe {
        c(
            cs.as_mut_ptr().cast(),
            ptr_or_dangling(input).cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            option,
        )
    };
    let rn = unsafe {
        r(
            rs.as_mut_ptr().cast(),
            ptr_or_dangling(input).cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            option,
        )
    };
    assert_eq!(cn, rn, "{name:?}");
    if cn > 0 {
        assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
    }
}

#[test]
fn dictionary_and_fast_decoder_variants_match() {
    unsafe {
        let libs = Libraries::load();
        let mut rng = Rng::new(0x2ffd_72db_d01a_dfb7);
        for _ in 0..30 {
            let dictionary = rng.bytes(70_000);
            let mut input = Vec::new();
            for _ in 0..32 {
                let start = rng.next_u64() as usize % (dictionary.len() - 128);
                input.extend_from_slice(&dictionary[start..start + 128]);
            }
            let (c_create, r_create) =
                libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_createStream\0");
            let (c_load, r_load) = libs.pair::<unsafe extern "C" fn(
                *mut c_void,
                *const c_char,
                c_int,
                c_int,
            ) -> c_int>(b"LZ4_loadDict_internal\0");
            let (c_bound, _) =
                libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
            let (c_compress, r_compress) =
                libs.pair::<unsafe extern "C" fn(
                    *mut c_void,
                    *const c_char,
                    *mut c_char,
                    c_int,
                    c_int,
                    c_int,
                ) -> c_int>(b"LZ4_compress_fast_continue\0");
            let (c_free, r_free) =
                libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeStream\0");
            let cs = c_create();
            let rs = r_create();
            let mode = (rng.next_u64() & 1) as c_int;
            assert_eq!(
                c_load(
                    cs,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode
                ),
                r_load(
                    rs,
                    dictionary.as_ptr().cast(),
                    dictionary.len() as c_int,
                    mode
                )
            );
            let bound = c_bound(input.len() as c_int);
            let mut cc = vec![0u8; bound as usize];
            let mut rc = vec![0u8; bound as usize];
            let cn = c_compress(
                cs,
                input.as_ptr().cast(),
                cc.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
                1,
            );
            let rn = r_compress(
                rs,
                input.as_ptr().cast(),
                rc.as_mut_ptr().cast(),
                input.len() as c_int,
                bound,
                1,
            );
            assert_eq!(cn, rn);
            assert_eq!(&cc[..cn as usize], &rc[..rn as usize]);

            compare_dict_decoders(
                &libs,
                &cc[..cn as usize],
                &rc[..rn as usize],
                &input,
                &dictionary,
            );
            assert_eq!(c_free(cs), r_free(rs));
        }
    }
}

unsafe fn compare_dict_decoders(
    libs: &Libraries,
    cc: &[u8],
    rc: &[u8],
    original: &[u8],
    dictionary: &[u8],
) {
    let (c_safe, r_safe) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int>(b"LZ4_decompress_safe_usingDict\0")
    };
    let mut co = vec![0u8; original.len()];
    let mut ro = vec![0u8; original.len()];
    let cd = unsafe {
        c_safe(
            cc.as_ptr().cast(),
            co.as_mut_ptr().cast(),
            cc.len() as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    let rd = unsafe {
        r_safe(
            rc.as_ptr().cast(),
            ro.as_mut_ptr().cast(),
            rc.len() as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    assert_eq!(cd, rd);
    assert_eq!(co, ro);
    assert_eq!(co, original);

    let (c_force, r_force) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            *const c_void,
            usize,
        ) -> c_int>(b"LZ4_decompress_safe_forceExtDict\0")
    };
    co.fill(0);
    ro.fill(0);
    let cd = unsafe {
        c_force(
            cc.as_ptr().cast(),
            co.as_mut_ptr().cast(),
            cc.len() as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len(),
        )
    };
    let rd = unsafe {
        r_force(
            rc.as_ptr().cast(),
            ro.as_mut_ptr().cast(),
            rc.len() as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len(),
        )
    };
    assert_eq!(cd, rd);
    assert_eq!(co, ro);

    let (c_partial, r_partial) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int>(b"LZ4_decompress_safe_partial_usingDict\0")
    };
    let target = original.len() / 2;
    co.fill(0);
    ro.fill(0);
    let cd = unsafe {
        c_partial(
            cc.as_ptr().cast(),
            co.as_mut_ptr().cast(),
            cc.len() as c_int,
            target as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    let rd = unsafe {
        r_partial(
            rc.as_ptr().cast(),
            ro.as_mut_ptr().cast(),
            rc.len() as c_int,
            target as c_int,
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    assert_eq!(cd, rd);
    assert_eq!(&co[..cd as usize], &ro[..rd as usize]);

    let (c_fast, r_fast) = unsafe {
        libs.pair::<unsafe extern "C" fn(
            *const c_char,
            *mut c_char,
            c_int,
            *const c_char,
            c_int,
        ) -> c_int>(b"LZ4_decompress_fast_usingDict\0")
    };
    co.fill(0);
    ro.fill(0);
    let cd = unsafe {
        c_fast(
            cc.as_ptr().cast(),
            co.as_mut_ptr().cast(),
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    let rd = unsafe {
        r_fast(
            rc.as_ptr().cast(),
            ro.as_mut_ptr().cast(),
            original.len() as c_int,
            dictionary.as_ptr().cast(),
            dictionary.len() as c_int,
        )
    };
    assert_eq!(cd, rd);
    assert_eq!(co, ro);
    assert_eq!(co, original);
}

#[test]
fn old_context_fill_output_and_exported_hc_search_match() {
    unsafe {
        let libs = Libraries::load();
        let input = vec![b'x'; 8192];
        let (c_bound, _) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let bound = c_bound(input.len() as c_int);

        let (c_create, r_create) =
            libs.pair::<unsafe extern "C" fn(*mut c_char) -> *mut c_void>(b"LZ4_create\0");
        let (c_free, r_free) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeStream\0");
        let cs = c_create(input.as_ptr() as *mut c_char);
        let rs = r_create(input.as_ptr() as *mut c_char);
        let (c_reset, r_reset) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int>(
                b"LZ4_resetStreamState\0",
            );
        assert_eq!(
            c_reset(cs, input.as_ptr() as *mut c_char),
            r_reset(rs, input.as_ptr() as *mut c_char)
        );
        let (c_slide, r_slide) = libs
            .pair::<unsafe extern "C" fn(*mut c_void) -> *mut c_char>(b"LZ4_slideInputBuffer\0");
        assert_eq!(c_slide(cs).is_null(), r_slide(rs).is_null());
        assert_eq!(c_free(cs), r_free(rs));

        let (c_hc_create, r_hc_create) =
            libs.pair::<unsafe extern "C" fn(*const c_char) -> *mut c_void>(b"LZ4_createHC\0");
        let (c_hc_free, r_hc_free) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeHC\0");
        let ch = c_hc_create(input.as_ptr().cast());
        let rh = r_hc_create(input.as_ptr().cast());
        let (c_hc_continue, r_hc_continue) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int>(b"LZ4_compressHC2_continue\0");
        let mut co = vec![0u8; bound as usize];
        let mut ro = vec![0u8; bound as usize];
        let cn = c_hc_continue(
            ch,
            input.as_ptr().cast(),
            co.as_mut_ptr().cast(),
            input.len() as c_int,
            9,
        );
        let rn = r_hc_continue(
            rh,
            input.as_ptr().cast(),
            ro.as_mut_ptr().cast(),
            input.len() as c_int,
            9,
        );
        assert_eq!(cn, rn);
        assert_eq!(&co[..cn as usize], &ro[..rn as usize]);

        let (c_search, r_search) = libs.pair::<unsafe extern "C" fn(
            *const u8,
            c_uint,
            *const u8,
            *const u8,
            *const c_void,
            c_uint,
            c_int,
            c_int,
        ) -> HcMatch>(b"LZ4HC_searchExtDict\0");
        let c_match = c_search(
            input.as_ptr(),
            65_536,
            input.as_ptr(),
            input.as_ptr().add(input.len()),
            ch,
            65_536,
            3,
            0,
        );
        let r_match = r_search(
            input.as_ptr(),
            65_536,
            input.as_ptr(),
            input.as_ptr().add(input.len()),
            rh,
            65_536,
            3,
            0,
        );
        assert_eq!(c_match, r_match);
        assert_eq!(c_hc_free(ch), r_hc_free(rh));
    }
}
