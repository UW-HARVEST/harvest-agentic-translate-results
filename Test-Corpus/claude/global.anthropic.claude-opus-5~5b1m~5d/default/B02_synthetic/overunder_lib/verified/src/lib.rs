// Rust translation of c_src/src/lib.c
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

use core::ffi::{c_char, c_double, c_int, c_void};

// The C translation unit performs all of its output through the C standard
// library's `printf`. We call the very same function here so that byte-for-byte
// identical text is produced *and* so that it shares the same `stdout` buffer
// (and therefore the same interleaving) as any C code linked alongside us.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;

    /// The C translation unit copies bytes with `memcpy`, so we call the very
    /// same libc routine.
    ///
    /// This matters for fidelity: `core::ptr::copy_nonoverlapping` carries
    /// debug-only preconditions (pointers non-null, ranges non-overlapping)
    /// that make it *panic* on inputs `memcpy` merely passes through — a NULL
    /// argument, or `dest == src`. The C has no such checks, so using `memcpy`
    /// keeps the observable behaviour (including the SIGSEGV on a NULL
    /// pointer) identical in debug and release builds alike.
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// `typedef struct { int id; double value; char label[20]; } DataBlock;`
///
/// On the LP64 SysV ABI this is 40 bytes: `id` at 0, 4 bytes of padding,
/// `value` at 8, `label` at 16, then 4 trailing pad bytes for 8-byte alignment.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

impl DataBlock {
    /// Mimics C's declaration of an automatic aggregate: the storage exists but
    /// holds no meaningful value yet. Zeroing keeps Rust's semantics defined
    /// without changing any observable behaviour of the translated code.
    const fn uninit() -> DataBlock {
        DataBlock {
            id: 0,
            value: 0.0,
            label: [0; 20],
        }
    }
}

// ---------------------------------------------------------------------------
// int safe_double_to_int(double d)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        c_int::MAX
    } else if d < c_int::MIN as c_double {
        c_int::MIN
    } else if d.is_nan() {
        0
    } else {
        // The two range tests above guarantee that the value is in
        // [INT_MIN, INT_MAX], so truncation toward zero matches C's cast.
        d as c_int
    }
}

// ---------------------------------------------------------------------------
// int process_with_fallthrough(int code, int base_value)
//
// The C switch deliberately falls through from 5 -> 4 -> 3 -> 2 -> 1, so the
// accumulated additions are performed in exactly that order (wrapping, which
// is what the C compiler emits for signed overflow).
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result: c_int = base_value;

    match code {
        5 => {
            result = result.wrapping_add(50);
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        4 => {
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        3 => {
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        2 => {
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        1 => {
            result = result.wrapping_add(10);
        }
        0 => {
            result = 0;
        }
        _ => {
            result = -1;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// void copy_data_block(DataBlock *dest, const DataBlock *src)
//
// memcpy(dest, src, sizeof(DataBlock))
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe {
        memcpy(
            dest.cast::<c_void>(),
            src.cast::<c_void>(),
            size_of::<DataBlock>(),
        );
    }
}

// ---------------------------------------------------------------------------
// int handle_pointer_operations(int value)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);

    let ptr: &c_int = &local_value;

    let result: c_int = (*ptr).wrapping_add(100);

    result
}

// ---------------------------------------------------------------------------
// int overunder(int a, int b, int c, int d)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    // `int total = 0;` -- the C code initialises it here and only later
    // overwrites it wholesale; kept verbatim for fidelity to the source.
    let mut total: c_int = 0;

    // MAKE_VAR_NAME(result, _N) token-pastes into result_1 .. result_4
    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    // PRINT_VAR(name) => printf(#name " = %d\n", name)
    unsafe {
        printf(c"result_1 = %d\n".as_ptr(), result_1);
        printf(c"result_2 = %d\n".as_ptr(), result_2);
    }

    let temp1: c_double = a as c_double * 1.5;
    let temp2: c_double = b as c_double * 2.7;
    let temp3: c_double = c as c_double / 3.3;
    // `d * d + a * a` is int arithmetic in C and may overflow; reproduce the
    // two's-complement wrap-around the C compiler emits, then convert.
    let temp4: c_double =
        (d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) as c_double).sqrt();

    let conv1: c_int = safe_double_to_int(temp1);
    let conv2: c_int = safe_double_to_int(temp2);
    let conv3: c_int = safe_double_to_int(temp3);
    let conv4: c_int = safe_double_to_int(temp4);

    unsafe {
        printf(
            c"Converted values: %d, %d, %d, %d\n".as_ptr(),
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    let switch_result: c_int = process_with_fallthrough(a.wrapping_rem(6), b);
    unsafe {
        printf(
            c"Switch fall-through result: %d\n".as_ptr(),
            switch_result,
        );
    }

    let mut source_block: DataBlock = DataBlock::uninit();
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(source_block.label, "Source", sizeof(label) - 1) copies "Source"
    // and zero-pads the remaining 13 of the 19 bytes; label[19] = '\0'.
    strncpy_fixed(&mut source_block.label, b"Source", 19);
    source_block.label[19] = 0;

    let mut dest_block: DataBlock = DataBlock::uninit();
    unsafe {
        copy_data_block(&mut dest_block, &source_block);
    }

    unsafe {
        printf(
            c"Copied block: id=%d, value=%.2f, label=%s\n".as_ptr(),
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result: c_int = handle_pointer_operations(c);
    unsafe {
        printf(c"Pointer operation result: %d\n".as_ptr(), ptr_result);
    }

    total = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: c_double = 1e15;
    let safe_conv: c_int = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            c"Overflow protected conversion: %d\n".as_ptr(),
            safe_conv,
        );
    }

    let underflow_test: c_double = -1e15;
    let safe_conv2: c_int = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            c"Underflow protected conversion: %d\n".as_ptr(),
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [c_int; 5] = [0; 5];

    // memcpy(array2, array1, sizeof(array1))
    unsafe {
        memcpy(
            array2.as_mut_ptr().cast::<c_void>(),
            array1.as_ptr().cast::<c_void>(),
            size_of::<[c_int; 5]>(),
        );
    }

    unsafe {
        printf(c"Array copied via memcpy: ".as_ptr());
    }
    for i in 0..5usize {
        unsafe {
            printf(c"%d ".as_ptr(), array2[i]);
        }
        total = total.wrapping_add(array2[i]);
    }
    unsafe {
        printf(c"\n".as_ptr());
    }

    total
}

/// Faithful `strncpy(dst, src, n)` for a fixed-size destination buffer:
/// copies at most `n` bytes from the NUL-terminated `src` and, if `src` is
/// shorter than `n`, zero-fills the remainder of those `n` bytes.
fn strncpy_fixed(dst: &mut [c_char], src: &[u8], n: usize) {
    let mut i = 0usize;
    while i < n && i < src.len() {
        dst[i] = src[i] as c_char;
        i += 1;
    }
    while i < n {
        dst[i] = 0;
        i += 1;
    }
}
