// Translation of c_src/src/lib.c to Rust producing byte-identical output.
//
// The C uses printf for stdout. To preserve byte-for-byte output (including
// %.2f and %s formatting), we call libc's printf directly.

use std::ffi::{c_char, c_double, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX as c_double {
        INT_MAX
    } else if d < INT_MIN as c_double {
        INT_MIN
    } else if d.is_nan() {
        0
    } else {
        // bounds already checked; saturating cast matches C truncation.
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result = base_value;

    // Replicates the C switch with fall-through. Cases 5,4,3,2 each fall
    // through into the next; case 1 ends with break; case 0 sets 0; default -1.
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    // Mirrors C memcpy of the full struct, padding bytes included.
    std::ptr::copy_nonoverlapping(
        src as *const u8,
        dest as *mut u8,
        std::mem::size_of::<DataBlock>(),
    );
}

fn copy_data_block_safe(dest: &mut DataBlock, src: &DataBlock) {
    unsafe {
        copy_data_block(dest as *mut DataBlock, src as *const DataBlock);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value.wrapping_mul(2);
    let ptr: *const c_int = &local_value;
    // *ptr + 100
    let result = unsafe { *ptr }.wrapping_add(100);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
        printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    let temp1: c_double = (a as c_double) * 1.5;
    let temp2: c_double = (b as c_double) * 2.7;
    let temp3: c_double = (c as c_double) / 3.3;
    let temp4: c_double = ((d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a))) as c_double).sqrt();

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

    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(
            b"Switch fall-through result: %d\n\0".as_ptr() as *const c_char,
            switch_result,
        );
    }

    // Build source DataBlock. Note: C declared `DataBlock source_block;` with
    // no initializer, so its memory is indeterminate. We fill the same fields
    // and set label like the C code does. Padding bytes remain undefined in
    // both implementations and are not observed via printf.
    let mut source_block: DataBlock = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(label, "Source", sizeof(label) - 1):
    //   copies "Source" (6 bytes), then null-pads up to 19 bytes total written.
    //   Then `label[19] = '\0'` ensures index 19 is null.
    // We pre-zeroed the array, so simply copy "Source" bytes 0..6.
    let src_str = b"Source";
    for (i, &byte) in src_str.iter().enumerate() {
        source_block.label[i] = byte as c_char;
    }
    source_block.label[19] = 0;

    let mut dest_block: DataBlock = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    copy_data_block_safe(&mut dest_block, &source_block);

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

    let mut total: c_int = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: c_double = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            b"Overflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv,
        );
    }

    let underflow_test: c_double = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            b"Underflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [c_int; 5] = [0; 5];
    unsafe {
        std::ptr::copy_nonoverlapping(
            array1.as_ptr() as *const u8,
            array2.as_mut_ptr() as *mut u8,
            std::mem::size_of::<[c_int; 5]>(),
        );
    }

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
