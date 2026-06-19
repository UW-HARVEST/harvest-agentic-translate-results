use core::ffi::{c_char, c_double, c_int, c_void};
use libc::{memcpy, printf};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

const RESULT_1_FMT: &[u8] = b"result_1 = %d\n\0";
const RESULT_2_FMT: &[u8] = b"result_2 = %d\n\0";
const CONVERTED_VALUES_FMT: &[u8] = b"Converted values: %d, %d, %d, %d\n\0";
const SWITCH_RESULT_FMT: &[u8] = b"Switch fall-through result: %d\n\0";
const COPIED_BLOCK_FMT: &[u8] = b"Copied block: id=%d, value=%.2f, label=%s\n\0";
const POINTER_RESULT_FMT: &[u8] = b"Pointer operation result: %d\n\0";
const OVERFLOW_FMT: &[u8] = b"Overflow protected conversion: %d\n\0";
const UNDERFLOW_FMT: &[u8] = b"Underflow protected conversion: %d\n\0";
const ARRAY_PREFIX_FMT: &[u8] = b"Array copied via memcpy: \0";
const ARRAY_ITEM_FMT: &[u8] = b"%d \0";
const NEWLINE_FMT: &[u8] = b"\n\0";
const SOURCE_LABEL: &[u8] = b"Source\0";

#[inline]
unsafe fn c_printf_1(fmt: &[u8], arg1: c_int) {
    printf(fmt.as_ptr().cast(), arg1);
}

#[inline]
unsafe fn c_printf_4(fmt: &[u8], arg1: c_int, arg2: c_int, arg3: c_int, arg4: c_int) {
    printf(fmt.as_ptr().cast(), arg1, arg2, arg3, arg4);
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
    memcpy(
        dest.cast::<c_void>(),
        src.cast::<c_void>(),
        core::mem::size_of::<DataBlock>(),
    );
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
        c_printf_1(RESULT_1_FMT, result_1);
        c_printf_1(RESULT_2_FMT, result_2);
    }

    let temp1 = (a as c_double) * 1.5f64;
    let temp2 = (b as c_double) * 2.7f64;
    let temp3 = (c as c_double) / 3.3f64;
    let dd = d.wrapping_mul(d);
    let aa = a.wrapping_mul(a);
    let temp4 = (dd.wrapping_add(aa) as c_double).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    unsafe {
        c_printf_4(CONVERTED_VALUES_FMT, conv1, conv2, conv3, conv4);
    }

    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        c_printf_1(SWITCH_RESULT_FMT, switch_result);
    }

    let mut source_block = DataBlock {
        id: a,
        value: temp1,
        label: [0; 20],
    };
    for (dst, src) in source_block
        .label
        .iter_mut()
        .zip(SOURCE_LABEL[..SOURCE_LABEL.len() - 1].iter().copied())
    {
        *dst = src as c_char;
    }
    source_block.label[19] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    unsafe {
        copy_data_block(&mut dest_block, &source_block);
        printf(
            COPIED_BLOCK_FMT.as_ptr().cast(),
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result = handle_pointer_operations(c);
    unsafe {
        c_printf_1(POINTER_RESULT_FMT, ptr_result);
    }

    total = total.wrapping_add(conv1);
    total = total.wrapping_add(conv2);
    total = total.wrapping_add(conv3);
    total = total.wrapping_add(conv4);
    total = total.wrapping_add(switch_result);
    total = total.wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test = 1e15f64;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        c_printf_1(OVERFLOW_FMT, safe_conv);
    }

    let underflow_test = -1e15f64;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        c_printf_1(UNDERFLOW_FMT, safe_conv2);
    }

    let array1 = [a, b, c, d, a.wrapping_add(b)];
    let mut array2 = [0; 5];
    unsafe {
        memcpy(
            array2.as_mut_ptr().cast::<c_void>(),
            array1.as_ptr().cast::<c_void>(),
            core::mem::size_of_val(&array1),
        );

        printf(ARRAY_PREFIX_FMT.as_ptr().cast());
        for item in array2 {
            printf(ARRAY_ITEM_FMT.as_ptr().cast(), item);
            total = total.wrapping_add(item);
        }
        printf(NEWLINE_FMT.as_ptr().cast());
    }

    total
}
