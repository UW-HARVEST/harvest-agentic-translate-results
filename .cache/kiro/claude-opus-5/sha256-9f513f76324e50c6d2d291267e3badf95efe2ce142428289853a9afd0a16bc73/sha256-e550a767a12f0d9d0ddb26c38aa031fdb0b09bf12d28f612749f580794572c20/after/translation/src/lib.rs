// Rust translation of c_src/src/driver.c
//
// Original C library: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/src/driver.c for the full notice).
//
// The C library exports exactly four public symbols:
//     printIntPtrLine, bad, good, driver
// All four are reproduced here with the same linker names and signatures.
// driver.h contains no namespace-renaming macros, so the source-level names are
// also the final linker names.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};
#[cfg(not(target_arch = "x86_64"))]
use core::mem::MaybeUninit;

unsafe extern "C" {
    /// Use the C library's own `printf` so that output formatting, stream
    /// buffering and interleaving with any other libc output are byte-identical
    /// to the original.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string literal `"%d\n"` including its NUL terminator, matching the C
/// source exactly.
const FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// C:
///
///     void printIntPtrLine(const int *intNumber)
///     {
///         printf("%d\n", *intNumber);
///     }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        c_printf(FMT_D_NL.as_ptr(), *intNumber);
    }
}

/// C:
///
///     void bad()
///     {
///         int *data;
///         printIntPtrLine(data);
///     }
///
/// This is the intentional defect of the original test case: `data` is an
/// uninitialized automatic pointer that is then dereferenced. The bug is
/// preserved rather than fixed, as required.
///
/// Reproducing it faithfully requires reproducing *where* the uninitialized read
/// lands. Idiomatic Rust (`MaybeUninit` + `read_volatile`) reads a red-zone slot
/// at `entry_rsp - 8`, whereas the C compiler builds a frame and reads
/// `entry_rsp - 16`; that slot holds stale data left by earlier, deeper calls, so
/// the C version dereferences a stale-but-mapped pointer and prints garbage
/// instead of faulting. To match, `bad` is emitted as a naked function carrying
/// the same prologue, load offset, call and epilogue as the C `-O0` codegen.
///
/// The value printed is garbage in both implementations and differs from run to
/// run under ASLR even for two runs of the original C library.
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn bad() {
    // push rbp / mov rbp,rsp / sub rsp,0x10 / mov rax,[rbp-8] / mov rdi,rax
    // / call printIntPtrLine / leave / ret
    core::arch::naked_asm!(
        "push rbp",
        "mov rbp, rsp",
        "sub rsp, 16",
        "mov rax, [rbp - 8]",
        "mov rdi, rax",
        "call {print_int_ptr_line}",
        "leave",
        "ret",
        print_int_ptr_line = sym printIntPtrLine,
    );
}

/// Portable fallback for non-x86_64 targets: read an uninitialized stack slot
/// through a volatile load so the optimizer cannot exploit the `undef` value and
/// delete the call.
#[cfg(not(target_arch = "x86_64"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data_slot: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data: *const c_int = unsafe { core::ptr::read_volatile(data_slot.as_ptr()) };
    unsafe {
        printIntPtrLine(data);
    }
}

/// C:
///
///     void good()
///     {
///         int data;
///         data = 5;
///         int *data_addr;
///         data_addr = &data;
///         printIntPtrLine(data_addr);
///     }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // C declares `int data;` then assigns 5 on the next statement; the observable
    // result is identical to initializing directly.
    let data: c_int = 5;
    let data_addr: *const c_int = &raw const data;
    unsafe {
        printIntPtrLine(data_addr);
    }
}

/// C:
///
///     void driver(int useGood)
///     {
///         if (useGood) { good(); } else { bad(); }
///     }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    unsafe {
        if useGood != 0 {
            good();
        } else {
            bad();
        }
    }
}
