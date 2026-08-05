//! Faithful translation of compress/zstd_cwksp.h — the workspace arena allocator.
//!
//! Build config: ZSTD_ADDRESS_SANITIZER=0, ZSTD_MEMORY_SANITIZER=0, so all the
//! sanitizer redzone/poison blocks are compiled out and omitted here.
//! `assert()` -> `debug_assert!`. All functions are `pub` (crate-internal).
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use crate::common::allocations::{zstd_custom_free, zstd_custom_malloc, ZSTD_customMem};
use crate::common::error::{code, err_is_error, error};
use crate::common::zstd_internal::{
    ZSTD_WORKSPACETOOLARGE_FACTOR, ZSTD_WORKSPACETOOLARGE_MAXDURATION,
};

use core::ffi::c_void;

/*-*************************************
*  Constants
***************************************/

/* Redzone size used only under ASAN (compiled out here, kept for reference). */
pub const ZSTD_CWKSP_ASAN_REDZONE_SIZE: usize = 128;

/* Set our tables and aligneds to align by 64 bytes */
pub const ZSTD_CWKSP_ALIGNMENT_BYTES: usize = 64;

/*-*************************************
*  Structures
***************************************/

// ZSTD_cwksp_alloc_phase_e
pub type ZSTD_cwksp_alloc_phase_e = u32;
pub const ZSTD_cwksp_alloc_objects: ZSTD_cwksp_alloc_phase_e = 0;
pub const ZSTD_cwksp_alloc_aligned_init_once: ZSTD_cwksp_alloc_phase_e = 1;
pub const ZSTD_cwksp_alloc_aligned: ZSTD_cwksp_alloc_phase_e = 2;
pub const ZSTD_cwksp_alloc_buffers: ZSTD_cwksp_alloc_phase_e = 3;

// ZSTD_cwksp_static_alloc_e
pub type ZSTD_cwksp_static_alloc_e = u32;
pub const ZSTD_cwksp_dynamic_alloc: ZSTD_cwksp_static_alloc_e = 0;
pub const ZSTD_cwksp_static_alloc: ZSTD_cwksp_static_alloc_e = 1;

/// Zstd fits all its internal datastructures into a single continuous buffer.
/// See the C header for the extensive documentation of the layout.
#[repr(C)]
pub struct ZSTD_cwksp {
    pub workspace: *mut c_void,
    pub workspaceEnd: *mut c_void,

    pub objectEnd: *mut c_void,
    pub tableEnd: *mut c_void,
    pub tableValidEnd: *mut c_void,
    pub allocStart: *mut c_void,
    pub initOnceStart: *mut c_void,

    pub allocFailed: u8,
    pub workspaceOversizedDuration: i32,
    pub phase: ZSTD_cwksp_alloc_phase_e,
    pub isStatic: ZSTD_cwksp_static_alloc_e,
}

/// ZSTD_isPower2(x) = x != 0 && (x & (x-1)) == 0
#[inline]
pub fn ZSTD_isPower2(x: usize) -> bool {
    x != 0 && (x & (x.wrapping_sub(1))) == 0
}

/*-*************************************
*  Functions
***************************************/

#[inline]
pub unsafe fn ZSTD_cwksp_assert_internal_consistency(ws: *mut ZSTD_cwksp) {
    let _ = ws;
    debug_assert!((*ws).workspace <= (*ws).objectEnd);
    debug_assert!((*ws).objectEnd <= (*ws).tableEnd);
    debug_assert!((*ws).objectEnd <= (*ws).tableValidEnd);
    debug_assert!((*ws).tableEnd <= (*ws).allocStart);
    debug_assert!((*ws).tableValidEnd <= (*ws).allocStart);
    debug_assert!((*ws).allocStart <= (*ws).workspaceEnd);
    debug_assert!((*ws).initOnceStart <= ZSTD_cwksp_initialAllocStart(ws));
    debug_assert!((*ws).workspace <= (*ws).initOnceStart);
}

/// Align must be a power of 2.
#[inline]
pub fn ZSTD_cwksp_align(size: usize, align: usize) -> usize {
    let mask = align - 1;
    debug_assert!(ZSTD_isPower2(align));
    (size + mask) & !mask
}

