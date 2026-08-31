#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Lz4fCustomMem {
    pub custom_alloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    pub custom_calloc:
        Option<unsafe extern "C" fn(opaque: *mut c_void, size: usize) -> *mut c_void>,
    pub custom_free:
        Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>,
    pub opaque_state: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Lz4hcMatch {
    pub off: c_int,
    pub len: c_int,
    pub back: c_int,
}

macro_rules! forward {
    ($name:ident($($argument:ident: $argument_type:ty),* $(,)?) -> $return_type:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            $($argument: $argument_type),*
        ) -> $return_type {
            unsafe extern "C" {
                #[link_name = concat!("lz4rs_backend_", stringify!($name))]
                fn backend($($argument: $argument_type),*) -> $return_type;
            }
            unsafe { backend($($argument),*) }
        }
    };
    ($name:ident($($argument:ident: $argument_type:ty),* $(,)?)) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($argument: $argument_type),*) {
            unsafe extern "C" {
                #[link_name = concat!("lz4rs_backend_", stringify!($name))]
                fn backend($($argument: $argument_type),*);
            }
            unsafe { backend($($argument),*) }
        }
    };
}

forward!(LZ4F_compressBegin(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressBegin_internal(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, dictBuffer: *const c_void, dictSize: usize, cdict: *const c_void, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingCDict(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, cdict: *const c_void, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingDict(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, dict: *const c_void, dictSize: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressBegin_usingDictOnce(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, dict: *const c_void, dictSize: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressBound(srcSize: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressEnd(cctxPtr: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, compressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_compressFrame(dstBuffer: *mut c_void, dstCapacity: usize, srcBuffer: *const c_void, srcSize: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressFrameBound(srcSize: usize, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressFrame_usingCDict(cctx: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, srcBuffer: *const c_void, srcSize: usize, cdict: *const c_void, preferencesPtr: *const c_void) -> usize);
forward!(LZ4F_compressUpdate(cctxPtr: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, srcBuffer: *const c_void, srcSize: usize, compressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_compressionLevel_max() -> c_int);
forward!(LZ4F_createCDict(dictBuffer: *const c_void, dictSize: usize) -> *mut c_void);
forward!(LZ4F_createCDict_advanced(cmem: Lz4fCustomMem, dictBuffer: *const c_void, dictSize: usize) -> *mut c_void);
forward!(LZ4F_createCompressionContext(LZ4F_compressionContextPtr: *mut *mut c_void, version: c_uint) -> usize);
forward!(LZ4F_createCompressionContext_advanced(customMem: Lz4fCustomMem, version: c_uint) -> *mut c_void);
forward!(LZ4F_createDecompressionContext(LZ4F_decompressionContextPtr: *mut *mut c_void, versionNumber: c_uint) -> usize);
forward!(LZ4F_createDecompressionContext_advanced(customMem: Lz4fCustomMem, version: c_uint) -> *mut c_void);
forward!(LZ4F_decompress(dctx: *mut c_void, dstBuffer: *mut c_void, dstSizePtr: *mut usize, srcBuffer: *const c_void, srcSizePtr: *mut usize, decompressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_decompress_usingDict(dctx: *mut c_void, dstBuffer: *mut c_void, dstSizePtr: *mut usize, srcBuffer: *const c_void, srcSizePtr: *mut usize, dict: *const c_void, dictSize: usize, decompressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_flush(cctxPtr: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, compressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_freeCDict(cdict: *mut c_void));
forward!(LZ4F_freeCompressionContext(cctxPtr: *mut c_void) -> usize);
forward!(LZ4F_freeDecompressionContext(dctx: *mut c_void) -> usize);
forward!(LZ4F_getBlockSize(blockSizeID: c_uint) -> usize);
forward!(LZ4F_getErrorCode(functionResult: usize) -> c_int);
forward!(LZ4F_getErrorName(code: usize) -> *const c_char);
forward!(LZ4F_getFrameInfo(dctx: *mut c_void, frameInfoPtr: *mut c_void, srcBuffer: *const c_void, srcSizePtr: *mut usize) -> usize);
forward!(LZ4F_getVersion() -> c_uint);
forward!(LZ4F_headerSize(src: *const c_void, srcSize: usize) -> usize);
forward!(LZ4F_isError(code: usize) -> c_uint);
forward!(LZ4F_read(lz4fRead: *mut c_void, buf: *mut c_void, size: usize) -> usize);
forward!(LZ4F_readClose(lz4fRead: *mut c_void) -> usize);
forward!(LZ4F_readOpen(lz4fRead: *mut *mut c_void, fp: *mut c_void) -> usize);
forward!(LZ4F_resetDecompressionContext(dctx: *mut c_void));
forward!(LZ4F_uncompressedUpdate(cctxPtr: *mut c_void, dstBuffer: *mut c_void, dstCapacity: usize, srcBuffer: *const c_void, srcSize: usize, compressOptionsPtr: *const c_void) -> usize);
forward!(LZ4F_write(lz4fWrite: *mut c_void, buf: *const c_void, size: usize) -> usize);
forward!(LZ4F_writeClose(lz4fWrite: *mut c_void) -> usize);
forward!(LZ4F_writeOpen(lz4fWrite: *mut *mut c_void, fp: *mut c_void, prefsPtr: *const c_void) -> usize);
forward!(LZ4HC_searchExtDict(ip: *const u8, ipIndex: c_uint, iLowLimit: *const u8, iHighLimit: *const u8, dictCtx: *const c_void, gDictEndIndex: c_uint, currentBestML: c_int, nbAttempts: c_int) -> Lz4hcMatch);
forward!(LZ4_XXH32(input: *const c_void, len: usize, seed: c_uint) -> c_uint);
forward!(LZ4_XXH32_canonicalFromHash(dst: *mut c_void, hash: c_uint));
forward!(LZ4_XXH32_copyState(dstState: *mut c_void, srcState: *const c_void));
forward!(LZ4_XXH32_createState() -> *mut c_void);
forward!(LZ4_XXH32_digest(state_in: *const c_void) -> c_uint);
forward!(LZ4_XXH32_freeState(statePtr: *mut c_void) -> c_int);
forward!(LZ4_XXH32_hashFromCanonical(src: *const c_void) -> c_uint);
forward!(LZ4_XXH32_reset(statePtr: *mut c_void, seed: c_uint) -> c_int);
forward!(LZ4_XXH32_update(state_in: *mut c_void, input: *const c_void, len: usize) -> c_int);
forward!(LZ4_XXH64(input: *const c_void, len: usize, seed: c_ulonglong) -> c_ulonglong);
forward!(LZ4_XXH64_canonicalFromHash(dst: *mut c_void, hash: c_ulonglong));
forward!(LZ4_XXH64_copyState(dstState: *mut c_void, srcState: *const c_void));
forward!(LZ4_XXH64_createState() -> *mut c_void);
forward!(LZ4_XXH64_digest(state_in: *const c_void) -> c_ulonglong);
forward!(LZ4_XXH64_freeState(statePtr: *mut c_void) -> c_int);
forward!(LZ4_XXH64_hashFromCanonical(src: *const c_void) -> c_ulonglong);
forward!(LZ4_XXH64_reset(statePtr: *mut c_void, seed: c_ulonglong) -> c_int);
forward!(LZ4_XXH64_update(state_in: *mut c_void, input: *const c_void, len: usize) -> c_int);
forward!(LZ4_XXH_versionNumber() -> c_uint);
forward!(LZ4_attach_HC_dictionary(working_stream: *mut c_void, dictionary_stream: *const c_void));
forward!(LZ4_attach_dictionary(workingStream: *mut c_void, dictionaryStream: *const c_void));
forward!(LZ4_compress(src: *const c_char, dest: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_compressBound(isize: c_int) -> c_int);
forward!(LZ4_compressHC(src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_compressHC2(src: *const c_char, dst: *mut c_char, srcSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC2_continue(LZ4HC_Data: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput(src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput_continue(LZ4HC_Data: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC2_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC2_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compressHC_continue(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput(src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput_continue(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int);
forward!(LZ4_compressHC_limitedOutput_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, maxDstSize: c_int) -> c_int);
forward!(LZ4_compressHC_withStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_compress_HC(src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, compressionLevel: c_int) -> c_int);
forward!(LZ4_compress_HC_continue(LZ4_streamHCPtr: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int) -> c_int);
forward!(LZ4_compress_HC_continue_destSize(LZ4_streamHCPtr: *mut c_void, src: *const c_char, dst: *mut c_char, srcSizePtr: *mut c_int, targetDestSize: c_int) -> c_int);
forward!(LZ4_compress_HC_destSize(state: *mut c_void, source: *const c_char, dest: *mut c_char, sourceSizePtr: *mut c_int, targetDestSize: c_int, cLevel: c_int) -> c_int);
forward!(LZ4_compress_HC_extStateHC(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, compressionLevel: c_int) -> c_int);
forward!(LZ4_compress_HC_extStateHC_fastReset(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, compressionLevel: c_int) -> c_int);
forward!(LZ4_compress_continue(LZ4_stream: *mut c_void, source: *const c_char, dest: *mut c_char, inputSize: c_int) -> c_int);
forward!(LZ4_compress_default(src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int) -> c_int);
forward!(LZ4_compress_destSize(src: *const c_char, dst: *mut c_char, srcSizePtr: *mut c_int, targetDstSize: c_int) -> c_int);
forward!(LZ4_compress_destSize_extState(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSizePtr: *mut c_int, targetDstSize: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast(src: *const c_char, dest: *mut c_char, srcSize: c_int, dstCapacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_continue(LZ4_stream: *mut c_void, source: *const c_char, dest: *mut c_char, inputSize: c_int, maxOutputSize: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_extState(state: *mut c_void, source: *const c_char, dest: *mut c_char, inputSize: c_int, maxOutputSize: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_fast_extState_fastReset(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, acceleration: c_int) -> c_int);
forward!(LZ4_compress_forceExtDict(LZ4_dict: *mut c_void, source: *const c_char, dest: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput(source: *const c_char, dest: *mut c_char, inputSize: c_int, maxOutputSize: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput_continue(LZ4_stream: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int) -> c_int);
forward!(LZ4_compress_limitedOutput_withState(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstSize: c_int) -> c_int);
forward!(LZ4_compress_withState(state: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int) -> c_int);
forward!(LZ4_create(inputBuffer: *mut c_char) -> *mut c_void);
forward!(LZ4_createHC(inputBuffer: *const c_char) -> *mut c_void);
forward!(LZ4_createStream() -> *mut c_void);
forward!(LZ4_createStreamDecode() -> *mut c_void);
forward!(LZ4_createStreamHC() -> *mut c_void);
forward!(LZ4_decoderRingBufferSize(maxBlockSize: c_int) -> c_int);
forward!(LZ4_decompress_fast(source: *const c_char, dest: *mut c_char, originalSize: c_int) -> c_int);
forward!(LZ4_decompress_fast_continue(LZ4_streamDecode: *mut c_void, source: *const c_char, dest: *mut c_char, originalSize: c_int) -> c_int);
forward!(LZ4_decompress_fast_usingDict(source: *const c_char, dest: *mut c_char, originalSize: c_int, dictStart: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_decompress_fast_withPrefix64k(source: *const c_char, dest: *mut c_char, originalSize: c_int) -> c_int);
forward!(LZ4_decompress_safe(source: *const c_char, dest: *mut c_char, compressedSize: c_int, maxDecompressedSize: c_int) -> c_int);
forward!(LZ4_decompress_safe_continue(LZ4_streamDecode: *mut c_void, source: *const c_char, dest: *mut c_char, compressedSize: c_int, maxOutputSize: c_int) -> c_int);
forward!(LZ4_decompress_safe_forceExtDict(source: *const c_char, dest: *mut c_char, compressedSize: c_int, maxOutputSize: c_int, dictStart: *const c_void, dictSize: usize) -> c_int);
forward!(LZ4_decompress_safe_partial(src: *const c_char, dst: *mut c_char, compressedSize: c_int, targetOutputSize: c_int, dstCapacity: c_int) -> c_int);
forward!(LZ4_decompress_safe_partial_forceExtDict(source: *const c_char, dest: *mut c_char, compressedSize: c_int, targetOutputSize: c_int, dstCapacity: c_int, dictStart: *const c_void, dictSize: usize) -> c_int);
forward!(LZ4_decompress_safe_partial_usingDict(source: *const c_char, dest: *mut c_char, compressedSize: c_int, targetOutputSize: c_int, dstCapacity: c_int, dictStart: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_decompress_safe_usingDict(source: *const c_char, dest: *mut c_char, compressedSize: c_int, maxOutputSize: c_int, dictStart: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_decompress_safe_withPrefix64k(source: *const c_char, dest: *mut c_char, compressedSize: c_int, maxOutputSize: c_int) -> c_int);
forward!(LZ4_favorDecompressionSpeed(LZ4_streamHCPtr: *mut c_void, favor: c_int));
forward!(LZ4_freeHC(LZ4HC_Data: *mut c_void) -> c_int);
forward!(LZ4_freeStream(LZ4_stream: *mut c_void) -> c_int);
forward!(LZ4_freeStreamDecode(LZ4_stream: *mut c_void) -> c_int);
forward!(LZ4_freeStreamHC(LZ4_streamHCPtr: *mut c_void) -> c_int);
forward!(LZ4_initStream(buffer: *mut c_void, size: usize) -> *mut c_void);
forward!(LZ4_initStreamHC(buffer: *mut c_void, size: usize) -> *mut c_void);
forward!(LZ4_loadDict(LZ4_dict: *mut c_void, dictionary: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_loadDictHC(LZ4_streamHCPtr: *mut c_void, dictionary: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_loadDictSlow(LZ4_dict: *mut c_void, dictionary: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_loadDict_internal(LZ4_dict: *mut c_void, dictionary: *const c_char, dictSize: c_int, _ld: c_int) -> c_int);
forward!(LZ4_resetStream(LZ4_stream: *mut c_void));
forward!(LZ4_resetStreamHC(LZ4_streamHCPtr: *mut c_void, compressionLevel: c_int));
forward!(LZ4_resetStreamHC_fast(LZ4_streamHCPtr: *mut c_void, compressionLevel: c_int));
forward!(LZ4_resetStreamState(state: *mut c_void, inputBuffer: *mut c_char) -> c_int);
forward!(LZ4_resetStreamStateHC(state: *mut c_void, inputBuffer: *mut c_char) -> c_int);
forward!(LZ4_resetStream_fast(ctx: *mut c_void));
forward!(LZ4_saveDict(LZ4_dict: *mut c_void, safeBuffer: *mut c_char, dictSize: c_int) -> c_int);
forward!(LZ4_saveDictHC(LZ4_streamHCPtr: *mut c_void, safeBuffer: *mut c_char, dictSize: c_int) -> c_int);
forward!(LZ4_setCompressionLevel(LZ4_streamHCPtr: *mut c_void, compressionLevel: c_int));
forward!(LZ4_setStreamDecode(LZ4_streamDecode: *mut c_void, dictionary: *const c_char, dictSize: c_int) -> c_int);
forward!(LZ4_sizeofState() -> c_int);
forward!(LZ4_sizeofStateHC() -> c_int);
forward!(LZ4_sizeofStreamState() -> c_int);
forward!(LZ4_sizeofStreamStateHC() -> c_int);
forward!(LZ4_slideInputBuffer(state: *mut c_void) -> *mut c_char);
forward!(LZ4_slideInputBufferHC(LZ4HC_Data: *mut c_void) -> *mut c_char);
forward!(LZ4_uncompress(source: *const c_char, dest: *mut c_char, outputSize: c_int) -> c_int);
forward!(LZ4_uncompress_unknownOutputSize(source: *const c_char, dest: *mut c_char, isize: c_int, maxOutputSize: c_int) -> c_int);
forward!(LZ4_versionNumber() -> c_int);
forward!(LZ4_versionString() -> *const c_char);
