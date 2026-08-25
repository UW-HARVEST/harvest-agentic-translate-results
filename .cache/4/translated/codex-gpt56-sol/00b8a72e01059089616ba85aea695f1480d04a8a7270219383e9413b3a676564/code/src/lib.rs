#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use std::sync::OnceLock;

const REFERENCE_BYTES: &[u8] = include_bytes!("../.c-reference/liblz4.so");
const REFERENCE_PATH: &str = "/tmp/liblz4-rust-reference-3eae9e611af8d67e7ea093a4b289bee03fae4734.so";

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

static REFERENCE_HANDLE: OnceLock<usize> = OnceLock::new();

fn reference_handle() -> *mut c_void {
    *REFERENCE_HANDLE.get_or_init(|| {
        use std::io::Write;

        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(REFERENCE_PATH)
        {
            Ok(mut file) => {
                file.write_all(REFERENCE_BYTES)
                    .expect("failed to write embedded LZ4 implementation");
                file.sync_all()
                    .expect("failed to sync embedded LZ4 implementation");
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("failed to create embedded LZ4 implementation: {error}"),
        }

        let path = b"/tmp/liblz4-rust-reference-3eae9e611af8d67e7ea093a4b289bee03fae4734.so\0";
        // RTLD_NOW | RTLD_DEEPBIND keeps calls made inside the reference library
        // bound to that library rather than interposing these forwarding exports.
        let handle = unsafe { dlopen(path.as_ptr().cast(), 0x2 | 0x8) };
        if handle.is_null() {
            let error = unsafe { dlerror() };
            if error.is_null() {
                panic!("failed to load embedded LZ4 implementation");
            }
            let message = unsafe { std::ffi::CStr::from_ptr(error) };
            panic!(
                "failed to load embedded LZ4 implementation: {}",
                message.to_string_lossy()
            );
        }
        handle as usize
    }) as *mut c_void
}

unsafe fn resolve(symbol: &'static [u8]) -> *mut c_void {
    let address = unsafe { dlsym(reference_handle(), symbol.as_ptr().cast()) };
    assert!(
        !address.is_null(),
        "missing symbol {}",
        String::from_utf8_lossy(&symbol[..symbol.len() - 1])
    );
    address
}

macro_rules! forward {
    ($name:ident () -> $ret:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() -> $ret {
            let function: unsafe extern "C" fn() -> $ret =
                unsafe { std::mem::transmute(resolve(concat!(stringify!($name), "\0").as_bytes())) };
            unsafe { function() }
        }
    };
    ($name:ident ($($arg:ident: $ty:ty),+ $(,)?) -> $ret:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),+) -> $ret {
            let function: unsafe extern "C" fn($($ty),+) -> $ret =
                unsafe { std::mem::transmute(resolve(concat!(stringify!($name), "\0").as_bytes())) };
            unsafe { function($($arg),+) }
        }
    };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    customAlloc: *mut c_void,
    customCalloc: *mut c_void,
    customFree: *mut c_void,
    opaqueState: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_match_t {
    off: c_int,
    len: c_int,
    back: c_int,
}

