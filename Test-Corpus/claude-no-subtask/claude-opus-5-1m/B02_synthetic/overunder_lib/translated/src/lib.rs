// Translated from c_src/src/lib.c
// Produces byte-identical output via libc::printf.

use core::ffi::{c_char, c_int};
use libc::printf;

#[repr(C)]
#[derive(Copy, Clone)]
struct DataBlock {
    id: c_int,
    value: f64,
    label: [c_char; 20],
}

fn safe_double_to_int(d: f64) -> c_int {
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

fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result = base_value;

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

fn copy_data_block(dest: &mut DataBlock, src: &DataBlock) {
    *dest = *src;
}

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value.wrapping_mul(2);
    local_value.wrapping_add(100)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    unsafe {
        printf(c"result_1 = %d\n".as_ptr(), result_1);
        printf(c"result_2 = %d\n".as_ptr(), result_2);
    }

    let temp1: f64 = a as f64 * 1.5;
    let temp2: f64 = b as f64 * 2.7;
    let temp3: f64 = c as f64 / 3.3;
    let dd = d.wrapping_mul(d);
    let aa = a.wrapping_mul(a);
    let temp4: f64 = (dd.wrapping_add(aa) as f64).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    unsafe {
        printf(
            c"Converted values: %d, %d, %d, %d\n".as_ptr(),
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(
            c"Switch fall-through result: %d\n".as_ptr(),
            switch_result,
        );
    }

    let mut source_block = DataBlock {
        id: a,
        value: temp1,
        label: [0; 20],
    };
    // Emulate: strncpy(source_block.label, "Source", 19); label[19] = '\0';
    let src_bytes: &[u8] = b"Source";
    for (i, &byte) in src_bytes.iter().enumerate() {
        source_block.label[i] = byte as c_char;
    }
    // strncpy zero-fills remaining bytes; our buffer was already zeroed.
    source_block.label[19] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    copy_data_block(&mut dest_block, &source_block);

    unsafe {
        printf(
            c"Copied block: id=%d, value=%.2f, label=%s\n".as_ptr(),
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result = handle_pointer_operations(c);
    unsafe {
        printf(c"Pointer operation result: %d\n".as_ptr(), ptr_result);
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
        printf(c"Overflow protected conversion: %d\n".as_ptr(), safe_conv);
    }

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(c"Underflow protected conversion: %d\n".as_ptr(), safe_conv2);
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [c_int; 5] = [0; 5];
    array2.copy_from_slice(&array1);

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
