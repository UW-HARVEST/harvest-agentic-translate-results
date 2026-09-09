mod common;

use common::{Libraries, Rng, ptr_or_dangling};
use std::ffi::{c_char, c_int, c_void};

#[test]
fn fast_streaming_dictionary_and_decode_lifecycles_match() {
    unsafe {
        let libs = Libraries::load();
        let (c_create, r_create) =
            libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_createStream\0");
        let (c_free, r_free) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeStream\0");
        let (c_load, r_load) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int>(
                b"LZ4_loadDict\0",
            );
        let (c_load_slow, r_load_slow) =
            libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int>(
                b"LZ4_loadDictSlow\0",
            );
        let (c_continue, r_continue) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
            c_int,
        ) -> c_int>(b"LZ4_compress_fast_continue\0");
        let (c_save, r_save) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int>(
                b"LZ4_saveDict\0",
            );
        let (c_attach, r_attach) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *const c_void)>(b"LZ4_attach_dictionary\0");
        let (c_bound, _) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");

        let mut rng = Rng::new(0x082e_fa98_ec4e_6c89);
        for slow in [false, true] {
            let dictionary = rng.bytes(70_000);
            let cs = c_create();
            let rs = r_create();
            let cd = c_create();
            let rd = r_create();
            assert!(!cs.is_null() && !rs.is_null() && !cd.is_null() && !rd.is_null());
            let loaded_c = if slow {
                c_load_slow(cd, dictionary.as_ptr().cast(), dictionary.len() as c_int)
            } else {
                c_load(cd, dictionary.as_ptr().cast(), dictionary.len() as c_int)
            };
            let loaded_r = if slow {
                r_load_slow(rd, dictionary.as_ptr().cast(), dictionary.len() as c_int)
            } else {
                r_load(rd, dictionary.as_ptr().cast(), dictionary.len() as c_int)
            };
            assert_eq!(loaded_c, loaded_r);
            c_attach(cs, cd);
            r_attach(rs, rd);

            let mut compressed_c = Vec::new();
            let mut compressed_r = Vec::new();
            let mut originals = Vec::new();
            for block_index in 0..12 {
                let len = (rng.next_u64() as usize) % 8193;
                let mut input = rng.bytes(len);
                if block_index % 2 == 0 {
                    for byte in &mut input {
                        *byte &= 15;
                    }
                }
                let bound = c_bound(len as c_int);
                let mut co = vec![0u8; bound as usize];
                let mut ro = vec![0u8; bound as usize];
                let acceleration = [0, 1, 3, 65_538][block_index % 4];
                let cn = c_continue(
                    cs,
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    len as c_int,
                    bound,
                    acceleration,
                );
                let rn = r_continue(
                    rs,
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    len as c_int,
                    bound,
                    acceleration,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
                co.truncate(cn as usize);
                ro.truncate(rn as usize);
                compressed_c.push(co);
                compressed_r.push(ro);
                originals.push(input);
            }

            let mut c_safe = vec![0u8; 65_536];
            let mut r_safe = vec![0u8; 65_536];
            assert_eq!(
                c_save(cs, c_safe.as_mut_ptr().cast(), c_safe.len() as c_int),
                r_save(rs, r_safe.as_mut_ptr().cast(), r_safe.len() as c_int)
            );

            let (c_dec_create, r_dec_create) =
                libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_createStreamDecode\0");
            let (c_dec_free, r_dec_free) =
                libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeStreamDecode\0");
            let (c_dec_set, r_dec_set) =
                libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int>(
                    b"LZ4_setStreamDecode\0",
                );
            let (c_dec, r_dec) = libs.pair::<unsafe extern "C" fn(
                *mut c_void,
                *const c_char,
                *mut c_char,
                c_int,
                c_int,
            ) -> c_int>(b"LZ4_decompress_safe_continue\0");
            let cdc = c_dec_create();
            let rdc = r_dec_create();
            assert_eq!(
                c_dec_set(cdc, dictionary.as_ptr().cast(), dictionary.len() as c_int),
                r_dec_set(rdc, dictionary.as_ptr().cast(), dictionary.len() as c_int)
            );
            let mut c_outputs: Vec<Vec<u8>> = Vec::new();
            let mut r_outputs: Vec<Vec<u8>> = Vec::new();
            for ((cc, rc), original) in compressed_c.iter().zip(&compressed_r).zip(&originals) {
                let mut co = vec![0u8; original.len().max(1)];
                let mut ro = vec![0u8; original.len().max(1)];
                let cn = c_dec(
                    cdc,
                    cc.as_ptr().cast(),
                    co.as_mut_ptr().cast(),
                    cc.len() as c_int,
                    original.len() as c_int,
                );
                let rn = r_dec(
                    rdc,
                    rc.as_ptr().cast(),
                    ro.as_mut_ptr().cast(),
                    rc.len() as c_int,
                    original.len() as c_int,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn.max(0) as usize], &ro[..rn.max(0) as usize]);
                if cn >= 0 {
                    assert_eq!(&co[..cn as usize], original);
                }
                c_outputs.push(co);
                r_outputs.push(ro);
            }
            assert_eq!(c_dec_free(cdc), r_dec_free(rdc));
            assert_eq!(c_free(cs), r_free(rs));
            assert_eq!(c_free(cd), r_free(rd));
            assert_eq!(c_free(std::ptr::null_mut()), r_free(std::ptr::null_mut()));
        }
    }
}