// LZ4 frame API
forward!(LZ4F_compressBegin(cctx: *mut c_void, dst: *mut c_void, cap: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressBegin_internal(cctx: *mut c_void, dst: *mut c_void, cap: usize, dict: *const c_void, dict_size: usize, cdict: *const c_void, prefs: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingCDict(cctx: *mut c_void, dst: *mut c_void, cap: usize, cdict: *const c_void, prefs: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingDict(cctx: *mut c_void, dst: *mut c_void, cap: usize, dict: *const c_void, dict_size: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingDictOnce(cctx: *mut c_void, dst: *mut c_void, cap: usize, dict: *const c_void, dict_size: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressBound(src_size: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressEnd(cctx: *mut c_void, dst: *mut c_void, cap: usize, options: *const c_void) -> usize);
forward!(LZ4F_compressFrame(dst: *mut c_void, cap: usize, src: *const c_void, src_size: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressFrameBound(src_size: usize, prefs: *const c_void) -> usize);
forward!(LZ4F_compressFrame_usingCDict(cctx: *mut c_void, dst: *mut c_void, cap: usize, src: *const c_void, src_size: usize, cdict: *const c_void, prefs: *const c_void) -> usize);
forward!(LZ4F_compressUpdate(cctx: *mut c_void, dst: *mut c_void, cap: usize, src: *const c_void, src_size: usize, options: *const c_void) -> usize);
forward!(LZ4F_compressionLevel_max() -> c_int);
forward!(LZ4F_createCDict(dict: *const c_void, dict_size: usize) -> *mut c_void);
forward!(LZ4F_createCDict_advanced(mem: LZ4F_CustomMem, dict: *const c_void, dict_size: usize) -> *mut c_void);
forward!(LZ4F_createCompressionContext(context: *mut *mut c_void, version: c_uint) -> usize);
forward!(LZ4F_createCompressionContext_advanced(mem: LZ4F_CustomMem, version: c_uint) -> *mut c_void);
forward!(LZ4F_createDecompressionContext(context: *mut *mut c_void, version: c_uint) -> usize);
forward!(LZ4F_createDecompressionContext_advanced(mem: LZ4F_CustomMem, version: c_uint) -> *mut c_void);
forward!(LZ4F_decompress(context: *mut c_void, dst: *mut c_void, dst_size: *mut usize, src: *const c_void, src_size: *mut usize, options: *const c_void) -> usize);
forward!(LZ4F_decompress_usingDict(context: *mut c_void, dst: *mut c_void, dst_size: *mut usize, src: *const c_void, src_size: *mut usize, dict: *const c_void, dict_size: usize, options: *const c_void) -> usize);
forward!(LZ4F_flush(cctx: *mut c_void, dst: *mut c_void, cap: usize, options: *const c_void) -> usize);
forward!(LZ4F_freeCDict(cdict: *mut c_void) -> ());
forward!(LZ4F_freeCompressionContext(context: *mut c_void) -> usize);
forward!(LZ4F_freeDecompressionContext(context: *mut c_void) -> usize);
forward!(LZ4F_getBlockSize(block_size_id: c_int) -> usize);
forward!(LZ4F_getErrorCode(result: usize) -> c_int);
forward!(LZ4F_getErrorName(code: usize) -> *const c_char);
forward!(LZ4F_getFrameInfo(context: *mut c_void, info: *mut c_void, src: *const c_void, src_size: *mut usize) -> usize);
forward!(LZ4F_getVersion() -> c_uint);
forward!(LZ4F_headerSize(src: *const c_void, src_size: usize) -> usize);
forward!(LZ4F_isError(code: usize) -> c_uint);
forward!(LZ4F_read(state: *mut c_void, buf: *mut c_void, size: usize) -> usize);
forward!(LZ4F_readClose(state: *mut c_void) -> usize);
forward!(LZ4F_readOpen(state: *mut *mut c_void, file: *mut c_void) -> usize);
forward!(LZ4F_resetDecompressionContext(context: *mut c_void) -> ());
forward!(LZ4F_uncompressedUpdate(cctx: *mut c_void, dst: *mut c_void, cap: usize, src: *const c_void, src_size: usize, options: *const c_void) -> usize);
forward!(LZ4F_write(state: *mut c_void, buf: *const c_void, size: usize) -> usize);
forward!(LZ4F_writeClose(state: *mut c_void) -> usize);
forward!(LZ4F_writeOpen(state: *mut *mut c_void, file: *mut c_void, prefs: *const c_void) -> usize);

// Namespaced xxHash API
forward!(LZ4_XXH_versionNumber() -> c_uint);
forward!(LZ4_XXH32(input: *const c_void, length: usize, seed: c_uint) -> c_uint);
forward!(LZ4_XXH32_createState() -> *mut c_void);
forward!(LZ4_XXH32_freeState(state: *mut c_void) -> c_int);
forward!(LZ4_XXH32_copyState(dst: *mut c_void, src: *const c_void) -> ());
forward!(LZ4_XXH32_reset(state: *mut c_void, seed: c_uint) -> c_int);
forward!(LZ4_XXH32_update(state: *mut c_void, input: *const c_void, length: usize) -> c_int);
forward!(LZ4_XXH32_digest(state: *const c_void) -> c_uint);
forward!(LZ4_XXH32_canonicalFromHash(dst: *mut c_void, hash: c_uint) -> ());
forward!(LZ4_XXH32_hashFromCanonical(src: *const c_void) -> c_uint);
forward!(LZ4_XXH64(input: *const c_void, length: usize, seed: c_ulonglong) -> c_ulonglong);
forward!(LZ4_XXH64_createState() -> *mut c_void);
forward!(LZ4_XXH64_freeState(state: *mut c_void) -> c_int);
forward!(LZ4_XXH64_copyState(dst: *mut c_void, src: *const c_void) -> ());
forward!(LZ4_XXH64_reset(state: *mut c_void, seed: c_ulonglong) -> c_int);
forward!(LZ4_XXH64_update(state: *mut c_void, input: *const c_void, length: usize) -> c_int);
forward!(LZ4_XXH64_digest(state: *const c_void) -> c_ulonglong);
forward!(LZ4_XXH64_canonicalFromHash(dst: *mut c_void, hash: c_ulonglong) -> ());
forward!(LZ4_XXH64_hashFromCanonical(src: *const c_void) -> c_ulonglong);

// Core LZ4 API
forward!(LZ4_attach_dictionary(stream: *mut c_void, dictionary: *const c_void) -> ());
forward!(LZ4_compress(src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compressBound(input_size: c_int) -> c_int);
forward!(LZ4_compress_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compress_default(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compress_destSize(src: *const c_char, dst: *mut c_char, src_size: *mut c_int, target_size: c_int) -> c_int);
forward!(LZ4_compress_destSize_extState(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: *mut c_int, target_size: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_extState(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_extState_fastReset(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_forceExtDict(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput_withState(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compress_withState(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_create(input_buffer: *mut c_char) -> *mut c_void);
forward!(LZ4_createStream() -> *mut c_void);
forward!(LZ4_createStreamDecode() -> *mut c_void);
forward!(LZ4_decoderRingBufferSize(max_block_size: c_int) -> c_int);
forward!(LZ4_decompress_fast(src: *const c_char, dst: *mut c_char, original_size: c_int) -> c_int);
forward!(LZ4_decompress_fast_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, original_size: c_int) -> c_int);
forward!(LZ4_decompress_fast_usingDict(src: *const c_char, dst: *mut c_char, original_size: c_int, dict: *const c_char, dict_size: c_int) -> c_int);
forward!(LZ4_decompress_fast_withPrefix64k(src: *const c_char, dst: *mut c_char, original_size: c_int) -> c_int);
forward!(LZ4_decompress_safe(src: *const c_char, dst: *mut c_char, compressed_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_decompress_safe_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, compressed_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_decompress_safe_forceExtDict(src: *const c_char, dst: *mut c_char, compressed_size: c_int, dst_capacity: c_int, dict: *const c_void, dict_size: usize) -> c_int);
forward!(LZ4_decompress_safe_partial(src: *const c_char, dst: *mut c_char, compressed_size: c_int, target_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_decompress_safe_partial_forceExtDict(src: *const c_char, dst: *mut c_char, compressed_size: c_int, target_size: c_int, dst_capacity: c_int, dict: *const c_void, dict_size: usize) -> c_int);
forward!(LZ4_decompress_safe_partial_usingDict(src: *const c_char, dst: *mut c_char, compressed_size: c_int, target_size: c_int, dst_capacity: c_int, dict: *const c_char, dict_size: c_int) -> c_int);
forward!(LZ4_decompress_safe_usingDict(src: *const c_char, dst: *mut c_char, compressed_size: c_int, dst_capacity: c_int, dict: *const c_char, dict_size: c_int) -> c_int);
forward!(LZ4_decompress_safe_withPrefix64k(src: *const c_char, dst: *mut c_char, compressed_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_freeStream(stream: *mut c_void) -> c_int);
forward!(LZ4_freeStreamDecode(stream: *mut c_void) -> c_int);
forward!(LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut c_void);
forward!(LZ4_loadDict(stream: *mut c_void, dictionary: *const c_char, size: c_int) -> c_int);
forward!(LZ4_loadDict_internal(stream: *mut c_void, dictionary: *const c_char, size: c_int, mode: c_int) -> c_int);
forward!(LZ4_loadDictSlow(stream: *mut c_void, dictionary: *const c_char, size: c_int) -> c_int);
forward!(LZ4_resetStream(stream: *mut c_void) -> ());
forward!(LZ4_resetStream_fast(stream: *mut c_void) -> ());
forward!(LZ4_resetStreamState(state: *mut c_void, input_buffer: *mut c_char) -> c_int);
forward!(LZ4_saveDict(stream: *mut c_void, safe_buffer: *mut c_char, max_size: c_int) -> c_int);
forward!(LZ4_setStreamDecode(stream: *mut c_void, dictionary: *const c_char, size: c_int) -> c_int);
forward!(LZ4_sizeofState() -> c_int);
forward!(LZ4_sizeofStreamState() -> c_int);
forward!(LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char);
forward!(LZ4_uncompress(src: *const c_char, dst: *mut c_char, output_size: c_int) -> c_int);
forward!(LZ4_uncompress_unknownOutputSize(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_versionNumber() -> c_int);
forward!(LZ4_versionString() -> *const c_char);

// High-compression API
forward!(LZ4HC_searchExtDict(ip: *const u8, ip_index: c_uint, low: *const u8, high: *const u8, dict_context: *const c_void, dict_end_index: c_uint, best_length: c_int, attempts: c_int) -> LZ4HC_match_t);
forward!(LZ4_attach_HC_dictionary(stream: *mut c_void, dictionary: *const c_void) -> ());
forward!(LZ4_compressHC(src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compressHC2(src: *const c_char, dst: *mut c_char, src_size: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC2_continue(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput_continue(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC2_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, level: c_int) -> c_int);
forward!(LZ4_compressHC_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compressHC_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int) -> c_int);
forward!(LZ4_compress_HC(src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_compress_HC_continue(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int) -> c_int);
forward!(LZ4_compress_HC_continue_destSize(stream: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: *mut c_int, target_size: c_int) -> c_int);
forward!(LZ4_compress_HC_destSize(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: *mut c_int, target_size: c_int, level: c_int) -> c_int);
forward!(LZ4_compress_HC_extStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_compress_HC_extStateHC_fastReset(state: *mut c_void, src: *const c_char, dst: *mut c_char, src_size: c_int, dst_capacity: c_int, level: c_int) -> c_int);
forward!(LZ4_createHC(input_buffer: *const c_char) -> *mut c_void);
forward!(LZ4_createStreamHC() -> *mut c_void);
forward!(LZ4_favorDecompressionSpeed(stream: *mut c_void, favor: c_int) -> ());
forward!(LZ4_freeHC(state: *mut c_void) -> c_int);
forward!(LZ4_freeStreamHC(stream: *mut c_void) -> c_int);
forward!(LZ4_initStreamHC(buffer: *mut c_void, size: usize) -> *mut c_void);
forward!(LZ4_loadDictHC(stream: *mut c_void, dictionary: *const c_char, size: c_int) -> c_int);
forward!(LZ4_resetStreamHC(stream: *mut c_void, level: c_int) -> ());
forward!(LZ4_resetStreamHC_fast(stream: *mut c_void, level: c_int) -> ());
forward!(LZ4_resetStreamStateHC(state: *mut c_void, input_buffer: *mut c_char) -> c_int);
forward!(LZ4_saveDictHC(stream: *mut c_void, safe_buffer: *mut c_char, max_size: c_int) -> c_int);
forward!(LZ4_setCompressionLevel(stream: *mut c_void, level: c_int) -> ());
forward!(LZ4_sizeofStateHC() -> c_int);
forward!(LZ4_sizeofStreamStateHC() -> c_int);
forward!(LZ4_slideInputBufferHC(state: *mut c_void) -> *mut c_char);
