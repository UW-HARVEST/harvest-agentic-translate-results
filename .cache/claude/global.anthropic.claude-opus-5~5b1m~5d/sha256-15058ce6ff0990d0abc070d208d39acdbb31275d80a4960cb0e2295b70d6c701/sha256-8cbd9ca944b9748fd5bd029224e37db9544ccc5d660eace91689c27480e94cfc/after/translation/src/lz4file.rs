//! Translation of c_src/src/lz4file.c

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::lz4::memcpy_n;
use crate::lz4frame::{
    LZ4F_compressBegin, LZ4F_compressBound, LZ4F_compressEnd, LZ4F_compressUpdate,
    LZ4F_createCompressionContext, LZ4F_createDecompressionContext, LZ4F_decompress,
    LZ4F_errorCode_t, LZ4F_frameInfo_t, LZ4F_freeCompressionContext, LZ4F_freeDecompressionContext,
    LZ4F_getFrameInfo, LZ4F_isError, LZ4F_preferences_t, LZ4F_cctx, LZ4F_dctx,
    LZ4F_ERROR_allocation_failed, LZ4F_ERROR_io_read, LZ4F_ERROR_io_write,
    LZ4F_ERROR_maxBlockSize_invalid, LZ4F_ERROR_parameter_null, LZ4F_HEADER_SIZE_MAX,
    LZ4F_OK_NoError, LZ4F_VERSION,
};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn fread(ptr: *mut c_void, size: usize, n: usize, stream: *mut c_void) -> usize;
    fn fwrite(ptr: *const c_void, size: usize, n: usize, stream: *mut c_void) -> usize;
}

#[inline]
fn return_error_code(code: core::ffi::c_int) -> LZ4F_errorCode_t {
    (0isize).wrapping_sub(code as isize) as usize
}

/* =====   read API   ===== */

#[repr(C)]
pub struct LZ4_readFile_t {
    pub dctxPtr: *mut LZ4F_dctx,
    pub fp: *mut c_void,
    pub srcBuf: *mut u8,
    pub srcBufNext: usize,
    pub srcBufSize: usize,
    pub srcBufMaxSize: usize,
}

unsafe fn LZ4F_freeReadFile(lz4f_read: *mut LZ4_readFile_t) {
    if lz4f_read.is_null() {
        return;
    }
    LZ4F_freeDecompressionContext((*lz4f_read).dctxPtr);
    free((*lz4f_read).srcBuf as *mut c_void);
    free(lz4f_read as *mut c_void);
}

