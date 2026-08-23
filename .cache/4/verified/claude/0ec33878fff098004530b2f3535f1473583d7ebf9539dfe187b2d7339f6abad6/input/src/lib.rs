//! mujs 1.3.8 -- a direct Rust transliteration of the C library in c_src/.
//!
//! The crate is a cdylib that exports the exact same public ABI as the C build.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(unused_imports)]
#![allow(unused_labels)]

use std::arch::naked_asm;

pub mod jsi;

use crate::jsi::*;

/* ================================================================ *
 * Variadic entry points.
 *
 * The public API exposes printf-style variadic functions.  Stable Rust
 * cannot define C variadic functions, so each one is a naked assembly
 * trampoline that builds an x86-64 SysV va_list and forwards to a normal
 * Rust function.
 * ================================================================ */

macro_rules! va_trampoline {
    ($name:ident => $target:path, gp = $gp:expr, nargs = 2) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() {
            naked_asm!(
                "push %rbp",
                "mov %rsp, %rbp",
                "sub $208, %rsp",
                "movq %rdi, 0(%rsp)",
                "movq %rsi, 8(%rsp)",
                "movq %rdx, 16(%rsp)",
                "movq %rcx, 24(%rsp)",
                "movq %r8, 32(%rsp)",
                "movq %r9, 40(%rsp)",
                "testb %al, %al",
                "je 91f",
                "movaps %xmm0, 48(%rsp)",
                "movaps %xmm1, 64(%rsp)",
                "movaps %xmm2, 80(%rsp)",
                "movaps %xmm3, 96(%rsp)",
                "movaps %xmm4, 112(%rsp)",
                "movaps %xmm5, 128(%rsp)",
                "movaps %xmm6, 144(%rsp)",
                "movaps %xmm7, 160(%rsp)",
                "91:",
                concat!("movl $", $gp, ", 176(%rsp)"),
                "movl $48, 180(%rsp)",
                "leaq 16(%rbp), %rax",
                "movq %rax, 184(%rsp)",
                "movq %rsp, %rax",
                "movq %rax, 192(%rsp)",
                "movq 0(%rsp), %rdi",
                "movq 8(%rsp), %rsi",
                "leaq 176(%rsp), %rdx",
                "xorl %eax, %eax",
                "call {target}",
                "movq %rbp, %rsp",
                "pop %rbp",
                "ret",
                target = sym $target,
                options(att_syntax)
            )
        }
    };
    ($name:ident => $target:path, gp = $gp:expr, nargs = 3) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name() {
            naked_asm!(
                "push %rbp",
                "mov %rsp, %rbp",
                "sub $208, %rsp",
                "movq %rdi, 0(%rsp)",
                "movq %rsi, 8(%rsp)",
                "movq %rdx, 16(%rsp)",
                "movq %rcx, 24(%rsp)",
                "movq %r8, 32(%rsp)",
                "movq %r9, 40(%rsp)",
                "testb %al, %al",
                "je 91f",
                "movaps %xmm0, 48(%rsp)",
                "movaps %xmm1, 64(%rsp)",
                "movaps %xmm2, 80(%rsp)",
                "movaps %xmm3, 96(%rsp)",
                "movaps %xmm4, 112(%rsp)",
                "movaps %xmm5, 128(%rsp)",
                "movaps %xmm6, 144(%rsp)",
                "movaps %xmm7, 160(%rsp)",
                "91:",
                concat!("movl $", $gp, ", 176(%rsp)"),
                "movl $48, 180(%rsp)",
                "leaq 16(%rbp), %rax",
                "movq %rax, 184(%rsp)",
                "movq %rsp, %rax",
                "movq %rax, 192(%rsp)",
                "movq 0(%rsp), %rdi",
                "movq 8(%rsp), %rsi",
                "movq 16(%rsp), %rdx",
                "leaq 176(%rsp), %rcx",
                "xorl %eax, %eax",
                "call {target}",
                "movq %rbp, %rsp",
                "pop %rbp",
                "ret",
                target = sym $target,
                options(att_syntax)
            )
        }
    };
}

