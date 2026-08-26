//! Translation of decompress/zstd_ddict.c (+ zstd_ddict.h)
//!
//! zstd_ddict.c :
//! concentrates all logic that needs to know the internals of ZSTD_DDict object
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

/*-*******************************************************
*  Dependencies
*********************************************************/
use crate::error_private::*;
use crate::huf::*;
use crate::mem::*;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_decompress_internal::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/*-*******************************************************
*  Types
*********************************************************/
/* `struct ZSTD_DDict_s` is declared in `crate::zstd_decompress_internal`
 * (verified field-for-field identical to the C definition):
 *     void* dictBuffer;
 *     const void* dictContent;
 *     size_t dictSize;
 *     ZSTD_entropyDTables_t entropy;
 *     U32 dictID;
 *     U32 entropyPresent;
 *     ZSTD_customMem cMem;
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DDict_dictContent(
    ddict: *const ZSTD_DDict,
) -> *const core::ffi::c_void {
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
    (*dctx).dictEnd = ((*ddict).dictContent as *const BYTE).wrapping_add((*ddict).dictSize)
        as *const core::ffi::c_void;
    (*dctx).previousDstEnd = (*dctx).dictEnd;
    if (*ddict).entropyPresent != 0 {
        (*dctx).litEntropy = 1;
        (*dctx).fseEntropy = 1;
        (*dctx).LLTptr = core::ptr::addr_of!((*ddict).entropy.LLTable) as *const ZSTD_seqSymbol;
        (*dctx).MLTptr = core::ptr::addr_of!((*ddict).entropy.MLTable) as *const ZSTD_seqSymbol;
        (*dctx).OFTptr = core::ptr::addr_of!((*ddict).entropy.OFTable) as *const ZSTD_seqSymbol;
        (*dctx).HUFptr = core::ptr::addr_of!((*ddict).entropy.hufTable) as *const HUF_DTable;
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
) -> usize {
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
        let magic: U32 = MEM_readLE32((*ddict).dictContent as *const BYTE);
        if magic != ZSTD_MAGIC_DICTIONARY {
            if dictContentType == ZSTD_dct_fullDict {
                return ERROR(ZSTD_error_dictionary_corrupted); /* only accept specified dictionaries */
            }
            return 0; /* pure content mode */
        }
    }
    (*ddict).dictID = MEM_readLE32(
        ((*ddict).dictContent as *const core::ffi::c_char).wrapping_add(ZSTD_FRAMEIDSIZE)
            as *const BYTE,
    );

    /* load entropy tables */
    if ZSTD_isError(crate::zstd_decompress::ZSTD_loadDEntropy(
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

pub unsafe fn ZSTD_initDDict_internal(
    ddict: *mut ZSTD_DDict,
    dict: *const core::ffi::c_void,
    mut dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    if (dictLoadMethod == ZSTD_dlm_byRef) || (dict.is_null()) || (dictSize == 0) {
        (*ddict).dictBuffer = core::ptr::null_mut();
        (*ddict).dictContent = dict;
        if dict.is_null() {
            dictSize = 0;
        }
    } else {
        let internalBuffer: *mut core::ffi::c_void =
            ZSTD_customMalloc(dictSize, (*ddict).cMem) as *mut core::ffi::c_void;
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
        (ZSTD_HUFFDTABLE_CAPACITY_LOG).wrapping_mul(0x1000001) as HUF_DTable;

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
    dict: *const core::ffi::c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_DDict {
    if ((customMem.customAlloc.is_none() as core::ffi::c_int)
        ^ (customMem.customFree.is_none() as core::ffi::c_int))
        != 0
    {
        return core::ptr::null_mut();
    }

    {
        let ddict: *mut ZSTD_DDict = ZSTD_customMalloc(
            core::mem::size_of::<ZSTD_DDict>(),
            customMem,
        ) as *mut ZSTD_DDict;
        if ddict.is_null() {
            return core::ptr::null_mut();
        }
        (*ddict).cMem = customMem;
        {
            let initResult: usize =
                ZSTD_initDDict_internal(ddict, dict, dictSize, dictLoadMethod, dictContentType);
            if ZSTD_isError(initResult) != 0 {
                ZSTD_freeDDict(ddict);
                return core::ptr::null_mut();
            }
        }
        return ddict;
    }
}

/* ZSTD_createDDict() :
*   Create a digested dictionary, to start decompression without startup delay.
*   `dict` content is copied inside DDict.
*   Consequently, `dict` can be released after `ZSTD_DDict` creation */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict(
    dict: *const core::ffi::c_void,
    dictSize: usize,
) -> *mut ZSTD_DDict {
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
    dictBuffer: *const core::ffi::c_void,
    dictSize: usize,
) -> *mut ZSTD_DDict {
    let allocator: ZSTD_customMem = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(
        dictBuffer,
        dictSize,
        ZSTD_dlm_byRef,
        ZSTD_dct_auto,
        allocator,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDDict(
    sBuffer: *mut core::ffi::c_void,
    sBufferSize: usize,
    mut dict: *const core::ffi::c_void,
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
    if ((sBuffer as usize) & 7) != 0 {
        return core::ptr::null(); /* 8-aligned */
    }
    if sBufferSize < neededSpace {
        return core::ptr::null();
    }
    if dictLoadMethod == ZSTD_dlm_byCopy {
        ZSTD_memcpy(ddict.add(1) as *mut u8, dict as *const u8, dictSize); /* local copy */
        dict = ddict.add(1) as *const core::ffi::c_void;
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
    ddict as *const ZSTD_DDict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> usize {
    if ddict.is_null() {
        return 0; /* support free on NULL */
    }
    {
        let cMem: ZSTD_customMem = (*ddict).cMem;
        ZSTD_customFree((*ddict).dictBuffer as *mut u8, cMem);
        ZSTD_customFree(ddict as *mut u8, cMem);
        return 0;
    }
}

/* ZSTD_estimateDDictSize() :
 *  Estimate amount of memory that will be needed to create a dictionary for decompression.
 *  Note : dictionary created by reference using ZSTD_dlm_byRef are smaller */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateDDictSize(
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

/* ZSTD_getDictID_fromDDict() :
 *  Provides the dictID of the dictionary loaded into `ddict`.
 *  If @return == 0, the dictionary is not conformant to Zstandard specification, or empty.
 *  Non-conformant dictionaries can still be loaded, but as content-only dictionaries. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDDict(
    ddict: *const ZSTD_DDict,
) -> core::ffi::c_uint {
    if ddict.is_null() {
        return 0;
    }
    (*ddict).dictID
}
