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

//! Translation of the `checkshift` entry point from `c_src/src/lib.c` - the
//! only function declared in the public `include/lib.h`.

use core::ffi::{c_char, c_int, c_void};

use crate::cio::{free, malloc, print_i, print_i4, print_lit, print_u};
use crate::ops::get_operation;
use crate::state::{apply_operation, compute_checksum, init_state, ComputeState};

/// `int checkshift(int param1, int param2, int param3, int param4)`
///
/// Runs the four operations in sequence over a heap-allocated `ComputeState`,
/// logging every step, and returns
/// `(accumulator + shift_result) ^ checksum`.
///
/// The state is allocated with the C allocator, and a failed allocation logs an
/// error and returns `-1`, just as in the original.
#[unsafe(no_mangle)]
pub extern "C" fn checkshift(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    print_lit(b"\n=== Starting foo function ===\n\0");
    print_i4(b"Parameters: %d, %d, %d, %d\n\0", param1, param2, param3, param4);

    // SAFETY: a plain `malloc` of one `ComputeState`.
    let state = unsafe { malloc(core::mem::size_of::<ComputeState>()) } as *mut ComputeState;

    if state.is_null() {
        print_lit(b"Error: Failed to allocate memory for state\n\0");
        return -1;
    }

    // SAFETY: `state` is a fresh, suitably sized and aligned allocation.
    unsafe { init_state(state, param1) };

    let mut params: [c_int; 4] = [param1, param2, param3, param4];

    let mult_op = get_operation(0);
    let add_op = get_operation(1);
    let xor_op = get_operation(2);
    let shift_op = get_operation(3);

    print_lit(b"\n--- Operation 1: Multiply ---\n\0");
    // SAFETY: `state` was initialised by `init_state`; `mult_op` is valid.
    unsafe { apply_operation(state, param2, mult_op) };

    print_lit(b"\n--- Operation 2: Add ---\n\0");
    // SAFETY: as above.
    unsafe { apply_operation(state, param3, add_op) };

    print_lit(b"\n--- Operation 3: XOR ---\n\0");
    // SAFETY: `state` is initialised; the name literal is NUL-terminated.
    let xor_result = unsafe {
        crate::ops::execute_operation(
            xor_op,
            (*state).accumulator,
            param4,
            b"XOR\0".as_ptr() as *const c_char,
        )
    };

    print_lit(b"\n--- Operation 4: Shift ---\n\0");
    // SAFETY: the name literal is NUL-terminated.
    let shift_result = unsafe {
        crate::ops::execute_operation(
            shift_op,
            xor_result,
            param2,
            b"SHIFT\0".as_ptr() as *const c_char,
        )
    };

    // SAFETY: `params` is a live array of exactly four `int`s, and `state` is
    // an initialised allocation.
    unsafe {
        (*state).checksum = compute_checksum(params.as_mut_ptr(), 4);
        print_u(b"\nComputed checksum: 0x%04X\n\0", (*state).checksum);
    }

    // `(state->accumulator + shift_result) ^ state->checksum`: the signed sum
    // wraps, then converts to `unsigned int` for the XOR, and the `unsigned`
    // result converts back to `int` on assignment.
    // SAFETY: `state` is initialised.
    let final_result = unsafe {
        ((*state).accumulator.wrapping_add(shift_result) as u32 ^ (*state).checksum) as c_int
    };

    // SAFETY: `state` is initialised.
    unsafe {
        print_i(b"\nFinal accumulator: %d\n\0", (*state).accumulator);
        print_i(b"Operation count: %d\n\0", (*state).operation_count);
    }
    print_i(b"Final result: %d\n\0", final_result);

    // SAFETY: `state` came from `malloc` and is freed exactly once here.
    unsafe { free(state as *mut c_void) };

    print_lit(b"=== Ending foo function ===\n\n\0");

    final_result
}
