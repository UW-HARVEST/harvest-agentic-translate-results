//! Translation of `pcre2_serialize.c`.
//!
//! Functions for serializing and deserializing a sequence of compiled codes.

use crate::internal::*;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};

// ---------------------------------------------------------------------------
// Local constants (from config.h; not present in consts.rs)
// ---------------------------------------------------------------------------

/// `TABLES_LENGTH` = `ctypes_offset + 256` = `cbits_offset(512) + cbit_length(320) + 256`.
/// This equals the length of the default tables blob (1088 bytes).
const TABLES_LENGTH: usize = 1088;

/// `MAX_NAME_SIZE` from config.h.
const MAX_NAME_SIZE: u16 = 128;
/// `MAX_NAME_COUNT` from config.h.
const MAX_NAME_COUNT: u16 = 10000;
/// `IMM2_SIZE` in 8-bit mode.
const IMM2_SIZE: u16 = IMM2_SIZE_U as u16;

/// `CODE_BLOCKSIZE_TYPE` is `PCRE2_SIZE` in this configuration.
type CODE_BLOCKSIZE_TYPE = PCRE2_SIZE;

// ---------------------------------------------------------------------------
// Magic / version / config words written into the serialized header.
// ---------------------------------------------------------------------------

/// Magic number to provide a small check against being handed junk.
const SERIALIZED_DATA_MAGIC: u32 = 0x5052_3253;

/// Deserialization is limited to the current PCRE version.
/// `(PCRE2_MAJOR) | ((PCRE2_MINOR) << 16)`.
const SERIALIZED_DATA_VERSION: u32 = (PCRE2_MAJOR as u32) | ((PCRE2_MINOR as u32) << 16);

/// `(sizeof(PCRE2_UCHAR) | (sizeof(void*) << 8) | (sizeof(PCRE2_SIZE) << 16))`.
const SERIALIZED_DATA_CONFIG: u32 = (size_of::<PCRE2_UCHAR>() as u32)
    | ((size_of::<*mut c_void>() as u32) << 8)
    | ((size_of::<PCRE2_SIZE>() as u32) << 16);

// ---------------------------------------------------------------------------
// On-disk / wire header. Byte-for-byte layout matters.
// ---------------------------------------------------------------------------

/// `pcre2_serialized_data` — the header written to the byte stream.
#[repr(C)]
struct pcre2_serialized_data {
    magic: u32,
    version: u32,
    config: u32,
    number_of_codes: i32,
}

/// Convenience: the default compile context's memctl, used when no gcontext is
/// supplied.
#[inline]
unsafe fn default_memctl() -> *const pcre2_memctl {
    unsafe { &raw const crate::context::_pcre2_default_compile_context_8.memctl }
}

// ---------------------------------------------------------------------------
// Serialize compiled patterns
// ---------------------------------------------------------------------------

