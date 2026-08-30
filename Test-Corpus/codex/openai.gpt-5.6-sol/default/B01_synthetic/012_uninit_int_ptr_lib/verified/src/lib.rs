use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(int_number: *const c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), *int_number);
    }
}

#[cfg(not(target_arch = "x86_64"))]
compile_error!("the C bad path requires the x86_64 stack-frame translation");

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call printIntPtrLine",
        "nop",
        "leave",
        "ret",
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data: c_int = 5;
    unsafe {
        printIntPtrLine(&data);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(_use_good: c_int) {
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], edi",
        "cmp dword ptr [rbp - 4], 0",
        "je 2f",
        "xor eax, eax",
        "call good",
        "jmp 3f",
        "2:",
        "xor eax, eax",
        "call bad",
        "3:",
        "nop",
        "leave",
        "ret",
    );
}
