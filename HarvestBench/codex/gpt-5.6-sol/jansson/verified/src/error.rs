use crate::types::json_error_t;
use std::arch::{global_asm, naked_asm};
use std::ffi::{CStr, c_char, c_int};
use std::ptr;

pub const JSON_ERROR_UNKNOWN: u8 = 0;
pub const JSON_ERROR_OUT_OF_MEMORY: u8 = 1;
pub const JSON_ERROR_CANNOT_OPEN_FILE: u8 = 3;
pub const JSON_ERROR_INVALID_ARGUMENT: u8 = 4;
pub const JSON_ERROR_INVALID_UTF8: u8 = 5;
pub const JSON_ERROR_PREMATURE_END: u8 = 6;
pub const JSON_ERROR_END_EXPECTED: u8 = 7;
pub const JSON_ERROR_INVALID_SYNTAX: u8 = 8;
pub const JSON_ERROR_NULL_CHARACTER: u8 = 11;
pub const JSON_ERROR_NULL_BYTE_IN_KEY: u8 = 13;
pub const JSON_ERROR_DUPLICATE_KEY: u8 = 14;
pub const JSON_ERROR_NUMERIC_OVERFLOW: u8 = 15;

#[repr(C)]
pub struct VaList {
    pub gp_offset: u32,
    pub fp_offset: u32,
    pub overflow_arg_area: *mut u8,
    pub reg_save_area: *mut u8,
}

unsafe extern "C" {
    fn vsnprintf(buffer: *mut c_char, size: usize, format: *const c_char, ap: *mut VaList)
    -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_init(error: *mut json_error_t, source: *const c_char) {
    let Some(error) = error.as_mut() else {
        return;
    };
    error.text[0] = 0;
    error.line = -1;
    error.column = -1;
    error.position = 0;
    if source.is_null() {
        error.source[0] = 0;
    } else {
        jsonp_error_set_source(error, source);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set_source(error: *mut json_error_t, source: *const c_char) {
    if error.is_null() || source.is_null() {
        return;
    }
    let bytes = CStr::from_ptr(source).to_bytes();
    let destination = &mut (*error).source;
    if bytes.len() < destination.len() {
        ptr::copy_nonoverlapping(source, destination.as_mut_ptr(), bytes.len() + 1);
    } else {
        destination[0] = b'.' as c_char;
        destination[1] = b'.' as c_char;
        destination[2] = b'.' as c_char;
        let tail = &bytes[bytes.len() - (destination.len() - 4)..];
        ptr::copy_nonoverlapping(
            tail.as_ptr().cast(),
            destination.as_mut_ptr().add(3),
            tail.len(),
        );
        destination[destination.len() - 1] = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_vset(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: c_int,
    message: *const c_char,
    ap: *mut VaList,
) {
    if error.is_null() || (*error).text[0] != 0 {
        return;
    }
    (*error).line = line;
    (*error).column = column;
    (*error).position = position as c_int;
    vsnprintf((*error).text.as_mut_ptr(), 159, message, ap);
    (*error).text[158] = 0;
    (*error).text[159] = code as c_char;
}

global_asm!(
    r#"
    .text
    .type __jsonp_error_set_asm,@function
__jsonp_error_set_asm:
    subq $232, %rsp
    movl $48, 0(%rsp)
    movl $48, 4(%rsp)
    leaq 240(%rsp), %rax
    movq %rax, 8(%rsp)
    leaq 32(%rsp), %rax
    movq %rax, 16(%rsp)
    movq %rdi, 32(%rsp)
    movq %rsi, 40(%rsp)
    movq %rdx, 48(%rsp)
    movq %rcx, 56(%rsp)
    movq %r8, 64(%rsp)
    movq %r9, 72(%rsp)
    movaps %xmm0, 80(%rsp)
    movaps %xmm1, 96(%rsp)
    movaps %xmm2, 112(%rsp)
    movaps %xmm3, 128(%rsp)
    movaps %xmm4, 144(%rsp)
    movaps %xmm5, 160(%rsp)
    movaps %xmm6, 176(%rsp)
    movaps %xmm7, 192(%rsp)
    movq %rsp, %rax
    subq $16, %rsp
    movq %rax, 0(%rsp)
    movq 48(%rsp), %rdi
    movq 56(%rsp), %rsi
    movq 64(%rsp), %rdx
    movq 72(%rsp), %rcx
    movq 80(%rsp), %r8
    movq 88(%rsp), %r9
    call jsonp_error_vset
    addq $248, %rsp
    ret
    .size __jsonp_error_set_asm, .-__jsonp_error_set_asm
"#,
    options(att_syntax)
);

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_error_set(
    _error: *mut json_error_t,
    _line: c_int,
    _column: c_int,
    _position: usize,
    _code: c_int,
    _message: *const c_char,
) {
    naked_asm!("jmp __jsonp_error_set_asm");
}

pub unsafe fn set_error(
    error: *mut json_error_t,
    line: c_int,
    column: c_int,
    position: usize,
    code: u8,
    message: &str,
) {
    if error.is_null() || (*error).text[0] != 0 {
        return;
    }
    (*error).line = line;
    (*error).column = column;
    (*error).position = position as c_int;
    let bytes = message.as_bytes();
    let length = bytes.len().min(158);
    ptr::copy_nonoverlapping(bytes.as_ptr(), (*error).text.as_mut_ptr().cast(), length);
    (*error).text[length] = 0;
    (*error).text[159] = code as c_char;
}
