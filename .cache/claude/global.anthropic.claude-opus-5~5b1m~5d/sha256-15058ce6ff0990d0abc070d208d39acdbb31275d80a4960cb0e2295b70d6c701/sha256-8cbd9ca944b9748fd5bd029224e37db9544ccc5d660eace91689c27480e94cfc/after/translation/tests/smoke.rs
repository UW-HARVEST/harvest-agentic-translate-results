//! Harness smoke test: proves both .so files load and that every symbol the C
//! library exports is also present in the Rust library (Phase A / Phase D
//! symbol parity, checked at runtime through `dlsym`).
mod common;
use common::*;
use std::os::raw::c_char;

/// The complete `nm -D --defined-only` symbol list of the C shared library.
const C_SYMBOLS: &[&str] = &[
    "LZ4F_compressBegin",
    "LZ4F_compressBegin_internal",
    "LZ4F_compressBegin_usingCDict",
    "LZ4F_compressBegin_usingDict",
    "LZ4F_compressBegin_usingDictOnce",
    "LZ4F_compressBound",
    "LZ4F_compressEnd",
    "LZ4F_compressFrame",
    "LZ4F_compressFrameBound",
    "LZ4F_compressFrame_usingCDict",
    "LZ4F_compressUpdate",
    "LZ4F_compressionLevel_max",
    "LZ4F_createCDict",
    "LZ4F_createCDict_advanced",
    "LZ4F_createCompressionContext",
    "LZ4F_createCompressionContext_advanced",
    "LZ4F_createDecompressionContext",
    "LZ4F_createDecompressionContext_advanced",
    "LZ4F_decompress",
    "LZ4F_decompress_usingDict",
    "LZ4F_flush",
    "LZ4F_freeCDict",
    "LZ4F_freeCompressionContext",
    "LZ4F_freeDecompressionContext",
    "LZ4F_getBlockSize",
    "LZ4F_getErrorCode",
    "LZ4F_getErrorName",
    "LZ4F_getFrameInfo",
    "LZ4F_getVersion",
    "LZ4F_headerSize",
    "LZ4F_isError",
    "LZ4F_read",
    "LZ4F_readClose",
    "LZ4F_readOpen",
    "LZ4F_resetDecompressionContext",
    "LZ4F_uncompressedUpdate",
    "LZ4F_write",
    "LZ4F_writeClose",
    "LZ4F_writeOpen",
    "LZ4HC_searchExtDict",
    "LZ4_XXH32",
    "LZ4_XXH32_canonicalFromHash",
    "LZ4_XXH32_copyState",
    "LZ4_XXH32_createState",
    "LZ4_XXH32_digest",
    "LZ4_XXH32_freeState",
    "LZ4_XXH32_hashFromCanonical",
    "LZ4_XXH32_reset",
    "LZ4_XXH32_update",
    "LZ4_XXH64",
    "LZ4_XXH64_canonicalFromHash",
    "LZ4_XXH64_copyState",
    "LZ4_XXH64_createState",
    "LZ4_XXH64_digest",
    "LZ4_XXH64_freeState",
    "LZ4_XXH64_hashFromCanonical",
    "LZ4_XXH64_reset",
    "LZ4_XXH64_update",
    "LZ4_XXH_versionNumber",
    "LZ4_attach_HC_dictionary",
    "LZ4_attach_dictionary",
    "LZ4_compress",
    "LZ4_compressBound",
    "LZ4_compressHC",
    "LZ4_compressHC2",
    "LZ4_compressHC2_continue",
    "LZ4_compressHC2_limitedOutput",
    "LZ4_compressHC2_limitedOutput_continue",
    "LZ4_compressHC2_limitedOutput_withStateHC",
    "LZ4_compressHC2_withStateHC",
    "LZ4_compressHC_continue",
    "LZ4_compressHC_limitedOutput",
    "LZ4_compressHC_limitedOutput_continue",
    "LZ4_compressHC_limitedOutput_withStateHC",
    "LZ4_compressHC_withStateHC",
    "LZ4_compress_HC",
    "LZ4_compress_HC_continue",
    "LZ4_compress_HC_continue_destSize",
    "LZ4_compress_HC_destSize",
    "LZ4_compress_HC_extStateHC",
    "LZ4_compress_HC_extStateHC_fastReset",
    "LZ4_compress_continue",
    "LZ4_compress_default",
    "LZ4_compress_destSize",
    "LZ4_compress_destSize_extState",
    "LZ4_compress_fast",
    "LZ4_compress_fast_continue",
    "LZ4_compress_fast_extState",
    "LZ4_compress_fast_extState_fastReset",
    "LZ4_compress_forceExtDict",
    "LZ4_compress_limitedOutput",
    "LZ4_compress_limitedOutput_continue",
    "LZ4_compress_limitedOutput_withState",
    "LZ4_compress_withState",
    "LZ4_create",
    "LZ4_createHC",
    "LZ4_createStream",
    "LZ4_createStreamDecode",
    "LZ4_createStreamHC",
    "LZ4_decoderRingBufferSize",
    "LZ4_decompress_fast",
    "LZ4_decompress_fast_continue",
    "LZ4_decompress_fast_usingDict",
    "LZ4_decompress_fast_withPrefix64k",
    "LZ4_decompress_safe",
    "LZ4_decompress_safe_continue",
    "LZ4_decompress_safe_forceExtDict",
    "LZ4_decompress_safe_partial",
    "LZ4_decompress_safe_partial_forceExtDict",
    "LZ4_decompress_safe_partial_usingDict",
    "LZ4_decompress_safe_usingDict",
    "LZ4_decompress_safe_withPrefix64k",
    "LZ4_favorDecompressionSpeed",
    "LZ4_freeHC",
    "LZ4_freeStream",
    "LZ4_freeStreamDecode",
    "LZ4_freeStreamHC",
    "LZ4_initStream",
    "LZ4_initStreamHC",
    "LZ4_loadDict",
    "LZ4_loadDictHC",
    "LZ4_loadDictSlow",
    "LZ4_loadDict_internal",
    "LZ4_resetStream",
    "LZ4_resetStreamHC",
    "LZ4_resetStreamHC_fast",
    "LZ4_resetStreamState",
    "LZ4_resetStreamStateHC",
    "LZ4_resetStream_fast",
    "LZ4_saveDict",
    "LZ4_saveDictHC",
    "LZ4_setCompressionLevel",
    "LZ4_setStreamDecode",
    "LZ4_sizeofState",
    "LZ4_sizeofStateHC",
    "LZ4_sizeofStreamState",
    "LZ4_sizeofStreamStateHC",
    "LZ4_slideInputBuffer",
    "LZ4_slideInputBufferHC",
    "LZ4_uncompress",
    "LZ4_uncompress_unknownOutputSize",
    "LZ4_versionNumber",
    "LZ4_versionString",
];

