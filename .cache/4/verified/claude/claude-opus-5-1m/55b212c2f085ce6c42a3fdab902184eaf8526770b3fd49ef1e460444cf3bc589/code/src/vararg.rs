//! C variadic entry points.
//!
//! Defining `extern "C"` variadic functions is still unstable in Rust, so the
//! six variadic symbols exported by libjansson are implemented as naked
//! functions containing hand written x86-64 System V trampolines.  Each one
//! materialises a `va_list` (`__va_list_tag`) on the stack exactly the way
//! `va_start` does and forwards to the corresponding `v*` function implemented
//! in Rust.
//!
//! Stack frame used by every trampoline (`rbp` is 16-byte aligned on entry):
//!
//! ```text
//!   rbp+16 .. : incoming stack arguments  (overflow_arg_area)
//!   rbp-176   : register save area, 6 GP quadwords then 8 XMM oct-words
//!   rbp-208   : the __va_list_tag itself
//! ```
//!
//! `gp_offset` is initialised to `8 * <number of fixed integer arguments>` and
//! `fp_offset` to 48 (no function below takes a fixed floating point
//! argument), matching what GCC's `va_start` produces.

use crate::types::{json_error_t, json_t};
use core::arch::naked_asm;
use core::ffi::{c_char, c_int};

/* --------------------------------------------------------- jsonp_error_set */

/// `void jsonp_error_set(json_error_t *, int, int, size_t, enum, const char *, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set(
    _error: *mut json_error_t,
    _line: c_int,
    _column: c_int,
    _position: usize,
    _code: c_int,
    _msg: *const c_char,
) {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $240, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $48, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "leaq -208(%rbp), %rax",
        "movq %rax, -240(%rbp)",
        "xorl %eax, %eax",
        "call jsonp_error_vset",
        "leave",
        "ret",
        options(att_syntax)
    )
}

/* ---------------------------------------------------------------- json_pack */

/// `json_t *json_pack(const char *fmt, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack(_fmt: *const c_char) -> *mut json_t {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $224, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $8, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "movq -176(%rbp), %rdx",
        "xorl %edi, %edi",
        "xorl %esi, %esi",
        "leaq -208(%rbp), %rcx",
        "xorl %eax, %eax",
        "call json_vpack_ex",
        "leave",
        "ret",
        options(att_syntax)
    )
}

/* ------------------------------------------------------------- json_pack_ex */

/// `json_t *json_pack_ex(json_error_t *error, size_t flags, const char *fmt, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack_ex(
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> *mut json_t {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $224, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $24, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "leaq -208(%rbp), %rcx",
        "xorl %eax, %eax",
        "call json_vpack_ex",
        "leave",
        "ret",
        options(att_syntax)
    )
}

/* -------------------------------------------------------------- json_unpack */

/// `int json_unpack(json_t *root, const char *fmt, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack(_root: *mut json_t, _fmt: *const c_char) -> c_int {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $224, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $16, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "movq -176(%rbp), %rdi",
        "movq -168(%rbp), %rcx",
        "xorl %esi, %esi",
        "xorl %edx, %edx",
        "leaq -208(%rbp), %r8",
        "xorl %eax, %eax",
        "call json_vunpack_ex",
        "leave",
        "ret",
        options(att_syntax)
    )
}

/* ----------------------------------------------------------- json_unpack_ex */

/// `int json_unpack_ex(json_t *root, json_error_t *error, size_t flags, const char *fmt, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack_ex(
    _root: *mut json_t,
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> c_int {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $224, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $32, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "leaq -208(%rbp), %r8",
        "xorl %eax, %eax",
        "call json_vunpack_ex",
        "leave",
        "ret",
        options(att_syntax)
    )
}

/* ------------------------------------------------------------ json_sprintf */

/// `json_t *json_sprintf(const char *fmt, ...)`
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_sprintf(_fmt: *const c_char) -> *mut json_t {
    naked_asm!(
        "pushq %rbp",
        "movq %rsp, %rbp",
        "subq $224, %rsp",
        "movq %rdi, -176(%rbp)",
        "movq %rsi, -168(%rbp)",
        "movq %rdx, -160(%rbp)",
        "movq %rcx, -152(%rbp)",
        "movq %r8, -144(%rbp)",
        "movq %r9, -136(%rbp)",
        "testb %al, %al",
        "je 1f",
        "movaps %xmm0, -128(%rbp)",
        "movaps %xmm1, -112(%rbp)",
        "movaps %xmm2, -96(%rbp)",
        "movaps %xmm3, -80(%rbp)",
        "movaps %xmm4, -64(%rbp)",
        "movaps %xmm5, -48(%rbp)",
        "movaps %xmm6, -32(%rbp)",
        "movaps %xmm7, -16(%rbp)",
        "1:",
        "movl $8, -208(%rbp)",
        "movl $48, -204(%rbp)",
        "leaq 16(%rbp), %rax",
        "movq %rax, -200(%rbp)",
        "leaq -176(%rbp), %rax",
        "movq %rax, -192(%rbp)",
        "movq -176(%rbp), %rdi",
        "leaq -208(%rbp), %rsi",
        "xorl %eax, %eax",
        "call json_vsprintf",
        "leave",
        "ret",
        options(att_syntax)
    )
}
