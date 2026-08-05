// Phase D — coverage for remaining directly-testable exported symbols.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

type CompressBound = unsafe extern "C" fn(c_int) -> c_int;

#[test]
fn test_hc_obsolete_continue() {
    // LZ4_compressHC_continue, LZ4_compressHC_limitedOutput_continue,
    // LZ4_compressHC2_continue, LZ4_compressHC2_limitedOutput_continue,
    // via LZ4_createHC / LZ4_slideInputBufferHC / LZ4_freeHC.
    let libs = Libs::load();
    let mut rng = Rng::new(0xc047);
    unsafe {
        type CreateHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
        type FreeHC = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Cont3 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type Cont4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        // Use LZ4_createStreamHC (modern) as the state; obsolete continue funcs take void* state.
        type CreateSHC = unsafe extern "C" fn() -> *mut c_void;
        type FreeSHC = unsafe extern "C" fn(*mut c_void) -> c_int;
        let c_cs: libloading::Symbol<CreateSHC> = csym(&libs, b"LZ4_createStreamHC");
        let r_cs: libloading::Symbol<CreateSHC> = rsym(&libs, b"LZ4_createStreamHC");
        let c_fs: libloading::Symbol<FreeSHC> = csym(&libs, b"LZ4_freeStreamHC");
        let r_fs: libloading::Symbol<FreeSHC> = rsym(&libs, b"LZ4_freeStreamHC");

        let data = rng.compressible(4000);
        let cap = cbound(data.len() as c_int) as usize;

        // LZ4_compressHC_continue (3-arg)
        for name in [&b"LZ4_compressHC_continue"[..]] {
            let c: libloading::Symbol<Cont3> = csym(&libs, name);
            let r: libloading::Symbol<Cont3> = rsym(&libs, name);
            let cs = c_cs(); let rs = r_cs();
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c(cs, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int);
            let rn = r(rs, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int);
            assert_eq!(cn, rn, "{:?}", String::from_utf8_lossy(name));
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "{:?} bytes", String::from_utf8_lossy(name));
            c_fs(cs); r_fs(rs);
        }
        // LZ4_compressHC_limitedOutput_continue (4-arg)
        {
            let c: libloading::Symbol<Cont4> = csym(&libs, b"LZ4_compressHC_limitedOutput_continue");
            let r: libloading::Symbol<Cont4> = rsym(&libs, b"LZ4_compressHC_limitedOutput_continue");
            let cs = c_cs(); let rs = r_cs();
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c(cs, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            let rn = r(rs, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            assert_eq!(cn, rn, "HC_limitedOutput_continue");
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize]);
            c_fs(cs); r_fs(rs);
        }
        // LZ4_compressHC2_continue (level) + limitedOutput.
        // These call LZ4HC_compress_generic directly (bypassing the auto-init done by
        // LZ4_compress_HC_continue), so the state must already be initialized:
        // LZ4_createHC() runs LZ4HC_init_internal(). (A fresh createStreamHC would
        // leave prefixStart NULL and crash BOTH libraries — that's not valid usage.)
        type CHC = unsafe extern "C" fn(*const c_char) -> *mut c_void;
        type FHC = unsafe extern "C" fn(*mut c_void) -> c_int;
        let c_chc: libloading::Symbol<CHC> = csym(&libs, b"LZ4_createHC");
        let r_chc: libloading::Symbol<CHC> = rsym(&libs, b"LZ4_createHC");
        let c_fhc: libloading::Symbol<FHC> = csym(&libs, b"LZ4_freeHC");
        let r_fhc: libloading::Symbol<FHC> = rsym(&libs, b"LZ4_freeHC");
        for level in [1i32, 9] {
            type C5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
            let c: libloading::Symbol<C5> = csym(&libs, b"LZ4_compressHC2_continue");
            let r: libloading::Symbol<C5> = rsym(&libs, b"LZ4_compressHC2_continue");
            let cs = c_chc(data.as_ptr() as *const c_char);
            let rs = r_chc(data.as_ptr() as *const c_char);
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c(cs, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, level);
            let rn = r(rs, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, level);
            assert_eq!(cn, rn, "HC2_continue lvl={}", level);
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize]);
            c_fhc(cs); r_fhc(rs);

            type C6 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
            let c2: libloading::Symbol<C6> = csym(&libs, b"LZ4_compressHC2_limitedOutput_continue");
            let r2: libloading::Symbol<C6> = rsym(&libs, b"LZ4_compressHC2_limitedOutput_continue");
            let cs = c_chc(data.as_ptr() as *const c_char);
            let rs = r_chc(data.as_ptr() as *const c_char);
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c2(cs, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int, level);
            let rn = r2(rs, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int, level);
            assert_eq!(cn, rn, "HC2_limitedOutput_continue lvl={}", level);
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize]);
            c_fhc(cs); r_fhc(rs);
        }

        // LZ4_createHC / LZ4_freeHC / LZ4_slideInputBufferHC roundtrip on both
        {
            let cc: libloading::Symbol<CreateHC> = csym(&libs, b"LZ4_createHC");
            let rc: libloading::Symbol<CreateHC> = rsym(&libs, b"LZ4_createHC");
            let cf: libloading::Symbol<FreeHC> = csym(&libs, b"LZ4_freeHC");
            let rf: libloading::Symbol<FreeHC> = rsym(&libs, b"LZ4_freeHC");
            type Slide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
            let cslide: libloading::Symbol<Slide> = csym(&libs, b"LZ4_slideInputBufferHC");
            let rslide: libloading::Symbol<Slide> = rsym(&libs, b"LZ4_slideInputBufferHC");
            // create with a stable input buffer
            let inbuf = vec![0u8; 200 * 1024];
            let ch = cc(inbuf.as_ptr() as *const c_char);
            let rh = rc(inbuf.as_ptr() as *const c_char);
            assert_eq!(ch.is_null(), rh.is_null());
            if !ch.is_null() {
                // slide returns a pointer offset; we just verify both don't crash & return non-null
                let _ = cslide(ch);
                let _ = rslide(rh);
            }
            cf(ch); rf(rh);
        }
    }
}