#[test]
fn both_libraries_load() {
    assert_eq!(c().which, "C");
    assert_eq!(r().which, "Rust");
}

#[test]
fn symbol_parity() {
    assert_eq!(C_SYMBOLS.len(), 143, "C symbol list must have 143 entries");
    let mut missing_c = Vec::new();
    let mut missing_r = Vec::new();
    for s in C_SYMBOLS {
        if !c().has(s) {
            missing_c.push(*s);
        }
        if !r().has(s) {
            missing_r.push(*s);
        }
    }
    assert!(missing_c.is_empty(), "not exported by C .so: {:?}", missing_c);
    assert!(
        missing_r.is_empty(),
        "not exported by Rust .so: {:?}",
        missing_r
    );
}

#[test]
fn version_constants_match() {
    let (cv, rv) = both(|l| unsafe { l.get::<FnI>("LZ4_versionNumber")() });
    assert_eq!(cv, rv, "LZ4_versionNumber");

    let (cs, rs) = both(|l| unsafe { cstr(l.get::<FnStr>("LZ4_versionString")()) });
    assert_eq!(cs, rs, "LZ4_versionString");

    let (cv, rv) = both(|l| unsafe { l.get::<Fn_F_getVersion>("LZ4F_getVersion")() });
    assert_eq!(cv, rv, "LZ4F_getVersion");

    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_XXH_versionNumber>("LZ4_XXH_versionNumber")()
    });
    assert_eq!(cv, rv, "LZ4_XXH_versionNumber");

    let (cv, rv) = both(|l| unsafe {
        l.get::<Fn_F_compressionLevel_max>("LZ4F_compressionLevel_max")()
    });
    assert_eq!(cv, rv, "LZ4F_compressionLevel_max");

    for name in [
        "LZ4_sizeofState",
        "LZ4_sizeofStateHC",
        "LZ4_sizeofStreamState",
        "LZ4_sizeofStreamStateHC",
    ] {
        let (cv, rv) = both(|l| unsafe { l.get::<FnI>(name)() });
        assert_eq!(cv, rv, "{}", name);
        assert!(cv > 0, "{} returned {}", name, cv);
    }
}

#[test]
fn round_trip_through_both_libraries() {
    // C compresses, Rust decompresses, and vice versa.
    let mut rng = Rng::new(1);
    let src = gen(&mut rng, 4096, Shape::Text);
    let bound = unsafe { c().get::<Fn_compressBound>("LZ4_compressBound")(src.len() as i32) };
    assert!(bound > 0);

    let mut cbuf = vec![0u8; bound as usize];
    let mut rbuf = vec![0u8; bound as usize];
    let cn = unsafe {
        c().get::<Fn_compress_default>("LZ4_compress_default")(
            src.as_ptr() as *const c_char,
            cbuf.as_mut_ptr() as *mut c_char,
            src.len() as i32,
            bound,
        )
    };
    let rn = unsafe {
        r().get::<Fn_compress_default>("LZ4_compress_default")(
            src.as_ptr() as *const c_char,
            rbuf.as_mut_ptr() as *mut c_char,
            src.len() as i32,
            bound,
        )
    };
    assert_eq!(cn, rn, "compressed size differs");
    assert!(cn > 0);
    assert_bytes_eq!("compressed payload", cbuf[..cn as usize], rbuf[..rn as usize]);

    // cross decompress
    for (name, comp) in [("C-compressed", &cbuf), ("Rust-compressed", &rbuf)] {
        for lib in [c(), r()] {
            let mut out = vec![0u8; src.len()];
            let n = unsafe {
                lib.get::<Fn_decompress_safe>("LZ4_decompress_safe")(
                    comp.as_ptr() as *const c_char,
                    out.as_mut_ptr() as *mut c_char,
                    cn,
                    src.len() as i32,
                )
            };
            assert_eq!(n, src.len() as i32, "{} via {}", name, lib.which);
            assert_bytes_eq!(format!("{} via {}", name, lib.which), out, src);
        }
    }
}
