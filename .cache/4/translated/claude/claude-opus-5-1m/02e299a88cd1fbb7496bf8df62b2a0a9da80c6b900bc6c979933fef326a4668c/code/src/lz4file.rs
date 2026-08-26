//! Translation of `c_src/src/lz4file.c`

use crate::common::{calloc, free, malloc, mem_copy};
use crate::lz4frame::*;
use core::ffi::{c_char, c_void};

#[allow(non_camel_case_types)]
pub enum FILE {}

extern "C" {
    fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
    fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut FILE) -> usize;
}

#[inline]
fn returnErrorCode(code: i32) -> LZ4F_errorCode_t {
    (0isize.wrapping_sub(code as isize)) as usize
}

macro_rules! RETURN_ERROR {
    ($e:expr) => {
        return returnErrorCode($e)
    };
}

/* =====   read API   ===== */

#[repr(C)]
pub struct LZ4_readFile_t {
    pub dctxPtr: *mut LZ4F_dctx,
    pub fp: *mut FILE,
    pub srcBuf: *mut u8,
    pub srcBufNext: usize,
    pub srcBufSize: usize,
    pub srcBufMaxSize: usize,
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
    *statePtr = core::ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readOpen(
    lz4fRead: *mut *mut LZ4_readFile_t,
    fp: *mut FILE,
) -> LZ4F_errorCode_t {
    let mut buf: [c_char; LZ4F_HEADER_SIZE_MAX] = [0; LZ4F_HEADER_SIZE_MAX];
    let mut consumedSize: usize;
    let ret: LZ4F_errorCode_t;

    if fp.is_null() || lz4fRead.is_null() {
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }

    *lz4fRead = calloc(1, core::mem::size_of::<LZ4_readFile_t>()) as *mut LZ4_readFile_t;
    if (*lz4fRead).is_null() {
        RETURN_ERROR!(LZ4F_ERROR_allocation_failed);
    }

    ret = LZ4F_createDecompressionContext(&mut (**lz4fRead).dctxPtr, LZ4F_VERSION);
    if LZ4F_isError_(ret) {
        LZ4F_freeAndNullReadFile(lz4fRead);
        return ret;
    }

    (**lz4fRead).fp = fp;
    consumedSize = fread(
        buf.as_mut_ptr() as *mut c_void,
        1,
        LZ4F_HEADER_SIZE_MAX,
        (**lz4fRead).fp,
    );
    if consumedSize != LZ4F_HEADER_SIZE_MAX {
        LZ4F_freeAndNullReadFile(lz4fRead);
        RETURN_ERROR!(LZ4F_ERROR_io_read);
    }

    {
        let mut info: LZ4F_frameInfo_t = core::mem::zeroed();
        let r = LZ4F_getFrameInfo(
            (**lz4fRead).dctxPtr,
            &mut info,
            buf.as_ptr() as *const c_void,
            &mut consumedSize,
        );
        if LZ4F_isError_(r) {
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
                RETURN_ERROR!(LZ4F_ERROR_maxBlockSize_invalid);
            }
        }
    }

    (**lz4fRead).srcBuf = malloc((**lz4fRead).srcBufMaxSize) as *mut u8;
    if (**lz4fRead).srcBuf.is_null() {
        LZ4F_freeAndNullReadFile(lz4fRead);
        RETURN_ERROR!(LZ4F_ERROR_allocation_failed);
    }

    (**lz4fRead).srcBufSize = LZ4F_HEADER_SIZE_MAX - consumedSize;
    mem_copy(
        (**lz4fRead).srcBuf,
        (buf.as_ptr() as *const u8).wrapping_add(consumedSize),
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
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }

    while next < size {
        let mut srcsize: usize = (*lz4fRead).srcBufSize - (*lz4fRead).srcBufNext;
        let mut dstsize: usize = size - next;
        let mut ret: usize;

        if srcsize == 0 {
            ret = fread(
                (*lz4fRead).srcBuf as *mut c_void,
                1,
                (*lz4fRead).srcBufMaxSize,
                (*lz4fRead).fp,
            );
            if ret > 0 {
                (*lz4fRead).srcBufSize = ret;
                srcsize = (*lz4fRead).srcBufSize;
                (*lz4fRead).srcBufNext = 0;
            } else if ret == 0 {
                break;
            } else {
                RETURN_ERROR!(LZ4F_ERROR_io_read);
            }
        }

        ret = LZ4F_decompress(
            (*lz4fRead).dctxPtr,
            p as *mut c_void,
            &mut dstsize,
            ((*lz4fRead).srcBuf as *const u8).wrapping_add((*lz4fRead).srcBufNext)
                as *const c_void,
            &mut srcsize,
            core::ptr::null(),
        );
        if LZ4F_isError_(ret) {
            return ret;
        }

        (*lz4fRead).srcBufNext += srcsize;
        next += dstsize;
        p = p.wrapping_add(dstsize);
    }