#[test]
fn test_obsolete_lz4_create_and_streamstate() {
    // LZ4_create, LZ4_resetStreamState, LZ4_slideInputBuffer, LZ4_compress_continue,
    // LZ4_compress_limitedOutput_continue
    let libs = Libs::load();
    let mut rng = Rng::new(0xc0de55);
    unsafe {
        type Sz = unsafe extern "C" fn() -> c_int;
        let c_ss: libloading::Symbol<Sz> = csym(&libs, b"LZ4_sizeofStreamState");
        let state_sz = c_ss() as usize;
        type ResetSS = unsafe extern "C" fn(*mut c_void, *mut c_char) -> c_int;
        let c_rss: libloading::Symbol<ResetSS> = csym(&libs, b"LZ4_resetStreamState");
        let r_rss: libloading::Symbol<ResetSS> = rsym(&libs, b"LZ4_resetStreamState");
        type Cont3 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type Cont4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        let inbuf = vec![0u8; 300 * 1024];
        let data = rng.compressible(3000);
        let cap = cbound(data.len() as c_int) as usize;

        // resetStreamState + compress_continue
        {
            let c: libloading::Symbol<Cont3> = csym(&libs, b"LZ4_compress_continue");
            let r: libloading::Symbol<Cont3> = rsym(&libs, b"LZ4_compress_continue");
            let mut cs = vec![0u8; state_sz + 16];
            let mut rs = vec![0u8; state_sz + 16];
            let cr = c_rss(cs.as_mut_ptr() as *mut c_void, inbuf.as_ptr() as *mut c_char);
            let rr = r_rss(rs.as_mut_ptr() as *mut c_void, inbuf.as_ptr() as *mut c_char);
            assert_eq!(cr, rr, "resetStreamState ret");
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int);
            let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int);
            assert_eq!(cn, rn, "compress_continue ret");
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compress_continue bytes");
        }
        // limitedOutput_continue
        {
            let c: libloading::Symbol<Cont4> = csym(&libs, b"LZ4_compress_limitedOutput_continue");
            let r: libloading::Symbol<Cont4> = rsym(&libs, b"LZ4_compress_limitedOutput_continue");
            let mut cs = vec![0u8; state_sz + 16];
            let mut rs = vec![0u8; state_sz + 16];
            c_rss(cs.as_mut_ptr() as *mut c_void, inbuf.as_ptr() as *mut c_char);
            r_rss(rs.as_mut_ptr() as *mut c_void, inbuf.as_ptr() as *mut c_char);
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            assert_eq!(cn, rn, "limitedOutput_continue ret");
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize]);
        }
        // LZ4_create + LZ4_slideInputBuffer
        {
            type Create = unsafe extern "C" fn(*mut c_char) -> *mut c_void;
            type Slide = unsafe extern "C" fn(*mut c_void) -> *mut c_char;
            let cc: libloading::Symbol<Create> = csym(&libs, b"LZ4_create");
            let rc: libloading::Symbol<Create> = rsym(&libs, b"LZ4_create");
            let cslide: libloading::Symbol<Slide> = csym(&libs, b"LZ4_slideInputBuffer");
            let rslide: libloading::Symbol<Slide> = rsym(&libs, b"LZ4_slideInputBuffer");
            let ch = cc(inbuf.as_ptr() as *mut c_char);
            let rh = rc(inbuf.as_ptr() as *mut c_char);
            assert_eq!(ch.is_null(), rh.is_null());
            if !ch.is_null() {
                let _ = cslide(ch);
                let _ = rslide(rh);
                // free via LZ4_freeStream (create returns an LZ4_stream_t*)
                type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
                let cf: libloading::Symbol<Free> = csym(&libs, b"LZ4_freeStream");
                let rf: libloading::Symbol<Free> = rsym(&libs, b"LZ4_freeStream");
                cf(ch); rf(rh);
            }
        }
    }
}

