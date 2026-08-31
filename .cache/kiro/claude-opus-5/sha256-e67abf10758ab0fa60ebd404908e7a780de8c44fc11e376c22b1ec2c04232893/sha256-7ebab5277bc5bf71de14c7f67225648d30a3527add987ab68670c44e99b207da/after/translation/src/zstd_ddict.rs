//! Translation of `decompress/zstd_ddict.c`.
#![allow(dead_code)]

use core::ffi::{c_uint, c_void};

use crate::allocations::{zstd_custom_free, zstd_custom_malloc, ZSTD_customMem};
use crate::error::*;
use crate::huf::HUF_DTable;
use crate::mem::*;
use crate::zstd_decompress_internal::{ZSTD_DCtx, ZSTD_DDict, ZSTD_HUFFDTABLE_CAPACITY_LOG};
use crate::zstd_internal::ZSTD_FRAMEIDSIZE;
use crate::zstd_public::{
    ZSTD_dct_auto, ZSTD_dct_fullDict, ZSTD_dct_rawContent, ZSTD_dictContentType_e,
    ZSTD_dictLoadMethod_e, ZSTD_dlm_byCopy, ZSTD_dlm_byRef, ZSTD_MAGIC_DICTIONARY,
};

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
    /* DEBUGLOG(4, "ZSTD_copyDDictParameters"); */
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

/// `ZSTD_loadEntropy_intoDDict()`
unsafe fn zstd_load_entropy_into_ddict(
    ddict: *mut ZSTD_DDict,
    dict_content_type: ZSTD_dictContentType_e,
) -> usize {
    (*ddict).dictID = 0;
    (*ddict).entropyPresent = 0;
    if dict_content_type == ZSTD_dct_rawContent {
        return 0;
    }

    if (*ddict).dictSize < 8 {
        if dict_content_type == ZSTD_dct_fullDict {
            return err_code(ZSTD_error_dictionary_corrupted); /* only accept specified dictionaries */
        }
        return 0; /* pure content mode */
    }
    {
        let magic: U32 = mem_read_le32((*ddict).dictContent as *const u8);
        if magic != ZSTD_MAGIC_DICTIONARY {
            if dict_content_type == ZSTD_dct_fullDict {
                return err_code(ZSTD_error_dictionary_corrupted); /* only accept specified dictionaries */
            }
            return 0; /* pure content mode */
        }
    }
    (*ddict).dictID =
        mem_read_le32(((*ddict).dictContent as *const u8).add(ZSTD_FRAMEIDSIZE));

    /* load entropy tables */
    if err_is_error(crate::zstd_decompress::ZSTD_loadDEntropy(
        &mut (*ddict).entropy,
        (*ddict).dictContent,
        (*ddict).dictSize,
    )) {
        return err_code(ZSTD_error_dictionary_corrupted);
    }
    (*ddict).entropyPresent = 1;
    0
}

/// `ZSTD_initDDict_internal()`
unsafe fn zstd_init_ddict_internal(
    ddict: *mut ZSTD_DDict,
    dict: *const c_void,
    mut dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> usize {
    if (dict_load_method == ZSTD_dlm_byRef) || dict.is_null() || (dict_size == 0) {
        (*ddict).dictBuffer = core::ptr::null_mut();
        (*ddict).dictContent = dict;
        if dict.is_null() {
            dict_size = 0;
        }
    } else {
        let internal_buffer: *mut c_void = zstd_custom_malloc(dict_size, (*ddict).cMem);
        (*ddict).dictBuffer = internal_buffer;
        (*ddict).dictContent = internal_buffer;
        if internal_buffer.is_null() {
            return err_code(ZSTD_error_memory_allocation);
        }
        core::ptr::copy_nonoverlapping(dict as *const u8, internal_buffer as *mut u8, dict_size);
    }
    (*ddict).dictSize = dict_size;
    (*ddict).entropy.hufTable[0] =
        (ZSTD_HUFFDTABLE_CAPACITY_LOG.wrapping_mul(0x0100_0001)) as HUF_DTable; /* cover both little and big endian */

    /* parse dictionary content */
    {
        let err = zstd_load_entropy_into_ddict(ddict, dict_content_type);
        if err_is_error(err) {
            return err;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict_advanced(
    dict: *const c_void,
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
    custom_mem: ZSTD_customMem,
) -> *mut ZSTD_DDict {
    if (custom_mem.customAlloc.is_none()) ^ (custom_mem.customFree.is_none()) {
        return core::ptr::null_mut();
    }

    {
        let ddict: *mut ZSTD_DDict =
            zstd_custom_malloc(core::mem::size_of::<ZSTD_DDict>(), custom_mem) as *mut ZSTD_DDict;
        if ddict.is_null() {
            return core::ptr::null_mut();
        }
        (*ddict).cMem = custom_mem;
        {
            let init_result: usize = zstd_init_ddict_internal(
                ddict,
                dict,
                dict_size,
                dict_load_method,
                dict_content_type,
            );
            if err_is_error(init_result) {
                ZSTD_freeDDict(ddict);
                return core::ptr::null_mut();
            }
        }
        ddict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict(dict: *const c_void, dict_size: usize) -> *mut ZSTD_DDict {
    let allocator = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dict, dict_size, ZSTD_dlm_byCopy, ZSTD_dct_auto, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDDict_byReference(
    dict_buffer: *const c_void,
    dict_size: usize,
) -> *mut ZSTD_DDict {
    let allocator = ZSTD_customMem {
        customAlloc: None,
        customFree: None,
        opaque: core::ptr::null_mut(),
    };
    ZSTD_createDDict_advanced(dict_buffer, dict_size, ZSTD_dlm_byRef, ZSTD_dct_auto, allocator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDDict(
    s_buffer: *mut c_void,
    s_buffer_size: usize,
    mut dict: *const c_void,
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
    dict_content_type: ZSTD_dictContentType_e,
) -> *const ZSTD_DDict {
    let needed_space: usize = core::mem::size_of::<ZSTD_DDict>()
        + (if dict_load_method == ZSTD_dlm_byRef {
            0
        } else {
            dict_size
        });
    let ddict: *mut ZSTD_DDict = s_buffer as *mut ZSTD_DDict;
    debug_assert!(!s_buffer.is_null());
    debug_assert!(!dict.is_null());
    if (s_buffer as usize) & 7 != 0 {
        return core::ptr::null(); /* 8-aligned */
    }
    if s_buffer_size < needed_space {
        return core::ptr::null();
    }
    if dict_load_method == ZSTD_dlm_byCopy {
        core::ptr::copy_nonoverlapping(
            dict as *const u8,
            ddict.add(1) as *mut u8,
            dict_size,
        ); /* local copy */
        dict = ddict.add(1) as *const c_void;
    }
    if err_is_error(zstd_init_ddict_internal(
        ddict,
        dict,
        dict_size,
        ZSTD_dlm_byRef,
        dict_content_type,
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
        let c_mem: ZSTD_customMem = (*ddict).cMem;
        zstd_custom_free((*ddict).dictBuffer, c_mem);
        zstd_custom_free(ddict as *mut c_void, c_mem);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDDictSize(
    dict_size: usize,
    dict_load_method: ZSTD_dictLoadMethod_e,
) -> usize {
    core::mem::size_of::<ZSTD_DDict>()
        + (if dict_load_method == ZSTD_dlm_byRef {
            0
        } else {
            dict_size
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> c_uint {
    if ddict.is_null() {
        return 0;
    }
    (*ddict).dictID
}