/// Use this to determine how much space in the workspace we will consume to
/// allocate this object. (Under ASAN we pad; that path is compiled out here.)
#[inline]
pub fn ZSTD_cwksp_alloc_size(size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    size
}

#[inline]
pub fn ZSTD_cwksp_aligned_alloc_size(size: usize, alignment: usize) -> usize {
    ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(size, alignment))
}

/// Returns an adjusted alloc size that is the nearest larger multiple of 64 bytes.
#[inline]
pub fn ZSTD_cwksp_aligned64_alloc_size(size: usize) -> usize {
    ZSTD_cwksp_aligned_alloc_size(size, ZSTD_CWKSP_ALIGNMENT_BYTES)
}

/// Returns the amount of additional space the cwksp must allocate
/// for internal purposes (currently only alignment).
#[inline]
pub fn ZSTD_cwksp_slack_space_required() -> usize {
    let slackSpace = ZSTD_CWKSP_ALIGNMENT_BYTES * 2;
    slackSpace
}

/// Return the number of additional bytes required to align a pointer to the
/// given number of bytes. alignBytes must be a power of two.
#[inline]
pub fn ZSTD_cwksp_bytes_to_align_ptr(ptr: *mut c_void, alignBytes: usize) -> usize {
    let alignBytesMask = alignBytes - 1;
    let bytes = (alignBytes - ((ptr as usize) & alignBytesMask)) & alignBytesMask;
    debug_assert!(ZSTD_isPower2(alignBytes));
    debug_assert!(bytes < alignBytes);
    bytes
}

/// Returns the initial value for allocStart which is used to determine the
/// position from which we can allocate from the end of the workspace.
#[inline]
pub unsafe fn ZSTD_cwksp_initialAllocStart(ws: *mut ZSTD_cwksp) -> *mut c_void {
    let mut endPtr = (*ws).workspaceEnd as *mut u8;
    debug_assert!(ZSTD_isPower2(ZSTD_CWKSP_ALIGNMENT_BYTES));
    endPtr = endPtr.offset(-(((endPtr as usize) % ZSTD_CWKSP_ALIGNMENT_BYTES) as isize));
    endPtr as *mut c_void
}

