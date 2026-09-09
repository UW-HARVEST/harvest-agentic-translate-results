//! Translation of `compress/zstd_cwksp.h` — the single-buffer workspace allocator.
//!
//! Literal, semantics-preserving transliteration. Build configuration:
//! `DEBUGLEVEL 0` (asserts / DEBUGLOG dropped), sanitizers disabled
//! (`ZSTD_ADDRESS_SANITIZER` / `ZSTD_MEMORY_SANITIZER` are 0), so all
//! `__asan_*` / `__msan_*` code paths vanish.
//!
//! This header carries no exported symbols: everything is `pub` types,
//! `pub const`s and `pub unsafe fn` for the `MEM_STATIC` inline functions.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]

use core::ffi::{c_int, c_void};
use core::ptr::null_mut;

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_internal::*;

/*-*************************************
*  Constants
***************************************/

/* Since the workspace is effectively its own little malloc implementation /
 * arena, when we run under ASAN, we should similarly insert redzones between
 * each internal element of the workspace, so ASAN will catch overruns that
 * reach outside an object but that stay inside the workspace.
 *
 * This defines the size of that redzone.
 */
pub const ZSTD_CWKSP_ASAN_REDZONE_SIZE: size_t = 128;

/* Set our tables and aligneds to align by 64 bytes */
pub const ZSTD_CWKSP_ALIGNMENT_BYTES: size_t = 64;

/*-*************************************
*  Structures
***************************************/
pub type ZSTD_cwksp_alloc_phase_e = core::ffi::c_uint;
pub const ZSTD_cwksp_alloc_objects: ZSTD_cwksp_alloc_phase_e = 0;
pub const ZSTD_cwksp_alloc_aligned_init_once: ZSTD_cwksp_alloc_phase_e = 1;
pub const ZSTD_cwksp_alloc_aligned: ZSTD_cwksp_alloc_phase_e = 2;
pub const ZSTD_cwksp_alloc_buffers: ZSTD_cwksp_alloc_phase_e = 3;

/**
 * Used to describe whether the workspace is statically allocated (and will not
 * necessarily ever be freed), or if it's dynamically allocated and we can
 * expect a well-formed caller to free this.
 */
pub type ZSTD_cwksp_static_alloc_e = core::ffi::c_uint;
pub const ZSTD_cwksp_dynamic_alloc: ZSTD_cwksp_static_alloc_e = 0;
pub const ZSTD_cwksp_static_alloc: ZSTD_cwksp_static_alloc_e = 1;

/**
 * Zstd fits all its internal datastructures into a single continuous buffer,
 * so that it only needs to perform a single OS allocation (or so that a buffer
 * can be provided to it and it can perform no allocations at all). This buffer
 * is called the workspace.
 *
 * See the C source for the full description of the workspace layout.
 */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_cwksp {
    pub workspace: *mut c_void,
    pub workspaceEnd: *mut c_void,

    pub objectEnd: *mut c_void,
    pub tableEnd: *mut c_void,
    pub tableValidEnd: *mut c_void,
    pub allocStart: *mut c_void,
    pub initOnceStart: *mut c_void,

    pub allocFailed: BYTE,
    pub workspaceOversizedDuration: c_int,
    pub phase: ZSTD_cwksp_alloc_phase_e,
    pub isStatic: ZSTD_cwksp_static_alloc_e,
}

impl Default for ZSTD_cwksp {
    fn default() -> Self {
        ZSTD_cwksp {
            workspace: null_mut(),
            workspaceEnd: null_mut(),
            objectEnd: null_mut(),
            tableEnd: null_mut(),
            tableValidEnd: null_mut(),
            allocStart: null_mut(),
            initOnceStart: null_mut(),
            allocFailed: 0,
            workspaceOversizedDuration: 0,
            phase: ZSTD_cwksp_alloc_objects,
            isStatic: ZSTD_cwksp_dynamic_alloc,
        }
    }
}

/*-*************************************
*  Functions
***************************************/

pub unsafe fn ZSTD_cwksp_assert_internal_consistency(ws: *mut ZSTD_cwksp) {
    let _ = ws;
    /* asserts (DEBUGLEVEL 0) and MSAN checks are dropped */
}

/**
 * Align must be a power of 2.
 */
pub unsafe fn ZSTD_cwksp_align(size: size_t, align: size_t) -> size_t {
    let mask: size_t = align - 1;
    (size + mask) & !mask
}

