// Phase B — differential tests for obsolete/deprecated & remaining public API.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

type CompressBound = unsafe extern "C" fn(c_int) -> c_int;

#[test]
fn test_obsolete_compress() {
    // LZ4_compress, LZ4_compress_limitedOutput, LZ4_compress_withState,
    // LZ4_compress_limitedOutput_withState
    let libs = Libs::load();
    let mut rng = Rng::new(0x0b501e7e);
    unsafe {
        type F3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        type F4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type S4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type S5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        type Sz = unsafe extern "C" fn() -> c_int;
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let c_ss = { let s: libloading::Symbol<Sz> = csym(&libs, b"LZ4_sizeofState"); s };

        for &sz in &[1usize, 100, 4096, 20000] {
            let data = rng.compressible(sz);
            let cap = cbound(sz as c_int) as usize;

            // LZ4_compress (3-arg)
            {
                let c: libloading::Symbol<F3> = csym(&libs, b"LZ4_compress");
                let r: libloading::Symbol<F3> = rsym(&libs, b"LZ4_compress");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int);
                let rn = r(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "LZ4_compress sz={}", sz);
            }
            // LZ4_compress_limitedOutput
            {
                let c: libloading::Symbol<F4> = csym(&libs, b"LZ4_compress_limitedOutput");
                let r: libloading::Symbol<F4> = rsym(&libs, b"LZ4_compress_limitedOutput");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                let rn = r(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "limitedOutput sz={}", sz);
            }
            // withState variants
            let state_sz = c_ss() as usize + 16;
            {
                let c: libloading::Symbol<S4> = csym(&libs, b"LZ4_compress_withState");
                let r: libloading::Symbol<S4> = rsym(&libs, b"LZ4_compress_withState");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int);
                let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "withState sz={}", sz);
            }
            {
                let c: libloading::Symbol<S5> = csym(&libs, b"LZ4_compress_limitedOutput_withState");
                let r: libloading::Symbol<S5> = rsym(&libs, b"LZ4_compress_limitedOutput_withState");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "limitedOutput_withState sz={}", sz);
            }
        }
    }
}

#[test]
fn test_obsolete_hc() {
    // LZ4_compressHC, LZ4_compressHC_limitedOutput, LZ4_compressHC2,
    // LZ4_compressHC2_limitedOutput, and withStateHC variants.
    let libs = Libs::load();
    let mut rng = Rng::new(0x0b50c);
    unsafe {
        type F3 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
        type F4 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type F4L = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type S4 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int) -> c_int;
        type S5 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        type Sz = unsafe extern "C" fn() -> c_int;
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let c_ss = { let s: libloading::Symbol<Sz> = csym(&libs, b"LZ4_sizeofStateHC"); s };
        let state_sz = c_ss() as usize + 16;

        for &sz in &[1usize, 100, 4096, 20000] {
            let data = rng.compressible(sz);
            let cap = cbound(sz as c_int) as usize;
            // LZ4_compressHC (3-arg)
            {
                let c: libloading::Symbol<F3> = csym(&libs, b"LZ4_compressHC");
                let r: libloading::Symbol<F3> = rsym(&libs, b"LZ4_compressHC");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int);
                let rn = r(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC sz={}", sz);
            }
            // LZ4_compressHC_limitedOutput
            {
                let c: libloading::Symbol<F4L> = csym(&libs, b"LZ4_compressHC_limitedOutput");
                let r: libloading::Symbol<F4L> = rsym(&libs, b"LZ4_compressHC_limitedOutput");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                let rn = r(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC_limitedOutput sz={}", sz);
            }
            // LZ4_compressHC2 / limitedOutput with several levels
            for level in [0i32, 1, 6, 9, 12] {
                let c: libloading::Symbol<F4> = csym(&libs, b"LZ4_compressHC2");
                let r: libloading::Symbol<F4> = rsym(&libs, b"LZ4_compressHC2");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, level);
                let rn = r(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, level);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC2 sz={} lvl={}", sz, level);

                type F5 = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
                let c2: libloading::Symbol<F5> = csym(&libs, b"LZ4_compressHC2_limitedOutput");
                let r2: libloading::Symbol<F5> = rsym(&libs, b"LZ4_compressHC2_limitedOutput");
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c2(data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                let rn = r2(data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC2_limitedOutput");
            }
            // withStateHC variants
            {
                let c: libloading::Symbol<S4> = csym(&libs, b"LZ4_compressHC_withStateHC");
                let r: libloading::Symbol<S4> = rsym(&libs, b"LZ4_compressHC_withStateHC");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int);
                let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC_withStateHC");

                let c2: libloading::Symbol<S5> = csym(&libs, b"LZ4_compressHC_limitedOutput_withStateHC");
                let r2: libloading::Symbol<S5> = rsym(&libs, b"LZ4_compressHC_limitedOutput_withStateHC");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c2(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                let rn = r2(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC_limitedOutput_withStateHC");
            }
            // HC2 withStateHC
            for level in [1i32, 9] {
                let c: libloading::Symbol<S5> = csym(&libs, b"LZ4_compressHC2_withStateHC");
                let r: libloading::Symbol<S5> = rsym(&libs, b"LZ4_compressHC2_withStateHC");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, level);
                let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, level);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC2_withStateHC lvl={}", level);

                type S6 = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
                let c2: libloading::Symbol<S6> = csym(&libs, b"LZ4_compressHC2_limitedOutput_withStateHC");
                let r2: libloading::Symbol<S6> = rsym(&libs, b"LZ4_compressHC2_limitedOutput_withStateHC");
                let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
                let cn = c2(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                let rn = r2(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int, level);
                assert_eq!(cn, rn); assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "compressHC2_limitedOutput_withStateHC");
            }
        }
    }
}

