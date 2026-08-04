use std::ffi::{c_char, c_double, c_int};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[link(name = "m")]
unsafe extern "C" {
    fn sqrt(x: c_double) -> c_double;
}

const INT_MAX_F64: c_double = c_int::MAX as c_double;
const INT_MIN_F64: c_double = c_int::MIN as c_double;

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX_F64 {
        c_int::MAX
    } else if d < INT_MIN_F64 {
        c_int::MIN
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe {
        ptr::copy_nonoverlapping(src.cast::<u8>(), dest.cast::<u8>(), size_of::<DataBlock>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value.wrapping_mul(2);
    local_value.wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    unsafe {
        printf(c"result_1 = %d\n".as_ptr(), result_1);
        printf(c"result_2 = %d\n".as_ptr(), result_2);
    }

    let temp1 = (a as c_double) * 1.5;
    let temp2 = (b as c_double) * 2.7;
    let temp3 = (c as c_double) / 3.3;
    let sum_squares = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
    let temp4 = unsafe { sqrt(sum_squares as c_double) };

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
    let source = b"Source";
    for (idx, byte) in source.iter().enumerate() {
        source_block.label[idx] = *byte as c_char;
    }
    source_block.label[source_block.label.len() - 1] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    unsafe {
        copy_data_block(&mut dest_block, &source_block);
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

    total = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            c"Overflow protected conversion: %d\n".as_ptr(),
            safe_conv,
        );
    }

    let underflow_test = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            c"Underflow protected conversion: %d\n".as_ptr(),
            safe_conv2,
        );
    }

    let array1 = [a, b, c, d, a.wrapping_add(b)];
    let mut array2 = [0; 5];
    array2.copy_from_slice(&array1);

    unsafe {
        printf(c"Array copied via memcpy: ".as_ptr());
    }
    for value in array2 {
        unsafe {
            printf(c"%d ".as_ptr(), value);
        }
        total = total.wrapping_add(value);
    }
    unsafe {
        printf(c"\n".as_ptr());
    }

    total
}