/// `pcre2_serialize_encode()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_encode_8(
    codes: *const *const pcre2_code,
    number_of_codes: i32,
    serialized_bytes: *mut *mut u8,
    serialized_size: *mut PCRE2_SIZE,
    gcontext: *mut pcre2_general_context,
) -> i32 {
    unsafe {
        let memctl: *const pcre2_memctl = if !gcontext.is_null() {
            &raw const (*gcontext).memctl
        } else {
            default_memctl()
        };

        if codes.is_null() || serialized_bytes.is_null() || serialized_size.is_null() {
            return PCRE2_ERROR_NULL as i32;
        }

        if number_of_codes <= 0 {
            return PCRE2_ERROR_BADDATA as i32;
        }

        // Compute total size.
        let mut total_size: PCRE2_SIZE = size_of::<pcre2_serialized_data>() + TABLES_LENGTH;
        let mut tables: *const u8 = core::ptr::null();

        let mut i: i32 = 0;
        while i < number_of_codes {
            let code_i = *codes.offset(i as isize);
            if code_i.is_null() {
                return PCRE2_ERROR_NULL as i32;
            }
            let re = code_i as *const pcre2_real_code;
            if (*re).magic_number != MAGIC_NUMBER as u32 {
                return PCRE2_ERROR_BADMAGIC as i32;
            }
            if tables.is_null() {
                tables = (*re).tables;
            } else if tables != (*re).tables {
                return PCRE2_ERROR_MIXEDTABLES as i32;
            }
            total_size += (*re).blocksize;
            i += 1;
        }

        // Initialize the byte stream.
        let mut bytes = ((*memctl).malloc.unwrap())(
            total_size + size_of::<pcre2_memctl>(),
            (*memctl).memory_data,
        ) as *mut u8;
        if bytes.is_null() {
            return PCRE2_ERROR_NOMEMORY as i32;
        }

        // The controller is stored as a hidden parameter.
        c_memcpy(
            bytes as *mut c_void,
            memctl as *const c_void,
            size_of::<pcre2_memctl>(),
        );
        bytes = bytes.add(size_of::<pcre2_memctl>());

        let data = bytes as *mut pcre2_serialized_data;
        (*data).magic = SERIALIZED_DATA_MAGIC;
        (*data).version = SERIALIZED_DATA_VERSION;
        (*data).config = SERIALIZED_DATA_CONFIG;
        (*data).number_of_codes = number_of_codes;

        // Copy all compiled code data.
        let mut dst_bytes = bytes.add(size_of::<pcre2_serialized_data>());
        c_memcpy(
            dst_bytes as *mut c_void,
            tables as *const c_void,
            TABLES_LENGTH,
        );
        dst_bytes = dst_bytes.add(TABLES_LENGTH);

        let mut i: i32 = 0;
        while i < number_of_codes {
            let re = *codes.offset(i as isize) as *const pcre2_real_code;
            c_memcpy(
                dst_bytes as *mut c_void,
                re as *const c_void,
                (*re).blocksize,
            );

            // Certain fields are re-set during deserialization; zero them here
            // so the serialized stream is stable for the same pattern.
            c_memset(
                dst_bytes.add(offset_of!(pcre2_real_code, memctl)) as *mut c_void,
                0,
                size_of::<pcre2_memctl>(),
            );
            c_memset(
                dst_bytes.add(offset_of!(pcre2_real_code, tables)) as *mut c_void,
                0,
                size_of::<*mut c_void>(),
            );
            c_memset(
                dst_bytes.add(offset_of!(pcre2_real_code, executable_jit)) as *mut c_void,
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

// ---------------------------------------------------------------------------
// Deserialize compiled patterns
// ---------------------------------------------------------------------------

/// `pcre2_serialize_decode()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_decode_8(
    codes: *mut *mut pcre2_code,
    mut number_of_codes: i32,
    bytes: *const u8,
    gcontext: *mut pcre2_general_context,
) -> i32 {
    unsafe {
        let data = bytes as *const pcre2_serialized_data;
        let memctl: *const pcre2_memctl = if !gcontext.is_null() {
            &raw const (*gcontext).memctl
        } else {
            default_memctl()
        };

        let mut dst_re: *mut pcre2_real_code = core::ptr::null_mut();
        let mut i: i32 = 0;
        let mut error: i32 = 0;

        // Sanity checks.
        if data.is_null() || codes.is_null() {
            return PCRE2_ERROR_NULL as i32;
        }
        if number_of_codes <= 0 {
            return PCRE2_ERROR_BADDATA as i32;
        }
        if (*data).number_of_codes <= 0 {
            return PCRE2_ERROR_BADSERIALIZEDDATA as i32;
        }
        if (*data).magic != SERIALIZED_DATA_MAGIC {
            return PCRE2_ERROR_BADMAGIC as i32;
        }
        if (*data).version != SERIALIZED_DATA_VERSION {
            return PCRE2_ERROR_BADMODE as i32;
        }
        if (*data).config != SERIALIZED_DATA_CONFIG {
            return PCRE2_ERROR_BADMODE as i32;
        }

        if number_of_codes > (*data).number_of_codes {
            number_of_codes = (*data).number_of_codes;
        }

        let mut src_bytes = bytes.add(size_of::<pcre2_serialized_data>());

        // Decode tables. The reference count for the tables is stored
        // immediately following them.
        let tables = ((*memctl).malloc.unwrap())(
            TABLES_LENGTH + size_of::<PCRE2_SIZE>(),
            (*memctl).memory_data,
        ) as *mut u8;
        if tables.is_null() {
            return PCRE2_ERROR_NOMEMORY as i32;
        }

        c_memcpy(
            tables as *mut c_void,
            src_bytes as *const c_void,
            TABLES_LENGTH,
        );
        *(tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE) = number_of_codes as PCRE2_SIZE;
        src_bytes = src_bytes.add(TABLES_LENGTH);

        // Decode the byte stream. We must not read the size directly from the
        // (possibly unaligned) compiled code block in the stream.
        'decode: while i < number_of_codes {
            let mut blocksize: CODE_BLOCKSIZE_TYPE = 0;
            c_memcpy(
                &mut blocksize as *mut CODE_BLOCKSIZE_TYPE as *mut c_void,
                src_bytes.add(offset_of!(pcre2_real_code, blocksize)) as *const c_void,
                size_of::<CODE_BLOCKSIZE_TYPE>(),
            );
            if blocksize <= size_of::<pcre2_real_code>() {
                error = PCRE2_ERROR_BADSERIALIZEDDATA as i32;
                break 'decode;
            }

            // The allocator provided by gcontext replaces the original one.
            dst_re = crate::context::_pcre2_memctl_malloc_8(blocksize, gcontext as *mut pcre2_memctl)
                as *mut pcre2_real_code;
            if dst_re.is_null() {
                error = PCRE2_ERROR_NOMEMORY as i32;
                break 'decode;
            }

            // The new allocator must be preserved.
            c_memcpy(
                (dst_re as *mut u8).add(size_of::<pcre2_memctl>()) as *mut c_void,
                src_bytes.add(size_of::<pcre2_memctl>()) as *const c_void,
                blocksize - size_of::<pcre2_memctl>(),
            );
            if (*dst_re).magic_number != MAGIC_NUMBER as u32
                || (*dst_re).name_entry_size > MAX_NAME_SIZE + IMM2_SIZE + 1
                || (*dst_re).name_count > MAX_NAME_COUNT
            {
                error = PCRE2_ERROR_BADSERIALIZEDDATA as i32;
                break 'decode;
            }

            // At the moment only one table is supported.
            (*dst_re).tables = tables;
            (*dst_re).executable_jit = core::ptr::null_mut();
            (*dst_re).flags |= PCRE2_DEREF_TABLES as u32;

            *codes.offset(i as isize) = dst_re;
            dst_re = core::ptr::null_mut();
            src_bytes = src_bytes.add(blocksize);

            i += 1;
            continue 'decode;
        }

        if i >= number_of_codes {
            return number_of_codes;
        }

        // cleanup:
        if !dst_re.is_null() {
            ((*memctl).free.unwrap())(dst_re as *mut c_void, (*memctl).memory_data);
        }
        ((*memctl).free.unwrap())(tables as *mut c_void, (*memctl).memory_data);
        let mut j: i32 = 0;
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

// ---------------------------------------------------------------------------
// Get the number of serialized patterns
// ---------------------------------------------------------------------------

/// `pcre2_serialize_get_number_of_codes()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_get_number_of_codes_8(bytes: *const u8) -> i32 {
    unsafe {
        let data = bytes as *const pcre2_serialized_data;

        if data.is_null() {
            return PCRE2_ERROR_NULL as i32;
        }
        if (*data).magic != SERIALIZED_DATA_MAGIC {
            return PCRE2_ERROR_BADMAGIC as i32;
        }
        if (*data).version != SERIALIZED_DATA_VERSION {
            return PCRE2_ERROR_BADMODE as i32;
        }
        if (*data).config != SERIALIZED_DATA_CONFIG {
            return PCRE2_ERROR_BADMODE as i32;
        }

        (*data).number_of_codes
    }
}

// ---------------------------------------------------------------------------
// Free the allocated stream
// ---------------------------------------------------------------------------

/// `pcre2_serialize_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_free_8(bytes: *mut u8) {
    unsafe {
        if !bytes.is_null() {
            let memctl = bytes.sub(size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

// End of pcre2_serialize.c