#[test]
fn test_obsolete_decompress() {
    // LZ4_uncompress, LZ4_uncompress_unknownOutputSize, LZ4_decompress_fast,
    // LZ4_decompress_fast_withPrefix64k, LZ4_decompress_safe_withPrefix64k
    let libs = Libs::load();
    let mut rng = Rng::new(0xdec0);
    unsafe {
        type C = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        let comp: libloading::Symbol<C> = csym(&libs, b"LZ4_compress_default");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        for &sz in &[1usize, 100, 4096, 20000] {
            let data = rng.compressible(sz);
            let cap = cbound(sz as c_int) as usize;
            let mut comp_buf = vec![0u8; cap];
            let cn = comp(data.as_ptr() as *const c_char, comp_buf.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);

            // LZ4_uncompress (knows output size)
            {
                type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
                let c: libloading::Symbol<F> = csym(&libs, b"LZ4_uncompress");
                let r: libloading::Symbol<F> = rsym(&libs, b"LZ4_uncompress");
                let mut co = vec![0u8; sz]; let mut ro = vec![0u8; sz];
                let cr = c(comp_buf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, sz as c_int);
                let rr = r(comp_buf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cr, rr, "LZ4_uncompress ret sz={}", sz);
                assert_eq!(co, data); assert_eq!(ro, data);
            }
            // LZ4_uncompress_unknownOutputSize
            {
                type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
                let c: libloading::Symbol<F> = csym(&libs, b"LZ4_uncompress_unknownOutputSize");
                let r: libloading::Symbol<F> = rsym(&libs, b"LZ4_uncompress_unknownOutputSize");
                let mut co = vec![0u8; sz]; let mut ro = vec![0u8; sz];
                let cr = c(comp_buf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, sz as c_int);
                let rr = r(comp_buf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, sz as c_int);
                assert_eq!(cr, rr, "uncompress_unknownOutputSize ret sz={}", sz);
                assert_eq!(&co[..cr as usize], &data[..]); assert_eq!(&ro[..rr as usize], &data[..]);
            }
            // LZ4_decompress_fast
            {
                type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int) -> c_int;
                let c: libloading::Symbol<F> = csym(&libs, b"LZ4_decompress_fast");
                let r: libloading::Symbol<F> = rsym(&libs, b"LZ4_decompress_fast");
                let mut co = vec![0u8; sz]; let mut ro = vec![0u8; sz];
                let cr = c(comp_buf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, sz as c_int);
                let rr = r(comp_buf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, sz as c_int);
                assert_eq!(cr, rr, "decompress_fast ret sz={}", sz);
                assert_eq!(co, data); assert_eq!(ro, data);
            }
            // LZ4_decompress_safe_withPrefix64k (no prefix, treated as prefix at start)
            {
                type F = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
                let c: libloading::Symbol<F> = csym(&libs, b"LZ4_decompress_safe_withPrefix64k");
                let r: libloading::Symbol<F> = rsym(&libs, b"LZ4_decompress_safe_withPrefix64k");
                let mut co = vec![0u8; sz]; let mut ro = vec![0u8; sz];
                let cr = c(comp_buf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, sz as c_int);
                let rr = r(comp_buf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, sz as c_int);
                assert_eq!(cr, rr, "decompress_safe_withPrefix64k ret sz={}", sz);
                if cr >= 0 { assert_eq!(&co[..cr as usize], &ro[..rr as usize]); }
            }
        }
    }
}

#[test]
fn test_sizeof_and_reset_state() {
    // LZ4_sizeofStreamState, LZ4_sizeofStreamStateHC, LZ4_resetStreamState,
    // LZ4_resetStreamStateHC, LZ4_resetStream, LZ4_resetStreamHC, LZ4_initStreamHC
    let libs = Libs::load();
    unsafe {
        for name in [&b"LZ4_sizeofStreamState"[..], &b"LZ4_sizeofStreamStateHC"[..]] {
            type F = unsafe extern "C" fn() -> c_int;
            let c: libloading::Symbol<F> = csym(&libs, name);
            let r: libloading::Symbol<F> = rsym(&libs, name);
            assert_eq!(c(), r(), "{:?}", String::from_utf8_lossy(name));
        }
    }
}

