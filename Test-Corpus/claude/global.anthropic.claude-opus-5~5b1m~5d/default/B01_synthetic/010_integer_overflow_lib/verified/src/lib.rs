// Rust translation of the MIT Lincoln Laboratory `driver` C library.
//
// Original copyright notice from c_src (reproduced verbatim):
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

//! Faithful translation of `c_src/src/driver.c` / `c_src/include/driver.h`.
//!
//! Public ABI (as reported by `nm -D` on the C shared object):
//!   * `printHexCharLine`
//!   * `driver`
//!
//! Both are `void f(char)`.

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

// The C implementation writes through `printf` from `<stdio.h>`.  We call the
// very same function so that the emitted bytes -- and the stdio buffering /
// flush-at-exit behaviour that decides *when* those bytes appear -- are
// bit-for-bit identical to the original library.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;

    // `printHexCharLine` is defined below *and* re-declared here as an external
    // symbol so that `driver` reaches it the way the C does: through the PLT.
    //
    // In the C library `printHexCharLine` is a non-`static` global, so gcc emits
    // `call printHexCharLine@plt` inside `driver` -- which means the call is
    // *interposable*: an `LD_PRELOAD`ed (or otherwise globally-scoped)
    // definition replaces the one `driver` uses.  If `driver` simply called the
    // Rust `printHexCharLine` by name, LLVM would inline it in release builds
    // and that observable ABI property would be lost.  Routing the call through
    // this `extern` declaration keeps the indirection, so C and Rust behave
    // identically under symbol interposition as well.
    #[link_name = "printHexCharLine"]
    fn printHexCharLine_via_plt(charHex: c_int);
}

/// Address of the exported `printHexCharLine`, materialised as data so that the
/// reference is a *relocation against the symbol* rather than a direct branch to
/// the local definition.  See `driver` for why this matters.
static PRINT_HEX_CHAR_LINE: unsafe extern "C" fn(c_int) = printHexCharLine_via_plt;

/// C: `printf("%02x\n", charHex);`
///
/// Note the (deliberately preserved) quirk of the original: `charHex` has type
/// `char`, so it undergoes the default argument promotion to `int` before being
/// consumed by `%x`, which reinterprets it as `unsigned int`.  On targets where
/// `char` is signed (e.g. x86-64 Linux) a negative value therefore prints as
/// eight hex digits -- `driver(0x7f)` yields `ffffff80`, not `80`.  This is not
/// a bug we fix here; it is reproduced exactly.
///
/// ## Why the parameter is `c_int` and not `c_char`
///
/// The C prototype is `void printHexCharLine(char)`, but on x86-64 the psABI
/// leaves the upper 24 bits of the argument register **unspecified** for a
/// sub-word parameter, and gcc's callee therefore does not trust them: it
/// re-truncates with `mov %edi,%eax; mov %al,slot` before sign-extending with
/// `movsbl`.  So the C's observable behaviour for *any* 32-bit value a caller
/// puts in `%edi` is "use the low byte, sign-extended".
///
/// Declaring this as `extern "C" fn(c_char)` would make rustc attach LLVM's
/// `signext` attribute to the parameter, i.e. *assume* the caller already
/// sign-extended.  In `--release` LLVM then drops the truncation entirely
/// (`mov %edi,%esi`), and a caller that passes e.g. `0x000000ff` gets `ff`
/// from Rust but `ffffffff` from C.  Taking `c_int` and truncating explicitly
/// reproduces gcc's codegen for every possible register value, and is
/// indistinguishable from the `char` prototype for well-behaved callers.
#[unsafe(no_mangle)]
pub extern "C" fn printHexCharLine(charHex: c_int) {
    // gcc's `mov %al, ...` -- keep the low byte only.
    let charHex: c_char = charHex as c_char;
    // b"%02x\n\0" -- identical format string to the C source.
    const FORMAT: &[u8; 6] = b"%02x\n\0";
    unsafe {
        // gcc's `movsbl` -- the default argument promotion of a signed `char`.
        printf(FORMAT.as_ptr() as *const c_char, charHex as c_int);
    }
}

/// C:
/// ```c
/// void driver(char data)
/// {
///     char result = data + 1;
///     printHexCharLine(result);
/// }
/// ```
///
/// `data + 1` is evaluated in `int` and then converted back to `char`; GCC/Clang
/// implement that narrowing conversion as a two's-complement truncation, which
/// `wrapping_add` reproduces (so `driver(0x7f)` produces `result == -128`).
/// The forwarding call deliberately goes through the `extern` re-declaration
/// (see the `extern` block above) so that, exactly like gcc's
/// `call printHexCharLine@plt`, it can be interposed by a preloaded definition.
///
/// The parameter is `c_int` for the same reason as in `printHexCharLine`: gcc
/// emits `mov %edi,%eax; mov %al,slot; movzbl slot,%eax; add $1; mov %al,...`,
/// i.e. it uses only the low byte of whatever the caller left in `%edi`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    // gcc's `mov %al, ...` -- keep the low byte only, then `+ 1` truncated back
    // to `char`.
    let data: c_char = data as c_char;
    let result: c_char = data.wrapping_add(1);
    // The `read_volatile` stops LLVM from folding the pointer back to the local
    // definition and inlining it (LLVM assumes ELF symbols are not interposed
    // unless told otherwise, and rustc has no `-fsemantic-interposition`).  The
    // resulting indirect call reproduces gcc's `call printHexCharLine@plt`.
    let f = unsafe { core::ptr::read_volatile(&PRINT_HEX_CHAR_LINE) };
    // gcc's `movsbl` of `result` into the argument register.
    unsafe { f(result as c_int) };
}
