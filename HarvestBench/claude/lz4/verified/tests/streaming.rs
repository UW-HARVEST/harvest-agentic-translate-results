// Phase B — streaming block API differential tests (lz4.c / lz4hc.c).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

type CreateStream = unsafe extern "C" fn() -> *mut c_void;
type FreeStream = unsafe extern "C" fn(*mut c_void) -> c_int;
type LoadDict = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type CompressContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
type HCContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type CompressBound = unsafe extern "C" fn(c_int) -> c_int;
type DecompressUsingDict = unsafe extern "C" fn(
    *const c_char,
    *mut c_char,
    c_int,
    c_int,
    *const c_char,
    c_int,
) -> c_int;
type SetStreamDecode = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
type DecompressContinue =
    unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
type ResetStreamHCFast = unsafe extern "C" fn(*mut c_void, c_int);
type FavorDecSpeed = unsafe extern "C" fn(*mut c_void, c_int);

/// Compress a sequence of blocks with LZ4_compress_fast_continue on both C and Rust
/// (each keeps its own stream), assert compressed blocks match, then decompress with
/// LZ4_decompress_safe_continue and assert roundtrip.
#[test]
fn test_stream_fast_continue_and_decode() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x51ea);
    unsafe {
        let c_create: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStream");
        let r_create: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStream");
        let c_free: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStream");
        let r_free: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStream");
        let c_cont: libloading::Symbol<CompressContinue> = csym(&libs, b"LZ4_compress_fast_continue");
        let r_cont: libloading::Symbol<CompressContinue> = rsym(&libs, b"LZ4_compress_fast_continue");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        // decode-side (use C decode as reference; both must match)
        let c_dcreate: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStreamDecode");
        let r_dcreate: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStreamDecode");
        let c_dfree: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStreamDecode");
        let r_dfree: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStreamDecode");
        let c_dcont: libloading::Symbol<DecompressContinue> = csym(&libs, b"LZ4_decompress_safe_continue");
        let r_dcont: libloading::Symbol<DecompressContinue> = rsym(&libs, b"LZ4_decompress_safe_continue");

        for accel in [1i32, 3, 50] {
            // Contiguous buffer of several blocks so history is preserved at same address.
            let nblocks = 5usize;
            let blocksz = 3000usize;
            let total = nblocks * blocksz;
            let data = rng.compressible(total);

            let cs = c_create();
            let rs = r_create();
            assert!(!cs.is_null() && !rs.is_null());

            let mut c_blocks: Vec<Vec<u8>> = Vec::new();
            let mut r_blocks: Vec<Vec<u8>> = Vec::new();

            for b in 0..nblocks {
                let off = b * blocksz;
                let cap = cbound(blocksz as c_int) as usize;
                let mut cdst = vec![0u8; cap];
                let mut rdst = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let cn = c_cont(cs, src, cdst.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, accel);
                let rn = r_cont(rs, src, rdst.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, accel);
                assert_eq!(cn, rn, "continue block {} accel {}", b, accel);
                assert!(cn > 0);
                cdst.truncate(cn as usize);
                rdst.truncate(rn as usize);
                assert_eq!(cdst, rdst, "continue bytes block {} accel {}", b, accel);
                c_blocks.push(cdst);
                r_blocks.push(rdst);
            }

            // Decode with streaming decode into a contiguous output buffer.
            let mut c_out = vec![0u8; total];
            let mut r_out = vec![0u8; total];
            let cds = c_dcreate();
            let rds = r_dcreate();
            for b in 0..nblocks {
                let off = b * blocksz;
                let cdn = c_dcont(cds, c_blocks[b].as_ptr() as *const c_char, c_out.as_mut_ptr().add(off) as *mut c_char, c_blocks[b].len() as c_int, blocksz as c_int);
                let rdn = r_dcont(rds, r_blocks[b].as_ptr() as *const c_char, r_out.as_mut_ptr().add(off) as *mut c_char, r_blocks[b].len() as c_int, blocksz as c_int);
                assert_eq!(cdn, rdn, "decode continue block {}", b);
                assert_eq!(cdn as usize, blocksz);
            }
            assert_eq!(c_out, data, "C stream decode mismatch");
            assert_eq!(r_out, data, "Rust stream decode mismatch");
            c_dfree(cds);
            r_dfree(rds);
            c_free(cs);
            r_free(rs);
        }
    }
}

