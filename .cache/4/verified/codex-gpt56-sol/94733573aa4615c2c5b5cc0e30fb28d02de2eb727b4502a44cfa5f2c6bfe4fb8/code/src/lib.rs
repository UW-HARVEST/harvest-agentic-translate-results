use std::arch::naked_asm;
use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static INT_LINE_FORMAT: [u8; 4] = *b"%d\n\0";

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn printIntPtrLine(_int_number: *const c_int) {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi",
        "mov rax, qword ptr [rbp - 8]",
        "mov eax, dword ptr [rax]",
        "mov esi, eax",
        "lea rdi, [rip + {format}]",
        "xor eax, eax",
        "call {printf}",
        "nop",
        "leave",
        "ret",
        format = sym INT_LINE_FORMAT,
        printf = sym printf,
    );
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn good() {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 12], 5",
        "lea rax, [rbp - 12]",
        "mov qword ptr [rbp - 8], rax",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
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
        "xor eax, eax",
        "call {good}",
        "jmp 3f",
        "2:",
        "xor eax, eax",
        "call {bad}",
        "3:",
        "nop",
        "leave",
        "ret",
        good = sym good,
        bad = sym bad,
    );
}
