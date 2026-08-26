use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn sqrt(x: f64) -> f64;
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d > i32::MAX as f64 {
        i32::MAX
    } else if d < (i32::MIN as f64) {
        i32::MIN
    } else if d.is_nan() {
        0
    } else {
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result = base_value;
    match code {
        5 => { result = result.wrapping_add(50).wrapping_add(40).wrapping_add(30).wrapping_add(20).wrapping_add(10); }
        4 => { result = result.wrapping_add(40).wrapping_add(30).wrapping_add(20).wrapping_add(10); }
        3 => { result = result.wrapping_add(30).wrapping_add(20).wrapping_add(10); }
        2 => { result = result.wrapping_add(20).wrapping_add(10); }
        1 => { result = result.wrapping_add(10); }
        0 => { result = 0; }
        _ => { result = -1; }
    }
    result
}

#[repr(C)]
struct DataBlock {
    id: c_int,
    value: f64,
    label: [u8; 20],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe { std::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, std::mem::size_of::<DataBlock>()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value.wrapping_mul(2);
    local_value.wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr(), result_1);
        printf(b"result_2 = %d\n\0".as_ptr(), result_2);
    }

    let temp1 = a as f64 * 1.5;
    let temp2 = b as f64 * 2.7;
    let temp3 = c as f64 / 3.3;
    let temp4 = unsafe { sqrt(d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) as f64) };

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    unsafe {
        printf(b"Converted values: %d, %d, %d, %d\n\0".as_ptr(), conv1, conv2, conv3, conv4);
    }

    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(b"Switch fall-through result: %d\n\0".as_ptr(), switch_result);
    }

    let mut source_block = DataBlock { id: a, value: temp1, label: [0u8; 20] };
    let src_bytes = b"Source";
    source_block.label[..src_bytes.len()].copy_from_slice(src_bytes);
    // label[6..19] already zero, label[19] = '\0' already zero

    let mut dest_block = DataBlock { id: 0, value: 0.0, label: [0u8; 20] };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &source_block as *const DataBlock as *const u8,
            &mut dest_block as *mut DataBlock as *mut u8,
            std::mem::size_of::<DataBlock>(),
        );
        printf(
            b"Copied block: id=%d, value=%.2f, label=%s\n\0".as_ptr(),
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result = handle_pointer_operations(c);
    unsafe {
        printf(b"Pointer operation result: %d\n\0".as_ptr(), ptr_result);
    }

    total = conv1.wrapping_add(conv2).wrapping_add(conv3).wrapping_add(conv4).wrapping_add(switch_result).wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: f64 = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(b"Overflow protected conversion: %d\n\0".as_ptr(), safe_conv);
    }

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(b"Underflow protected conversion: %d\n\0".as_ptr(), safe_conv2);
    }

    let array2 = [a, b, c, d, a.wrapping_add(b)];

    unsafe {
        printf(b"Array copied via memcpy: \0".as_ptr());
        for i in 0..5 {
            printf(b"%d \0".as_ptr(), array2[i]);
            total = total.wrapping_add(array2[i]);
        }
        printf(b"\n\0".as_ptr());
    }

    total
}