#[test]
fn test_loaddict_and_usingdict() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xd1c7);
    unsafe {
        let c_create: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStream");
        let r_create: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStream");
        let c_free: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStream");
        let r_free: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStream");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let c_dud: libloading::Symbol<DecompressUsingDict> = csym(&libs, b"LZ4_decompress_safe_usingDict");
        let r_dud: libloading::Symbol<DecompressUsingDict> = rsym(&libs, b"LZ4_decompress_safe_usingDict");

        for loaddict_name in [&b"LZ4_loadDict"[..], &b"LZ4_loadDictSlow"[..]] {
            let c_load: libloading::Symbol<LoadDict> = csym(&libs, loaddict_name);
            let r_load: libloading::Symbol<LoadDict> = rsym(&libs, loaddict_name);
            let c_cont: libloading::Symbol<CompressContinue> = csym(&libs, b"LZ4_compress_fast_continue");
            let r_cont: libloading::Symbol<CompressContinue> = rsym(&libs, b"LZ4_compress_fast_continue");

            for &dictsz in &[0usize, 100, 4096, 70000] {
                let dict = rng.compressible(dictsz);
                let msg = rng.compressible(2000);
                let cap = cbound(msg.len() as c_int) as usize;

                let cs = c_create();
                let rs = r_create();
                let cl = c_load(cs, dict.as_ptr() as *const c_char, dictsz as c_int);
                let rl = r_load(rs, dict.as_ptr() as *const c_char, dictsz as c_int);
                assert_eq!(cl, rl, "loadDict ret dictsz {}", dictsz);

                let mut cdst = vec![0u8; cap];
                let mut rdst = vec![0u8; cap];
                let cn = c_cont(cs, msg.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, msg.len() as c_int, cap as c_int, 1);
                let rn = r_cont(rs, msg.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, msg.len() as c_int, cap as c_int, 1);
                assert_eq!(cn, rn, "dict continue ret dictsz {}", dictsz);
                assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "dict continue bytes dictsz {}", dictsz);

                // decompress with dict
                let mut cout = vec![0u8; msg.len()];
                let mut rout = vec![0u8; msg.len()];
                let cd = c_dud(cdst.as_ptr() as *const c_char, cout.as_mut_ptr() as *mut c_char, cn, msg.len() as c_int, dict.as_ptr() as *const c_char, dictsz as c_int);
                let rd = r_dud(rdst.as_ptr() as *const c_char, rout.as_mut_ptr() as *mut c_char, rn, msg.len() as c_int, dict.as_ptr() as *const c_char, dictsz as c_int);
                assert_eq!(cd, rd, "decode usingDict ret dictsz {}", dictsz);
                assert_eq!(cd as usize, msg.len());
                assert_eq!(cout, msg);
                assert_eq!(rout, msg);

                c_free(cs);
                r_free(rs);
            }
        }
    }
}

#[test]
fn test_hc_continue() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x8c00);
    unsafe {
        let c_create: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStreamHC");
        let r_create: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStreamHC");
        let c_free: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStreamHC");
        let r_free: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStreamHC");
        let c_reset: libloading::Symbol<ResetStreamHCFast> = csym(&libs, b"LZ4_resetStreamHC_fast");
        let r_reset: libloading::Symbol<ResetStreamHCFast> = rsym(&libs, b"LZ4_resetStreamHC_fast");
        let c_cont: libloading::Symbol<HCContinue> = csym(&libs, b"LZ4_compress_HC_continue");
        let r_cont: libloading::Symbol<HCContinue> = rsym(&libs, b"LZ4_compress_HC_continue");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        for level in [2i32, 6, 9, 11, 12] {
            let nblocks = 4usize;
            let blocksz = 2500usize;
            let total = nblocks * blocksz;
            let data = rng.compressible(total);
            let cs = c_create();
            let rs = r_create();
            c_reset(cs, level);
            r_reset(rs, level);
            for b in 0..nblocks {
                let off = b * blocksz;
                let cap = cbound(blocksz as c_int) as usize;
                let mut cdst = vec![0u8; cap];
                let mut rdst = vec![0u8; cap];
                let src = data.as_ptr().add(off) as *const c_char;
                let cn = c_cont(cs, src, cdst.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int);
                let rn = r_cont(rs, src, rdst.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int);
                assert_eq!(cn, rn, "HC continue ret level {} block {}", level, b);
                assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "HC continue bytes level {} block {}", level, b);
            }
            c_free(cs);
            r_free(rs);
        }
    }
}

