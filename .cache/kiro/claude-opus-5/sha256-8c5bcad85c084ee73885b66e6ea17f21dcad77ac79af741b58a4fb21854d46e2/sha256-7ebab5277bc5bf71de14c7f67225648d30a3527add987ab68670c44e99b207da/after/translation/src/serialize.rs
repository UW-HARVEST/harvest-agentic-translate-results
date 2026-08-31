//! Translation of `c_src/src/pcre2_serialize.c`.
//!
//! Functions for serializing and deserializing a sequence of compiled codes.

#![allow(non_snake_case, non_upper_case_globals)]

use core::ffi::c_void;
use core::mem::{offset_of, size_of};

use crate::context::_pcre2_default_compile_context_8;
use crate::internal::*;

/* Magic number to provide a small check against being handed junk. */
const SERIALIZED_DATA_MAGIC: u32 = 0x5052_3253;

/* Deserialization is limited to the current PCRE version and character width. */
const SERIALIZED_DATA_VERSION: u32 = PCRE2_MAJOR | (PCRE2_MINOR << 16);

/* sizeof(PCRE2_UCHAR) == 1, sizeof(void*) == 8, sizeof(PCRE2_SIZE) == 8. */
const SERIALIZED_DATA_CONFIG: u32 = (size_of::<PCRE2_UCHAR>() as u32)
    | ((size_of::<*mut c_void>() as u32) << 8)
    | ((size_of::<PCRE2_SIZE>() as u32) << 16);

/* From config.h */
const MAX_NAME_SIZE: u16 = 128;
const MAX_NAME_COUNT: u16 = 10000;

/// `pcre2_serialize_encode`
///
/// Serialize compiled patterns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_encode_8(
    codes: *const *const pcre2_real_code,
    number_of_codes: i32,
    serialized_bytes: *mut *mut u8,
    serialized_size: *mut PCRE2_SIZE,
    gcontext: *mut pcre2_real_general_context,
) -> i32 {
    unsafe {
        let mut bytes: *mut u8;
        let mut dst_bytes: *mut u8;
        let mut i: i32;
        let mut total_size: PCRE2_SIZE;
        let mut re: *const pcre2_real_code;
        let mut tables: *const u8;
        let data: *mut pcre2_serialized_data;

        let memctl: *const pcre2_memctl = if !gcontext.is_null() {
            &(*gcontext).memctl
        } else {
            &(*(&raw const _pcre2_default_compile_context_8)).memctl
        };

        if codes.is_null() || serialized_bytes.is_null() || serialized_size.is_null() {
            return PCRE2_ERROR_NULL;
        }

        if number_of_codes <= 0 {
            return PCRE2_ERROR_BADDATA;
        }

        /* Compute total size. */
        total_size = size_of::<pcre2_serialized_data>() + TABLES_LENGTH;
        tables = core::ptr::null();

        i = 0;
        while i < number_of_codes {
            if (*codes.offset(i as isize)).is_null() {
                return PCRE2_ERROR_NULL;
            }
            re = *codes.offset(i as isize);
            if (*re).magic_number != MAGIC_NUMBER {
                return PCRE2_ERROR_BADMAGIC;
            }
            if tables.is_null() {
                tables = (*re).tables;
            } else if tables != (*re).tables {
                return PCRE2_ERROR_MIXEDTABLES;
            }
            total_size += (*re).blocksize;
            i += 1;
        }

        /* Initialize the byte stream. */
        bytes = ((*memctl).malloc.unwrap())(
            total_size + size_of::<pcre2_memctl>(),
            (*memctl).memory_data,
        ) as *mut u8;
        if bytes.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }

        /* The controller is stored as a hidden parameter. */
        memcpy(bytes, memctl as *const u8, size_of::<pcre2_memctl>());
        bytes = bytes.add(size_of::<pcre2_memctl>());

        data = bytes as *mut pcre2_serialized_data;
        (*data).magic = SERIALIZED_DATA_MAGIC;
        (*data).version = SERIALIZED_DATA_VERSION;
        (*data).config = SERIALIZED_DATA_CONFIG;
        (*data).number_of_codes = number_of_codes;

        /* Copy all compiled code data. */
        dst_bytes = bytes.add(size_of::<pcre2_serialized_data>());
        memcpy(dst_bytes, tables, TABLES_LENGTH);
        dst_bytes = dst_bytes.add(TABLES_LENGTH);

        i = 0;
        while i < number_of_codes {
            re = *codes.offset(i as isize);
            memcpy(dst_bytes, re as *const u8, (*re).blocksize);

            /* Certain fields in the compiled code block are re-set during
            deserialization. In order to ensure that the serialized data stream is
            always the same for the same pattern, set them to zero here. */

            memset(
                dst_bytes.add(offset_of!(pcre2_real_code, memctl)),
                0,
                size_of::<pcre2_memctl>(),
            );
            memset(
                dst_bytes.add(offset_of!(pcre2_real_code, tables)),
                0,
                size_of::<*mut c_void>(),
            );
            memset(
                dst_bytes.add(offset_of!(pcre2_real_code, executable_jit)),
                0,
                size_of::<*mut c_void>(),
            );

            dst_bytes = dst_bytes.add((*re).blocksize);
            i += 1;
        }

        *serialized_bytes = bytes;
        *serialized_size = total_size;
        number_of_codes
    }
}

