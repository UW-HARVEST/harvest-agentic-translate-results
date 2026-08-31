//! Translation of `lz4file.c` (LZ4 1.10.0).

use core::ffi::{c_char, c_void};

use crate::lz4frame::*;
use crate::util::*;

unsafe extern "C" {
    fn fread(ptr: *mut c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
    fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
}

#[inline]
fn ret_err(code: usize) -> usize {
    (0usize).wrapping_sub(code)
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

unsafe fn free_read_file(lz4f_read: *mut LZ4_readFile_t) {
    unsafe {
        if lz4f_read.is_null() {
            return;
        }
        LZ4F_freeDecompressionContext((*lz4f_read).dctxPtr);
        free((*lz4f_read).srcBuf as *mut c_void);
        free(lz4f_read as *mut c_void);
    }
}

unsafe fn free_and_null_read_file(state_ptr: *mut *mut LZ4_readFile_t) {
    unsafe {
        free_read_file(*state_ptr);
        *state_ptr = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readOpen(
    lz4f_read: *mut *mut LZ4_readFile_t,
    fp: *mut c_void,
) -> usize {
    unsafe {
        let mut buf = [0u8; LZ4F_HEADER_SIZE_MAX];

        if fp.is_null() || lz4f_read.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }

        *lz4f_read =
            alloc_and_zero(core::mem::size_of::<LZ4_readFile_t>()) as *mut LZ4_readFile_t;
        if (*lz4f_read).is_null() {
            return ret_err(LZ4F_ERROR_allocation_failed);
        }

        let ret = LZ4F_createDecompressionContext(&mut (**lz4f_read).dctxPtr, LZ4F_VERSION);
        if LZ4F_isError(ret) != 0 {
            free_and_null_read_file(lz4f_read);
            return ret;
        }

        (**lz4f_read).fp = fp;
        let mut consumed_size = fread(
            buf.as_mut_ptr() as *mut c_void,
            1,
            LZ4F_HEADER_SIZE_MAX,
            (**lz4f_read).fp,
        );
        if consumed_size != LZ4F_HEADER_SIZE_MAX {
            free_and_null_read_file(lz4f_read);
            return ret_err(LZ4F_ERROR_io_read);
        }

        {
            let mut info = core::mem::MaybeUninit::<LZ4F_frameInfo_t>::zeroed();
            let r = LZ4F_getFrameInfo(
                (**lz4f_read).dctxPtr,
                info.as_mut_ptr(),
                buf.as_ptr() as *const c_void,
                &mut consumed_size,
            );
            if LZ4F_isError(r) != 0 {
                free_and_null_read_file(lz4f_read);
                return r;
            }
            let info = info.assume_init();

            match info.blockSizeID {
                0 | 4 => (**lz4f_read).srcBufMaxSize = 64 * 1024,
                5 => (**lz4f_read).srcBufMaxSize = 256 * 1024,
                6 => (**lz4f_read).srcBufMaxSize = 1024 * 1024,
                7 => (**lz4f_read).srcBufMaxSize = 4 * 1024 * 1024,
                _ => {
                    free_and_null_read_file(lz4f_read);
                    return ret_err(LZ4F_ERROR_maxBlockSize_invalid);
                }
            }
        }

        (**lz4f_read).srcBuf = malloc((**lz4f_read).srcBufMaxSize) as *mut u8;
        if (**lz4f_read).srcBuf.is_null() {
            free_and_null_read_file(lz4f_read);
            return ret_err(LZ4F_ERROR_allocation_failed);
        }

        (**lz4f_read).srcBufSize = LZ4F_HEADER_SIZE_MAX - consumed_size;
        mem_copy(
            (**lz4f_read).srcBuf,
            buf.as_ptr().add(consumed_size),
            (**lz4f_read).srcBufSize,
        );

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_read(
    lz4f_read: *mut LZ4_readFile_t,
    buf: *mut c_void,
    size: usize,
) -> usize {
    unsafe {
        let mut p = buf as *mut u8;
        let mut next: usize = 0;

        if lz4f_read.is_null() || buf.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }

        while next < size {
            let mut srcsize = (*lz4f_read).srcBufSize - (*lz4f_read).srcBufNext;
            let mut dstsize = size - next;

            if srcsize == 0 {
                let ret = fread(
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
                    /* fread() returns a size_t, so `ret == 0` is the only
                     * remaining case; the C code's `else` branch is dead. */
                    break;
                }
            }

            let ret = LZ4F_decompress(
                (*lz4f_read).dctxPtr,
                p as *mut c_void,
                &mut dstsize,
                (*lz4f_read).srcBuf.add((*lz4f_read).srcBufNext) as *const c_void,
                &mut srcsize,
                core::ptr::null(),
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_readClose(lz4f_read: *mut LZ4_readFile_t) -> usize {
    unsafe {
        if lz4f_read.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }
        free_read_file(lz4f_read);
        LZ4F_OK_NoError
    }
}

/* =====   write API   ===== */

#[repr(C)]
pub struct LZ4_writeFile_t {
    pub cctxPtr: *mut LZ4F_cctx,
    pub fp: *mut c_void,
    pub dstBuf: *mut u8,
    pub maxWriteSize: usize,
    pub dstBufMaxSize: usize,
    pub errCode: usize,
}

unsafe fn free_write_file(state: *mut LZ4_writeFile_t) {
    unsafe {
        if state.is_null() {
            return;
        }
        LZ4F_freeCompressionContext((*state).cctxPtr);
        free((*state).dstBuf as *mut c_void);
        free(state as *mut c_void);
    }
}

unsafe fn free_and_null_write_file(state_ptr: *mut *mut LZ4_writeFile_t) {
    unsafe {
        free_write_file(*state_ptr);
        *state_ptr = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeOpen(
    lz4f_write: *mut *mut LZ4_writeFile_t,
    fp: *mut c_void,
    prefs_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        let mut buf = [0u8; LZ4F_HEADER_SIZE_MAX];

        if fp.is_null() || lz4f_write.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }

        *lz4f_write =
            alloc_and_zero(core::mem::size_of::<LZ4_writeFile_t>()) as *mut LZ4_writeFile_t;
        if (*lz4f_write).is_null() {
            return ret_err(LZ4F_ERROR_allocation_failed);
        }
        if !prefs_ptr.is_null() {
            match (*prefs_ptr).frameInfo.blockSizeID {
                0 | 4 => (**lz4f_write).maxWriteSize = 64 * 1024,
                5 => (**lz4f_write).maxWriteSize = 256 * 1024,
                6 => (**lz4f_write).maxWriteSize = 1024 * 1024,
                7 => (**lz4f_write).maxWriteSize = 4 * 1024 * 1024,
                _ => {
                    free_and_null_write_file(lz4f_write);
                    return ret_err(LZ4F_ERROR_maxBlockSize_invalid);
                }
            }
        } else {
            (**lz4f_write).maxWriteSize = 64 * 1024;
        }

        (**lz4f_write).dstBufMaxSize =
            LZ4F_compressBound((**lz4f_write).maxWriteSize, prefs_ptr);
        (**lz4f_write).dstBuf = malloc((**lz4f_write).dstBufMaxSize) as *mut u8;
        if (**lz4f_write).dstBuf.is_null() {
            free_and_null_write_file(lz4f_write);
            return ret_err(LZ4F_ERROR_allocation_failed);
        }

        let mut ret = LZ4F_createCompressionContext(&mut (**lz4f_write).cctxPtr, LZ4F_VERSION);
        if LZ4F_isError(ret) != 0 {
            free_and_null_write_file(lz4f_write);
            return ret;
        }

        ret = LZ4F_compressBegin(
            (**lz4f_write).cctxPtr,
            buf.as_mut_ptr() as *mut c_void,
            LZ4F_HEADER_SIZE_MAX,
            prefs_ptr,
        );
        if LZ4F_isError(ret) != 0 {
            free_and_null_write_file(lz4f_write);
            return ret;
        }

        if ret != fwrite(buf.as_ptr() as *const c_void, 1, ret, fp) {
            free_and_null_write_file(lz4f_write);
            return ret_err(LZ4F_ERROR_io_write);
        }

        (**lz4f_write).fp = fp;
        (**lz4f_write).errCode = LZ4F_OK_NoError;
        LZ4F_OK_NoError
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_write(
    lz4f_write: *mut LZ4_writeFile_t,
    buf: *const c_void,
    size: usize,
) -> usize {
    unsafe {
        let mut p = buf as *const u8;
        let mut remain = size;

        if lz4f_write.is_null() || buf.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }
        while remain != 0 {
            let chunk = if remain > (*lz4f_write).maxWriteSize {
                (*lz4f_write).maxWriteSize
            } else {
                remain
            };

            let ret = LZ4F_compressUpdate(
                (*lz4f_write).cctxPtr,
                (*lz4f_write).dstBuf as *mut c_void,
                (*lz4f_write).dstBufMaxSize,
                p as *const c_void,
                chunk,
                core::ptr::null(),
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
                (*lz4f_write).errCode = ret_err(LZ4F_ERROR_io_write);
                return ret_err(LZ4F_ERROR_io_write);
            }

            p = p.add(chunk);
            remain -= chunk;
        }

        size
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_writeClose(lz4f_write: *mut LZ4_writeFile_t) -> usize {
    unsafe {
        let mut ret = LZ4F_OK_NoError;

        if lz4f_write.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }

        if (*lz4f_write).errCode == LZ4F_OK_NoError {
            ret = LZ4F_compressEnd(
                (*lz4f_write).cctxPtr,
                (*lz4f_write).dstBuf as *mut c_void,
                (*lz4f_write).dstBufMaxSize,
                core::ptr::null(),
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
                    ret = ret_err(LZ4F_ERROR_io_write);
                }
            }
        }

        free_write_file(lz4f_write);
        ret
    }
}

/* silence an unused-import warning for c_char */
const _: Option<*const c_char> = None;
