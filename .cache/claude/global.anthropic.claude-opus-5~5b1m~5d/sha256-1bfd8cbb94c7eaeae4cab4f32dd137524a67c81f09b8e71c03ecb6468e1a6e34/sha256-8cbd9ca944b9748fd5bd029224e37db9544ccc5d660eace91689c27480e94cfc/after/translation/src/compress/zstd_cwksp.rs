//! Translation of `compress/zstd_cwksp.h`
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

use crate::cmem::*;
use crate::error_private::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

pub const ZSTD_CWKSP_ALIGNMENT_BYTES: usize = 64;

pub type ZSTD_cwksp_alloc_phase_e = u32;
pub const ZSTD_cwksp_alloc_objects: ZSTD_cwksp_alloc_phase_e = 0;
pub const ZSTD_cwksp_alloc_aligned_init_once: ZSTD_cwksp_alloc_phase_e = 1;
pub const ZSTD_cwksp_alloc_aligned: ZSTD_cwksp_alloc_phase_e = 2;
pub const ZSTD_cwksp_alloc_buffers: ZSTD_cwksp_alloc_phase_e = 3;

pub type ZSTD_cwksp_static_alloc_e = u32;
pub const ZSTD_cwksp_dynamic_alloc: ZSTD_cwksp_static_alloc_e = 0;
pub const ZSTD_cwksp_static_alloc: ZSTD_cwksp_static_alloc_e = 1;

#[repr(C)]
#[derive(Copy, Clone)]
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
            workspace: core::ptr::null_mut(),
            workspaceEnd: core::ptr::null_mut(),
            objectEnd: core::ptr::null_mut(),
            tableEnd: core::ptr::null_mut(),
            tableValidEnd: core::ptr::null_mut(),
            allocStart: core::ptr::null_mut(),
            initOnceStart: core::ptr::null_mut(),
            allocFailed: 0,
            workspaceOversizedDuration: 0,
            phase: ZSTD_cwksp_alloc_objects,
            isStatic: ZSTD_cwksp_dynamic_alloc,
        }
    }
}

#[inline(always)]
pub fn ZSTD_cwksp_align(size: usize, align: usize) -> usize {
    let mask = align - 1;
    (size.wrapping_add(mask)) & !mask
}

#[inline(always)]
pub fn ZSTD_cwksp_alloc_size(size: usize) -> usize {
    if size == 0 {
        return 0;
    }
    size
}

#[inline(always)]
pub fn ZSTD_cwksp_aligned_alloc_size(size: usize, alignment: usize) -> usize {
    ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(size, alignment))
}

#[inline(always)]
pub fn ZSTD_cwksp_aligned64_alloc_size(size: usize) -> usize {
    ZSTD_cwksp_aligned_alloc_size(size, ZSTD_CWKSP_ALIGNMENT_BYTES)
}

#[inline(always)]
pub fn ZSTD_cwksp_slack_space_required() -> usize {
    ZSTD_CWKSP_ALIGNMENT_BYTES * 2
}

