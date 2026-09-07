// Rust translation of c_src/src/staticalias.c
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

// The cdylib is named `StaticAlias` to match the C build's `libStaticAlias.so`.
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int};

// The C source prints via `printf` from <stdio.h>. Bind directly to libc's
// `printf` so that stdout buffering, ordering and flushing-at-exit semantics
// are bit-for-bit identical to the C library (Rust's own `std::io::stdout`
// uses a separate buffer and would interleave differently).
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `"%d\n"` format string used by `driver`, NUL terminated.
const FMT_D_NL: &[u8; 4] = b"%d\n\0";

/// Function-local `static int inner = 1;` from `static_alias`.
///
/// In C this has static storage duration, so its value persists across every
/// call to `static_alias` (including calls made through `driver`) for the whole
/// lifetime of the process. A module-level `static mut` reproduces that exactly,
/// including the fact that it is not thread safe.
static mut INNER: c_int = 1;

/// ```c
/// int *
/// static_alias(int *outer) {
///   static int inner = 1;
///   if (*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
///
/// Note that `outer` may itself alias `inner` (that happens on every call after
/// the first time the `then` branch is taken, because `driver` feeds the
/// returned pointer straight back in). The reads below are therefore ordered so
/// that the aliasing case computes `inner + inner`, matching C.
///
/// # Safety
///
/// `outer` must be a valid, dereferenceable and writable pointer to an `int`,
/// exactly as required by the C original. Alignment is *not* required: the C
/// original places no alignment requirement on the generated access either (see
/// the note in the body), and a misaligned `outer` behaves identically to the C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let inner: *mut c_int = &raw mut INNER;

    // `outer` is accessed with `read_unaligned` / `write_unaligned` rather than
    // `*outer`. gcc compiles the C `*outer` to a plain x86-64 `mov`, which places
    // no alignment requirement on the address and faults only when the address
    // itself is unmapped. `read_unaligned`/`write_unaligned` lower to exactly that
    // and, unlike `*outer`, carry no alignment precondition, so a misaligned
    // `int*` reads and writes normally, matching the C artifact, instead of
    // tripping a debug-build assertion. (A null or unmapped `outer` raises SIGSEGV
    // exactly as the C does in the release cdylib; in a `debug-assertions` build
    // `core`'s optional UB precondition check may abort first instead — that check
    // is documented as optional and is absent from the shipped artifact.)
    //
    // `inner` points at `INNER`, which is always well-aligned and never null, so
    // it is read/written directly.
    unsafe {
        if outer.read_unaligned() >= inner.read() {
            // `inner += *outer;` — read the addend first so that the
            // `outer == inner` aliasing case doubles `inner`, as C does.
            let addend = outer.read_unaligned();
            // wrapping_add reproduces the two's-complement wraparound that the
            // C signed-overflow (UB) case exhibits in practice; it is not a
            // behaviour change for any non-overflowing input.
            inner.write(inner.read().wrapping_add(addend));
            inner
        } else {
            // `*outer += inner;`
            let addend = inner.read();
            outer.write_unaligned(outer.read_unaligned().wrapping_add(addend));
            outer
        }
    }
}

/// ```c
/// /*
///   Maintain a sum leveraging multiple references to a static variable
///  */
/// void
/// driver(int initial_value, int iterations) {
///   int *running_sum = &initial_value;
///   for (int i = 0; i < iterations; i++) {
///     running_sum = static_alias(running_sum);
///     printf("%d\n", *running_sum);
///   }
///   return;
/// }
/// ```
///
/// `initial_value` is a by-value parameter whose address is taken, so the
/// pointer initially refers to `driver`'s own copy; `static_alias` is free to
/// mutate it or to hand back a pointer to its own static instead.
///
/// # Safety
///
/// Safe to call with any argument values, but shares `static_alias`'s global
/// mutable state, so concurrent calls are a data race just as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    // Local, mutable copy of the parameter — this is the object `&initial_value`
    // designates in C.
    let mut initial_value: c_int = initial_value;
    let mut running_sum: *mut c_int = &raw mut initial_value;

    let mut i: c_int = 0;
    while i < iterations {
        unsafe {
            running_sum = static_alias(running_sum);
            printf(FMT_D_NL.as_ptr() as *const c_char, *running_sum);
        }
        i = i.wrapping_add(1);
    }
}