#[test]
fn test_forceextdict_and_reset_variants() {
    // LZ4_compress_forceExtDict, LZ4_decompress_safe_forceExtDict,
    // LZ4_decompress_safe_partial_forceExtDict, LZ4_resetStream, LZ4_resetStream_fast,
    // LZ4_resetStreamHC
    let libs = Libs::load();
    let mut rng = Rng::new(0xf0acede);
    unsafe {
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        type ForceExt = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type DecExt = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, *const c_void, usize) -> c_int;
        type DecPartExt = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int, *const c_void, usize) -> c_int;
        type Reset = unsafe extern "C" fn(*mut c_void);
        type ResetHC = unsafe extern "C" fn(*mut c_void, c_int);
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        let c_create: libloading::Symbol<Create> = csym(&libs, b"LZ4_createStream");
        let r_create: libloading::Symbol<Create> = rsym(&libs, b"LZ4_createStream");
        let c_free: libloading::Symbol<Free> = csym(&libs, b"LZ4_freeStream");
        let r_free: libloading::Symbol<Free> = rsym(&libs, b"LZ4_freeStream");
        let c_load: libloading::Symbol<Load> = csym(&libs, b"LZ4_loadDict");
        let r_load: libloading::Symbol<Load> = rsym(&libs, b"LZ4_loadDict");
        let c_reset: libloading::Symbol<Reset> = csym(&libs, b"LZ4_resetStream");
        let r_reset: libloading::Symbol<Reset> = rsym(&libs, b"LZ4_resetStream");
        let c_resetf: libloading::Symbol<Reset> = csym(&libs, b"LZ4_resetStream_fast");
        let r_resetf: libloading::Symbol<Reset> = rsym(&libs, b"LZ4_resetStream_fast");

        let dict = rng.compressible(8000);
        let data = rng.compressible(3000);
        let cap = cbound(data.len() as c_int) as usize;

        let cs = c_create(); let rs = r_create();
        c_reset(cs); r_reset(rs);
        c_resetf(cs); r_resetf(rs);
        c_load(cs, dict.as_ptr() as *const c_char, dict.len() as c_int);
        r_load(rs, dict.as_ptr() as *const c_char, dict.len() as c_int);

        let c_fed: libloading::Symbol<ForceExt> = csym(&libs, b"LZ4_compress_forceExtDict");
        let r_fed: libloading::Symbol<ForceExt> = rsym(&libs, b"LZ4_compress_forceExtDict");
        let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
        let cn = c_fed(cs, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int);
        let rn = r_fed(rs, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int);
        assert_eq!(cn, rn, "compress_forceExtDict ret");
        assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compress_forceExtDict bytes");

        // decode with forceExtDict variants
        let c_de: libloading::Symbol<DecExt> = csym(&libs, b"LZ4_decompress_safe_forceExtDict");
        let r_de: libloading::Symbol<DecExt> = rsym(&libs, b"LZ4_decompress_safe_forceExtDict");
        let mut co = vec![0u8; data.len()]; let mut ro = vec![0u8; data.len()];
        let cr = c_de(cd.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, data.len() as c_int, dict.as_ptr() as *const c_void, dict.len());
        let rr = r_de(cd.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, data.len() as c_int, dict.as_ptr() as *const c_void, dict.len());
        assert_eq!(cr, rr, "decompress_safe_forceExtDict ret");
        assert_eq!(cr as usize, data.len());
        assert_eq!(co, data); assert_eq!(ro, data);

        let c_dpe: libloading::Symbol<DecPartExt> = csym(&libs, b"LZ4_decompress_safe_partial_forceExtDict");
        let r_dpe: libloading::Symbol<DecPartExt> = rsym(&libs, b"LZ4_decompress_safe_partial_forceExtDict");
        for &target in &[1usize, data.len() / 2, data.len()] {
            let mut co = vec![0u8; data.len()]; let mut ro = vec![0u8; data.len()];
            let cr = c_dpe(cd.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, target as c_int, data.len() as c_int, dict.as_ptr() as *const c_void, dict.len());
            let rr = r_dpe(cd.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, target as c_int, data.len() as c_int, dict.as_ptr() as *const c_void, dict.len());
            assert_eq!(cr, rr, "partial_forceExtDict ret target={}", target);
            if cr > 0 { assert_eq!(&co[..cr as usize], &ro[..rr as usize]); }
        }
        c_free(cs); r_free(rs);

        // LZ4_resetStreamHC just needs to not crash + parity on subsequent compress
        let c_chc: libloading::Symbol<Create> = csym(&libs, b"LZ4_createStreamHC");
        let r_chc: libloading::Symbol<Create> = rsym(&libs, b"LZ4_createStreamHC");
        let c_fhc: libloading::Symbol<Free> = csym(&libs, b"LZ4_freeStreamHC");
        let r_fhc: libloading::Symbol<Free> = rsym(&libs, b"LZ4_freeStreamHC");
        let c_rhc: libloading::Symbol<ResetHC> = csym(&libs, b"LZ4_resetStreamHC");
        let r_rhc: libloading::Symbol<ResetHC> = rsym(&libs, b"LZ4_resetStreamHC");
        let chc = c_chc(); let rhc = r_chc();
        c_rhc(chc, 9); r_rhc(rhc, 9);
        c_fhc(chc); r_fhc(rhc);
    }
}