unsafe fn LZ4F_freeAndNullReadFile(state_ptr: *mut *mut LZ4_readFile_t) {
    LZ4F_freeReadFile(*state_ptr);
    *state_ptr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readOpen(
    lz4f_read: *mut *mut LZ4_readFile_t,
    fp: *mut c_void,
) -> LZ4F_errorCode_t {
    let mut buf: [u8; LZ4F_HEADER_SIZE_MAX] = [0; LZ4F_HEADER_SIZE_MAX];
    let mut consumed_size: usize;
    let ret: LZ4F_errorCode_t;

    if fp.is_null() || lz4f_read.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }

    *lz4f_read = calloc(1, core::mem::size_of::<LZ4_readFile_t>()) as *mut LZ4_readFile_t;
    if (*lz4f_read).is_null() {
        return return_error_code(LZ4F_ERROR_allocation_failed);
    }

    ret = LZ4F_createDecompressionContext(&mut (**lz4f_read).dctxPtr, LZ4F_VERSION);
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullReadFile(lz4f_read);
        return ret;
    }

    (**lz4f_read).fp = fp;
    consumed_size = fread(
        buf.as_mut_ptr() as *mut c_void,
        1,
        LZ4F_HEADER_SIZE_MAX,
        (**lz4f_read).fp,
    );
    if consumed_size != LZ4F_HEADER_SIZE_MAX {
        LZ4F_freeAndNullReadFile(lz4f_read);
        return return_error_code(LZ4F_ERROR_io_read);
    }

    {
        let mut info: LZ4F_frameInfo_t = core::mem::zeroed();
        let r = LZ4F_getFrameInfo(
            (**lz4f_read).dctxPtr,
            &mut info,
            buf.as_ptr() as *const c_void,
            &mut consumed_size,
        );
        if LZ4F_isError(r) != 0 {
            LZ4F_freeAndNullReadFile(lz4f_read);
            return r;
        }

        match info.blockSizeID {
            0 | 4 => {
                (**lz4f_read).srcBufMaxSize = 64 * 1024;
            }
            5 => {
                (**lz4f_read).srcBufMaxSize = 256 * 1024;
            }
            6 => {
                (**lz4f_read).srcBufMaxSize = 1024 * 1024;
            }
            7 => {
                (**lz4f_read).srcBufMaxSize = 4 * 1024 * 1024;
            }
            _ => {
                LZ4F_freeAndNullReadFile(lz4f_read);
                return return_error_code(LZ4F_ERROR_maxBlockSize_invalid);
            }
        }
    }

    (**lz4f_read).srcBuf = malloc((**lz4f_read).srcBufMaxSize) as *mut u8;
    if (**lz4f_read).srcBuf.is_null() {
        LZ4F_freeAndNullReadFile(lz4f_read);
        return return_error_code(LZ4F_ERROR_allocation_failed);
    }

    (**lz4f_read).srcBufSize = LZ4F_HEADER_SIZE_MAX - consumed_size;
    memcpy_n(
        (**lz4f_read).srcBuf,
        buf.as_ptr().add(consumed_size),
        (**lz4f_read).srcBufSize,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_read(
    lz4f_read: *mut LZ4_readFile_t,
    buf: *mut c_void,
    size: usize,
) -> usize {
    let mut p = buf as *mut u8;
    let mut next: usize = 0;

    if lz4f_read.is_null() || buf.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }

    while next < size {
        let mut srcsize = (*lz4f_read).srcBufSize - (*lz4f_read).srcBufNext;
        let mut dstsize = size - next;
        let mut ret: usize;

        if srcsize == 0 {
            ret = fread(
                (*lz4f_read).srcBuf as *mut c_void,
                1,
                (*lz4f_read).srcBufMaxSize,
                (*lz4f_read).fp,
            );
            if ret > 0 {
                (*lz4f_read).srcBufSize = ret;
                srcsize = (*lz4f_read).srcBufSize;
                (*lz4f_read).srcBufNext = 0;
            } else {
                break;
            }
        }

        ret = LZ4F_decompress(
            (*lz4f_read).dctxPtr,
            p as *mut c_void,
            &mut dstsize,
            (*lz4f_read).srcBuf.add((*lz4f_read).srcBufNext) as *const c_void,
            &mut srcsize,
            ptr::null(),
        );
        if LZ4F_isError(ret) != 0 {
            return ret;
        }

        (*lz4f_read).srcBufNext += srcsize;
        next += dstsize;
        p = p.add(dstsize);
    }

    next
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readClose(lz4f_read: *mut LZ4_readFile_t) -> LZ4F_errorCode_t {
    if lz4f_read.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }
    LZ4F_freeReadFile(lz4f_read);
    LZ4F_OK_NoError as usize
}

/* =====   write API   ===== */

#[repr(C)]
pub struct LZ4_writeFile_t {
    pub cctxPtr: *mut LZ4F_cctx,
    pub fp: *mut c_void,
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

unsafe fn LZ4F_freeAndNullWriteFile(state_ptr: *mut *mut LZ4_writeFile_t) {
    LZ4F_freeWriteFile(*state_ptr);
    *state_ptr = ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeOpen(
    lz4f_write: *mut *mut LZ4_writeFile_t,
    fp: *mut c_void,
    prefs_ptr: *const LZ4F_preferences_t,
) -> LZ4F_errorCode_t {
    let mut buf: [u8; LZ4F_HEADER_SIZE_MAX] = [0; LZ4F_HEADER_SIZE_MAX];
    let mut ret: usize;

    if fp.is_null() || lz4f_write.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }

    *lz4f_write = calloc(1, core::mem::size_of::<LZ4_writeFile_t>()) as *mut LZ4_writeFile_t;
    if (*lz4f_write).is_null() {
        return return_error_code(LZ4F_ERROR_allocation_failed);
    }
    if !prefs_ptr.is_null() {
        match (*prefs_ptr).frameInfo.blockSizeID {
            0 | 4 => {
                (**lz4f_write).maxWriteSize = 64 * 1024;
            }
            5 => {
                (**lz4f_write).maxWriteSize = 256 * 1024;
            }
            6 => {
                (**lz4f_write).maxWriteSize = 1024 * 1024;
            }
            7 => {
                (**lz4f_write).maxWriteSize = 4 * 1024 * 1024;
            }
            _ => {
                LZ4F_freeAndNullWriteFile(lz4f_write);
                return return_error_code(LZ4F_ERROR_maxBlockSize_invalid);
            }
        }
    } else {
        (**lz4f_write).maxWriteSize = 64 * 1024;
    }

    (**lz4f_write).dstBufMaxSize = LZ4F_compressBound((**lz4f_write).maxWriteSize, prefs_ptr);
    (**lz4f_write).dstBuf = malloc((**lz4f_write).dstBufMaxSize) as *mut u8;
    if (**lz4f_write).dstBuf.is_null() {
        LZ4F_freeAndNullWriteFile(lz4f_write);
        return return_error_code(LZ4F_ERROR_allocation_failed);
    }

    ret = LZ4F_createCompressionContext(&mut (**lz4f_write).cctxPtr, LZ4F_VERSION);
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullWriteFile(lz4f_write);
        return ret;
    }