#[test]
fn test_favor_decspeed_hc() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xfa02);
    unsafe {
        let c_create: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStreamHC");
        let r_create: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStreamHC");
        let c_free: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStreamHC");
        let r_free: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStreamHC");
        let c_reset: libloading::Symbol<ResetStreamHCFast> = csym(&libs, b"LZ4_resetStreamHC_fast");
        let r_reset: libloading::Symbol<ResetStreamHCFast> = rsym(&libs, b"LZ4_resetStreamHC_fast");
        let c_favor: libloading::Symbol<FavorDecSpeed> = csym(&libs, b"LZ4_favorDecompressionSpeed");
        let r_favor: libloading::Symbol<FavorDecSpeed> = rsym(&libs, b"LZ4_favorDecompressionSpeed");
        let c_cont: libloading::Symbol<HCContinue> = csym(&libs, b"LZ4_compress_HC_continue");
        let r_cont: libloading::Symbol<HCContinue> = rsym(&libs, b"LZ4_compress_HC_continue");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        for level in [10i32, 11, 12] {
            let data = rng.compressible(20000);
            let cs = c_create();
            let rs = r_create();
            c_reset(cs, level);
            r_reset(rs, level);
            c_favor(cs, 1);
            r_favor(rs, 1);
            let cap = cbound(data.len() as c_int) as usize;
            let mut cdst = vec![0u8; cap];
            let mut rdst = vec![0u8; cap];
            let cn = c_cont(cs, data.as_ptr() as *const c_char, cdst.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            let rn = r_cont(rs, data.as_ptr() as *const c_char, rdst.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            assert_eq!(cn, rn, "favorDecSpeed ret level {}", level);
            assert_eq!(&cdst[..cn as usize], &rdst[..rn as usize], "favorDecSpeed bytes level {}", level);
            c_free(cs);
            r_free(rs);
        }
    }
}

#[test]
fn test_setstreamdecode_and_savedict() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x5a7e);
    unsafe {
        let c_create: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStream");
        let r_create: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStream");
        let c_free: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStream");
        let r_free: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStream");
        let c_save: libloading::Symbol<LoadDict> = csym(&libs, b"LZ4_saveDict");
        let r_save: libloading::Symbol<LoadDict> = rsym(&libs, b"LZ4_saveDict");
        let c_cont: libloading::Symbol<CompressContinue> = csym(&libs, b"LZ4_compress_fast_continue");
        let r_cont: libloading::Symbol<CompressContinue> = rsym(&libs, b"LZ4_compress_fast_continue");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let c_setdec: libloading::Symbol<SetStreamDecode> = csym(&libs, b"LZ4_setStreamDecode");
        let r_setdec: libloading::Symbol<SetStreamDecode> = rsym(&libs, b"LZ4_setStreamDecode");

        let blocksz = 4000usize;
        let data = rng.compressible(blocksz * 2);
        let cs = c_create();
        let rs = r_create();
        // block 0
        let cap = cbound(blocksz as c_int) as usize;
        let mut c0 = vec![0u8; cap];
        let mut r0 = vec![0u8; cap];
        let cn = c_cont(cs, data.as_ptr() as *const c_char, c0.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, 1);
        let rn = r_cont(rs, data.as_ptr() as *const c_char, r0.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, 1);
        assert_eq!(&c0[..cn as usize], &r0[..rn as usize]);

        // saveDict into separate buffer
        let mut c_dictbuf = vec![0u8; 65536];
        let mut r_dictbuf = vec![0u8; 65536];
        let csd = c_save(cs, c_dictbuf.as_mut_ptr() as *mut c_char, 65536);
        let rsd = r_save(rs, r_dictbuf.as_mut_ptr() as *mut c_char, 65536);
        assert_eq!(csd, rsd, "saveDict ret");
        assert_eq!(&c_dictbuf[..csd as usize], &r_dictbuf[..rsd as usize], "saveDict bytes");

        // block 1 with saved dict
        let mut c1 = vec![0u8; cap];
        let mut r1 = vec![0u8; cap];
        let cn1 = c_cont(cs, data.as_ptr().add(blocksz) as *const c_char, c1.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, 1);
        let rn1 = r_cont(rs, data.as_ptr().add(blocksz) as *const c_char, r1.as_mut_ptr() as *mut c_char, blocksz as c_int, cap as c_int, 1);
        assert_eq!(&c1[..cn1 as usize], &r1[..rn1 as usize], "block1 bytes");

        // setStreamDecode returns 1/0 identically
        let cds: libloading::Symbol<CreateStream> = csym(&libs, b"LZ4_createStreamDecode");
        let rds: libloading::Symbol<CreateStream> = rsym(&libs, b"LZ4_createStreamDecode");
        let cd = cds();
        let rd = rds();
        let csr = c_setdec(cd, c_dictbuf.as_ptr() as *const c_char, csd);
        let rsr = r_setdec(rd, r_dictbuf.as_ptr() as *const c_char, rsd);
        assert_eq!(csr, rsr, "setStreamDecode ret");
        let cdf: libloading::Symbol<FreeStream> = csym(&libs, b"LZ4_freeStreamDecode");
        let rdf: libloading::Symbol<FreeStream> = rsym(&libs, b"LZ4_freeStreamDecode");
        cdf(cd);
        rdf(rd);
        c_free(cs);
        r_free(rs);
    }
}
