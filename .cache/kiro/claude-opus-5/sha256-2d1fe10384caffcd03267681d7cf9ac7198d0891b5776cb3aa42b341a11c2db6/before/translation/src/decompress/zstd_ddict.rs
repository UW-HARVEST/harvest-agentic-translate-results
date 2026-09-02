//! Rust transliteration of `c_src/src/decompress/zstd_ddict.c` and
//! `c_src/src/decompress/zstd_ddict.h`.
//!
//! concentrates all logic that needs to know the internals of ZSTD_DDict object.
//!
//! Build configuration: DEBUGLEVEL 0 (assert/DEBUGLOG dropped),
//! no FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use core::ffi::{c_uint, c_void};

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;

use super::zstd_decompress_internal::{
    ZSTD_entropyDTables_t, ZSTD_DCtx, ZSTD_HUFFDTABLE_CAPACITY_LOG,
};

/* `ZSTD_customMem` (from zstd.h) resolves to the zstd_internal definition,
 * which is the type accepted by ZSTD_customMalloc/ZSTD_customFree.
 * A module-local alias resolves the glob-import ambiguity. */
type ZSTD_customMem = crate::common::zstd_internal::ZSTD_customMem;

/* ZSTD_loadDEntropy() is defined in zstd_decompress.c, and is an exported
 * symbol of the same cdylib, so it links correctly through an extern block. */
unsafe extern "C" {
    fn ZSTD_loadDEntropy(
        entropy: *mut ZSTD_entropyDTables_t,
        dict: *const c_void,
        dictSize: size_t,
    ) -> size_t;
}

/*-*******************************************************
 *  Types
 *********************************************************/