/// Internal function. Do not use directly.
/// Reserves the given number of bytes within the aligned/buffer segment of the
/// wksp, which counts from the end of the wksp.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_internal_buffer_space(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
) -> *mut c_void {
    let alloc = ((*ws).allocStart as *mut u8).offset(-(bytes as isize)) as *mut c_void;
    let bottom = (*ws).tableEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
    debug_assert!(alloc >= bottom);
    if alloc < bottom {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    /* the area is reserved from the end of wksp.
     * If it overlaps with tableValidEnd, it voids guarantees on values' range */
    if alloc < (*ws).tableValidEnd {
        (*ws).tableValidEnd = alloc;
    }
    (*ws).allocStart = alloc;
    alloc
}

/// Moves the cwksp to the next phase, and does any necessary allocations.
/// Returns 0 on success, or a zstd error.
#[inline]
pub unsafe fn ZSTD_cwksp_internal_advance_phase(
    ws: *mut ZSTD_cwksp,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> usize {
    debug_assert!(phase >= (*ws).phase);
    if phase > (*ws).phase {
        /* Going from allocating objects to allocating initOnce / tables */
        if (*ws).phase < ZSTD_cwksp_alloc_aligned_init_once
            && phase >= ZSTD_cwksp_alloc_aligned_init_once
        {
            (*ws).tableValidEnd = (*ws).objectEnd;
            (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);

            {
                /* Align the start of the tables to 64 bytes. Use [0, 63] bytes */
                let alloc = (*ws).objectEnd;
                let bytesToAlign =
                    ZSTD_cwksp_bytes_to_align_ptr(alloc, ZSTD_CWKSP_ALIGNMENT_BYTES);
                let objectEnd = (alloc as *mut u8).add(bytesToAlign) as *mut c_void;
                if objectEnd > (*ws).workspaceEnd {
                    return error(code::MEMORY_ALLOCATION);
                }
                (*ws).objectEnd = objectEnd;
                (*ws).tableEnd = objectEnd; /* table area starts being empty */
                if (*ws).tableValidEnd < (*ws).tableEnd {
                    (*ws).tableValidEnd = (*ws).tableEnd;
                }
            }
        }
        (*ws).phase = phase;
        ZSTD_cwksp_assert_internal_consistency(ws);
    }
    0
}

/// Returns whether this object/buffer/etc was allocated in this workspace.
#[inline]
pub unsafe fn ZSTD_cwksp_owns_buffer(ws: *const ZSTD_cwksp, ptr: *const c_void) -> i32 {
    ((!ptr.is_null()) && ((*ws).workspace as *const c_void <= ptr) && (ptr < (*ws).workspaceEnd))
        as i32
}

/// Internal function. Do not use directly.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_internal(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> *mut c_void {
    let alloc: *mut c_void;
    if err_is_error(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 || bytes == 0 {
        return core::ptr::null_mut();
    }

    alloc = ZSTD_cwksp_reserve_internal_buffer_space(ws, bytes);

    alloc
}

/// Reserves and returns unaligned memory.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_buffer(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut u8 {
    ZSTD_cwksp_reserve_internal(ws, bytes, ZSTD_cwksp_alloc_buffers) as *mut u8
}

/// Reserves and returns memory sized on and aligned on 64 bytes, that has been
/// initialized at least once in the past.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_aligned_init_once(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
) -> *mut c_void {
    let alignedBytes = ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES);
    let ptr = ZSTD_cwksp_reserve_internal(ws, alignedBytes, ZSTD_cwksp_alloc_aligned_init_once);
    debug_assert!(((ptr as usize) & (ZSTD_CWKSP_ALIGNMENT_BYTES - 1)) == 0);
    if !ptr.is_null() && ptr < (*ws).initOnceStart {
        /* We assume the memory following the current allocation is either not
         * usable, another initOnce buffer, or (ASAN) a redzone. So it should be
         * fine to not explicitly zero every byte up to ws->initOnceStart. */
        let diff = (*ws).initOnceStart as usize - ptr as usize;
        let n = if diff < alignedBytes { diff } else { alignedBytes };
        core::ptr::write_bytes(ptr as *mut u8, 0, n);
        (*ws).initOnceStart = ptr;
    }
    ptr
}

/// Reserves and returns memory sized on and aligned on 64 bytes.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_aligned64(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    let ptr = ZSTD_cwksp_reserve_internal(
        ws,
        ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES),
        ZSTD_cwksp_alloc_aligned,
    );
    debug_assert!(((ptr as usize) & (ZSTD_CWKSP_ALIGNMENT_BYTES - 1)) == 0);
    ptr
}

