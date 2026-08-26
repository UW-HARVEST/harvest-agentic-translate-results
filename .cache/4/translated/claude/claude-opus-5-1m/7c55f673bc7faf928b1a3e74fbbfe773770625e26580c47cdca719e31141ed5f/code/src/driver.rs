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

//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_char, c_int};
use core::ptr;

unsafe extern "C" {
    /// The C source writes `printf("%s\n", line)`. Every gcc optimization level
    /// (`-O0` through `-O3`) rewrites that call into `puts(line)`, which is what
    /// the reference `libdriver.so` actually invokes. Calling `puts` directly
    /// keeps the emitted bytes, the trailing newline, and the C `stdout` stream
    /// buffering semantics identical to the C library.
    safe fn puts(s: *const c_char) -> c_int;
}

// ---------------------------------------------------------------------------
// printLine
// ---------------------------------------------------------------------------

/// ```c
/// void printLine(const char *line)
/// {
///     if (line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
///
/// The NULL guard comes first, exactly as in the C, and is what makes the
/// `bad()` path below silent.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        puts(line);
    }
}

// ---------------------------------------------------------------------------
// helperBad / bad  (CWE-562: return of stack variable address)
// ---------------------------------------------------------------------------

/// ```c
/// static char *helperBad()
/// {
///     char charString[] = "helperBad string";
///     return charString;
/// }
/// ```
///
/// This returns the address of an automatic (stack) array, which is undefined
/// behavior; gcc even diagnoses it as `-Wreturn-local-addr`. The bug is
/// deliberately *not* fixed here — instead the observable behavior of the
/// compiled C library is reproduced exactly.
///
/// gcc resolves the undefined return by yielding a null pointer. Verified
/// against the disassembly of the reference `libdriver.so` at every
/// optimization level:
///
/// ```text
///  -O0/-O1  helperBad:  ...  mov  eax,0x0   ; return NULL
///                            ret
///  -O2/-O3  bad:             xor  edi,edi   ; helperBad inlined -> NULL
///                            call printLine
/// ```
///
/// Consequently `bad()` passes NULL to `printLine`, the NULL guard fires, and
/// nothing at all is printed. Confirmed empirically: `bad()` and `driver(0)`
/// each produce zero bytes of output for all of `-O0`, `-O1`, `-O2`, `-O3`.
///
/// Note that faithfully emulating a *dangling stack pointer* would be both
/// unsound in Rust and would print "helperBad string", which the real C library
/// demonstrably does not do. Returning null is the byte-identical translation.
#[inline]
fn helperBad() -> *mut c_char {
    ptr::null_mut()
}

/// ```c
/// void bad()
/// {
///     printLine(helperBad());
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    unsafe { printLine(helperBad()) }
}

// ---------------------------------------------------------------------------
// helperGood1 / good
// ---------------------------------------------------------------------------

/// Backing storage for `helperGood1`'s `static char charString[]`.
///
/// The C array has static storage duration, so the pointer stays valid after
/// the helper returns. 19 bytes = 18 characters plus the NUL terminator, which
/// matches the `charString.0` object in the C shared library.
static HELPER_GOOD1_STRING: [u8; 19] = *b"helperGood1 string\0";

/// ```c
/// static char *helperGood1()
/// {
///     static char charString[] = "helperGood1 string";
///     return charString;
/// }
/// ```
#[inline]
fn helperGood1() -> *const c_char {
    HELPER_GOOD1_STRING.as_ptr().cast::<c_char>()
}

/// ```c
/// void good()
/// {
///     printLine(helperGood1());
/// }
/// ```
///
/// Prints `helperGood1 string\n`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    unsafe { printLine(helperGood1()) }
}

// ---------------------------------------------------------------------------
// driver
// ---------------------------------------------------------------------------

/// ```c
/// void driver(int useGood)
/// {
///     if (useGood)
///     {
///         good();
///     }
///     else
///     {
///         bad();
///     }
/// }
/// ```
///
/// C truthiness is "non-zero", so any `useGood != 0` selects `good()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() }
    } else {
        unsafe { bad() }
    }
}
