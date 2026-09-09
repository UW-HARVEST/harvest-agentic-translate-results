//! Naked trampolines for the C-variadic public entry points.
//!
//! Defining a `extern "C"` variadic function is not stable Rust, so each
//! `js_xxxerror(js_State*, const char *fmt, ...)` export is a naked function
//! that materialises a SysV x86-64 `va_list` on its own stack frame and
//! forwards to a normal Rust implementation.
#![allow(non_snake_case)]

use crate::jsi::*;

unsafe extern "C" {
    pub fn vsnprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ap: *mut c_void) -> c_int;
}

/// Build a `va_list` for a function with 2 named arguments and pass it as the
/// 3rd argument (rdx) of `$imp`.
macro_rules! variadic_2 {
    ($name:ident, $imp:path) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $name(J: *mut js_State, fmt: *const c_char) {
            core::arch::naked_asm!(
                ".cfi_startproc",
                "push rbp",
                ".cfi_def_cfa_offset 16",
                ".cfi_offset rbp, -16",
                "mov rbp, rsp",
                ".cfi_def_cfa_register rbp",
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
                "mov dword ptr [rbp-208], 16",
                "mov dword ptr [rbp-204], 48",
                "lea rax, [rbp+16]",
                "mov [rbp-200], rax",
                "lea rax, [rbp-176]",
                "mov [rbp-192], rax",
                "lea rdx, [rbp-208]",
                "call {f}",
                "leave",
                ".cfi_def_cfa rsp, 8",
                "ret",
                ".cfi_endproc",
                f = sym $imp,
            );
        }
    };
}

/// Build a `va_list` for a function with 3 named arguments and pass it as the
/// 4th argument (rcx) of `$imp`.
macro_rules! variadic_3 {
    ($name:ident, $imp:path) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $name(J: *mut js_State, node: *mut js_Ast, fmt: *const c_char) {
            core::arch::naked_asm!(
                ".cfi_startproc",
                "push rbp",
                ".cfi_def_cfa_offset 16",
                ".cfi_offset rbp, -16",
                "mov rbp, rsp",
                ".cfi_def_cfa_register rbp",
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
                "mov dword ptr [rbp-208], 24",
                "mov dword ptr [rbp-204], 48",
                "lea rax, [rbp+16]",
                "mov [rbp-200], rax",
                "lea rax, [rbp-176]",
                "mov [rbp-192], rax",
                "lea rcx, [rbp-208]",
                "call {f}",
                "leave",
                ".cfi_def_cfa rsp, 8",
                "ret",
                ".cfi_endproc",
                f = sym $imp,
            );
        }
    };
}

variadic_2!(js_error, crate::jserror::js_error_v);
variadic_2!(js_evalerror, crate::jserror::js_evalerror_v);
variadic_2!(js_rangeerror, crate::jserror::js_rangeerror_v);
variadic_2!(js_referenceerror, crate::jserror::js_referenceerror_v);
variadic_2!(js_syntaxerror, crate::jserror::js_syntaxerror_v);
variadic_2!(js_typeerror, crate::jserror::js_typeerror_v);
variadic_2!(js_urierror, crate::jserror::js_urierror_v);
variadic_3!(jsC_error, crate::jscompile::jsC_error_v);