#[repr(C)]
pub struct ZSTD_DDict_s {
    pub dictBuffer: *mut c_void,
    pub dictContent: *const c_void,
    pub dictSize: size_t,
    pub entropy: ZSTD_entropyDTables_t,
    pub dictID: U32,
    pub entropyPresent: U32,
    pub cMem: ZSTD_customMem,
}
/* typedef'd to ZSTD_DDict within "zstd.h" */
pub type ZSTD_DDict = ZSTD_DDict_s;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictContent(ddict: *const ZSTD_DDict) -> *const c_void {
    /* assert(ddict != NULL); dropped (DEBUGLEVEL 0) */
    (*ddict).dictContent
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictSize(ddict: *const ZSTD_DDict) -> size_t {
    /* assert(ddict != NULL); dropped */
    (*ddict).dictSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDDictParameters(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict) {
    /* DEBUGLOG / assert dropped */
    (*dctx).dictID = (*ddict).dictID;
    (*dctx).prefixStart = (*ddict).dictContent;
    (*dctx).virtualStart = (*ddict).dictContent;
    (*dctx).dictEnd =
        ((*ddict).dictContent as *const BYTE).add((*ddict).dictSize) as *const c_void;
    (*dctx).previousDstEnd = (*dctx).dictEnd;
    /* FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION not defined */
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

pub unsafe fn ZSTD_loadEntropy_intoDDict(
    ddict: *mut ZSTD_DDict,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    (*ddict).dictID = 0;
    (*ddict).entropyPresent = 0;
    if dictContentType == ZSTD_dct_rawContent {
        return 0;
    }

    if (*ddict).dictSize < 8 {
        if dictContentType == ZSTD_dct_fullDict {
            return ERROR(ZSTD_error_dictionary_corrupted); /* only accept specified dictionaries */
        }
        return 0; /* pure content mode */
    }
    {
        let magic: U32 = MEM_readLE32((*ddict).dictContent as *const u8);
        if magic != ZSTD_MAGIC_DICTIONARY {
            if dictContentType == ZSTD_dct_fullDict {
                return ERROR(ZSTD_error_dictionary_corrupted); /* only accept specified dictionaries */
            }
            return 0; /* pure content mode */
        }
    }
    (*ddict).dictID =
        MEM_readLE32(((*ddict).dictContent as *const core::ffi::c_char).add(ZSTD_FRAMEIDSIZE)
            as *const u8);

    /* load entropy tables */
    /* RETURN_ERROR_IF(ZSTD_isError(ZSTD_loadDEntropy(...)), dictionary_corrupted, "") */
    if ZSTD_isError(ZSTD_loadDEntropy(
        &mut (*ddict).entropy,
        (*ddict).dictContent,
        (*ddict).dictSize,
    )) != 0
    {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*ddict).entropyPresent = 1;
    0
}

pub unsafe fn ZSTD_initDDict_internal(
    ddict: *mut ZSTD_DDict,
    dict: *const c_void,
    mut dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    if (dictLoadMethod == ZSTD_dlm_byRef) || (dict.is_null()) || (dictSize == 0) {
        (*ddict).dictBuffer = core::ptr::null_mut();
        (*ddict).dictContent = dict;
        if dict.is_null() {
            dictSize = 0;
        }
    } else {
        let internalBuffer: *mut c_void = ZSTD_customMalloc(dictSize, (*ddict).cMem);
        (*ddict).dictBuffer = internalBuffer;
        (*ddict).dictContent = internalBuffer;
        if internalBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        ZSTD_memcpy(internalBuffer as *mut u8, dict as *const u8, dictSize);
    }
    (*ddict).dictSize = dictSize;
    /* cover both little and big endian */
    (*ddict).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x1000001)) as HUF_DTable_local;

    /* parse dictionary content */
    /* FORWARD_IF_ERROR( ZSTD_loadEntropy_intoDDict(ddict, dictContentType) , "") */
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
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_DDict {
    if ((customMem.customAlloc.is_none()) as core::ffi::c_int)
        ^ ((customMem.customFree.is_none()) as core::ffi::c_int)
        != 0
    {
        return core::ptr::null_mut();
    }

    {
        let ddict: *mut ZSTD_DDict =
            ZSTD_customMalloc(core::mem::size_of::<ZSTD_DDict>(), customMem) as *mut ZSTD_DDict;
        if ddict.is_null() {
            return core::ptr::null_mut();
        }
        (*ddict).cMem = customMem;
        {
            let initResult: size_t =
                ZSTD_initDDict_internal(ddict, dict, dictSize, dictLoadMethod, dictContentType);
            if ZSTD_isError(initResult) != 0 {
                ZSTD_freeDDict(ddict);
                return core::ptr::null_mut();
            }
        }
        ddict
    }
}

/* ZSTD_createDDict() :
*   Create a digested dictionary, to start decompression without startup delay.
*   `dict` content is copied inside DDict.
*   Consequently, `dict` can be released after `ZSTD_DDict` creation */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict(dict: *const c_void, dictSize: size_t) -> *mut ZSTD_DDict {
    let allocator: ZSTD_customMem = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto, allocator)
}

/* ZSTD_createDDict_byReference() :
 *  Create a digested dictionary, to start decompression without startup delay.
 *  Dictionary content is simply referenced, it will be accessed during decompression.
 *  Warning : dictBuffer must outlive DDict (DDict must be freed before dictBuffer) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict_byReference(
    dictBuffer: *const c_void,
    dictSize: size_t,
) -> *mut ZSTD_DDict {
    let allocator: ZSTD_customMem = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dictBuffer, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDDict(
    sBuffer: *mut c_void,
    sBufferSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    let neededSpace: size_t = core::mem::size_of::<ZSTD_DDict>()
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            dictSize
        });
    let ddict: *mut ZSTD_DDict = sBuffer as *mut ZSTD_DDict;
    /* assert(sBuffer != NULL); assert(dict != NULL); dropped */
    if (sBuffer as size_t) & 7 != 0 {
        return core::ptr::null(); /* 8-aligned */
    }
    if sBufferSize < neededSpace {
        return core::ptr::null();
    }
    let mut dict = dict;
    if dictLoadMethod == ZSTD_dlm_byCopy {
        ZSTD_memcpy(ddict.add(1) as *mut u8, dict as *const u8, dictSize); /* local copy */
        dict = ddict.add(1) as *const c_void;
    }
    if ZSTD_isError(ZSTD_initDDict_internal(
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
pub unsafe extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> size_t {
    if ddict.is_null() {
        return 0; /* support free on NULL */
    }
    {
        let cMem: ZSTD_customMem = (*ddict).cMem;
        ZSTD_customFree((*ddict).dictBuffer, cMem);
        ZSTD_customFree(ddict as *mut c_void, cMem);
        0
    }
}

/* ZSTD_estimateDDictSize() :
 *  Estimate amount of memory that will be needed to create a dictionary for decompression.
 *  Note : dictionary created by reference using ZSTD_dlm_byRef are smaller */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDDictSize(
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> size_t {
    core::mem::size_of::<ZSTD_DDict>()
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            dictSize
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> size_t {
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

/* ZSTD_getDictID_fromDDict() :
 *  Provides the dictID of the dictionary loaded into `ddict`.
 *  If @return == 0, the dictionary is not conformant to Zstandard specification, or empty.
 *  Non-conformant dictionaries can still be loaded, but as content-only dictionaries. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> c_uint {
    if ddict.is_null() {
        return 0;
    }
    (*ddict).dictID
}

/* local alias for HUF_DTable to keep the hufTable[0] assignment readable */
type HUF_DTable_local = crate::common::huf::HUF_DTable;