va_trampoline!(js_error => crate::jserror::js_error_va, gp = "16", nargs = 2);
va_trampoline!(js_evalerror => crate::jserror::js_evalerror_va, gp = "16", nargs = 2);
va_trampoline!(js_rangeerror => crate::jserror::js_rangeerror_va, gp = "16", nargs = 2);
va_trampoline!(js_referenceerror => crate::jserror::js_referenceerror_va, gp = "16", nargs = 2);
va_trampoline!(js_syntaxerror => crate::jserror::js_syntaxerror_va, gp = "16", nargs = 2);
va_trampoline!(js_typeerror => crate::jserror::js_typeerror_va, gp = "16", nargs = 2);
va_trampoline!(js_urierror => crate::jserror::js_urierror_va, gp = "16", nargs = 2);
va_trampoline!(jsC_error => crate::jscompile::jsC_error_va, gp = "24", nargs = 3);

/* ================================================================ *
 * Formatting macros.
 *
 * C code calls e.g. `js_typeerror(J, "'%s' is read-only", name)`.  The Rust
 * translation calls `js_typeerror!(J, c"'%s' is read-only".as_ptr(), name)`
 * which formats into a 256 byte buffer exactly like the C implementation
 * (vsnprintf into `char buf[256]`) and then throws.
 * ================================================================ */

macro_rules! jsfmt256 {
    ($buf:ident, $($a:expr),*) => {
        let mut $buf: [c_char; 256] = [0; 256];
        crate::jsi::snprintf($buf.as_mut_ptr(), 256, $($a),*);
    };
}

macro_rules! js_error {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_error_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_evalerror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_evalerror_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_rangeerror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_rangeerror_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_referenceerror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_referenceerror_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_syntaxerror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_syntaxerror_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_typeerror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_typeerror_str($J, buf__.as_ptr())
    }};
}
macro_rules! js_urierror {
    ($J:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jserror::js_urierror_str($J, buf__.as_ptr())
    }};
}
macro_rules! jsC_error {
    ($J:expr, $node:expr, $($a:expr),*) => {{
        jsfmt256!(buf__, $($a),*);
        crate::jscompile::jsC_error_str($J, $node, buf__.as_ptr())
    }};
}


/// `setjmp(js_savetry(J))`
macro_rules! js_try {
    ($J:expr) => {
        crate::jsi::_setjmp(crate::jsrun::js_savetry($J) as *mut crate::jsi::jmp_buf) != 0
    };
}

/// `setjmp(js_savetrypc(J, PC))`
macro_rules! js_trypc {
    ($J:expr, $pc:expr) => {
        crate::jsi::_setjmp(crate::jsrun::js_savetrypc($J, $pc) as *mut crate::jsi::jmp_buf) != 0
    };
}

/* ================================================================ *
 * Modules -- one per C source file.
 * ================================================================ */

pub mod jsarray;
pub mod jsboolean;
pub mod jsbuiltin;
pub mod jscompile;
pub mod jsdate;
pub mod jsdtoa;
pub mod jserror;
pub mod jsfunction;
pub mod jsgc;
pub mod jsintern;
pub mod jslex;
pub mod jsmath;
pub mod jsnumber;
pub mod jsobject;
pub mod json;
pub mod jsparse;
pub mod jsproperty;
pub mod jsregexp;
pub mod jsrepr;
pub mod jsrun;
pub mod jsstate;
pub mod jsstring;
pub mod jsvalue;
pub mod regexp;
pub mod utf;
pub mod utfdata;

/// Everything that is a non-static function in the C sources, flattened into
/// one namespace so that translated modules can call each other exactly the
/// way the C code does.
pub mod prelude {
    pub use crate::jsarray::*;
    pub use crate::jsboolean::*;
    pub use crate::jsbuiltin::*;
    pub use crate::jscompile::*;
    pub use crate::jsdate::*;
    pub use crate::jsdtoa::*;
    pub use crate::jserror::*;
    pub use crate::jsfunction::*;
    pub use crate::jsgc::*;
    pub use crate::jsintern::*;
    pub use crate::jslex::*;
    pub use crate::jsmath::*;
    pub use crate::jsnumber::*;
    pub use crate::jsobject::*;
    pub use crate::json::*;
    pub use crate::jsparse::*;
    pub use crate::jsproperty::*;
    pub use crate::jsregexp::*;
    pub use crate::jsrepr::*;
    pub use crate::jsrun::*;
    pub use crate::jsstate::*;
    pub use crate::jsstring::*;
    pub use crate::jsvalue::*;
    pub use crate::regexp::*;
    pub use crate::utf::*;
    pub use crate::utfdata::*;
}