#[test]
fn test_setcompressionlevel_and_attach() {
    // LZ4_setCompressionLevel, LZ4_attach_dictionary, LZ4_attach_HC_dictionary,
    // LZ4_loadDictHC, LZ4_saveDictHC — used together in an HC stream w/ dict attach.
    let libs = Libs::load();
    let mut rng = Rng::new(0xa77a);
    unsafe {
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type LoadHC = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        type Attach = unsafe extern "C" fn(*mut c_void, *const c_void);
        type HCCont = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int) -> c_int;
        type SetLevel = unsafe extern "C" fn(*mut c_void, c_int);
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        for (create_n, free_n, load_n, attach_n) in [
            (&b"LZ4_createStreamHC"[..], &b"LZ4_freeStreamHC"[..], &b"LZ4_loadDictHC"[..], &b"LZ4_attach_HC_dictionary"[..]),
        ] {
            let c_create: libloading::Symbol<Create> = csym(&libs, create_n);
            let r_create: libloading::Symbol<Create> = rsym(&libs, create_n);
            let c_free: libloading::Symbol<Free> = csym(&libs, free_n);
            let r_free: libloading::Symbol<Free> = rsym(&libs, free_n);
            let c_load: libloading::Symbol<LoadHC> = csym(&libs, load_n);
            let r_load: libloading::Symbol<LoadHC> = rsym(&libs, load_n);
            let c_att: libloading::Symbol<Attach> = csym(&libs, attach_n);
            let r_att: libloading::Symbol<Attach> = rsym(&libs, attach_n);
            let c_lvl: libloading::Symbol<SetLevel> = csym(&libs, b"LZ4_setCompressionLevel");
            let r_lvl: libloading::Symbol<SetLevel> = rsym(&libs, b"LZ4_setCompressionLevel");
            let c_cont: libloading::Symbol<HCCont> = csym(&libs, b"LZ4_compress_HC_continue");
            let r_cont: libloading::Symbol<HCCont> = rsym(&libs, b"LZ4_compress_HC_continue");

            let dict = rng.compressible(8000);
            let data = rng.compressible(4000);

            // dictionary stream
            let cds = c_create(); let rds = r_create();
            c_lvl(cds, 9); r_lvl(rds, 9);
            c_load(cds, dict.as_ptr() as *const c_char, dict.len() as c_int);
            r_load(rds, dict.as_ptr() as *const c_char, dict.len() as c_int);

            // working stream + attach
            let cws = c_create(); let rws = r_create();
            c_lvl(cws, 9); r_lvl(rws, 9);
            c_att(cws, cds as *const c_void);
            r_att(rws, rds as *const c_void);

            let cap = cbound(data.len() as c_int) as usize;
            let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
            let cn = c_cont(cws, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            let rn = r_cont(rws, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int);
            assert_eq!(cn, rn, "attach_HC + HC_continue ret");
            assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "attach_HC + HC_continue bytes");

            c_free(cds); r_free(rds); c_free(cws); r_free(rws);
        }
    }
}

#[test]
fn test_attach_dictionary_lz4() {
    // LZ4_attach_dictionary (non-HC) with LZ4_compress_fast_continue.
    let libs = Libs::load();
    let mut rng = Rng::new(0xa77b);
    unsafe {
        type Create = unsafe extern "C" fn() -> *mut c_void;
        type Free = unsafe extern "C" fn(*mut c_void) -> c_int;
        type Load = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
        type Attach = unsafe extern "C" fn(*mut c_void, *const c_void);
        type Cont = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int) -> c_int;
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");

        let c_create: libloading::Symbol<Create> = csym(&libs, b"LZ4_createStream");
        let r_create: libloading::Symbol<Create> = rsym(&libs, b"LZ4_createStream");
        let c_free: libloading::Symbol<Free> = csym(&libs, b"LZ4_freeStream");
        let r_free: libloading::Symbol<Free> = rsym(&libs, b"LZ4_freeStream");
        let c_load: libloading::Symbol<Load> = csym(&libs, b"LZ4_loadDict");
        let r_load: libloading::Symbol<Load> = rsym(&libs, b"LZ4_loadDict");
        let c_att: libloading::Symbol<Attach> = csym(&libs, b"LZ4_attach_dictionary");
        let r_att: libloading::Symbol<Attach> = rsym(&libs, b"LZ4_attach_dictionary");
        let c_cont: libloading::Symbol<Cont> = csym(&libs, b"LZ4_compress_fast_continue");
        let r_cont: libloading::Symbol<Cont> = rsym(&libs, b"LZ4_compress_fast_continue");

        let dict = rng.compressible(8000);
        let data = rng.compressible(4000);
        let cds = c_create(); let rds = r_create();
        c_load(cds, dict.as_ptr() as *const c_char, dict.len() as c_int);
        r_load(rds, dict.as_ptr() as *const c_char, dict.len() as c_int);
        let cws = c_create(); let rws = r_create();
        c_att(cws, cds as *const c_void);
        r_att(rws, rds as *const c_void);
        let cap = cbound(data.len() as c_int) as usize;
        let mut cd = vec![0u8; cap]; let mut rd = vec![0u8; cap];
        let cn = c_cont(cws, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int, 1);
        let rn = r_cont(rws, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, data.len() as c_int, cap as c_int, 1);
        assert_eq!(cn, rn, "attach_dictionary ret");
        assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "attach_dictionary bytes");
        c_free(cds); r_free(rds); c_free(cws); r_free(rws);
    }
}

