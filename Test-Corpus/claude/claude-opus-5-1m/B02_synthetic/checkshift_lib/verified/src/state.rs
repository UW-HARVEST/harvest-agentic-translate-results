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

//! Translation of `ComputeState` and its helpers (`compute_checksum`,
//! `init_state`, `apply_operation`) from `c_src/src/lib.c`.

use core::ffi::{c_int, c_uint};

use crate::cio::{print_i, print_lit};
use crate::ops::OperationFunc;

/// `#define MAGIC_NUMBER 0xDEADBEEF`
const MAGIC_NUMBER: c_uint = 0xDEAD_BEEF;
/// `#define MASK_LOWER 0x0000FFFF`
const MASK_LOWER: c_uint = 0x0000_FFFF;

/// ```c
/// typedef struct {
///     int accumulator;
///     int operation_count;
///     unsigned int checksum;
/// } ComputeState;
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputeState {
    pub accumulator: c_int,
    pub operation_count: c_int,
    pub checksum: c_uint,
}

/// `unsigned int compute_checksum(int* values, int count)`
///
/// Copies up to the first four `int`s into a byte buffer, folds them into a
/// rotating checksum, mixes in `MAGIC_NUMBER` and returns the low 16 bits.
///
/// Note that the fold walks the raw object representation of the `int`s, so the
/// result depends on the host byte order exactly as the C version does. When
/// `values` is null or `count <= 0` nothing is folded and the result is `0`.
///
/// # Safety
///
/// If `values` is non-null and `count > 0`, `values` must be readable for
/// `min(count, 4)` `int`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_checksum(values: *mut c_int, count: c_int) -> c_uint {
    let mut checksum: c_uint = 0;
    // `unsigned char buffer[sizeof(int) * 4];` - left uninitialised in C, but
    // only the freshly copied prefix is ever read.
    let mut buffer = [0u8; core::mem::size_of::<c_int>() * 4];

    if !values.is_null() && count > 0 {
        let copy_count = if count > 4 { 4 } else { count } as usize;
        let byte_len = core::mem::size_of::<c_int>() * copy_count;

        // memcpy(buffer, values, sizeof(int) * copy_count);
        // SAFETY: the caller guarantees `values` holds at least `copy_count`
        // readable `int`s, and `byte_len <= buffer.len()`.
        unsafe {
            core::ptr::copy_nonoverlapping(values as *const u8, buffer.as_mut_ptr(), byte_len);
        }

        for &byte in &buffer[..byte_len] {
            checksum = (checksum << 1) ^ c_uint::from(byte);
        }

        checksum ^= MAGIC_NUMBER;
    }

    checksum & MASK_LOWER
}

/// `void init_state(ComputeState* state, int initial_value)`
///
/// Overwrites `*state` with `{initial_value, 0, 0}` and logs the accumulator.
/// A null `state` reports an error and returns.
///
/// # Safety
///
/// `state` must be null or point to a writable `ComputeState`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_state(state: *mut ComputeState, initial_value: c_int) {
    if state.is_null() {
        print_lit(b"Error: state pointer is NULL in init_state\n\0");
        return;
    }

    let template = ComputeState {
        accumulator: initial_value,
        operation_count: 0,
        checksum: 0x0000,
    };

    // memcpy(state, &template, sizeof(ComputeState));
    // SAFETY: `state` is non-null and, per the contract, points to a writable
    // `ComputeState`.
    unsafe { core::ptr::write(state, template) };

    // SAFETY: `state` was just fully initialised above.
    print_i(
        b"State initialized with accumulator = %d\n\0",
        unsafe { (*state).accumulator },
    );
}

/// `void apply_operation(ComputeState* state, int value, operation_func func)`
///
/// Folds `value` into `state->accumulator` using `func` and bumps
/// `state->operation_count`. A null `state` or `func` reports an error and
/// leaves the state untouched.
///
/// # Safety
///
/// `state` must be null or point to an initialised, writable `ComputeState`;
/// `func` must be null or callable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    state: *mut ComputeState,
    value: c_int,
    func: OperationFunc,
) {
    if state.is_null() {
        print_lit(b"Error: state pointer is NULL in apply_operation\n\0");
        return;
    }

    let Some(func) = func else {
        print_lit(b"Error: operation function pointer is NULL in apply_operation\n\0");
        return;
    };

    // SAFETY: `state` is non-null and points to an initialised `ComputeState`.
    unsafe {
        (*state).accumulator = func((*state).accumulator, value);
        (*state).operation_count = (*state).operation_count.wrapping_add(1);
    }
}
