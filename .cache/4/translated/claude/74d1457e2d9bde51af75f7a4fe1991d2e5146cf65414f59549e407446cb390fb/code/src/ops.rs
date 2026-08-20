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

//! Translation of the arithmetic primitives, the operation-table lookup and
//! the operation dispatcher from `c_src/src/lib.c`.

use core::ffi::{c_char, c_int};

use crate::cio::{print_i, print_s, print_s_i};

/// `typedef int (*operation_func)(int, int)`
///
/// Modelled as `Option<extern "C" fn(..)>` so a C `NULL` round-trips as `None`.
/// That representation is guaranteed ABI-identical to a bare function pointer
/// through the null-pointer optimisation.
pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>;

// ---------------------------------------------------------------------------
// File-scope `static` tuning values. Nothing in the library ever mutates them,
// so they are plain immutable statics here.
// ---------------------------------------------------------------------------

/// `static int static_multiplier = 3;`
static STATIC_MULTIPLIER: c_int = 3;
/// `static int static_addend = 100;`
static STATIC_ADDEND: c_int = 100;
/// `static int static_shift_amount = 2;`
static STATIC_SHIFT_AMOUNT: c_int = 2;

/// `int multiply_with_static(int a, int b)`
///
/// Returns `(a * b) * static_multiplier`. Both multiplications wrap, matching
/// what the C compiler emits for this two's-complement target.
#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_mul(STATIC_MULTIPLIER)
}

/// `int add_with_static(int a, int b)`
///
/// Returns `(a + b) + static_addend`, wrapping on overflow.
#[unsafe(no_mangle)]
pub extern "C" fn add_with_static(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(STATIC_ADDEND)
}

/// `int xor_operation(int a, int b)`
///
/// Returns `a ^ b ^ 0xABCD`.
#[unsafe(no_mangle)]
pub extern "C" fn xor_operation(a: c_int, b: c_int) -> c_int {
    a ^ b ^ 0xABCD
}

/// `int shift_with_static(int a, int b)`
///
/// Returns `(a << static_shift_amount) | (b >> static_shift_amount)`.
///
/// The left shift discards the bits shifted out of the word (`wrapping_shl`),
/// and the right shift is an arithmetic shift that sign-extends, which is how
/// GCC/Clang implement `>>` on a signed operand.
#[unsafe(no_mangle)]
pub extern "C" fn shift_with_static(a: c_int, b: c_int) -> c_int {
    let shift = STATIC_SHIFT_AMOUNT as u32;
    a.wrapping_shl(shift) | (b >> shift)
}

/// The lazily-populated `static operation_func ops[4]` inside `get_operation`.
///
/// In C the table starts out all-`NULL` and is filled in on the first call.
/// Because the stored values never change afterwards, the observable result is
/// identical to this constant table. The entries reference the `#[no_mangle]`
/// items above, so a C caller comparing the returned pointer against
/// `&multiply_with_static` (etc.) still sees equality.
const OPS: [OperationFunc; 4] = [
    Some(multiply_with_static as unsafe extern "C" fn(c_int, c_int) -> c_int),
    Some(add_with_static as unsafe extern "C" fn(c_int, c_int) -> c_int),
    Some(xor_operation as unsafe extern "C" fn(c_int, c_int) -> c_int),
    Some(shift_with_static as unsafe extern "C" fn(c_int, c_int) -> c_int),
];

/// `operation_func get_operation(int opcode)`
///
/// Returns the handler for `opcode` in `0..4`, otherwise `NULL`.
#[unsafe(no_mangle)]
pub extern "C" fn get_operation(opcode: c_int) -> OperationFunc {
    if opcode >= 0 && opcode < 4 {
        return OPS[opcode as usize];
    }

    None
}

/// `int execute_operation(operation_func func, int a, int b, const char* op_name)`
///
/// Logs both operands, invokes `func` and logs the result. A `NULL` `func`
/// reports an error and yields `0`.
///
/// # Safety
///
/// `func` must be null or a callable `int(int, int)`; `op_name` must be null or
/// point to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn execute_operation(
    func: OperationFunc,
    a: c_int,
    b: c_int,
    op_name: *const c_char,
) -> c_int {
    let Some(func) = func else {
        // SAFETY: `op_name` is forwarded to `printf` exactly as the C code does.
        unsafe {
            print_s(
                b"Error: Operation function pointer is NULL for %s\n\0",
                op_name,
            )
        };
        return 0;
    };

    // LOG_VALUE(a); LOG_VALUE(b);
    print_i(b"Variable a = %d\n\0", a);
    print_i(b"Variable b = %d\n\0", b);

    // SAFETY: `func` is a non-null `operation_func` supplied by the caller.
    let result = unsafe { func(a, b) };
    // SAFETY: `op_name` is forwarded to `printf` exactly as the C code does.
    unsafe { print_s_i(b"Result of %s: %d\n\0", op_name, result) };

    result
}
