//! Naked-function trampolines that give the exported variadic C entry points a
//! real C variadic ABI (stable Rust cannot define `extern "C"` variadic
//! functions).
//!
//! Each trampoline spills the SysV argument registers into a register save
//! area, synthesizes a `va_list`, and calls the corresponding `*_va`
//! implementation written in Rust, which forwards it to `vsnprintf`.
//!
//! They are written as `#[unsafe(naked)]` Rust items (rather than `global_asm!`)
//! so that rustc lists them in the cdylib's exported symbol set.
#![allow(dead_code)]

use crate::cstd::{c_char, c_void};
use crate::jsi::{js_Ast, js_State};

/// Body shared by all trampolines.
///
/// `$gp` is the initial `gp_offset` of the synthesized `va_list`, i.e.
/// 8 * (number of named integer arguments).
macro_rules! trampoline_body {
    () => {
        concat!(
            ".cfi_def_cfa_offset 16\n",
            ".cfi_offset 6, -16\n",
            "movq %rsp, %rbp\n",
            ".cfi_def_cfa_register 6\n",
            "subq $208, %rsp\n",
            /* SysV register save area at -176(%rbp): 6 GP regs then 8 xmm regs */
            "movq %rdi, -176(%rbp)\n",
            "movq %rsi, -168(%rbp)\n",
            "movq %rdx, -160(%rbp)\n",
            "movq %rcx, -152(%rbp)\n",
            "movq %r8, -144(%rbp)\n",
            "movq %r9, -136(%rbp)\n",
            "testb %al, %al\n",
            "je 9f\n",
            "movaps %xmm0, -128(%rbp)\n",
            "movaps %xmm1, -112(%rbp)\n",
            "movaps %xmm2, -96(%rbp)\n",
            "movaps %xmm3, -80(%rbp)\n",
            "movaps %xmm4, -64(%rbp)\n",
            "movaps %xmm5, -48(%rbp)\n",
            "movaps %xmm6, -32(%rbp)\n",
            "movaps %xmm7, -16(%rbp)\n",
            "9:\n",
        )
    };
}

macro_rules! trampoline_valist {
    ($gp:literal) => {
        concat!(
            /* the va_list itself lives at -208(%rbp) */
            "movl $", $gp, ", -208(%rbp)\n",
            "movl $48, -204(%rbp)\n",
            "leaq 16(%rbp), %rax\n",
            "movq %rax, -200(%rbp)\n",
            "leaq -176(%rbp), %rax\n",
            "movq %rax, -192(%rbp)\n",
        )
    };
}

macro_rules! vararg2 {
    ($name:ident, $target:literal) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $name(_J: *mut js_State, _fmt: *const c_char) -> ! {
            core::arch::naked_asm!(
                concat!(
                    ".cfi_startproc\n",
                    "pushq %rbp\n",
                    trampoline_body!(),
                    trampoline_valist!("16"),
                    "movq -176(%rbp), %rdi\n",
                    "movq -168(%rbp), %rsi\n",
                    "leaq -208(%rbp), %rdx\n",
                    "call ", $target, "@PLT\n",
                    "leave\n",
                    ".cfi_def_cfa 7, 8\n",
                    "ret\n",
                    ".cfi_endproc\n",
                ),
                options(att_syntax)
            )
        }
    };
}

macro_rules! vararg3 {
    ($name:ident, $target:literal) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C-unwind" fn $name(
            _J: *mut js_State,
            _node: *mut js_Ast,
            _fmt: *const c_char,
        ) -> ! {
            core::arch::naked_asm!(
                concat!(
                    ".cfi_startproc\n",
                    "pushq %rbp\n",
                    trampoline_body!(),
                    trampoline_valist!("24"),
                    "movq -176(%rbp), %rdi\n",
                    "movq -168(%rbp), %rsi\n",
                    "movq -160(%rbp), %rdx\n",
                    "leaq -208(%rbp), %rcx\n",
                    "call ", $target, "@PLT\n",
                    "leave\n",
                    ".cfi_def_cfa 7, 8\n",
                    "ret\n",
                    ".cfi_endproc\n",
                ),
                options(att_syntax)
            )
        }
    };
}

vararg2!(js_error, "js_error_va");
vararg2!(js_evalerror, "js_evalerror_va");
vararg2!(js_rangeerror, "js_rangeerror_va");
vararg2!(js_referenceerror, "js_referenceerror_va");
vararg2!(js_syntaxerror, "js_syntaxerror_va");
vararg2!(js_typeerror, "js_typeerror_va");
vararg2!(js_urierror, "js_urierror_va");
vararg3!(jsC_error, "jsC_error_va");

/// Silence the "unused" warning for the `c_void` import.
const _: Option<*mut c_void> = None;
