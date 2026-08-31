//! Translation of `common/zstd_trace.h`.
//!
//! On this platform (GCC/ELF/x86-64) `ZSTD_TRACE == 1`, so the tracing hooks are
//! declared as *weak* symbols. Since nothing in the library defines them, they
//! resolve to NULL at runtime and all tracing is disabled. We reproduce this by
//! declaring weak externs and NULL-checking them exactly like the C code does.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint};

pub const ZSTD_TRACE: u32 = 1;

pub type ZSTD_TraceCtx = u64;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_Trace {
    pub version: c_uint,
    pub streaming: c_int,
    pub dictionaryID: c_uint,
    pub dictionaryIsCold: c_int,
    pub dictionarySize: usize,
    pub uncompressedSize: usize,
    pub compressedSize: usize,
    pub params: *const crate::compress::zstd_compress_internal::ZSTD_CCtx_params,
    pub cctx: *const crate::compress::zstd_compress_internal::ZSTD_CCtx,
    pub dctx: *const crate::decompress::zstd_decompress_internal::ZSTD_DCtx,
}

impl Default for ZSTD_Trace {
    fn default() -> Self {
        ZSTD_Trace {
            version: 0,
            streaming: 0,
            dictionaryID: 0,
            dictionaryIsCold: 0,
            dictionarySize: 0,
            uncompressedSize: 0,
            compressedSize: 0,
            params: core::ptr::null(),
            cctx: core::ptr::null(),
            dctx: core::ptr::null(),
        }
    }
}

/* The C library declares these as weak and never defines them, so they are
 * always NULL in the produced shared object. Model them as permanently-NULL
 * function pointers, which makes every `if (fn != NULL)` guard false, exactly
 * as in the C build. */
pub type ZSTD_traceCompressBegin_f = Option<
    unsafe extern "C" fn(
        *const crate::compress::zstd_compress_internal::ZSTD_CCtx,
    ) -> ZSTD_TraceCtx,
>;
pub type ZSTD_traceCompressEnd_f =
    Option<unsafe extern "C" fn(ZSTD_TraceCtx, *const ZSTD_Trace)>;
pub type ZSTD_traceDecompressBegin_f = Option<
    unsafe extern "C" fn(
        *const crate::decompress::zstd_decompress_internal::ZSTD_DCtx,
    ) -> ZSTD_TraceCtx,
>;
pub type ZSTD_traceDecompressEnd_f =
    Option<unsafe extern "C" fn(ZSTD_TraceCtx, *const ZSTD_Trace)>;

pub static ZSTD_trace_compress_begin: ZSTD_traceCompressBegin_f = None;
pub static ZSTD_trace_compress_end: ZSTD_traceCompressEnd_f = None;
pub static ZSTD_trace_decompress_begin: ZSTD_traceDecompressBegin_f = None;
pub static ZSTD_trace_decompress_end: ZSTD_traceDecompressEnd_f = None;