    ret = LZ4F_compressBegin(
        (**lz4f_write).cctxPtr,
        buf.as_mut_ptr() as *mut c_void,
        LZ4F_HEADER_SIZE_MAX,
        prefs_ptr,
    );
    if LZ4F_isError(ret) != 0 {
        LZ4F_freeAndNullWriteFile(lz4f_write);
        return ret;
    }

    if ret != fwrite(buf.as_ptr() as *const c_void, 1, ret, fp) {
        LZ4F_freeAndNullWriteFile(lz4f_write);
        return return_error_code(LZ4F_ERROR_io_write);
    }

    (**lz4f_write).fp = fp;
    (**lz4f_write).errCode = LZ4F_OK_NoError as usize;
    LZ4F_OK_NoError as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_write(
    lz4f_write: *mut LZ4_writeFile_t,
    buf: *const c_void,
    size: usize,
) -> usize {
    let mut p = buf as *const u8;
    let mut remain = size;
    let mut chunk: usize;
    let mut ret: usize;

    if lz4f_write.is_null() || buf.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }
    while remain != 0 {
        if remain > (*lz4f_write).maxWriteSize {
            chunk = (*lz4f_write).maxWriteSize;
        } else {
            chunk = remain;
        }

        ret = LZ4F_compressUpdate(
            (*lz4f_write).cctxPtr,
            (*lz4f_write).dstBuf as *mut c_void,
            (*lz4f_write).dstBufMaxSize,
            p as *const c_void,
            chunk,
            ptr::null(),
        );
        if LZ4F_isError(ret) != 0 {
            (*lz4f_write).errCode = ret;
            return ret;
        }

        if ret
            != fwrite(
                (*lz4f_write).dstBuf as *const c_void,
                1,
                ret,
                (*lz4f_write).fp,
            )
        {
            (*lz4f_write).errCode = return_error_code(LZ4F_ERROR_io_write);
            return return_error_code(LZ4F_ERROR_io_write);
        }

        p = p.add(chunk);
        remain -= chunk;
    }

    size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeClose(lz4f_write: *mut LZ4_writeFile_t) -> LZ4F_errorCode_t {
    let mut ret: LZ4F_errorCode_t = LZ4F_OK_NoError as usize;

    if lz4f_write.is_null() {
        return return_error_code(LZ4F_ERROR_parameter_null);
    }

    if (*lz4f_write).errCode == LZ4F_OK_NoError as usize {
        ret = LZ4F_compressEnd(
            (*lz4f_write).cctxPtr,
            (*lz4f_write).dstBuf as *mut c_void,
            (*lz4f_write).dstBufMaxSize,
            ptr::null(),
        );
        if LZ4F_isError(ret) == 0 {
            if ret
                != fwrite(
                    (*lz4f_write).dstBuf as *const c_void,
                    1,
                    ret,
                    (*lz4f_write).fp,
                )
            {
                ret = return_error_code(LZ4F_ERROR_io_write);
            }
        }
    }

    LZ4F_freeWriteFile(lz4f_write);
    ret
}