    next
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readClose(lz4fRead: *mut LZ4_readFile_t) -> LZ4F_errorCode_t {
    if lz4fRead.is_null() {
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }
    LZ4F_freeReadFile(lz4fRead);
    LZ4F_OK_NoError as LZ4F_errorCode_t
}

/* =====   write API   ===== */

#[repr(C)]
pub struct LZ4_writeFile_t {
    pub cctxPtr: *mut LZ4F_cctx,
    pub fp: *mut FILE,
    pub dstBuf: *mut u8,
    pub maxWriteSize: usize,
    pub dstBufMaxSize: usize,
    pub errCode: LZ4F_errorCode_t,
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
    *statePtr = core::ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeOpen(
    lz4fWrite: *mut *mut LZ4_writeFile_t,
    fp: *mut FILE,
    prefsPtr: *const LZ4F_preferences_t,
) -> LZ4F_errorCode_t {
    let mut buf: [u8; LZ4F_HEADER_SIZE_MAX] = [0; LZ4F_HEADER_SIZE_MAX];
    let mut ret: usize;

    if fp.is_null() || lz4fWrite.is_null() {
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }

    *lz4fWrite = calloc(1, core::mem::size_of::<LZ4_writeFile_t>()) as *mut LZ4_writeFile_t;
    if (*lz4fWrite).is_null() {
        RETURN_ERROR!(LZ4F_ERROR_allocation_failed);
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
                RETURN_ERROR!(LZ4F_ERROR_maxBlockSize_invalid);
            }
        }
    } else {
        (**lz4fWrite).maxWriteSize = 64 * 1024;
    }

    (**lz4fWrite).dstBufMaxSize = LZ4F_compressBound((**lz4fWrite).maxWriteSize, prefsPtr);
    (**lz4fWrite).dstBuf = malloc((**lz4fWrite).dstBufMaxSize) as *mut u8;
    if (**lz4fWrite).dstBuf.is_null() {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        RETURN_ERROR!(LZ4F_ERROR_allocation_failed);
    }

    ret = LZ4F_createCompressionContext(&mut (**lz4fWrite).cctxPtr, LZ4F_VERSION);
    if LZ4F_isError_(ret) {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        return ret;
    }

    ret = LZ4F_compressBegin(
        (**lz4fWrite).cctxPtr,
        buf.as_mut_ptr() as *mut c_void,
        LZ4F_HEADER_SIZE_MAX,
        prefsPtr,
    );
    if LZ4F_isError_(ret) {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        return ret;
    }

    if ret != fwrite(buf.as_ptr() as *const c_void, 1, ret, fp) {
        LZ4F_freeAndNullWriteFile(lz4fWrite);
        RETURN_ERROR!(LZ4F_ERROR_io_write);
    }

    (**lz4fWrite).fp = fp;
    (**lz4fWrite).errCode = LZ4F_OK_NoError as LZ4F_errorCode_t;
    LZ4F_OK_NoError as LZ4F_errorCode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_write(
    lz4fWrite: *mut LZ4_writeFile_t,
    buf: *const c_void,
    size: usize,
) -> usize {
    let mut p = buf as *const u8;
    let mut remain: usize = size;
    let mut chunk: usize;
    let mut ret: usize;

    if lz4fWrite.is_null() || buf.is_null() {
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }
    while remain != 0 {
        if remain > (*lz4fWrite).maxWriteSize {
            chunk = (*lz4fWrite).maxWriteSize;
        } else {
            chunk = remain;
        }

        ret = LZ4F_compressUpdate(
            (*lz4fWrite).cctxPtr,
            (*lz4fWrite).dstBuf as *mut c_void,
            (*lz4fWrite).dstBufMaxSize,
            p as *const c_void,
            chunk,
            core::ptr::null(),
        );
        if LZ4F_isError_(ret) {
            (*lz4fWrite).errCode = ret;
            return ret;
        }

        if ret
            != fwrite(
                (*lz4fWrite).dstBuf as *const c_void,
                1,
                ret,
                (*lz4fWrite).fp,
            )
        {
            (*lz4fWrite).errCode = returnErrorCode(LZ4F_ERROR_io_write);
            RETURN_ERROR!(LZ4F_ERROR_io_write);
        }

        p = p.wrapping_add(chunk);
        remain -= chunk;
    }

    size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeClose(lz4fWrite: *mut LZ4_writeFile_t) -> LZ4F_errorCode_t {
    let mut ret: LZ4F_errorCode_t = LZ4F_OK_NoError as LZ4F_errorCode_t;

    if lz4fWrite.is_null() {
        RETURN_ERROR!(LZ4F_ERROR_parameter_null);
    }

    'out: {
        if (*lz4fWrite).errCode == LZ4F_OK_NoError as LZ4F_errorCode_t {
            ret = LZ4F_compressEnd(
                (*lz4fWrite).cctxPtr,
                (*lz4fWrite).dstBuf as *mut c_void,
                (*lz4fWrite).dstBufMaxSize,
                core::ptr::null(),
            );
            if LZ4F_isError_(ret) {
                break 'out;
            }

            if ret
                != fwrite(
                    (*lz4fWrite).dstBuf as *const c_void,
                    1,
                    ret,
                    (*lz4fWrite).fp,
                )
            {
                ret = returnErrorCode(LZ4F_ERROR_io_write);
            }
        }
    }

    LZ4F_freeWriteFile(lz4fWrite);
    ret
}
