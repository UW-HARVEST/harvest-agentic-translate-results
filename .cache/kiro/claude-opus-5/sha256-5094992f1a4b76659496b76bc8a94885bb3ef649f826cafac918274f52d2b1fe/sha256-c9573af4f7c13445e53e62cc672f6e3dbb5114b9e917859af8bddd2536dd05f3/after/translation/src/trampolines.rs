//! The C-variadic public entry points.
//!
//! Defining C-variadic functions is not available on stable Rust, so each
//! variadic symbol is emitted as a naked function containing the `va_start`
//! prologue that GCC generates for the x86-64 System V ABI, followed by a
//! call to the corresponding `v...` function which takes a `va_list`.
//!
//! Stack frame layout of the `<= 4 named integer arguments` variant
//! (`subq $216, %rsp`, which keeps `%rsp` 16-byte aligned):
//!
//! ```text
//!    0(%rsp) ..   8   unused padding
//!    8(%rsp) ..  32   the va_list structure
//!                     (gp_offset, fp_offset, overflow_arg_area, reg_save_area)
//!   32(%rsp) .. 208   register save area (6*8 GP registers + 8*16 SSE registers)
//!  224(%rsp)          the caller's first stack argument (overflow area)
//! ```
//!
//! `gp_offset` starts at `8 * <number of named integer arguments>` and
//! `fp_offset` at 48 (the size of the GP part of the register save area).

use crate::jtypes::{json_error_t, json_t};
use crate::valist::VaList;
use core::ffi::{c_char, c_int};
use core::ptr::null_mut;

/* ------------------------------------------------------------------ */
/* forwarding helpers                                                 */
/* ------------------------------------------------------------------ */

/// `json_pack()` forwards to `json_vpack_ex(NULL, 0, fmt, ap)`.
unsafe extern "C" fn pack_va(fmt: *const c_char, ap: VaList) -> *mut json_t {
    unsafe { crate::pack_unpack::json_vpack_ex(null_mut(), 0, fmt, ap) }
}

/// `json_unpack()` forwards to `json_vunpack_ex(root, NULL, 0, fmt, ap)`.
unsafe extern "C" fn unpack_va(root: *mut json_t, fmt: *const c_char, ap: VaList) -> c_int {
    unsafe { crate::pack_unpack::json_vunpack_ex(root, null_mut(), 0, fmt, ap) }
}

/* ------------------------------------------------------------------ */
/* json_pack(const char *fmt, ...)                                    */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn json_pack(_fmt: *const c_char) -> *mut json_t {
    core::arch::naked_asm!(
        "subq $216, %rsp",
        "movq %rsi, 40(%rsp)",
        "movq %rdx, 48(%rsp)",
        "movq %rcx, 56(%rsp)",
        "movq %r8, 64(%rsp)",
        "movq %r9, 72(%rsp)",
        "testb %al, %al",
        "je 10f",
        "movaps %xmm0, 80(%rsp)",
        "movaps %xmm1, 96(%rsp)",
        "movaps %xmm2, 112(%rsp)",
        "movaps %xmm3, 128(%rsp)",
        "movaps %xmm4, 144(%rsp)",
        "movaps %xmm5, 160(%rsp)",
        "movaps %xmm6, 176(%rsp)",
        "movaps %xmm7, 192(%rsp)",
        "10:",
        "movl $8, 8(%rsp)",
        "movl $48, 12(%rsp)",
        "leaq 224(%rsp), %rax",
        "movq %rax, 16(%rsp)",
        "leaq 32(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 8(%rsp), %rsi",
        "call {target}",
        "addq $216, %rsp",
        "ret",
        target = sym pack_va,
        options(att_syntax)
    )
}

