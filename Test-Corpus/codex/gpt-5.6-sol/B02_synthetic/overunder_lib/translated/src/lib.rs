use std::ffi::{c_char, c_double, c_int, c_void};
use std::mem::MaybeUninit;

#[repr(C)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

unsafe extern "C" {
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[link(name = "m")]
unsafe extern "C" {
    fn sqrt(value: c_double) -> c_double;
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d > c_int::MAX as c_double {
        c_int::MAX
    } else if d < c_int::MIN as c_double {
        c_int::MIN
    } else if d.is_nan() {
        0
    } else {
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    match code {
        5 => base_value
            .wrapping_add(50)
            .wrapping_add(40)
            .wrapping_add(30)
            .wrapping_add(20)
            .wrapping_add(10),
        4 => base_value
            .wrapping_add(40)
            .wrapping_add(30)
            .wrapping_add(20)
            .wrapping_add(10),
        3 => base_value
            .wrapping_add(30)
            .wrapping_add(20)
            .wrapping_add(10),
        2 => base_value.wrapping_add(20).wrapping_add(10),
        1 => base_value.wrapping_add(10),
        0 => 0,
        _ => -1,
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    value.wrapping_mul(2).wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    unsafe {
        printf(c"result_1 = %d\n".as_ptr(), a);
        printf(c"result_2 = %d\n".as_ptr(), b);
    }

    let temp1 = a as c_double * 1.5;
    let temp2 = b as c_double * 2.7;
    let temp3 = c as c_double / 3.3;
    let squared_sum = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
    let temp4 = unsafe { sqrt(squared_sum as c_double) };

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
        printf(c"Switch fall-through result: %d\n".as_ptr(), switch_result);
    }

    let source_block = DataBlock {
        id: a,
        value: temp1,
        label: [
            b'S' as c_char,
            b'o' as c_char,
            b'u' as c_char,
            b'r' as c_char,
            b'c' as c_char,
            b'e' as c_char,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
    };
    let mut dest_block = MaybeUninit::<DataBlock>::uninit();
    unsafe {
        copy_data_block(dest_block.as_mut_ptr(), &source_block);
    }
    let dest_block = unsafe { dest_block.assume_init() };

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

    let mut total = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let safe_conv = safe_double_to_int(1e15);
    unsafe {
        printf(c"Overflow protected conversion: %d\n".as_ptr(), safe_conv);
    }

    let safe_conv2 = safe_double_to_int(-1e15);
    unsafe {
        printf(c"Underflow protected conversion: %d\n".as_ptr(), safe_conv2);
    }

    let array1 = [a, b, c, d, a.wrapping_add(b)];

    unsafe {
        printf(c"Array copied via memcpy: ".as_ptr());
    }
    for value in array1 {
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
