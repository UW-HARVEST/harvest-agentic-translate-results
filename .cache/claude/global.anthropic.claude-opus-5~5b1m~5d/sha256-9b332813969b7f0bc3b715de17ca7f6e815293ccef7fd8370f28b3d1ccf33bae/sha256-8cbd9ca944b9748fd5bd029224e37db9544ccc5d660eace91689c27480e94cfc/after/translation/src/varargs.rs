//! Variadic entry points.
//!
//! The C library exposes a handful of variadic functions that simply build a
//! `va_list` and delegate to a `v*` counterpart. Rust cannot define C variadic
//! functions on stable, so each one is provided as a naked function containing
//! the standard x86-64 System V variadic prologue followed by a tail call into
//! the `v*` implementation.
//!
//! Frame layout (after `push rbp; mov rbp, rsp; sub rsp, 208`):
//!
//! ```text
//!   [rbp-208] .. [rbp-185]   va_list (gp_offset, fp_offset, overflow, reg_save)
//!   [rbp-176] .. [rbp-129]   general purpose register save area (rdi..r9)
//!   [rbp-128] .. [rbp-1]     xmm0..xmm7 register save area
//!   [rbp+16]                 first stack (overflow) argument
//! ```

use core::arch::naked_asm;

/// The register spill part of the variadic prologue, shared by every shim.
macro_rules! prologue {
    ($gp_offset:literal) => {
        concat!(
            "push rbp\n",
            "mov rbp, rsp\n",
            "sub rsp, 208\n",
            "mov qword ptr [rbp-176], rdi\n",
            "mov qword ptr [rbp-168], rsi\n",
            "mov qword ptr [rbp-160], rdx\n",
            "mov qword ptr [rbp-152], rcx\n",
            "mov qword ptr [rbp-144], r8\n",
            "mov qword ptr [rbp-136], r9\n",
            "test al, al\n",
            "je 2f\n",
            "movaps xmmword ptr [rbp-128], xmm0\n",
            "movaps xmmword ptr [rbp-112], xmm1\n",
            "movaps xmmword ptr [rbp-96], xmm2\n",
            "movaps xmmword ptr [rbp-80], xmm3\n",
            "movaps xmmword ptr [rbp-64], xmm4\n",
            "movaps xmmword ptr [rbp-48], xmm5\n",
            "movaps xmmword ptr [rbp-32], xmm6\n",
            "movaps xmmword ptr [rbp-16], xmm7\n",
            "2:\n",
            "mov dword ptr [rbp-208], ",
            stringify!($gp_offset),
            "\n",
            "mov dword ptr [rbp-204], 48\n",
            "lea rax, [rbp+16]\n",
            "mov qword ptr [rbp-200], rax\n",
            "lea rax, [rbp-176]\n",
            "mov qword ptr [rbp-192], rax\n",
        )
    };
}

macro_rules! epilogue {
    () => {
        concat!("mov rsp, rbp\n", "pop rbp\n", "ret\n")
    };
}

/* json_pack(fmt, ...) -> json_vpack_ex(NULL, 0, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack() -> *mut crate::jansson::json_t {
    naked_asm!(
        prologue!(8),
        "lea rcx, [rbp-208]",
        "mov rdx, rdi",
        "xor esi, esi",
        "xor edi, edi",
        "call {target}",
        epilogue!(),
        target = sym crate::pack_unpack::json_vpack_ex,
    )
}

/* json_pack_ex(error, flags, fmt, ...) -> json_vpack_ex(error, flags, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack_ex() -> *mut crate::jansson::json_t {
    naked_asm!(
        prologue!(24),
        "lea rcx, [rbp-208]",
        "call {target}",
        epilogue!(),
        target = sym crate::pack_unpack::json_vpack_ex,
    )
}

/* json_unpack(root, fmt, ...) -> json_vunpack_ex(root, NULL, 0, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack() -> core::ffi::c_int {
    naked_asm!(
        prologue!(16),
        "lea r8, [rbp-208]",
        "mov rcx, rsi",
        "xor esi, esi",
        "xor edx, edx",
        "call {target}",
        epilogue!(),
        target = sym crate::pack_unpack::json_vunpack_ex,
    )
}

/* json_unpack_ex(root, error, flags, fmt, ...)
   -> json_vunpack_ex(root, error, flags, fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack_ex() -> core::ffi::c_int {
    naked_asm!(
        prologue!(32),
        "lea r8, [rbp-208]",
        "call {target}",
        epilogue!(),
        target = sym crate::pack_unpack::json_vunpack_ex,
    )
}

/* json_sprintf(fmt, ...) -> json_vsprintf(fmt, ap) */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_sprintf() -> *mut crate::jansson::json_t {
    naked_asm!(
        prologue!(8),
        "lea rsi, [rbp-208]",
        "call {target}",
        epilogue!(),
        target = sym crate::value::json_vsprintf,
    )
}

/* jsonp_error_set(error, line, column, position, code, msg, ...)
   -> jsonp_error_vset(error, line, column, position, code, msg, ap)

   The va_list becomes the seventh integer argument, so it is passed on the
   stack. */
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set() {
    naked_asm!(
        prologue!(48),
        "lea rax, [rbp-208]",
        "sub rsp, 8",
        "push rax",
        "call {target}",
        epilogue!(),
        target = sym crate::error::jsonp_error_vset,
    )
}