#[inline(always)]
pub fn ZSTD_cwksp_bytes_to_align_ptr(ptr: *mut c_void, alignBytes: usize) -> usize {
    let alignBytesMask = alignBytes - 1;
    (alignBytes - ((ptr as usize) & alignBytesMask)) & alignBytesMask
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_initialAllocStart(ws: *mut ZSTD_cwksp) -> *mut c_void {
    let mut endPtr = (*ws).workspaceEnd as *mut u8;
    endPtr = endPtr.sub((endPtr as usize) % ZSTD_CWKSP_ALIGNMENT_BYTES);
    endPtr as *mut c_void
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_internal_buffer_space(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
) -> *mut c_void {
    let alloc = ((*ws).allocStart as *mut BYTE).wrapping_sub(bytes) as *mut c_void;
    let bottom = (*ws).tableEnd;
    if alloc < bottom {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    if alloc < (*ws).tableValidEnd {
        (*ws).tableValidEnd = alloc;
    }
    (*ws).allocStart = alloc;
    alloc
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_internal_advance_phase(
    ws: *mut ZSTD_cwksp,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> usize {
    if phase > (*ws).phase {
        if (*ws).phase < ZSTD_cwksp_alloc_aligned_init_once
            && phase >= ZSTD_cwksp_alloc_aligned_init_once
        {
            (*ws).tableValidEnd = (*ws).objectEnd;
            (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);

            let alloc = (*ws).objectEnd;
            let bytesToAlign = ZSTD_cwksp_bytes_to_align_ptr(alloc, ZSTD_CWKSP_ALIGNMENT_BYTES);
            let objectEnd = (alloc as *mut BYTE).add(bytesToAlign) as *mut c_void;
            if objectEnd > (*ws).workspaceEnd {
                return ERROR(ZSTD_error_memory_allocation);
            }
            (*ws).objectEnd = objectEnd;
            (*ws).tableEnd = objectEnd;
            if (*ws).tableValidEnd < (*ws).tableEnd {
                (*ws).tableValidEnd = (*ws).tableEnd;
            }
        }
        (*ws).phase = phase;
    }
    0
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_owns_buffer(ws: *const ZSTD_cwksp, ptr: *const c_void) -> c_int {
    (!ptr.is_null() && (*ws).workspace as *const c_void <= ptr && ptr < (*ws).workspaceEnd) as c_int
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_internal(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
    phase: ZSTD_cwksp_alloc_phase_e,
) -> *mut c_void {
    if ERR_isError(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 || bytes == 0 {
        return core::ptr::null_mut();
    }
    ZSTD_cwksp_reserve_internal_buffer_space(ws, bytes)
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_buffer(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut BYTE {
    ZSTD_cwksp_reserve_internal(ws, bytes, ZSTD_cwksp_alloc_buffers) as *mut BYTE
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_aligned_init_once(
    ws: *mut ZSTD_cwksp,
    bytes: usize,
) -> *mut c_void {
    let alignedBytes = ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES);
    let ptr = ZSTD_cwksp_reserve_internal(ws, alignedBytes, ZSTD_cwksp_alloc_aligned_init_once);
    if !ptr.is_null() && ptr < (*ws).initOnceStart {
        let avail = ((*ws).initOnceStart as *mut u8 as usize) - (ptr as *mut u8 as usize);
        ZSTD_memset(ptr, 0, if avail < alignedBytes { avail } else { alignedBytes });
        (*ws).initOnceStart = ptr;
    }
    ptr
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_aligned64(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    ZSTD_cwksp_reserve_internal(
        ws,
        ZSTD_cwksp_align(bytes, ZSTD_CWKSP_ALIGNMENT_BYTES),
        ZSTD_cwksp_alloc_aligned,
    )
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_table(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    let phase = ZSTD_cwksp_alloc_aligned_init_once;
    if (*ws).phase < phase {
        if ERR_isError(ZSTD_cwksp_internal_advance_phase(ws, phase)) != 0 {
            return core::ptr::null_mut();
        }
    }
    let alloc = (*ws).tableEnd;
    let end = (alloc as *mut BYTE).add(bytes) as *mut c_void;
    let top = (*ws).allocStart;

    if end > top {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    (*ws).tableEnd = end;
    alloc
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_object(ws: *mut ZSTD_cwksp, bytes: usize) -> *mut c_void {
    let roundedBytes = ZSTD_cwksp_align(bytes, core::mem::size_of::<*mut c_void>());
    let alloc = (*ws).objectEnd;
    let end = (alloc as *mut BYTE).add(roundedBytes) as *mut c_void;

    if (*ws).phase != ZSTD_cwksp_alloc_objects || end > (*ws).workspaceEnd {
        (*ws).allocFailed = 1;
        return core::ptr::null_mut();
    }
    (*ws).objectEnd = end;
    (*ws).tableEnd = end;
    (*ws).tableValidEnd = end;
    alloc
}

#[inline(always)]
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
    (((start as usize) + surplus) & !mask) as *mut c_void
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_mark_tables_dirty(ws: *mut ZSTD_cwksp) {
    (*ws).tableValidEnd = (*ws).objectEnd;
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_mark_tables_clean(ws: *mut ZSTD_cwksp) {
    if (*ws).tableValidEnd < (*ws).tableEnd {
        (*ws).tableValidEnd = (*ws).tableEnd;
    }
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_clean_tables(ws: *mut ZSTD_cwksp) {
    if (*ws).tableValidEnd < (*ws).tableEnd {
        ZSTD_memset(
            (*ws).tableValidEnd,
            0,
            ((*ws).tableEnd as *mut BYTE as usize) - ((*ws).tableValidEnd as *mut BYTE as usize),
        );
    }
    ZSTD_cwksp_mark_tables_clean(ws);
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_clear_tables(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_clear(ws: *mut ZSTD_cwksp) {
    (*ws).tableEnd = (*ws).objectEnd;
    (*ws).allocStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).allocFailed = 0;
    if (*ws).phase > ZSTD_cwksp_alloc_aligned_init_once {
        (*ws).phase = ZSTD_cwksp_alloc_aligned_init_once;
    }
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_sizeof(ws: *const ZSTD_cwksp) -> usize {
    ((*ws).workspaceEnd as *mut BYTE as usize) - ((*ws).workspace as *mut BYTE as usize)
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_used(ws: *const ZSTD_cwksp) -> usize {
    (((*ws).tableEnd as *mut BYTE as usize) - ((*ws).workspace as *mut BYTE as usize))
        + (((*ws).workspaceEnd as *mut BYTE as usize) - ((*ws).allocStart as *mut BYTE as usize))
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_init(
    ws: *mut ZSTD_cwksp,
    start: *mut c_void,
    size: usize,
    isStatic: ZSTD_cwksp_static_alloc_e,
) {
    (*ws).workspace = start;
    (*ws).workspaceEnd = (start as *mut BYTE).add(size) as *mut c_void;
    (*ws).objectEnd = (*ws).workspace;
    (*ws).tableValidEnd = (*ws).objectEnd;
    (*ws).initOnceStart = ZSTD_cwksp_initialAllocStart(ws);
    (*ws).phase = ZSTD_cwksp_alloc_objects;
    (*ws).isStatic = isStatic;
    ZSTD_cwksp_clear(ws);
    (*ws).workspaceOversizedDuration = 0;
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_create(
    ws: *mut ZSTD_cwksp,
    size: usize,
    customMem: ZSTD_customMem,
) -> usize {
    let workspace = ZSTD_customMalloc(size, customMem);
    if workspace.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    ZSTD_cwksp_init(ws, workspace, size, ZSTD_cwksp_dynamic_alloc);
    0
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_free(ws: *mut ZSTD_cwksp, customMem: ZSTD_customMem) {
    let ptr = (*ws).workspace;
    ZSTD_memset(
        ws as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_cwksp>(),
    );
    ZSTD_customFree(ptr, customMem);
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_move(dst: *mut ZSTD_cwksp, src: *mut ZSTD_cwksp) {
    *dst = *src;
    ZSTD_memset(
        src as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_cwksp>(),
    );
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_reserve_failed(ws: *const ZSTD_cwksp) -> c_int {
    (*ws).allocFailed as c_int
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_estimated_space_within_bounds(
    ws: *const ZSTD_cwksp,
    estimatedSpace: usize,
) -> c_int {
    ((estimatedSpace.wrapping_sub(ZSTD_cwksp_slack_space_required())) <= ZSTD_cwksp_used(ws)
        && ZSTD_cwksp_used(ws) <= estimatedSpace) as c_int
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_available_space(ws: *mut ZSTD_cwksp) -> usize {
    ((*ws).allocStart as *mut BYTE as usize).wrapping_sub((*ws).tableEnd as *mut BYTE as usize)
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_check_available(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> c_int {
    (ZSTD_cwksp_available_space(ws) >= additionalNeededSpace) as c_int
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_check_too_large(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> c_int {
    ZSTD_cwksp_check_available(
        ws,
        additionalNeededSpace.wrapping_mul(ZSTD_WORKSPACETOOLARGE_FACTOR),
    )
}

#[inline(always)]
pub unsafe fn ZSTD_cwksp_check_wasteful(ws: *mut ZSTD_cwksp, additionalNeededSpace: usize) -> c_int {
    (ZSTD_cwksp_check_too_large(ws, additionalNeededSpace) != 0
        && (*ws).workspaceOversizedDuration > ZSTD_WORKSPACETOOLARGE_MAXDURATION as c_int)
        as c_int
}

#[inline(always)]
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
