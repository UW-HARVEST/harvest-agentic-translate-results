use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

#[repr(C)]
#[derive(Copy, Clone)]
struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

fn safe_double_to_int(d: c_double) -> c_int {
    if d > INT_MAX as c_double {
        INT_MAX
    } else if d < INT_MIN as c_double {
        INT_MIN
    } else if d.is_nan() {
        0
    } else {
        d as c_int
    }
}

fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
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

fn copy_data_block(dest: &mut DataBlock, src: &DataBlock) {
    *dest = *src;
}

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);
    let ptr: *const c_int = &local_value;
    // Reproduce *ptr + 100
    let result = unsafe { *ptr }.wrapping_add(100);
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int;
    let _ = 0i32; // mirror C's int total = 0; (overwritten below)

    let result_1: c_int = a;
    let result_2: c_int = b;
    let result_3: c_int = c;
    let result_4: c_int = d;

    // PRINT_VAR(result_1) -> printf("result_1 = %d\n", result_1);
    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
        printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    // Suppress unused warnings; keep semantics identical by computing them
    let _ = result_3;
    let _ = result_4;

    let temp1: c_double = a as c_double * 1.5;
    let temp2: c_double = b as c_double * 2.7;
    let temp3: c_double = c as c_double / 3.3;
    // d * d + a * a -- C int multiplication (may overflow, wrapping)
    let dd = d.wrapping_mul(d);
    let aa = a.wrapping_mul(a);
    let sum = dd.wrapping_add(aa);
    let temp4: c_double = (sum as c_double).sqrt();

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

    let switch_result = process_with_fallthrough(a.rem_euclid_c(6), b);
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

    // strncpy(source_block.label, "Source", sizeof(label) - 1)
    let src_str = b"Source";
    let n = source_block.label.len() - 1;
    // strncpy copies up to n bytes; if src is shorter, pads with NULs
    for i in 0..n {
        if i < src_str.len() {
            source_block.label[i] = src_str[i] as c_char;
        } else {
            source_block.label[i] = 0;
        }
    }
    // label[sizeof(label)-1] = '\0'
    source_block.label[source_block.label.len() - 1] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    copy_data_block(&mut dest_block, &source_block);

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

    total = conv1
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

    // memcpy(array2, array1, sizeof(array1));
    array2.copy_from_slice(&array1);

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

// Trait extension to provide C-style modulo (truncated division remainder)
trait CModulo {
    fn rem_euclid_c(self, rhs: Self) -> Self;
}

impl CModulo for c_int {
    fn rem_euclid_c(self, rhs: c_int) -> c_int {
        // C's % operator is truncated remainder, which Rust's % also does for integers.
        self % rhs
    }
}
