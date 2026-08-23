/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

/* zstd_ddict.c :
 * concentrates all logic that needs to know the internals of ZSTD_DDict object */

use core::ffi::c_void;

use crate::common::allocations::{
    zstd_custom_free, zstd_custom_malloc, ZSTD_customMem,
};
use crate::common::error::{code, err_is_error, error};
use crate::common::mem::mem_read_le32;
use crate::common::zstd_internal::ZSTD_FRAMEIDSIZE;
use crate::zstd_h::{
    ZSTD_dct_fullDict, ZSTD_dct_rawContent, ZSTD_dictContentType_e, ZSTD_dictLoadMethod_e,
    ZSTD_dct_auto, ZSTD_dlm_byCopy, ZSTD_dlm_byRef, ZSTD_MAGIC_DICTIONARY,
};

use crate::decompress::zstd_decompress_internal::{
    HUF_DTable, ZSTD_entropyDTables_t, ZSTD_DCtx, ZSTD_DDict, ZSTD_HUFFDTABLE_CAPACITY_LOG,
};

extern "C" {
    fn ZSTD_loadDEntropy(
        entropy: *mut ZSTD_entropyDTables_t,
        dict: *const c_void,
        dictSize: usize,
    ) -> usize;
}

#[inline]
fn zstd_is_error(code: usize) -> bool {
    err_is_error(code) != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictContent(ddict: *const ZSTD_DDict) -> *const c_void {
    debug_assert!(!ddict.is_null());
    (*ddict).dictContent
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictSize(ddict: *const ZSTD_DDict) -> usize {
    debug_assert!(!ddict.is_null());
    (*ddict).dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDDictParameters(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict) {
    debug_assert!(!dctx.is_null());
    debug_assert!(!ddict.is_null());
    (*dctx).dictID = (*ddict).dictID;
    (*dctx).prefixStart = (*ddict).dictContent;
    (*dctx).virtualStart = (*ddict).dictContent;
    (*dctx).dictEnd = ((*ddict).dictContent as *const u8).add((*ddict).dictSize) as *const c_void;
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
            return error(code::DICTIONARY_CORRUPTED); /* only accept specified dictionaries */
        }
        return 0; /* pure content mode */
    }
    {
        let magic: u32 = mem_read_le32((*ddict).dictContent);
        if magic != ZSTD_MAGIC_DICTIONARY {
            if dictContentType == ZSTD_dct_fullDict {
                return error(code::DICTIONARY_CORRUPTED); /* only accept specified dictionaries */
            }
            return 0; /* pure content mode */
        }
    }
    (*ddict).dictID =
        mem_read_le32(((*ddict).dictContent as *const u8).add(ZSTD_FRAMEIDSIZE) as *const c_void);

    /* load entropy tables */
    if zstd_is_error(ZSTD_loadDEntropy(
        &mut (*ddict).entropy,
        (*ddict).dictContent,
        (*ddict).dictSize,
    )) {
        return error(code::DICTIONARY_CORRUPTED);
    }
    (*ddict).entropyPresent = 1;
    0
}

unsafe fn ZSTD_initDDict_internal(
    ddict: *mut ZSTD_DDict,
    mut dict: *const c_void,
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
        let internalBuffer: *mut c_void = zstd_custom_malloc(dictSize, (*ddict).cMem);
        (*ddict).dictBuffer = internalBuffer;
        (*ddict).dictContent = internalBuffer;
        if internalBuffer.is_null() {
            return error(code::MEMORY_ALLOCATION);
        }
        crate::common::allocations::memcpy(internalBuffer, dict, dictSize);
    }
    (*ddict).dictSize = dictSize;
    (*ddict).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x1000001)) as HUF_DTable; /* cover both little and big endian */

    /* parse dictionary content */
    {
        let _err = ZSTD_loadEntropy_intoDDict(ddict, dictContentType);
        if zstd_is_error(_err) {
            return _err;
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
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }

    {
        let ddict: *mut ZSTD_DDict =
            zstd_custom_malloc(core::mem::size_of::<ZSTD_DDict>(), customMem) as *mut ZSTD_DDict;
        if ddict.is_null() {
            return core::ptr::null_mut();
        }
        (*ddict).cMem = customMem;
        {
            let initResult: usize =
                ZSTD_initDDict_internal(ddict, dict, dictSize, dictLoadMethod, dictContentType);
            if zstd_is_error(initResult) {
                ZSTD_freeDDict(ddict);
                return core::ptr::null_mut();
            }
        }
        ddict
    }
}

/*  ZSTD_createDDict() :
 *   Create a digested dictionary, to start decompression without startup delay.
 *   `dict` content is copied inside DDict.
 *   Consequently, `dict` can be released after `ZSTD_DDict` creation */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict(dict: *const c_void, dictSize: usize) -> *mut ZSTD_DDict {
    let allocator = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto, allocator)
}

/*  ZSTD_createDDict_byReference() :
 *  Create a digested dictionary, to start decompression without startup delay.
 *  Dictionary content is simply referenced, it will be accessed during decompression.
 *  Warning : dictBuffer must outlive DDict (DDict must be freed before dictBuffer) */
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
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    let neededSpace: usize = core::mem::size_of::<ZSTD_DDict>()
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            dictSize
        });
    let ddict: *mut ZSTD_DDict = sBuffer as *mut ZSTD_DDict;
    let mut dict = dict;
    debug_assert!(!sBuffer.is_null());
    debug_assert!(!dict.is_null());
    if (sBuffer as usize) & 7 != 0 {
        return core::ptr::null(); /* 8-aligned */
    }
    if sBufferSize < neededSpace {
        return core::ptr::null();
    }
    if dictLoadMethod == ZSTD_dlm_byCopy {
        crate::common::allocations::memcpy(ddict.add(1) as *mut c_void, dict, dictSize); /* local copy */
        dict = ddict.add(1) as *const c_void;
    }
    if zstd_is_error(ZSTD_initDDict_internal(
        ddict,
        dict,
        dictSize,
        ZSTD_dlm_byRef,
        dictContentType,
    )) {
        return core::ptr::null();
    }
    ddict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize {
    if ddict.is_null() {
        return 0; /* support free on NULL */
    }
    {
        let cMem: ZSTD_customMem = (*ddict).cMem;
        zstd_custom_free((*ddict).dictBuffer, cMem);
        zstd_custom_free(ddict as *mut c_void, cMem);
        0
    }
}

/*  ZSTD_estimateDDictSize() :
 *  Estimate amount of memory that will be needed to create a dictionary for decompression.
 *  Note : dictionary created by reference using ZSTD_dlm_byRef are smaller */
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
        return 0; /* support sizeof on NULL */
    }
    core::mem::size_of::<ZSTD_DDict>()
        + (if !(*ddict).dictBuffer.is_null() {
            (*ddict).dictSize
        } else {
            0
        })
}

/*  ZSTD_getDictID_fromDDict() :
 *  Provides the dictID of the dictionary loaded into `ddict`.
 *  If @return == 0, the dictionary is not conformant to Zstandard specification, or empty.
 *  Non-conformant dictionaries can still be loaded, but as content-only dictionaries. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> core::ffi::c_uint {
    if ddict.is_null() {
        return 0;
    }
    (*ddict).dictID
}