/// Aligned on 64 bytes. These buffers keep their values constrained, allowing
/// reuse without memset()-ing them.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_table(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    let phase = ZSTD_cwksp_alloc_aligned_init_once;
    let alloc: *mut c_void;
    let end: *mut c_void;
    let top: *mut c_void;

    /* We can only start allocating tables after reserving space for objects */
    if (*ws).phase < phase {
        if err_is_error(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 {
            return core::ptr::null_mut();
        }
    }
    alloc = (*ws).tableEnd;
    end = (alloc as *mut u8).add(bytes) as *mut c_void;
    top = (*ws).allocStart;

    debug_assert!((bytes & (core::mem::size_of::<u32>() - 1)) == 0);
    ZSTD_cwksp_assert_internal_consistency(ws);
    debug_assert!(end <= top);
    if end > top {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    (*ws).tableEnd = end;

    debug_assert!((bytes & (ZSTD_CWKSP_ALIGNMENT_BYTES - 1)) == 0);
    debug_assert!(((alloc as usize) & (ZSTD_CWKSP_ALIGNMENT_BYTES - 1)) == 0);
    alloc
}

/// Aligned on sizeof(void*). Should happen only once, at first initialization.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_object(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    let roundedBytes = ZSTD_cwksp_align(bytes, core::mem::size_of::<*mut c_void>());
    let alloc = (*ws).objectEnd;
    let end = (alloc as *mut u8).add(roundedBytes) as *mut c_void;

    debug_assert!((alloc as usize) % core::mem::align_of::<*mut c_void>() == 0);
    debug_assert!(bytes % core::mem::align_of::<*mut c_void>() == 0);
    ZSTD_cwksp_assert_internal_consistency(ws);
    /* we must be in the first phase, no advance is possible */
    if (*ws).phase != ZSTD_cwksp_alloc_objects || end > (*ws).workspaceEnd {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    (*ws).objectEnd = end;
    (*ws).tableEnd = end;
    (*ws).tableValidEnd = end;

    alloc
}

/// With alignment control. Should happen only once, at first initialization.
#[inline]
pub unsafe fn ZSTD_cwksp_reserve_object_aligned(
    ws: *mut ZSTD_cwksp,
    byteSize: usize,
    alignment: usize,
) -> *mut c_void {
    let mask = alignment - 1;
    let surplus = if alignment > core::mem::size_of::<*mut c_void>() {
        alignment - core::mem::size_of::<*mut c_void>()
    } else {
        0
    };
    let start = ZSTD_cwksp_reserve_object(ws, byteSize + surplus);
    if start.is_null() {
        return core::ptr::null_mut();
    }
    if surplus == 0 {
        return start;
    }
    debug_assert!(ZSTD_isPower2(alignment));
    (((start as usize) + surplus) & !mask) as *mut c_void
}

#[inline]
pub unsafe fn ZSTD_cwksp_mark_tables_dirty(ws: *mut ZSTD_cwksp) {
    debug_assert!((*ws).tableValidEnd >= (*ws).objectEnd);
    debug_assert!((*ws).tableValidEnd <= (*ws).allocStart);
    (*ws).tableValidEnd = (*ws).objectEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

#[inline]
pub unsafe fn ZSTD_cwksp_mark_tables_clean(ws: *mut ZSTD_cwksp) {
    debug_assert!((*ws).tableValidEnd >= (*ws).objectEnd);
    debug_assert!((*ws).tableValidEnd <= (*ws).allocStart);
    if (*ws).tableValidEnd < (*ws).tableEnd {
        (*ws).tableValidEnd = (*ws).tableEnd;
    }
    ZSTD_cwksp_assert_internal_consistency(ws);
}

/// Zero the part of the allocated tables not already marked clean.
#[inline]
pub unsafe fn ZSTD_cwksp_clean_tables(ws: *mut ZSTD_cwksp) {
    debug_assert!((*ws).tableValidEnd >= (*ws).objectEnd);
    debug_assert!((*ws).tableValidEnd <= (*ws).allocStart);
    if (*ws).tableValidEnd < (*ws).tableEnd {
        let n = (*ws).tableEnd as usize - (*ws).tableValidEnd as usize;
        core::ptr::write_bytes((*ws).tableValidEnd as *mut u8, 0, n);
    }
    ZSTD_cwksp_mark_tables_clean(ws);
}

/// Invalidates table allocations. All other allocations remain valid.
#[inline]
pub unsafe fn ZSTD_cwksp_clear_tables(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

/// Invalidates all buffer, aligned, and table allocations.
/// Object allocations remain valid.
#[inline]
pub unsafe fn ZSTD_cwksp_clear(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
    (*ws).allocStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).allocFailed = 0;
    if (*ws).phase > ZSTD_cwksp_alloc_aligned_init_once {
        (*ws).phase = ZSTD_cwksp_alloc_aligned_init_once;
    }
    ZSTD_cwksp_assert_internal_consistency(ws);
}

#[inline]
pub unsafe fn ZSTD_cwksp_sizeof(ws: *const ZSTD_cwksp) -> usize {
    (*ws).workspaceEnd as usize - (*ws).workspace as usize
}

#[inline]
pub unsafe fn ZSTD_cwksp_used(ws: *const ZSTD_cwksp) -> usize {
    ((*ws).tableEnd as usize - (*ws).workspace as usize)
        + ((*ws).workspaceEnd as usize - (*ws).allocStart as usize)
}

/// The provided workspace takes ownership of the buffer [start, start+size).
#[inline]
pub unsafe fn ZSTD_cwksp_init(
    ws: *mut ZSTD_cwksp,
    start: *mut c_void,
    size: usize,
    isStatic: ZSTD_cwksp_static_alloc_e,
) {
    debug_assert!(((start as usize) & (core::mem::size_of::<*mut c_void>() - 1)) == 0);
    (*ws).workspace = start;
    (*ws).workspaceEnd = (start as *mut u8).add(size) as *mut c_void;
    (*ws).objectEnd = (*ws).workspace;
    (*ws).tableValidEnd = (*ws).objectEnd;
    (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).phase = ZSTD_cwksp_alloc_objects;
    (*ws).isStatic = isStatic;
    ZSTD_cwksp_clear(ws);
    (*ws).workspaceOversizedDuration = 0;
    ZSTD_cwksp_assert_internal_consistency(ws);
}

#[inline]
pub unsafe fn ZSTD_cwksp_create(
    ws: *mut ZSTD_cwksp,
    size: usize,
    customMem: ZSTD_customMem,
) -> usize {
    let workspace = zstd_custom_malloc(size, customMem);
    if workspace.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    ZSTD_cwksp_init(ws, workspace, size, ZSTD_cwksp_dynamic_alloc);
    0
}

#[inline]
pub unsafe fn ZSTD_cwksp_free(ws: *mut ZSTD_cwksp, customMem: ZSTD_customMem) {
    let ptr = (*ws).workspace;
    core::ptr::write_bytes(ws as *mut u8, 0, core::mem::size_of::<ZSTD_cwksp>());
    zstd_custom_free(ptr, customMem);
}

/// Moves the management of a workspace from one cwksp to another. The src cwksp
/// is left in an invalid state (must be re-init()'ed before use again).
#[inline]
pub unsafe fn ZSTD_cwksp_move(dst: *mut ZSTD_cwksp, src: *mut ZSTD_cwksp) {
    core::ptr::copy_nonoverlapping(src as *const ZSTD_cwksp, dst, 1);
    core::ptr::write_bytes(src as *mut u8, 0, core::mem::size_of::<ZSTD_cwksp>());
}

#[inline]
pub unsafe fn ZSTD_cwksp_reserve_failed(ws: *const ZSTD_cwksp) -> i32 {
    (*ws).allocFailed as i32
}

/*-*************************************
*  Functions Checking Free Space
***************************************/

/// Returns whether the estimated space needed for a wksp is within an
/// acceptable limit of the actual amount of space used.
#[inline]
pub unsafe fn ZSTD_cwksp_estimated_space_within_bounds(
    ws: *const ZSTD_cwksp,
    estimatedSpace: usize,
) -> i32 {
    ((estimatedSpace - ZSTD_cwksp_slack_space_required() <= ZSTD_cwksp_used(ws))
        && (ZSTD_cwksp_used(ws) <= estimatedSpace)) as i32
}

#[inline]
pub unsafe fn ZSTD_cwksp_available_space(ws: *mut ZSTD_cwksp) -> usize {
    (*ws).allocStart as usize - (*ws).tableEnd as usize
}

#[inline]
pub unsafe fn ZSTD_cwksp_check_available(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> i32 {
    (ZSTD_cwksp_available_space(ws) >= additionalNeededSpace) as i32
}

#[inline]
pub unsafe fn ZSTD_cwksp_check_too_large(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> i32 {
    ZSTD_cwksp_check_available(
        ws,
        additionalNeededSpace * ZSTD_WORKSPACETOOLARGE_FACTOR,
    )
}

#[inline]
pub unsafe fn ZSTD_cwksp_check_wasteful(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> i32 {
    ((ZSTD_cwksp_check_too_large(ws, additionalNeededSpace) != 0)
        && ((*ws).workspaceOversizedDuration as usize > ZSTD_WORKSPACETOOLARGE_MAXDURATION))
        as i32
}

#[inline]
pub unsafe fn ZSTD_cwksp_bump_oversized_duration(
    ws: *mut ZSTD_cwksp,
    additionalNeededSpace: usize,
) {
    if ZSTD_cwksp_check_too_large(ws, additionalNeededSpace) != 0 {
        (*ws).workspaceOversizedDuration += 1;
    } else {
        (*ws).workspaceOversizedDuration = 0;
    }
}