#[test]
fn hc_streaming_levels_flags_and_dictionaries_match() {
    unsafe {
        let libs = Libraries::load();
        let (c_create, r_create) =
            libs.pair::<unsafe extern "C" fn() -> *mut c_void>(b"LZ4_createStreamHC\0");
        let (c_free, r_free) =
            libs.pair::<unsafe extern "C" fn(*mut c_void) -> c_int>(b"LZ4_freeStreamHC\0");
        let (c_reset, r_reset) =
            libs.pair::<unsafe extern "C" fn(*mut c_void, c_int)>(b"LZ4_resetStreamHC_fast\0");
        let (c_level, r_level) =
            libs.pair::<unsafe extern "C" fn(*mut c_void, c_int)>(b"LZ4_setCompressionLevel\0");
        let (c_favor, r_favor) =
            libs.pair::<unsafe extern "C" fn(*mut c_void, c_int)>(b"LZ4_favorDecompressionSpeed\0");
        let (c_load, r_load) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int>(
                b"LZ4_loadDictHC\0",
            );
        let (c_attach, r_attach) = libs.pair::<unsafe extern "C" fn(*mut c_void, *const c_void)>(
            b"LZ4_attach_HC_dictionary\0",
        );
        let (c_continue, r_continue) = libs.pair::<unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *mut c_char,
            c_int,
            c_int,
        ) -> c_int>(b"LZ4_compress_HC_continue\0");
        let (c_save, r_save) = libs
            .pair::<unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int>(
                b"LZ4_saveDictHC\0",
            );
        let (c_bound, _) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        let mut rng = Rng::new(0x4528_21e6_38d0_1377);
        let dictionary = rng.bytes(70_000);

        for level in [2, 9, 10, 12, 99] {
            for favor in [0, 1] {
                let cs = c_create();
                let rs = r_create();
                let cd = c_create();
                let rd = r_create();
                c_reset(cs, level);
                r_reset(rs, level);
                c_level(cs, level);
                r_level(rs, level);
                c_favor(cs, favor);
                r_favor(rs, favor);
                assert_eq!(
                    c_load(cd, dictionary.as_ptr().cast(), dictionary.len() as c_int),
                    r_load(rd, dictionary.as_ptr().cast(), dictionary.len() as c_int)
                );
                c_attach(cs, cd);
                r_attach(rs, rd);
                let mut inputs = Vec::new();
                for _ in 0..8 {
                    let len = (rng.next_u64() as usize) % 6000;
                    let mut input = rng.bytes(len);
                    for byte in &mut input {
                        *byte &= 31;
                    }
                    inputs.push(input);
                    let input = inputs.last().unwrap();
                    let bound = c_bound(len as c_int);
                    let mut co = vec![0u8; bound as usize];
                    let mut ro = vec![0u8; bound as usize];
                    let cn = c_continue(
                        cs,
                        ptr_or_dangling(&input).cast(),
                        co.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                    );
                    let rn = r_continue(
                        rs,
                        ptr_or_dangling(&input).cast(),
                        ro.as_mut_ptr().cast(),
                        len as c_int,
                        bound,
                    );
                    assert_eq!(cn, rn);
                    assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
                }
                let mut cb = vec![0u8; 65_536];
                let mut rb = vec![0u8; 65_536];
                let cn = c_save(cs, cb.as_mut_ptr().cast(), cb.len() as c_int);
                let rn = r_save(rs, rb.as_mut_ptr().cast(), rb.len() as c_int);
                assert_eq!(cn, rn);
                assert_eq!(&cb[..cn as usize], &rb[..rn as usize]);
                assert_eq!(c_free(cs), r_free(rs));
                assert_eq!(c_free(cd), r_free(rd));
            }
        }
    }
}

#[test]
fn init_alignment_and_legacy_compatibility_exports_match() {
    unsafe {
        let libs = Libraries::load();
        for (size_name, init_name) in [
            (
                b"LZ4_sizeofState\0".as_slice(),
                b"LZ4_initStream\0".as_slice(),
            ),
            (
                b"LZ4_sizeofStateHC\0".as_slice(),
                b"LZ4_initStreamHC\0".as_slice(),
            ),
        ] {
            let (c_size, r_size) = libs.pair::<unsafe extern "C" fn() -> c_int>(size_name);
            let (c_init, r_init) =
                libs.pair::<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>(init_name);
            let size = c_size() as usize;
            assert_eq!(size, r_size() as usize);
            let mut cb = vec![0u64; (size + 15) / 8];
            let mut rb = vec![0u64; (size + 15) / 8];
            assert_eq!(
                c_init(cb.as_mut_ptr().cast(), size).is_null(),
                r_init(rb.as_mut_ptr().cast(), size).is_null()
            );
            assert_eq!(
                c_init((cb.as_mut_ptr() as *mut u8).add(1).cast(), size - 1).is_null(),
                r_init((rb.as_mut_ptr() as *mut u8).add(1).cast(), size - 1).is_null()
            );
        }

        let mut rng = Rng::new(0xbe54_66cf_34e9_0c6c);
        for _ in 0..40 {
            let len = (rng.next_u64() as usize) % 10_000;
            let input = rng.bytes(len);
            let (c_bound, _) =
                libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
            let bound = c_bound(len as c_int);
            for name in [b"LZ4_compress\0".as_slice(), b"LZ4_compressHC\0".as_slice()] {
                let (c, r) = libs
                    .pair::<unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int>(name);
                let mut co = vec![0u8; bound as usize];
                let mut ro = vec![0u8; bound as usize];
                let cn = c(
                    ptr_or_dangling(&input).cast(),
                    co.as_mut_ptr().cast(),
                    len as c_int,
                );
                let rn = r(
                    ptr_or_dangling(&input).cast(),
                    ro.as_mut_ptr().cast(),
                    len as c_int,
                );
                assert_eq!(cn, rn);
                assert_eq!(&co[..cn as usize], &ro[..rn as usize]);
            }
        }
    }
}