/* ------------------------------------------------------------------ */
/* json_pack_ex(json_error_t *error, size_t flags, const char *fmt, ...) */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn json_pack_ex(
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> *mut json_t {
    core::arch::naked_asm!(
        "subq $216, %rsp",
        "movq %rcx, 56(%rsp)",
        "movq %r8, 64(%rsp)",
        "movq %r9, 72(%rsp)",
        "testb %al, %al",
        "je 11f",
        "movaps %xmm0, 80(%rsp)",
        "movaps %xmm1, 96(%rsp)",
        "movaps %xmm2, 112(%rsp)",
        "movaps %xmm3, 128(%rsp)",
        "movaps %xmm4, 144(%rsp)",
        "movaps %xmm5, 160(%rsp)",
        "movaps %xmm6, 176(%rsp)",
        "movaps %xmm7, 192(%rsp)",
        "11:",
        "movl $24, 8(%rsp)",
        "movl $48, 12(%rsp)",
        "leaq 224(%rsp), %rax",
        "movq %rax, 16(%rsp)",
        "leaq 32(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 8(%rsp), %rcx",
        "call {target}",
        "addq $216, %rsp",
        "ret",
        target = sym crate::pack_unpack::json_vpack_ex,
        options(att_syntax)
    )
}

/* ------------------------------------------------------------------ */
/* json_sprintf(const char *fmt, ...)                                 */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn json_sprintf(_fmt: *const c_char) -> *mut json_t {
    core::arch::naked_asm!(
        "subq $216, %rsp",
        "movq %rsi, 40(%rsp)",
        "movq %rdx, 48(%rsp)",
        "movq %rcx, 56(%rsp)",
        "movq %r8, 64(%rsp)",
        "movq %r9, 72(%rsp)",
        "testb %al, %al",
        "je 12f",
        "movaps %xmm0, 80(%rsp)",
        "movaps %xmm1, 96(%rsp)",
        "movaps %xmm2, 112(%rsp)",
        "movaps %xmm3, 128(%rsp)",
        "movaps %xmm4, 144(%rsp)",
        "movaps %xmm5, 160(%rsp)",
        "movaps %xmm6, 176(%rsp)",
        "movaps %xmm7, 192(%rsp)",
        "12:",
        "movl $8, 8(%rsp)",
        "movl $48, 12(%rsp)",
        "leaq 224(%rsp), %rax",
        "movq %rax, 16(%rsp)",
        "leaq 32(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 8(%rsp), %rsi",
        "call {target}",
        "addq $216, %rsp",
        "ret",
        target = sym crate::value::json_vsprintf,
        options(att_syntax)
    )
}

/* ------------------------------------------------------------------ */
/* json_unpack(json_t *root, const char *fmt, ...)                    */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn json_unpack(_root: *mut json_t, _fmt: *const c_char) -> c_int {
    core::arch::naked_asm!(
        "subq $216, %rsp",
        "movq %rdx, 48(%rsp)",
        "movq %rcx, 56(%rsp)",
        "movq %r8, 64(%rsp)",
        "movq %r9, 72(%rsp)",
        "testb %al, %al",
        "je 13f",
        "movaps %xmm0, 80(%rsp)",
        "movaps %xmm1, 96(%rsp)",
        "movaps %xmm2, 112(%rsp)",
        "movaps %xmm3, 128(%rsp)",
        "movaps %xmm4, 144(%rsp)",
        "movaps %xmm5, 160(%rsp)",
        "movaps %xmm6, 176(%rsp)",
        "movaps %xmm7, 192(%rsp)",
        "13:",
        "movl $16, 8(%rsp)",
        "movl $48, 12(%rsp)",
        "leaq 224(%rsp), %rax",
        "movq %rax, 16(%rsp)",
        "leaq 32(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 8(%rsp), %rdx",
        "call {target}",
        "addq $216, %rsp",
        "ret",
        target = sym unpack_va,
        options(att_syntax)
    )
}

/* ------------------------------------------------------------------ */
/* json_unpack_ex(json_t *root, json_error_t *error, size_t flags,     */
/*                const char *fmt, ...)                               */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn json_unpack_ex(
    _root: *mut json_t,
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> c_int {
    core::arch::naked_asm!(
        "subq $216, %rsp",
        "movq %r8, 64(%rsp)",
        "movq %r9, 72(%rsp)",
        "testb %al, %al",
        "je 14f",
        "movaps %xmm0, 80(%rsp)",
        "movaps %xmm1, 96(%rsp)",
        "movaps %xmm2, 112(%rsp)",
        "movaps %xmm3, 128(%rsp)",
        "movaps %xmm4, 144(%rsp)",
        "movaps %xmm5, 160(%rsp)",
        "movaps %xmm6, 176(%rsp)",
        "movaps %xmm7, 192(%rsp)",
        "14:",
        "movl $32, 8(%rsp)",
        "movl $48, 12(%rsp)",
        "leaq 224(%rsp), %rax",
        "movq %rax, 16(%rsp)",
        "leaq 32(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 8(%rsp), %r8",
        "call {target}",
        "addq $216, %rsp",
        "ret",
        target = sym crate::pack_unpack::json_vunpack_ex,
        options(att_syntax)
    )
}

/* ------------------------------------------------------------------ */
/* jsonp_error_set(json_error_t *error, int line, int column,          */
/*                 size_t position, enum json_error_code code,         */
/*                 const char *msg, ...)                              */
/*                                                                    */
/* Six named integer arguments, so all GP registers are consumed and   */
/* the va_list pointer becomes the 7th argument, passed on the stack.  */
/* Frame: 0(%rsp) outgoing stack argument, 16(%rsp) va_list,           */
/*        48(%rsp) register save area, overflow area at 240(%rsp).     */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn jsonp_error_set(
    _error: *mut json_error_t,
    _line: c_int,
    _column: c_int,
    _position: usize,
    _code: c_int,
    _msg: *const c_char,
) {
    core::arch::naked_asm!(
        "subq $232, %rsp",
        "testb %al, %al",
        "je 15f",
        "movaps %xmm0, 96(%rsp)",
        "movaps %xmm1, 112(%rsp)",
        "movaps %xmm2, 128(%rsp)",
        "movaps %xmm3, 144(%rsp)",
        "movaps %xmm4, 160(%rsp)",
        "movaps %xmm5, 176(%rsp)",
        "movaps %xmm6, 192(%rsp)",
        "movaps %xmm7, 208(%rsp)",
        "15:",
        "movl $48, 16(%rsp)",
        "movl $48, 20(%rsp)",
        "leaq 240(%rsp), %rax",
        "movq %rax, 24(%rsp)",
        "leaq 48(%rsp), %rax",
        "movq %rax, 32(%rsp)",
        "leaq 16(%rsp), %rax",
        "movq %rax, (%rsp)",
        "call {target}",
        "addq $232, %rsp",
        "ret",
        target = sym crate::error::jsonp_error_vset,
        options(att_syntax)
    )
}
