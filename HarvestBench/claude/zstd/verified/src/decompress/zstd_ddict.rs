//! Translation of `decompress/zstd_ddict.c`
#![allow(dead_code)]

use crate::common::error_private::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use crate::decompress::zstd_decompress_internal::*;
use crate::libc::*;
use core::ffi::{c_char, c_uint, c_void};

extern "C" {
    /* decompress/zstd_decompress.c */
    fn ZSTD_loadDEntropy(
        entropy: *mut ZSTD_entropyDTables_t,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictContent(ddict: *const ZSTD_DDict) -> *const c_void {
    (*ddict).dictContent
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictSize(ddict: *const ZSTD_DDict) -> usize {
    (*ddict).dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDDictParameters(
    dctx: *mut ZSTD_DCtx,
    ddict: *const ZSTD_DDict,
) {
    (*dctx).dictID = (*ddict).dictID;
    (*dctx).prefixStart = (*ddict).dictContent;
    (*dctx).virtualStart = (*ddict).dictContent;
    (*dctx).dictEnd =
        ((*ddict).dictContent as *const BYTE).add((*ddict).dictSize) as *const c_void;
    (*dctx).previousDstEnd = (*dctx).dictEnd;
    if (*ddict).entropyPresent != 0 {
        (*dctx).litEntropy = 1;
        (*dctx).fseEntropy = 1;
        (*dctx).LLTptr = (*ddict).entropy.LLTable.as_ptr();
        (*dctx).MLTptr = (*ddict).entropy.MLTable.as_ptr();
        (*dctx).OFTptr = (*ddict).entropy.OFTable.as_ptr();
        (*dctx).HUFptr = (*ddict).entropy.hufTable.as_ptr();
        (*dctx).entropy.rep[0] = (*ddict).entropy.rep[0];
        (*dctx).entropy.rep[1] = (*ddict).entropy.rep[1];
        (*dctx).entropy.rep[2] = (*ddict).entropy.rep[2];
    } else {
        (*dctx).litEntropy = 0;
        (*dctx).fseEntropy = 0;
    }
}

unsafe fn ZSTD_loadEntropy_intoDDict(
    ddict: *mut ZSTD_DDict,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    (*ddict).dictID = 0;
    (*ddict).entropyPresent = 0;
    if dictContentType == ZSTD_dct_rawContent {
        return 0;
    }

    if (*ddict).dictSize < 8 {
        if dictContentType == ZSTD_dct_fullDict {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        return 0; /* pure content mode */
    }
    {
        let magic = MEM_readLE32((*ddict).dictContent);
        if magic != ZSTD_MAGIC_DICTIONARY {
            if dictContentType == ZSTD_dct_fullDict {
                return ERROR(ZSTD_error_dictionary_corrupted);
            }
            return 0; /* pure content mode */
        }
    }
    (*ddict).dictID = MEM_readLE32(
        ((*ddict).dictContent as *const c_char).add(ZSTD_FRAMEIDSIZE) as *const c_void,
    );

    /* load entropy tables */
    if ERR_isError(ZSTD_loadDEntropy(
        core::ptr::addr_of_mut!((*ddict).entropy),
        (*ddict).dictContent,
        (*ddict).dictSize,
    )) != 0
    {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*ddict).entropyPresent = 1;
    0
}

unsafe fn ZSTD_initDDict_internal(
    ddict: *mut ZSTD_DDict,
    dict: *const c_void,
    mut dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    if (dictLoadMethod == ZSTD_dlm_byRef) || dict.is_null() || (dictSize == 0) {
        (*ddict).dictBuffer = core::ptr::null_mut();
        (*ddict).dictContent = dict;
        if dict.is_null() {
            dictSize = 0;
        }
    } else {
        let internalBuffer = ZSTD_customMalloc(dictSize, (*ddict).cMem);
        (*ddict).dictBuffer = internalBuffer;
        (*ddict).dictContent = internalBuffer;
        if internalBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        ZSTD_memcpy(internalBuffer, dict, dictSize);
    }
    (*ddict).dictSize = dictSize;
    (*ddict).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x1000001)) as HUF_DTable;

    /* parse dictionary content */
    {
        let err_code = ZSTD_loadEntropy_intoDDict(ddict, dictContentType);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict_advanced(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_DDict {
    if (customMem.customAlloc.is_none() as i32) ^ (customMem.customFree.is_none() as i32) != 0 {
        return core::ptr::null_mut();
    }

    {
        let ddict =
            ZSTD_customMalloc(core::mem::size_of::<ZSTD_DDict>(), customMem) as *mut ZSTD_DDict;
        if ddict.is_null() {
            return core::ptr::null_mut();
        }
        (*ddict).cMem = customMem;
        {
            let initResult =
                ZSTD_initDDict_internal(ddict, dict, dictSize, dictLoadMethod, dictContentType);
            if ERR_isError(initResult) != 0 {
                ZSTD_freeDDict(ddict);
                return core::ptr::null_mut();
            }
        }
        ddict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict(
    dict: *const c_void,
    dictSize: usize,
) -> *mut ZSTD_DDict {
    let allocator = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict_byReference(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> *mut ZSTD_DDict {
    let allocator = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dictBuffer, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDDict(
    sBuffer: *mut c_void,
    sBufferSize: usize,
    mut dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    let neededSpace = core::mem::size_of::<ZSTD_DDict>()
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            dictSize
        });
    let ddict = sBuffer as *mut ZSTD_DDict;
    if (sBuffer as usize) & 7 != 0 {
        return core::ptr::null();
    }
    if sBufferSize < neededSpace {
        return core::ptr::null();
    }
    if dictLoadMethod == ZSTD_dlm_byCopy {
        ZSTD_memcpy(ddict.add(1) as *mut c_void, dict, dictSize);
        dict = ddict.add(1) as *const c_void;
    }
    if ERR_isError(ZSTD_initDDict_internal(
        ddict,
        dict,
        dictSize,
        ZSTD_dlm_byRef,
        dictContentType,
    )) != 0
    {
        return core::ptr::null();
    }
    ddict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize {
    if ddict.is_null() {
        return 0;
    }
    {
        let cMem = (*ddict).cMem;
        ZSTD_customFree((*ddict).dictBuffer, cMem);
        ZSTD_customFree(ddict as *mut c_void, cMem);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDDictSize(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    core::mem::size_of::<ZSTD_DDict>()
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            dictSize
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> usize {
    if ddict.is_null() {
        return 0;
    }
    core::mem::size_of::<ZSTD_DDict>()
        + (if !(*ddict).dictBuffer.is_null() {
            (*ddict).dictSize
        } else {
            0
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> c_uint {
    if ddict.is_null() {
        return 0;
    }
    (*ddict).dictID
}
