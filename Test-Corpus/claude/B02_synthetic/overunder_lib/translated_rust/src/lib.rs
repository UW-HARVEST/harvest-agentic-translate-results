// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Preserves the original behavior bit-for-bit.

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;

// Use the C runtime's printf directly to guarantee byte-identical output to the
// original C implementation across all platforms (especially for %f, %.2f, etc).
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        c_int::MAX
    } else if d < c_int::MIN as f64 {
        c_int::MIN
    } else if d.is_nan() {
        0
    } else {
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result: c_int = base_value;

    // Reproduce C's switch fall-through semantics exactly.
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

/// Public C-ABI export with the exact name `copy_data_block` used by the C library.
///
/// # Safety
/// `dest` and `src` must point to valid `DataBlock` values. They may not alias.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    // memcpy semantics matching the C implementation.
    std::ptr::copy_nonoverlapping(
        src as *const u8,
        dest as *mut u8,
        std::mem::size_of::<DataBlock>(),
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);
    let ptr: *const c_int = &local_value;
    // SAFETY: ptr points to local_value which is alive for the duration of this call.
    let deref = unsafe { *ptr };
    deref.wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    // PRINT_VAR(result_1) -> printf("result_1 = %d\n", result_1);
    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
        printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    let temp1: f64 = a as f64 * 1.5;
    let temp2: f64 = b as f64 * 2.7;
    let temp3: f64 = c as f64 / 3.3;
    // C does (d * d + a * a) as int (with potential overflow), then casts to double.
    let dd: c_int = d.wrapping_mul(d);
    let aa: c_int = a.wrapping_mul(a);
    let sum: c_int = dd.wrapping_add(aa);
    let temp4: f64 = (sum as f64).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    unsafe {
        printf(
            b"Converted values: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    // C: a % 6 -> truncated remainder, matches Rust's `%` for c_int.
    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(
            b"Switch fall-through result: %d\n\0".as_ptr() as *const c_char,
            switch_result,
        );
    }

    let mut source_block = DataBlock {
        id: a,
        value: temp1,
        label: [0; 20],
    };
    // strncpy(source_block.label, "Source", sizeof(label) - 1);
    // source_block.label[sizeof(label) - 1] = '\0';
    let src_str = b"Source";
    let max_copy = source_block.label.len() - 1;
    let mut i = 0usize;
    // strncpy fills with NULs after the source ends, but our buffer was
    // already zero-initialized so behavior is equivalent.
    while i < max_copy && i < src_str.len() {
        source_block.label[i] = src_str[i] as c_char;
        i += 1;
    }
    source_block.label[source_block.label.len() - 1] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    // SAFETY: dest_block and source_block are distinct, valid DataBlock values.
    unsafe { copy_data_block(&mut dest_block as *mut DataBlock, &source_block as *const DataBlock); }

    unsafe {
        printf(
            b"Copied block: id=%d, value=%.2f, label=%s\n\0".as_ptr() as *const c_char,
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result = handle_pointer_operations(c);
    unsafe {
        printf(
            b"Pointer operation result: %d\n\0".as_ptr() as *const c_char,
            ptr_result,
        );
    }

    total = total
        .wrapping_add(conv1)
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: f64 = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            b"Overflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv,
        );
    }

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            b"Underflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let array2: [c_int; 5] = array1; // memcpy equivalent for plain ints

    unsafe {
        printf(b"Array copied via memcpy: \0".as_ptr() as *const c_char);
    }
    for i in 0..5 {
        unsafe {
            printf(b"%d \0".as_ptr() as *const c_char, array2[i]);
        }
        total = total.wrapping_add(array2[i]);
    }
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }

    total
}
