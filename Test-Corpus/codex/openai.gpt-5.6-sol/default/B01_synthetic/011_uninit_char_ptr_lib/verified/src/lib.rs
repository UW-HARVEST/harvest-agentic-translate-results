use core::arch::naked_asm;
use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, [rbp - 8]",
        "mov rdi, rax",
        "call printLine",
        "nop",
        "leave",
        "ret",
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe {
        printLine(c"string".as_ptr());
    }
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn driver(_use_good: c_int) {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], edi",
        "cmp dword ptr [rbp - 4], 0",
        "je 2f",
        "mov eax, 0",
        "call good",
        "jmp 3f",
        "2:",
        "mov eax, 0",
        "call bad",
        "3:",
        "nop",
        "leave",
        "ret",
    );
}
