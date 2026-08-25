#![cfg(target_arch = "x86_64")]

use std::arch::naked_asm;
use std::ffi::{c_char, c_int};

unsafe extern "C" {
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
}

#[used]
static INTEGER_FORMAT: [u8; 3] = *b"%d\0";
#[used]
static STRING: [u8; 7] = *b"string\0";

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(_line: *const c_char) {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi",
        "cmp qword ptr [rbp - 8], 0",
        "je 2f",
        "mov rdi, qword ptr [rbp - 8]",
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
    naked_asm!(
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
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "lea rax, [rip + {string}]",
        "mov qword ptr [rbp - 8], rax",
        "mov rax, qword ptr [rbp - 8]",
        "mov rdi, rax",
        "call {print_line}",
        "nop",
        "leave",
        "ret",
        string = sym STRING,
        print_line = sym printLine,
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], 0",
        "lea rax, [rbp - 4]",
        "mov rsi, rax",
        "lea rax, [rip + {integer_format}]",
        "mov rdi, rax",
        "mov eax, 0",
        "call {scanf}",
        "mov eax, dword ptr [rbp - 4]",
        "test eax, eax",
        "je 2f",
        "mov eax, 0",
        "call {good}",
        "jmp 3f",
        "2:",
        "mov eax, 0",
        "call {bad}",
        "3:",
        "mov eax, 0",
        "leave",
        "ret",
        integer_format = sym INTEGER_FORMAT,
        scanf = sym scanf,
        good = sym good,
        bad = sym bad,
    );
}
