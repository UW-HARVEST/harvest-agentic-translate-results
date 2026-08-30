use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

static GOOD_STRING: [u8; 7] = *b"string\0";

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(_line: *const c_char) {
    std::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi",
        "cmp qword ptr [rbp - 8], 0",
        "je 2f",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {puts}",
        "2:",
        "nop",
        "leave",
        "ret",
        puts = sym puts,
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    std::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        print_line = sym printLine,
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    std::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "lea rax, [rip + {good_string}]",
        "mov qword ptr [rbp - 8], rax",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        good_string = sym GOOD_STRING,
        print_line = sym printLine,
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(_use_good: c_int) {
    std::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], edi",
        "cmp dword ptr [rbp - 4], 0",
        "je 2f",
        "mov eax, 0",
        "call {good}",
        "jmp 3f",
        "2:",
        "mov eax, 0",
        "call {bad}",
        "3:",
        "nop",
        "leave",
        "ret",
        bad = sym bad,
        good = sym good,
    );
}