/**
 * Use this to determine how much space in the workspace we will consume to
 * allocate this object.
 */
pub unsafe fn ZSTD_cwksp_alloc_size(size: size_t) -> size_t {
    if size == 0 {
        return 0;
    }
    /* non-ASAN path */
    size
}

pub unsafe fn ZSTD_cwksp_aligned_alloc_size(size: size_t, alignment: size_t) -> size_t {
    ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(size, alignment))
}

/**
 * Returns an adjusted alloc size that is the nearest larger multiple of 64 bytes.
 * Used to determine the number of bytes required for a given "aligned".
 */
pub unsafe fn ZSTD_cwksp_aligned64_alloc_size(size: size_t) -> size_t {
    ZSTD_cwksp_aligned_alloc_size(size, ZSTD_CWKSP_ALIGNMENT_BYTES)
}

/**
 * Returns the amount of additional space the cwksp must allocate
 * for internal purposes (currently only alignment).
 */
pub unsafe fn ZSTD_cwksp_slack_space_required() -> size_t {
    let slackSpace: size_t = ZSTD_CWKSP_ALIGNMENT_BYTES * 2;
    slackSpace
}

/**
 * Return the number of additional bytes required to align a pointer to the given number of bytes.
 * alignBytes must be a power of two.
 */
pub unsafe fn ZSTD_cwksp_bytes_to_align_ptr(ptr: *mut c_void, alignBytes: size_t) -> size_t {
    let alignBytesMask: size_t = alignBytes - 1;
    let bytes: size_t = (alignBytes - ((ptr as size_t) & alignBytesMask)) & alignBytesMask;
    bytes
}

/**
 * Returns the initial value for allocStart which is used to determine the position from
 * which we can allocate from the end of the workspace.
 */
pub unsafe fn ZSTD_cwksp_initialAllocStart(ws: *mut ZSTD_cwksp) -> *mut c_void {
    let mut endPtr: *mut u8 = (*ws).workspaceEnd as *mut u8;
    endPtr = endPtr.wrapping_sub((endPtr as size_t) % ZSTD_CWKSP_ALIGNMENT_BYTES);
    endPtr as *mut c_void
}

/**
 * Internal function. Do not use directly.
 * Reserves the given number of bytes within the aligned/buffer segment of the wksp,
 * which counts from the end of the wksp (as opposed to the object/table segment).
 *
 * Returns a pointer to the beginning of that space.
 */
pub unsafe fn ZSTD_cwksp_reserve_internal_buffer_space(
    ws: *mut ZSTD_cwksp,
    bytes: size_t,
) -> *mut c_void {
    let alloc: *mut c_void = ((*ws).allocStart as *mut u8).wrapping_sub(bytes) as *mut c_void;
    let bottom: *mut c_void = (*ws).tableEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
    if (alloc as *const u8) < (bottom as *const u8) {
        (*ws).allocFailed = 1;
        return null_mut();
    }
    /* the area is reserved from the end of wksp.
     * If it overlaps with tableValidEnd, it voids guarantees on values' range */
    if (alloc as *const u8) < ((*ws).tableValidEnd as *const u8) {
        (*ws).tableValidEnd = alloc;
    }
    (*ws).allocStart = alloc;
    alloc
}

/**
 * Moves the cwksp to the next phase, and does any necessary allocations.
 * cwksp initialization must necessarily go through each phase in order.
 * Returns a 0 on success, or zstd error
 */
