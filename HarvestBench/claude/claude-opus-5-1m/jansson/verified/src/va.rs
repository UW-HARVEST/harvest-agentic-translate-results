//! C-variadic entry points.
//!
//! Rust cannot (on stable) *define* C-variadic functions, so the six variadic
//! functions of the library are provided as naked functions that build an
//! x86-64 SysV `va_list` exactly like a C compiler would and then tail-call the
//! corresponding `v*` implementation.
#![allow(dead_code)]

use std::ffi::{c_char, c_int};

use crate::jansson::{json_error_t, json_t};

/*
 * Frame layout (rbp-relative), identical to what GCC emits:
 *
 *   [rbp-208 .. rbp-185]   va_list: gp_offset, fp_offset,
 *                                   overflow_arg_area, reg_save_area
 *   [rbp-176 .. rbp-129]   general purpose register save area (rdi..r9)
 *   [rbp-128 .. rbp-1]     xmm0..xmm7 save area
 *   [rbp+16 .. ]           incoming stack arguments (overflow_arg_area)
 */

macro_rules! va_prologue {
    () => {
        concat!(
            "push rbp\n",
            "mov rbp, rsp\n",
            "sub rsp, 224\n",
            "mov qword ptr [rbp - 176], rdi\n",
            "mov qword ptr [rbp - 168], rsi\n",
            "mov qword ptr [rbp - 160], rdx\n",
            "mov qword ptr [rbp - 152], rcx\n",
            "mov qword ptr [rbp - 144], r8\n",
            "mov qword ptr [rbp - 136], r9\n",
            "test al, al\n",
            "je 2f\n",
            "movaps xmmword ptr [rbp - 128], xmm0\n",
            "movaps xmmword ptr [rbp - 112], xmm1\n",
            "movaps xmmword ptr [rbp - 96], xmm2\n",
            "movaps xmmword ptr [rbp - 80], xmm3\n",
            "movaps xmmword ptr [rbp - 64], xmm4\n",
            "movaps xmmword ptr [rbp - 48], xmm5\n",
            "movaps xmmword ptr [rbp - 32], xmm6\n",
            "movaps xmmword ptr [rbp - 16], xmm7\n",
            "2:\n",
            "mov dword ptr [rbp - 204], 48\n",
            "lea rax, [rbp + 16]\n",
            "mov qword ptr [rbp - 200], rax\n",
            "lea rax, [rbp - 176]\n",
            "mov qword ptr [rbp - 192], rax\n",
        )
    };
}

/* json_t *json_pack(const char *fmt, ...)
 *     -> json_vpack_ex(NULL, 0, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack(fmt: *const c_char) -> *mut json_t {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 8",
        "mov rdx, rdi",
        "xor edi, edi",
        "xor esi, esi",
        "lea rcx, [rbp - 208]",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::pack_unpack::json_vpack_ex,
    )
}

/* json_t *json_pack_ex(json_error_t *error, size_t flags, const char *fmt, ...)
 *     -> json_vpack_ex(error, flags, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
) -> *mut json_t {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 24",
        "lea rcx, [rbp - 208]",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::pack_unpack::json_vpack_ex,
    )
}

/* int json_unpack(json_t *root, const char *fmt, ...)
 *     -> json_vunpack_ex(root, NULL, 0, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack(root: *mut json_t, fmt: *const c_char) -> c_int {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 16",
        "mov rcx, rsi",
        "xor esi, esi",
        "xor edx, edx",
        "lea r8, [rbp - 208]",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::pack_unpack::json_vunpack_ex,
    )
}

/* int json_unpack_ex(json_t *root, json_error_t *error, size_t flags,
 *                    const char *fmt, ...)
 *     -> json_vunpack_ex(root, error, flags, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
) -> c_int {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 32",
        "lea r8, [rbp - 208]",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::pack_unpack::json_vunpack_ex,
    )
}

/* json_t *json_sprintf(const char *fmt, ...)
 *     -> json_vsprintf(fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_sprintf(fmt: *const c_char) -> *mut json_t {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 8",
        "lea rsi, [rbp - 208]",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::value::json_vsprintf,
    )
}

/* void jsonp_error_set(json_error_t *error, int line, int column,
 *                      size_t position, enum json_error_code code,
 *                      const char *msg, ...)
 *     -> jsonp_error_vset(error, line, column, position, code, msg, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    msg: *const c_char,
) {
    core::arch::naked_asm!(
        va_prologue!(),
        "mov dword ptr [rbp - 208], 48",
        /* the 7th argument (ap) is passed on the stack */
        "lea rax, [rbp - 208]",
        "mov qword ptr [rsp], rax",
        "call {f}",
        "leave",
        "ret",
        f = sym crate::error::jsonp_error_vset,
    )
}