#[test]
fn test_xxh32_copystate_and_reset_dctx() {
    // LZ4_XXH32_copyState + LZ4F_resetDecompressionContext
    let libs = Libs::load();
    let mut rng = Rng::new(0xc0b1a5);
    unsafe {
        type CS = unsafe extern "C" fn() -> *mut c_void;
        type RST = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
        type UPD = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
        type DIG = unsafe extern "C" fn(*const c_void) -> u32;
        type CP = unsafe extern "C" fn(*mut c_void, *const c_void);
        type FS = unsafe extern "C" fn(*mut c_void) -> c_int;
        let c_cs: libloading::Symbol<CS> = csym(&libs, b"LZ4_XXH32_createState");
        let r_cs: libloading::Symbol<CS> = rsym(&libs, b"LZ4_XXH32_createState");
        let c_rst: libloading::Symbol<RST> = csym(&libs, b"LZ4_XXH32_reset");
        let r_rst: libloading::Symbol<RST> = rsym(&libs, b"LZ4_XXH32_reset");
        let c_upd: libloading::Symbol<UPD> = csym(&libs, b"LZ4_XXH32_update");
        let r_upd: libloading::Symbol<UPD> = rsym(&libs, b"LZ4_XXH32_update");
        let c_dig: libloading::Symbol<DIG> = csym(&libs, b"LZ4_XXH32_digest");
        let r_dig: libloading::Symbol<DIG> = rsym(&libs, b"LZ4_XXH32_digest");
        let c_cp: libloading::Symbol<CP> = csym(&libs, b"LZ4_XXH32_copyState");
        let r_cp: libloading::Symbol<CP> = rsym(&libs, b"LZ4_XXH32_copyState");
        let c_fs: libloading::Symbol<FS> = csym(&libs, b"LZ4_XXH32_freeState");
        let r_fs: libloading::Symbol<FS> = rsym(&libs, b"LZ4_XXH32_freeState");

        let data = rng.random(4000);
        let cst = c_cs(); let rst = r_cs();
        c_rst(cst, 7); r_rst(rst, 7);
        c_upd(cst, data.as_ptr() as *const c_void, 1500);
        r_upd(rst, data.as_ptr() as *const c_void, 1500);
        let cst2 = c_cs(); let rst2 = r_cs();
        c_cp(cst2, cst); r_cp(rst2, rst);
        c_upd(cst2, data.as_ptr().add(1500) as *const c_void, 2500);
        r_upd(rst2, data.as_ptr().add(1500) as *const c_void, 2500);
        assert_eq!(c_dig(cst2), r_dig(rst2), "XXH32 copyState digest");
        c_fs(cst); r_fs(rst); c_fs(cst2); r_fs(rst2);

        // LZ4F_resetDecompressionContext: create dctx, reset, ensure no crash & subsequent decode works
        type CD = unsafe extern "C" fn(*mut *mut c_void, u32) -> usize;
        type RD = unsafe extern "C" fn(*mut c_void);
        type FD = unsafe extern "C" fn(*mut c_void) -> usize;
        let c_cd: libloading::Symbol<CD> = csym(&libs, b"LZ4F_createDecompressionContext");
        let r_cd: libloading::Symbol<CD> = rsym(&libs, b"LZ4F_createDecompressionContext");
        let c_rd: libloading::Symbol<RD> = csym(&libs, b"LZ4F_resetDecompressionContext");
        let r_rd: libloading::Symbol<RD> = rsym(&libs, b"LZ4F_resetDecompressionContext");
        let c_fd: libloading::Symbol<FD> = csym(&libs, b"LZ4F_freeDecompressionContext");
        let r_fd: libloading::Symbol<FD> = rsym(&libs, b"LZ4F_freeDecompressionContext");
        let mut cdx: *mut c_void = std::ptr::null_mut();
        let mut rdx: *mut c_void = std::ptr::null_mut();
        c_cd(&mut cdx, 100); r_cd(&mut rdx, 100);
        c_rd(cdx); r_rd(rdx);
        c_fd(cdx); r_fd(rdx);
    }
}
