//! System V AMD64 `va_list` support.
//!
//! Stable Rust cannot define C variadic functions, so the variadic entry points
//! of the public jansson API are implemented as naked trampolines that build a
//! `va_list` exactly the way GCC's prologue does and then tail into the
//! corresponding `v*` function.

use core::ffi::c_void;

/// `__va_list_tag` from the System V AMD64 psABI. A C `va_list` is
/// `__va_list_tag[1]`, so it decays to a pointer to this structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VaListTag {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut u8,
    pub reg_save_area: *mut u8,
}

/// Fetch the next INTEGER-class argument (pointers and integers up to 8 bytes).
#[inline]
pub unsafe fn arg_gp(ap: *mut VaListTag) -> u64 {
    let t = &mut *ap;
    if t.gp_offset <= 48 - 8 {
        let p = t.reg_save_area.add(t.gp_offset as usize) as *const u64;
        t.gp_offset += 8;
        p.read_unaligned()
    } else {
        let p = t.overflow_arg_area as *const u64;
        t.overflow_arg_area = t.overflow_arg_area.add(8);
        p.read_unaligned()
    }
}

/// Fetch the next SSE-class argument (`double`).
#[inline]
pub unsafe fn arg_double(ap: *mut VaListTag) -> f64 {
    let t = &mut *ap;
    if t.fp_offset <= 176 - 16 {
        let p = t.reg_save_area.add(t.fp_offset as usize) as *const f64;
        t.fp_offset += 16;
        p.read_unaligned()
    } else {
        let p = t.overflow_arg_area as *const f64;
        t.overflow_arg_area = t.overflow_arg_area.add(8);
        p.read_unaligned()
    }
}

#[inline]
pub unsafe fn arg_int(ap: *mut VaListTag) -> i32 {
    arg_gp(ap) as u32 as i32
}

#[inline]
pub unsafe fn arg_size(ap: *mut VaListTag) -> usize {
    arg_gp(ap) as usize
}

#[inline]
pub unsafe fn arg_i64(ap: *mut VaListTag) -> i64 {
    arg_gp(ap) as i64
}

#[inline]
pub unsafe fn arg_ptr<T>(ap: *mut VaListTag) -> *mut T {
    arg_gp(ap) as *mut T
}

/// `va_copy()`
#[inline]
pub unsafe fn va_copy(ap: *mut VaListTag) -> VaListTag {
    *ap
}

pub type VoidPtr = *mut c_void;

/// Builds a `va_list` on the stack for a variadic function whose named
/// arguments occupy the first `$gp` bytes of the general purpose register save
/// area, then invokes `$target` after the `$tail` instructions have moved the
/// arguments into place.
///
/// Stack frame (relative to `rbp`):
///   `[rbp-176 .. rbp-129]` general purpose register save area (rdi..r9)
///   `[rbp-128 .. rbp-1]`   SSE register save area (xmm0..xmm7)
///   `[rbp-208 .. rbp-185]` the `__va_list_tag`
///   `[rbp-224 .. rbp-217]` outgoing stack argument slot
macro_rules! va_trampoline {
    ($name:ident, $gp:expr, $target:path, [$($tail:literal),* $(,)?]) => {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub unsafe extern "C" fn $name() -> VoidPtr {
            core::arch::naked_asm!(
                "push rbp",
                "mov rbp, rsp",
                "sub rsp, 224",
                "mov [rbp-176], rdi",
                "mov [rbp-168], rsi",
                "mov [rbp-160], rdx",
                "mov [rbp-152], rcx",
                "mov [rbp-144], r8",
                "mov [rbp-136], r9",
                "test al, al",
                "je 2f",
                "movaps [rbp-128], xmm0",
                "movaps [rbp-112], xmm1",
                "movaps [rbp-96], xmm2",
                "movaps [rbp-80], xmm3",
                "movaps [rbp-64], xmm4",
                "movaps [rbp-48], xmm5",
                "movaps [rbp-32], xmm6",
                "movaps [rbp-16], xmm7",
                "2:",
                concat!("mov dword ptr [rbp-208], ", $gp),
                "mov dword ptr [rbp-204], 48",
                "lea rax, [rbp+16]",
                "mov [rbp-200], rax",
                "lea rax, [rbp-176]",
                "mov [rbp-192], rax",
                $($tail,)*
                "call {t}",
                "leave",
                "ret",
                t = sym $target,
            )
        }
    };
}

/* json_pack(fmt, ...) -> json_vpack_ex(NULL, 0, fmt, ap) */
va_trampoline!(json_pack, 8, crate::pack_unpack::json_vpack_ex, [
    "mov rdx, rdi",
    "xor esi, esi",
    "xor edi, edi",
    "lea rcx, [rbp-208]",
]);

/* json_pack_ex(error, flags, fmt, ...) -> json_vpack_ex(error, flags, fmt, ap) */
va_trampoline!(json_pack_ex, 24, crate::pack_unpack::json_vpack_ex, [
    "lea rcx, [rbp-208]",
]);

/* json_sprintf(fmt, ...) -> json_vsprintf(fmt, ap) */
va_trampoline!(json_sprintf, 8, crate::value::json_vsprintf, [
    "lea rsi, [rbp-208]",
]);

/* json_unpack(root, fmt, ...) -> json_vunpack_ex(root, NULL, 0, fmt, ap) */
va_trampoline!(json_unpack, 16, crate::pack_unpack::json_vunpack_ex, [
    "mov rcx, rsi",
    "xor esi, esi",
    "xor edx, edx",
    "lea r8, [rbp-208]",
]);

/* json_unpack_ex(root, error, flags, fmt, ...)
   -> json_vunpack_ex(root, error, flags, fmt, ap) */
va_trampoline!(json_unpack_ex, 32, crate::pack_unpack::json_vunpack_ex, [
    "lea r8, [rbp-208]",
]);

/* jsonp_error_set(error, line, column, position, code, msg, ...)
   -> jsonp_error_vset(error, line, column, position, code, msg, ap) */
va_trampoline!(jsonp_error_set, 48, crate::error::jsonp_error_vset, [
    "lea rax, [rbp-208]",
    "mov [rsp], rax",
]);