pub unsafe fn ZSTD_cwksp_internal_advance_phase(
    ws: *mut ZSTD_cwksp,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> size_t {
    if phase > (*ws).phase {
        /* Going from allocating objects to allocating initOnce / tables */
        if (*ws).phase < ZSTD_cwksp_alloc_aligned_init_once
            && phase >= ZSTD_cwksp_alloc_aligned_init_once
        {
            (*ws).tableValidEnd = (*ws).objectEnd;
            (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);

            {
                /* Align the start of the tables to 64 bytes. Use [0, 63] bytes */
                let alloc: *mut c_void = (*ws).objectEnd;
                let bytesToAlign: size_t =
                    ZSTD_cwksp_bytes_to_align_ptr(alloc, ZSTD_CWKSP_ALIGNMENT_BYTES);
                let objectEnd: *mut c_void =
                    (alloc as *mut u8).wrapping_add(bytesToAlign) as *mut c_void;
                if (objectEnd as *const u8) > ((*ws).workspaceEnd as *const u8) {
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*ws).objectEnd = objectEnd;
                (*ws).tableEnd = objectEnd; /* table area starts being empty */
                if ((*ws).tableValidEnd as *const u8) < ((*ws).tableEnd as *const u8) {
                    (*ws).tableValidEnd = (*ws).tableEnd;
                }
            }
        }
        (*ws).phase = phase;
        ZSTD_cwksp_assert_internal_consistency(ws);
    }
    0
}

/**
 * Returns whether this object/buffer/etc was allocated in this workspace.
 */
pub unsafe fn ZSTD_cwksp_owns_buffer(ws: *const ZSTD_cwksp, ptr: *const c_void) -> c_int {
    ((ptr != null_mut())
        && ((*ws).workspace as *const c_void <= ptr)
        && (ptr < (*ws).workspaceEnd as *const c_void)) as c_int
}

/**
 * Internal function. Do not use directly.
 */
pub unsafe fn ZSTD_cwksp_reserve_internal(
    ws: *mut ZSTD_cwksp,
    bytes: size_t,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> *mut c_void {
    let alloc: *mut c_void;
    if ZSTD_isError(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 || bytes == 0 {
        return null_mut();
    }

    alloc = ZSTD_cwksp_reserve_internal_buffer_space(ws, bytes);

    alloc
}

/**
 * Reserves and returns unaligned memory.
 */
pub unsafe fn ZSTD_cwksp_reserve_buffer(ws: *mut ZSTD_cwksp, bytes: size_t) -> *mut BYTE {
    ZSTD_cwksp_reserve_internal(ws, bytes, ZSTD_cwksp_alloc_buffers) as *mut BYTE
}

/**
 * Reserves and returns memory sized on and aligned on ZSTD_CWKSP_ALIGNMENT_BYTES (64 bytes).
 * This memory has been initialized at least once in the past.
 */
pub unsafe fn ZSTD_cwksp_reserve_aligned_init_once(
    ws: *mut ZSTD_cwksp,
    bytes: size_t,
) -> *mut c_void {
    let alignedBytes: size_t = ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES);
    let ptr: *mut c_void =
        ZSTD_cwksp_reserve_internal(ws, alignedBytes, ZSTD_cwksp_alloc_aligned_init_once);
    if ptr != null_mut() && (ptr as *const u8) < ((*ws).initOnceStart as *const u8) {
        /* We assume the memory following the current allocation is either:
         * 1. Not usable as initOnce memory (end of workspace)
         * 2. Another initOnce buffer that has been allocated before (and so was previously memset)
         * 3. An ASAN redzone, in which case we don't want to write on it
         * For these reasons it should be fine to not explicitly zero every byte up to ws->initOnceStart.
         */
        ZSTD_memset(
            ptr as *mut u8,
            0,
            MIN(
                ((*ws).initOnceStart as *const u8).offset_from(ptr as *const u8) as size_t,
                alignedBytes,
            ),
        );
        (*ws).initOnceStart = ptr;
    }
    ptr
}

/**
 * Reserves and returns memory sized on and aligned on ZSTD_CWKSP_ALIGNMENT_BYTES (64 bytes).
 */
pub unsafe fn ZSTD_cwksp_reserve_aligned64(ws: *mut ZSTD_cwksp, bytes: size_t) -> *mut c_void {
    let ptr: *mut c_void = ZSTD_cwksp_reserve_internal(
        ws,
        ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES),
        ZSTD_cwksp_alloc_aligned,
    );
    ptr
}

/**
 * Aligned on 64 bytes. These buffers have the special property that
 * their values remain constrained, allowing us to reuse them without
 * memset()-ing them.
 */
pub unsafe fn ZSTD_cwksp_reserve_table(ws: *mut ZSTD_cwksp, bytes: size_t) -> *mut c_void {
    let phase: ZSTD_cwksp_alloc_phase_e = ZSTD_cwksp_alloc_aligned_init_once;
    let alloc: *mut c_void;
    let end: *mut c_void;
    let top: *mut c_void;

    /* We can only start allocating tables after we are done reserving space for objects at the
     * start of the workspace */
    if (*ws).phase < phase {
        if ZSTD_isError(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 {
            return null_mut();
        }
    }
    alloc = (*ws).tableEnd;
    end = (alloc as *mut u8).wrapping_add(bytes) as *mut c_void;
    top = (*ws).allocStart;

    ZSTD_cwksp_assert_internal_consistency(ws);
    if (end as *const u8) > (top as *const u8) {
        (*ws).allocFailed = 1;
        return null_mut();
    }
    (*ws).tableEnd = end;

    alloc
}

/**
 * Aligned on sizeof(void*).
 * Note : should happen only once, at workspace first initialization
 */
pub unsafe fn ZSTD_cwksp_reserve_object(ws: *mut ZSTD_cwksp, bytes: size_t) -> *mut c_void {
    let roundedBytes: size_t = ZSTD_cwksp_align(bytes, core::mem::size_of::<*mut c_void>() as size_t);
    let alloc: *mut c_void = (*ws).objectEnd;
    let end: *mut c_void = (alloc as *mut u8).wrapping_add(roundedBytes) as *mut c_void;

    ZSTD_cwksp_assert_internal_consistency(ws);
    /* we must be in the first phase, no advance is possible */
    if (*ws).phase != ZSTD_cwksp_alloc_objects || (end as *const u8) > ((*ws).workspaceEnd as *const u8)
    {
        (*ws).allocFailed = 1;
        return null_mut();
    }
    (*ws).objectEnd = end;
    (*ws).tableEnd = end;
    (*ws).tableValidEnd = end;

    alloc
}

/**
 * with alignment control
 * Note : should happen only once, at workspace first initialization
 */
pub unsafe fn ZSTD_cwksp_reserve_object_aligned(
    ws: *mut ZSTD_cwksp,
    byteSize: size_t,
    alignment: size_t,
) -> *mut c_void {
    let mask: size_t = alignment - 1;
    let surplus: size_t = if alignment > core::mem::size_of::<*mut c_void>() as size_t {
        alignment - core::mem::size_of::<*mut c_void>() as size_t
    } else {
        0
    };
    let start: *mut c_void = ZSTD_cwksp_reserve_object(ws, byteSize + surplus);
    if start == null_mut() {
        return null_mut();
    }
    if surplus == 0 {
        return start;
    }
    (((start as size_t) + surplus) & !mask) as *mut c_void
}

pub unsafe fn ZSTD_cwksp_mark_tables_dirty(ws: *mut ZSTD_cwksp) {
    (*ws).tableValidEnd = (*ws).objectEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

pub unsafe fn ZSTD_cwksp_mark_tables_clean(ws: *mut ZSTD_cwksp) {
    if ((*ws).tableValidEnd as *const u8) < ((*ws).tableEnd as *const u8) {
        (*ws).tableValidEnd = (*ws).tableEnd;
    }
    ZSTD_cwksp_assert_internal_consistency(ws);
}

/**
 * Zero the part of the allocated tables not already marked clean.
 */
pub unsafe fn ZSTD_cwksp_clean_tables(ws: *mut ZSTD_cwksp) {
    if ((*ws).tableValidEnd as *const u8) < ((*ws).tableEnd as *const u8) {
        ZSTD_memset(
            (*ws).tableValidEnd as *mut u8,
            0,
            ((*ws).tableEnd as *const u8).offset_from((*ws).tableValidEnd as *const u8) as size_t,
        );
    }
    ZSTD_cwksp_mark_tables_clean(ws);
}

/**
 * Invalidates table allocations.
 * All other allocations remain valid.
 */
pub unsafe fn ZSTD_cwksp_clear_tables(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

/**
 * Invalidates all buffer, aligned, and table allocations.
 * Object allocations remain valid.
 */
pub unsafe fn ZSTD_cwksp_clear(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
    (*ws).allocStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).allocFailed = 0;
    if (*ws).phase > ZSTD_cwksp_alloc_aligned_init_once {
        (*ws).phase = ZSTD_cwksp_alloc_aligned_init_once;
    }
    ZSTD_cwksp_assert_internal_consistency(ws);
}

pub unsafe fn ZSTD_cwksp_sizeof(ws: *const ZSTD_cwksp) -> size_t {
    ((*ws).workspaceEnd as *const u8).offset_from((*ws).workspace as *const u8) as size_t
}

pub unsafe fn ZSTD_cwksp_used(ws: *const ZSTD_cwksp) -> size_t {
    (((*ws).tableEnd as *const u8).offset_from((*ws).workspace as *const u8) as size_t)
        + (((*ws).workspaceEnd as *const u8).offset_from((*ws).allocStart as *const u8) as size_t)
}

/**
 * The provided workspace takes ownership of the buffer [start, start+size).
 */
pub unsafe fn ZSTD_cwksp_init(
    ws: *mut ZSTD_cwksp,
    start: *mut c_void,
    size: size_t,
    isStatic: ZSTD_cwksp_static_alloc_e,
) {
    (*ws).workspace = start;
    (*ws).workspaceEnd = (start as *mut u8).wrapping_add(size) as *mut c_void;
    (*ws).objectEnd = (*ws).workspace;
    (*ws).tableValidEnd = (*ws).objectEnd;
    (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).phase = ZSTD_cwksp_alloc_objects;
    (*ws).isStatic = isStatic;
    ZSTD_cwksp_clear(ws);
    (*ws).workspaceOversizedDuration = 0;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

pub unsafe fn ZSTD_cwksp_create(
    ws: *mut ZSTD_cwksp,
    size: size_t,
    customMem: ZSTD_customMem,
) -> size_t {
    let workspace: *mut c_void = ZSTD_customMalloc(size, customMem);
    if workspace == null_mut() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    ZSTD_cwksp_init(ws, workspace, size, ZSTD_cwksp_dynamic_alloc);
    0
}

pub unsafe fn ZSTD_cwksp_free(ws: *mut ZSTD_cwksp, customMem: ZSTD_customMem) {
    let ptr: *mut c_void = (*ws).workspace;
    ZSTD_memset(
        ws as *mut u8,
        0,
        core::mem::size_of::<ZSTD_cwksp>() as size_t,
    );
    ZSTD_customFree(ptr, customMem);
}

/**
 * Moves the management of a workspace from one cwksp to another. The src cwksp
 * is left in an invalid state (src must be re-init()'ed before it's used again).
 */
pub unsafe fn ZSTD_cwksp_move(dst: *mut ZSTD_cwksp, src: *mut ZSTD_cwksp) {
    *dst = *src;
    ZSTD_memset(
        src as *mut u8,
        0,
        core::mem::size_of::<ZSTD_cwksp>() as size_t,
    );
}

pub unsafe fn ZSTD_cwksp_reserve_failed(ws: *const ZSTD_cwksp) -> c_int {
    (*ws).allocFailed as c_int
}

/*-*************************************
*  Functions Checking Free Space
***************************************/

/* ZSTD_alignmentSpaceWithinBounds() :
 * Returns if the estimated space needed for a wksp is within an acceptable limit of the
 * actual amount of space used.
 */
pub unsafe fn ZSTD_cwksp_estimated_space_within_bounds(
    ws: *const ZSTD_cwksp,
    estimatedSpace: size_t,
) -> c_int {
    ((estimatedSpace - ZSTD_cwksp_slack_space_required() <= ZSTD_cwksp_used(ws))
        && (ZSTD_cwksp_used(ws) <= estimatedSpace)) as c_int
}

pub unsafe fn ZSTD_cwksp_available_space(ws: *mut ZSTD_cwksp) -> size_t {
    ((*ws).allocStart as *const u8).offset_from((*ws).tableEnd as *const u8) as size_t
}

pub unsafe fn ZSTD_cwksp_check_available(
    ws: *mut ZSTD_cwksp,
    additionalNeededSpace: size_t,
) -> c_int {
    (ZSTD_cwksp_available_space(ws) >= additionalNeededSpace) as c_int
}

pub unsafe fn ZSTD_cwksp_check_too_large(
    ws: *mut ZSTD_cwksp,
    additionalNeededSpace: size_t,
) -> c_int {
    ZSTD_cwksp_check_available(
        ws,
        additionalNeededSpace * (ZSTD_WORKSPACETOOLARGE_FACTOR as size_t),
    )
}

pub unsafe fn ZSTD_cwksp_check_wasteful(
    ws: *mut ZSTD_cwksp,
    additionalNeededSpace: size_t,
) -> c_int {
    ((ZSTD_cwksp_check_too_large(ws, additionalNeededSpace) != 0)
        && ((*ws).workspaceOversizedDuration > ZSTD_WORKSPACETOOLARGE_MAXDURATION as c_int))
        as c_int
}

pub unsafe fn ZSTD_cwksp_bump_oversized_duration(
    ws: *mut ZSTD_cwksp,
    additionalNeededSpace: size_t,
) {
    if ZSTD_cwksp_check_too_large(ws, additionalNeededSpace) != 0 {
        (*ws).workspaceOversizedDuration += 1;
    } else {
        (*ws).workspaceOversizedDuration = 0;
    }
}