/// `pcre2_serialize_decode`
///
/// Deserialize compiled patterns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_decode_8(
    codes: *mut *mut pcre2_real_code,
    mut number_of_codes: i32,
    bytes: *const u8,
    gcontext: *mut pcre2_real_general_context,
) -> i32 {
    unsafe {
        let data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;
        let memctl: *const pcre2_memctl = if !gcontext.is_null() {
            &(*gcontext).memctl
        } else {
            &(*(&raw const _pcre2_default_compile_context_8)).memctl
        };

        let mut src_bytes: *const u8;
        let mut dst_re: *mut pcre2_real_code = core::ptr::null_mut();
        let tables: *mut u8;
        let mut i: i32;
        let mut j: i32;
        let error: i32;

        /* Sanity checks. */

        if data.is_null() || codes.is_null() {
            return PCRE2_ERROR_NULL;
        }
        if number_of_codes <= 0 {
            return PCRE2_ERROR_BADDATA;
        }
        if (*data).number_of_codes <= 0 {
            return PCRE2_ERROR_BADSERIALIZEDDATA;
        }
        if (*data).magic != SERIALIZED_DATA_MAGIC {
            return PCRE2_ERROR_BADMAGIC;
        }
        if (*data).version != SERIALIZED_DATA_VERSION {
            return PCRE2_ERROR_BADMODE;
        }
        if (*data).config != SERIALIZED_DATA_CONFIG {
            return PCRE2_ERROR_BADMODE;
        }

        if number_of_codes > (*data).number_of_codes {
            number_of_codes = (*data).number_of_codes;
        }

        src_bytes = bytes.add(size_of::<pcre2_serialized_data>());

        /* Decode tables. The reference count for the tables is stored immediately
        following them. */

        tables = ((*memctl).malloc.unwrap())(
            TABLES_LENGTH + size_of::<PCRE2_SIZE>(),
            (*memctl).memory_data,
        ) as *mut u8;
        if tables.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }

        memcpy(tables, src_bytes, TABLES_LENGTH);
        *(tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE) = number_of_codes as PCRE2_SIZE;
        src_bytes = src_bytes.add(TABLES_LENGTH);

        /* Decode the byte stream. We must not try to read the size from the compiled
        code block in the stream, because it might be unaligned. */

        i = 0;
        'decode: loop {
            if i >= number_of_codes {
                return number_of_codes;
            }

            let mut blocksize: CODE_BLOCKSIZE_TYPE = 0;
            memcpy(
                &mut blocksize as *mut CODE_BLOCKSIZE_TYPE as *mut u8,
                src_bytes.add(offset_of!(pcre2_real_code, blocksize)),
                size_of::<CODE_BLOCKSIZE_TYPE>(),
            );
            if blocksize <= size_of::<pcre2_real_code>() {
                error = PCRE2_ERROR_BADSERIALIZEDDATA;
                break 'decode;
            }

            /* The allocator provided by gcontext replaces the original one. */

            dst_re = memctl_malloc(blocksize, gcontext as *mut pcre2_memctl)
                as *mut pcre2_real_code;
            if dst_re.is_null() {
                error = PCRE2_ERROR_NOMEMORY;
                break 'decode;
            }

            /* The new allocator must be preserved. */

            memcpy(
                (dst_re as *mut u8).add(size_of::<pcre2_memctl>()),
                src_bytes.add(size_of::<pcre2_memctl>()),
                blocksize - size_of::<pcre2_memctl>(),
            );
            if (*dst_re).magic_number != MAGIC_NUMBER
                || (*dst_re).name_entry_size > MAX_NAME_SIZE + IMM2_SIZE as u16 + 1
                || (*dst_re).name_count > MAX_NAME_COUNT
            {
                error = PCRE2_ERROR_BADSERIALIZEDDATA;
                break 'decode;
            }

            /* At the moment only one table is supported. */

            (*dst_re).tables = tables;
            (*dst_re).executable_jit = core::ptr::null_mut();
            (*dst_re).flags |= PCRE2_DEREF_TABLES;

            *codes.offset(i as isize) = dst_re;
            dst_re = core::ptr::null_mut();
            src_bytes = src_bytes.add(blocksize);
            i += 1;
        }

        /* cleanup */
        if !dst_re.is_null() {
            ((*memctl).free.unwrap())(dst_re as *mut c_void, (*memctl).memory_data);
        }
        ((*memctl).free.unwrap())(tables as *mut c_void, (*memctl).memory_data);
        j = 0;
        while j < i {
            ((*memctl).free.unwrap())(
                *codes.offset(j as isize) as *mut c_void,
                (*memctl).memory_data,
            );
            *codes.offset(j as isize) = core::ptr::null_mut();
            j += 1;
        }
        error
    }
}

/// `pcre2_serialize_get_number_of_codes`
///
/// Get the number of serialized patterns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_get_number_of_codes_8(bytes: *const u8) -> i32 {
    unsafe {
        let data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;

        if data.is_null() {
            return PCRE2_ERROR_NULL;
        }
        if (*data).magic != SERIALIZED_DATA_MAGIC {
            return PCRE2_ERROR_BADMAGIC;
        }
        if (*data).version != SERIALIZED_DATA_VERSION {
            return PCRE2_ERROR_BADMODE;
        }
        if (*data).config != SERIALIZED_DATA_CONFIG {
            return PCRE2_ERROR_BADMODE;
        }

        (*data).number_of_codes
    }
}

/// `pcre2_serialize_free`
///
/// Free the allocated stream.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_free_8(bytes: *mut u8) {
    unsafe {
        if !bytes.is_null() {
            let memctl = bytes.sub(size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

/// `CODE_BLOCKSIZE_TYPE` is `PCRE2_SIZE` (see `pcre2_intmodedep.h`).
type CODE_BLOCKSIZE_TYPE = PCRE2_SIZE;
