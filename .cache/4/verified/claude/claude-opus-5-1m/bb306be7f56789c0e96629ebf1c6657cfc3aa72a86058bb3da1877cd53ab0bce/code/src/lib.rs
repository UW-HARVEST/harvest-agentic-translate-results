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

unsafe extern "C" {
    // Use the C library's printf so that formatting and stdio buffering
    // semantics are byte-for-byte identical to the original C library.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Loads a 32-bit int with no language-level preconditions attached, which is
/// what GCC's `mov (%rax),%eax` for `*intNumber` actually is.
///
/// Neither of Rust's obvious spellings is faithful here, and both diverge from
/// the C in a way that is only visible in a debug build:
///
/// * `*intNumber` inserts a "misaligned pointer dereference" check, so it aborts
///   (`SIGABRT`) on any pointer that is not 4-byte aligned -- where the C simply
///   performs an unaligned load and succeeds.
/// * `core::ptr::read_unaligned` fixes that but routes through
///   `copy_nonoverlapping`, whose debug-only precondition rejects a null
///   pointer, so it aborts (`SIGABRT`) on NULL -- where the C faults with
///   `SIGSEGV`.
///
/// A bare `mov` has neither property: unaligned addresses load fine, and invalid
/// addresses raise `SIGSEGV` from the hardware, exactly as in the C.
#[inline(always)]
unsafe fn load_int_unchecked(p: *const c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let out: u32;
        unsafe {
            core::arch::asm!(
                "mov {out:e}, dword ptr [{p}]",
                p = in(reg) p,
                out = out(reg) out,
                options(readonly, nostack, preserves_flags)
            );
        }
        out as c_int
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { core::ptr::read_unaligned(p) }
    }
}

/// `void printIntPtrLine(const int *intNumber)` -> `printf("%d\n", *intNumber);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    // Dereference exactly as the C code does (no NULL check in the original).
    let value: c_int = unsafe { load_int_unchecked(intNumber) };
    unsafe {
        c_printf(c"%d\n".as_ptr(), value);
    }
}

// ---------------------------------------------------------------------------
// Stack-frame model shared by `bad()` and `good()`.
//
// The original C is compiled at -O0, where GCC gives both `bad()` and `good()`
// an identical 16-byte local frame (`sub $0x10,%rsp`) and places the `int *`
// local at the very same slot, `rbp-0x8`:
//
//     bad:                          good:
//       sub  $0x10,%rsp               sub  $0x10,%rsp
//       mov  -0x8(%rbp),%rax          movl $0x5,-0xc(%rbp)   ; data      = 5
//       mov  %rax,%rdi                lea  -0xc(%rbp),%rax
//       call printIntPtrLine          mov  %rax,-0x8(%rbp)   ; data_addr = &data
//                                     mov  -0x8(%rbp),%rax
//                                     mov  %rax,%rdi
//                                     call printIntPtrLine
//
// `bad()` therefore reads back whatever 8 bytes the previous occupant of that
// stack region left at `rbp-0x8`. The observable consequence that the C
// exhibits *deterministically* is that `good()` immediately followed by
// `bad()` prints `5` twice: `good()` stores `&data` into `rbp-0x8`, and the
// following `bad()` -- entered at the same stack depth with the same frame
// layout -- loads that identical pointer back out and dereferences it.
//
// To reproduce that, both functions here declare one identically sized and
// aligned 16-byte local frame and address the pointer slot at the same
// offset (+8 from the frame base, i.e. `rbp-0x8`), so the two functions
// coincide on the same stack address exactly as the C does.
const FRAME_LEN: usize = 16;
/// Byte offset of the `int *` local within the frame (`rbp-0x8`).
const PTR_SLOT: usize = 8;
/// Byte offset of `good()`'s `int data` local within the frame (`rbp-0xc`).
const INT_SLOT: usize = 4;

#[repr(C, align(16))]
struct Frame([u8; FRAME_LEN]);

/// Shared body of `bad()` (`init == false`) and `good()` (`init == true`).
///
/// Both C functions have byte-identical frame layouts, so when they are called
/// from the same place they occupy the very same stack addresses. Routing both
/// through a single `#[inline(never)]` helper reproduces that by construction:
/// the frame is literally the same function's frame, so the `int *` slot is
/// guaranteed to land on the same address for `good()` and for `bad()`.
#[inline(never)]
unsafe extern "C" fn frame_body(init: bool) {
    let mut frame: MaybeUninit<Frame> = MaybeUninit::uninit();
    let base = frame.as_mut_ptr() as *mut u8;
    let ptr_slot = unsafe { base.add(PTR_SLOT) } as *mut *const c_int;

    if init {
        // good():  int data; data = 5;
        let data: *mut c_int = unsafe { base.add(INT_SLOT) } as *mut c_int;
        unsafe { core::ptr::write_volatile(data, 5) };
        // int *data_addr; data_addr = &data;
        unsafe { core::ptr::write_volatile(ptr_slot, data as *const c_int) };
    }
    // bad() skips the two stores above, leaving `ptr_slot` holding whatever the
    // previous occupant of this stack region left behind -- exactly the
    // uninitialized `int *data` of the C original.

    let data: *const c_int = unsafe { core::ptr::read_volatile(ptr_slot) };
    // printIntPtrLine(data);
    unsafe { printIntPtrLine(data) };
}

/// `void bad(void)`
///
/// Faithfully reproduces the original defect (CWE-457/CWE-824: use of an
/// uninitialized pointer variable). The pointer is left uninitialized and
/// then dereferenced, exactly as in the C source. This is intentional --
/// the bug must NOT be fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // int *data;  /* never assigned */  printIntPtrLine(data);
    unsafe { frame_body(false) };
}

/// `void good(void)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    // int data = 5; int *data_addr = &data; printIntPtrLine(data_addr);
    unsafe { frame_body(true) };
}

/// `void driver(int useGood)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
