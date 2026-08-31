//! FFI helpers shared across the translated modules.

use core::ffi::c_void;

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
}

/// `calloc(1, s)`
#[inline]
pub unsafe fn alloc_and_zero(s: usize) -> *mut c_void {
    unsafe { calloc(1, s) }
}

/* ---- unaligned memory access helpers (LZ4_read16/32, LZ4_write16/32) ---- */

#[inline(always)]
pub unsafe fn read16(p: *const u8) -> u16 {
    unsafe { core::ptr::read_unaligned(p as *const u16) }
}

#[inline(always)]
pub unsafe fn read32(p: *const u8) -> u32 {
    unsafe { core::ptr::read_unaligned(p as *const u32) }
}

#[inline(always)]
pub unsafe fn read64(p: *const u8) -> u64 {
    unsafe { core::ptr::read_unaligned(p as *const u64) }
}

/// `LZ4_read_ARCH` : reg_t is U64 on x86_64
#[inline(always)]
pub unsafe fn read_arch(p: *const u8) -> u64 {
    unsafe { read64(p) }
}

#[inline(always)]
pub unsafe fn write16(p: *mut u8, v: u16) {
    unsafe { core::ptr::write_unaligned(p as *mut u16, v) }
}

#[inline(always)]
pub unsafe fn write32(p: *mut u8, v: u32) {
    unsafe { core::ptr::write_unaligned(p as *mut u32, v) }
}

#[inline(always)]
pub unsafe fn read_le16(p: *const u8) -> u16 {
    unsafe { u16::from_le(read16(p)) }
}

#[inline(always)]
pub unsafe fn write_le16(p: *mut u8, v: u16) {
    unsafe { write16(p, v.to_le()) }
}

/// 8-byte copy with load-then-store semantics (matches `memcpy(d,s,8)`).
#[inline(always)]
pub unsafe fn copy8(d: *mut u8, s: *const u8) {
    unsafe {
        let v = read64(s);
        core::ptr::write_unaligned(d as *mut u64, v);
    }
}

/// 16-byte copy: read both halves, then write both.
#[inline(always)]
pub unsafe fn copy16(d: *mut u8, s: *const u8) {
    unsafe {
        let a = read64(s);
        let b = read64(s.add(8));
        core::ptr::write_unaligned(d as *mut u64, a);
        core::ptr::write_unaligned(d.add(8) as *mut u64, b);
    }
}

#[inline(always)]
pub unsafe fn copy4(d: *mut u8, s: *const u8) {
    unsafe {
        let v = read32(s);
        write32(d, v);
    }
}

#[inline(always)]
pub unsafe fn copy2(d: *mut u8, s: *const u8) {
    unsafe {
        let v = read16(s);
        write16(d, v);
    }
}

/// `memcpy` for runtime-sized, non-overlapping regions.
#[inline(always)]
pub unsafe fn mem_copy(d: *mut u8, s: *const u8, n: usize) {
    unsafe { core::ptr::copy(s, d, n) }
}

/// `memmove`
#[inline(always)]
pub unsafe fn mem_move(d: *mut u8, s: *const u8, n: usize) {
    unsafe { core::ptr::copy(s, d, n) }
}

#[inline(always)]
pub unsafe fn mem_init(d: *mut u8, v: u8, n: usize) {
    unsafe { core::ptr::write_bytes(d, v, n) }
}
