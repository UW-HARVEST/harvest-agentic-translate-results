#![cfg_attr(panic = "abort", no_std)]

use core::arch::naked_asm;
use core::ffi::{c_char, c_int};
#[cfg(panic = "abort")]
use core::panic::PanicInfo;

static PRINT_FORMAT: [u8; 4] = *b"%d\n\0";
static SCAN_FORMAT: [u8; 3] = *b"%d\0";

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    #[link_name = "__isoc99_scanf"]
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[cfg(panic = "abort")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(_int_number: *const c_int) {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov qword ptr [rbp - 8], rdi",
        "mov rax, qword ptr [rbp - 8]",
        "mov eax, dword ptr [rax]",
        "mov esi, eax",
        "lea rdi, [rip + {print_format}]",
        "xor eax, eax",
        "call {printf}",
        "nop",
        "leave",
        "ret",
        print_format = sym PRINT_FORMAT,
        printf = sym printf,
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
        "call {print_int_ptr_line}",
        "nop",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
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

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov dword ptr [rbp - 4], 0",
        "lea rsi, [rbp - 4]",
        "lea rdi, [rip + {scan_format}]",
        "xor eax, eax",
        "call {scanf}",
        "mov eax, dword ptr [rbp - 4]",
        "test eax, eax",
        "je 2f",
        "xor eax, eax",
        "call {good}",
        "jmp 3f",
        "2:",
        "xor eax, eax",
        "call {bad}",
        "3:",
        "xor eax, eax",
        "leave",
        "ret",
        scan_format = sym SCAN_FORMAT,
        scanf = sym scanf,
        good = sym good,
        bad = sym bad,
    );
}
