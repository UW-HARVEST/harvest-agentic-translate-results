// Rust translation of c_src/src/driver.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};
use core::mem::MaybeUninit;

// Link against the platform C library's printf so that stdout buffering,
// formatting and flushing behaviour is byte-for-byte identical to the C
// original (including interleaving with any other libc stdio output).
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `"%d\n\0"` — the exact format string used by the C implementation.
const FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// Load the `int` at `p` with exactly C's `*p` semantics: a single 4-byte load
/// from that address, with NO validity, null or alignment check of any kind.
///
/// A plain Rust `*p` cannot be used here. `rustc` emits UB-checks for null and
/// misaligned dereferences whenever `debug-assertions` are on, and those checks
/// `panic!` -- which, inside an `extern "C"` function, is converted into a
/// process `abort()`. That makes the Rust library die with `SIGABRT` where the C
/// either succeeds (misaligned reads are legal on x86-64) or dies with
/// `SIGSEGV`, i.e. it diverges from the C on ERRORS.md rows 1-4. `read_volatile`
/// and `read_unaligned` carry the same preconditions, so they do not help.
///
/// The load is therefore issued as inline assembly, which is invisible to those
/// checks and behaves identically in every build profile: valid address -> the
/// 4 bytes at that address (little-endian, unaligned permitted); invalid
/// address -> the hardware raises SIGSEGV exactly as it does for the C.
#[inline(always)]
unsafe fn load_c_int(p: *const c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let v: i32;
        unsafe {
            core::arch::asm!(
                "mov {val:e}, dword ptr [{ptr}]",
                val = out(reg) v,
                ptr = in(reg) p,
                options(nostack, readonly),
            );
        }
        v as c_int
    }
    #[cfg(target_arch = "aarch64")]
    {
        let v: i32;
        unsafe {
            core::arch::asm!(
                "ldr {val:w}, [{ptr}]",
                val = out(reg) v,
                ptr = in(reg) p,
                options(nostack, readonly),
            );
        }
        v as c_int
    }
    // Portable fallback. Note this is only equivalent to the C when UB-checks
    // are disabled, which `[profile.dev] debug-assertions = false` in
    // Cargo.toml guarantees for this crate.
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        unsafe { core::ptr::read_unaligned(p) }
    }
}

// void printIntPtrLine(const int *intNumber)
// {
//     printf("%d\n", *intNumber);
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        printf(FMT_D_NL.as_ptr(), load_c_int(intNumber));
    }
}

// void bad()
// {
//     int *data;
//     printIntPtrLine(data);
// }
//
// CWE-457: `data` is never initialised, so an indeterminate pointer value is
// read off the stack and handed to printIntPtrLine, which dereferences it.
// This bug is reproduced faithfully (NOT fixed): the uninitialised stack slot
// is read through a volatile load so the compiler cannot fold the read away.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    let data_val: *const c_int = unsafe { core::ptr::read_volatile(data.as_ptr()) };
    unsafe {
        printIntPtrLine(data_val);
    }
}

// void good()
// {
//     int data;
//     data = 5;
//     int *data_addr;
//     data_addr = &data;
//     printIntPtrLine(data_addr);
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut data: MaybeUninit<c_int> = MaybeUninit::uninit();
    data.write(5);
    let data_addr: *const c_int = data.as_ptr();
    unsafe {
        printIntPtrLine(data_addr);
    }
}

// void driver(int useGood)
// {
//     if (useGood) { good(); } else { bad(); }
// }
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