#[test]
fn test_partial_usingdict_and_destsize_extstate() {
    // LZ4_decompress_safe_partial_usingDict, LZ4_compress_destSize_extState
    let libs = Libs::load();
    let mut rng = Rng::new(0xdead00);
    unsafe {
        type Sz = unsafe extern "C" fn() -> c_int;
        type DSES = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, *mut c_int, c_int, c_int) -> c_int;
        let c_ss = { let s: libloading::Symbol<Sz> = csym(&libs, b"LZ4_sizeofState"); s };
        let state_sz = c_ss() as usize + 16;
        let c: libloading::Symbol<DSES> = csym(&libs, b"LZ4_compress_destSize_extState");
        let r: libloading::Symbol<DSES> = rsym(&libs, b"LZ4_compress_destSize_extState");
        for &sz in &[100usize, 4096, 20000] {
            for &dstcap in &[8usize, 64, 500, 5000] {
                for accel in [1i32, 5] {
                    let data = rng.compressible(sz);
                    let mut cs = vec![0u8; state_sz]; let mut rs = vec![0u8; state_sz];
                    let mut csrc = sz as c_int; let mut rsrc = sz as c_int;
                    let mut cd = vec![0u8; dstcap]; let mut rd = vec![0u8; dstcap];
                    let cn = c(cs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, cd.as_mut_ptr() as *mut c_char, &mut csrc, dstcap as c_int, accel);
                    let rn = r(rs.as_mut_ptr() as *mut c_void, data.as_ptr() as *const c_char, rd.as_mut_ptr() as *mut c_char, &mut rsrc, dstcap as c_int, accel);
                    assert_eq!(cn, rn, "destSize_extState ret sz={} cap={} accel={}", sz, dstcap, accel);
                    assert_eq!(csrc, rsrc, "destSize_extState srcConsumed");
                    assert_eq!(&cd[..cn as usize], &rd[..rn as usize], "destSize_extState bytes");
                }
            }
        }

        // partial_usingDict
        type C = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int) -> c_int;
        type PUD = unsafe extern "C" fn(*const c_char, *mut c_char, c_int, c_int, c_int, *const c_char, c_int) -> c_int;
        let comp: libloading::Symbol<C> = csym(&libs, b"LZ4_compress_default");
        let cbound: libloading::Symbol<CompressBound> = csym(&libs, b"LZ4_compressBound");
        let cp: libloading::Symbol<PUD> = csym(&libs, b"LZ4_decompress_safe_partial_usingDict");
        let rp: libloading::Symbol<PUD> = rsym(&libs, b"LZ4_decompress_safe_partial_usingDict");
        for &sz in &[100usize, 4096] {
            let data = rng.compressible(sz);
            let cap = cbound(sz as c_int) as usize;
            let mut comp_buf = vec![0u8; cap];
            let cn = comp(data.as_ptr() as *const c_char, comp_buf.as_mut_ptr() as *mut c_char, sz as c_int, cap as c_int);
            for &target in &[1usize, sz / 2, sz] {
                let mut co = vec![0u8; sz]; let mut ro = vec![0u8; sz];
                let cr = cp(comp_buf.as_ptr() as *const c_char, co.as_mut_ptr() as *mut c_char, cn, target as c_int, sz as c_int, std::ptr::null(), 0);
                let rr = rp(comp_buf.as_ptr() as *const c_char, ro.as_mut_ptr() as *mut c_char, cn, target as c_int, sz as c_int, std::ptr::null(), 0);
                assert_eq!(cr, rr, "partial_usingDict ret sz={} target={}", sz, target);
                if cr > 0 { assert_eq!(&co[..cr as usize], &ro[..rr as usize]); }
            }
        }
    }
}
