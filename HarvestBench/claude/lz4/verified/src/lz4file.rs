// Translation of lz4file.c (LZ4 file v1.10.0).
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ffi::c_void;
use core::ptr;

use crate::lz4frame::{
    LZ4F_cctx, LZ4F_compressBegin, LZ4F_compressBound, LZ4F_compressEnd, LZ4F_compressUpdate,
    LZ4F_createCompressionContext, LZ4F_createDecompressionContext, LZ4F_decompress, LZ4F_dctx,
    LZ4F_default, LZ4F_errorCode_t, LZ4F_frameInfo_t, LZ4F_freeCompressionContext,
    LZ4F_freeDecompressionContext, LZ4F_getFrameInfo, LZ4F_isError, LZ4F_max1MB, LZ4F_max256KB,
    LZ4F_max4MB, LZ4F_max64KB, LZ4F_preferences_t,
};

const LZ4F_VERSION: core::ffi::c_uint = 100;
const LZ4F_HEADER_SIZE_MAX: usize = 19;

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

extern "C" {
    fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

mod err {
    pub const OK_NoError: usize = 0;
    pub const ERROR_maxBlockSize_invalid: usize = 2;
    pub const ERROR_allocation_failed: usize = 9;
    pub const ERROR_parameter_null: usize = 21;
    pub const ERROR_io_write: usize = 22;
    pub const ERROR_io_read: usize = 23;
}

#[inline]
fn returnErrorCode(code: usize) -> LZ4F_errorCode_t {
    (0usize).wrapping_sub(code)
}

macro_rules! RETURN_ERROR {
    ($e:ident) => {
        return returnErrorCode(err::$e)
    };
}

/* ===== read API ===== */
#[repr(C)]
pub struct LZ4_readFile_t {
    dctxPtr: *mut LZ4F_dctx,
    fp: *mut FILE,
    srcBuf: *mut u8,
    srcBufNext: usize,
    srcBufSize: usize,
    srcBufMaxSize: usize,
}

unsafe fn LZ4F_freeReadFile(lz4fRead: *mut LZ4_readFile_t) {
    if lz4fRead.is_null() {
        return;
    }
    LZ4F_freeDecompressionContext((*lz4fRead).dctxPtr);
    free((*lz4fRead).srcBuf as *mut c_void);
    free(lz4fRead as *mut c_void);
}

unsafe fn LZ4F_freeAndNullReadFile(statePtr: *mut *mut LZ4_readFile_t) {
    LZ4F_freeReadFile(*statePtr);
    *statePtr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readOpen(
    lz4fRead: *mut *mut LZ4_readFile_t,
    fp: *mut FILE,
) -> LZ4F_errorCode_t {
    let mut buf = [0u8; LZ4F_HEADER_SIZE_MAX];
    let mut consumedSize: usize;
    let ret: LZ4F_errorCode_t;

    if fp.is_null() || lz4fRead.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }

    *lz4fRead = calloc(1, core::mem::size_of::<LZ4_readFile_t>()) as *mut LZ4_readFile_t;
    if (*lz4fRead).is_null() {
        RETURN_ERROR!(ERROR_allocation_failed);
    }

    ret = LZ4F_createDecompressionContext(&mut (**lz4fRead).dctxPtr, LZ4F_VERSION);
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullReadFile(lz4fRead);
        return ret;
    }

    (**lz4fRead).fp = fp;
    consumedSize = fread(buf.as_mut_ptr() as *mut c_void, 1, buf.len(), (**lz4fRead).fp);
    if consumedSize != buf.len() {
        LZ4F_freeAndNullReadFile(lz4fRead);
        RETURN_ERROR!(ERROR_io_read);
    }

    {
        let mut info: LZ4F_frameInfo_t = core::mem::zeroed();
        let r = LZ4F_getFrameInfo(
            (**lz4fRead).dctxPtr,
            &mut info,
            buf.as_ptr() as *const c_void,
            &mut consumedSize,
        );
        if LZ4F_isError(r) != 0 {
            LZ4F_freeAndNullReadFile(lz4fRead);
            return r;
        }

        match info.blockSizeID {
            x if x == LZ4F_default || x == LZ4F_max64KB => {
                (**lz4fRead).srcBufMaxSize = 64 * 1024;
            }
            x if x == LZ4F_max256KB => {
                (**lz4fRead).srcBufMaxSize = 256 * 1024;
            }
            x if x == LZ4F_max1MB => {
                (**lz4fRead).srcBufMaxSize = 1 * 1024 * 1024;
            }
            x if x == LZ4F_max4MB => {
                (**lz4fRead).srcBufMaxSize = 4 * 1024 * 1024;
            }
            _ => {
                LZ4F_freeAndNullReadFile(lz4fRead);
                RETURN_ERROR!(ERROR_maxBlockSize_invalid);
            }
        }
    }

    (**lz4fRead).srcBuf = malloc((**lz4fRead).srcBufMaxSize) as *mut u8;
    if (**lz4fRead).srcBuf.is_null() {
        LZ4F_freeAndNullReadFile(lz4fRead);
        RETURN_ERROR!(ERROR_allocation_failed);
    }

    (**lz4fRead).srcBufSize = buf.len() - consumedSize;
    memcpy(
        (**lz4fRead).srcBuf as *mut c_void,
        buf.as_ptr().add(consumedSize) as *const c_void,
        (**lz4fRead).srcBufSize,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_read(
    lz4fRead: *mut LZ4_readFile_t,
    buf: *mut c_void,
    size: usize,
) -> usize {
    let mut p = buf as *mut u8;
    let mut next: usize = 0;

    if lz4fRead.is_null() || buf.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }

    while next < size {
        let mut srcsize = (*lz4fRead).srcBufSize - (*lz4fRead).srcBufNext;
        let mut dstsize = size - next;
        let ret: usize;

        if srcsize == 0 {
            let r = fread(
                (*lz4fRead).srcBuf as *mut c_void,
                1,
                (*lz4fRead).srcBufMaxSize,
                (*lz4fRead).fp,
            );
            if r > 0 {
                (*lz4fRead).srcBufSize = r;
                srcsize = (*lz4fRead).srcBufSize;
                (*lz4fRead).srcBufNext = 0;
            } else {
                // fread returns size_t; 0 => EOF => break
                break;
            }
        }

        ret = LZ4F_decompress(
            (*lz4fRead).dctxPtr,
            p as *mut c_void,
            &mut dstsize,
            (*lz4fRead).srcBuf.add((*lz4fRead).srcBufNext) as *const c_void,
            &mut srcsize,
            ptr::null(),
        );
        if LZ4F_isError(ret) != 0 {
            return ret;
        }

        (*lz4fRead).srcBufNext += srcsize;
        next += dstsize;
        p = p.add(dstsize);
    }

    next
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readClose(lz4fRead: *mut LZ4_readFile_t) -> LZ4F_errorCode_t {
    if lz4fRead.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }
    LZ4F_freeReadFile(lz4fRead);
    err::OK_NoError
}

/* ===== write API ===== */
#[repr(C)]
pub struct LZ4_writeFile_t {
    cctxPtr: *mut LZ4F_cctx,
    fp: *mut FILE,
    dstBuf: *mut u8,
    maxWriteSize: usize,
    dstBufMaxSize: usize,
    errCode: LZ4F_errorCode_t,
}

unsafe fn LZ4F_freeWriteFile(state: *mut LZ4_writeFile_t) {
    if state.is_null() {
        return;
    }
    LZ4F_freeCompressionContext((*state).cctxPtr);
    free((*state).dstBuf as *mut c_void);
    free(state as *mut c_void);
}

unsafe fn LZ4F_freeAndNullWriteFile(statePtr: *mut *mut LZ4_writeFile_t) {
    LZ4F_freeWriteFile(*statePtr);
    *statePtr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeOpen(
    lz4fWrite: *mut *mut LZ4_writeFile_t,
    fp: *mut FILE,
    prefsPtr: *const LZ4F_preferences_t,
) -> LZ4F_errorCode_t {
    let mut buf = [0u8; LZ4F_HEADER_SIZE_MAX];
    let mut ret: usize;

    if fp.is_null() || lz4fWrite.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }

    *lz4fWrite = calloc(1, core::mem::size_of::<LZ4_writeFile_t>()) as *mut LZ4_writeFile_t;
    if (*lz4fWrite).is_null() {
        RETURN_ERROR!(ERROR_allocation_failed);
    }
    if !prefsPtr.is_null() {
        match (*prefsPtr).frameInfo.blockSizeID {
            x if x == LZ4F_default || x == LZ4F_max64KB => {
                (**lz4fWrite).maxWriteSize = 64 * 1024;
            }
            x if x == LZ4F_max256KB => {
                (**lz4fWrite).maxWriteSize = 256 * 1024;
            }
            x if x == LZ4F_max1MB => {
                (**lz4fWrite).maxWriteSize = 1 * 1024 * 1024;
            }
            x if x == LZ4F_max4MB => {
                (**lz4fWrite).maxWriteSize = 4 * 1024 * 1024;
            }
            _ => {
                LZ4F_freeAndNullWriteFile(lz4fWrite);
                RETURN_ERROR!(ERROR_maxBlockSize_invalid);
            }
        }
    } else {
        (**lz4fWrite).maxWriteSize = 64 * 1024;
    }

    (**lz4fWrite).dstBufMaxSize = LZ4F_compressBound((**lz4fWrite).maxWriteSize, prefsPtr);
    (**lz4fWrite).dstBuf = malloc((**lz4fWrite).dstBufMaxSize) as *mut u8;
    if (**lz4fWrite).dstBuf.is_null() {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        RETURN_ERROR!(ERROR_allocation_failed);
    }

    ret = LZ4F_createCompressionContext(&mut (**lz4fWrite).cctxPtr, LZ4F_VERSION);
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        return ret;
    }

    ret = LZ4F_compressBegin(
        (**lz4fWrite).cctxPtr,
        buf.as_mut_ptr() as *mut c_void,
        LZ4F_HEADER_SIZE_MAX,
        prefsPtr,
    );
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        return ret;
    }

    if ret != fwrite(buf.as_ptr() as *const c_void, 1, ret, fp) {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        RETURN_ERROR!(ERROR_io_write);
    }

    (**lz4fWrite).fp = fp;
    (**lz4fWrite).errCode = err::OK_NoError;
    err::OK_NoError
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_write(
    lz4fWrite: *mut LZ4_writeFile_t,
    buf: *const c_void,
    size: usize,
) -> usize {
    let mut p = buf as *const u8;
    let mut remain = size;
    let mut chunk: usize;

    if lz4fWrite.is_null() || buf.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }
    while remain != 0 {
        if remain > (*lz4fWrite).maxWriteSize {
            chunk = (*lz4fWrite).maxWriteSize;
        } else {
            chunk = remain;
        }

        let r = LZ4F_compressUpdate(
            (*lz4fWrite).cctxPtr,
            (*lz4fWrite).dstBuf as *mut c_void,
            (*lz4fWrite).dstBufMaxSize,
            p as *const c_void,
            chunk,
            ptr::null(),
        );
        if LZ4F_isError(r) != 0 {
            (*lz4fWrite).errCode = r;
            return r;
        }

        if r != fwrite((*lz4fWrite).dstBuf as *const c_void, 1, r, (*lz4fWrite).fp) {
            (*lz4fWrite).errCode = returnErrorCode(err::ERROR_io_write);
            RETURN_ERROR!(ERROR_io_write);
        }

        p = p.add(chunk);
        remain -= chunk;
    }

    size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeClose(lz4fWrite: *mut LZ4_writeFile_t) -> LZ4F_errorCode_t {
    let mut ret: LZ4F_errorCode_t = err::OK_NoError;

    if lz4fWrite.is_null() {
        RETURN_ERROR!(ERROR_parameter_null);
    }

    if (*lz4fWrite).errCode == err::OK_NoError {
        ret = LZ4F_compressEnd(
            (*lz4fWrite).cctxPtr,
            (*lz4fWrite).dstBuf as *mut c_void,
            (*lz4fWrite).dstBufMaxSize,
            ptr::null(),
        );
        if LZ4F_isError(ret) != 0 {
            LZ4F_freeWriteFile(lz4fWrite);
            return ret;
        }

        if ret != fwrite((*lz4fWrite).dstBuf as *const c_void, 1, ret, (*lz4fWrite).fp) {
            ret = returnErrorCode(err::ERROR_io_write);
        }
    }

    LZ4F_freeWriteFile(lz4fWrite);
    ret
}
