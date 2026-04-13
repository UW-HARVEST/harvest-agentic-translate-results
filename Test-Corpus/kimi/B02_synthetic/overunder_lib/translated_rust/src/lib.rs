use std::ffi::{c_char, c_double, c_int, CStr, CString};
use std::os::raw::c_void;
use std::ptr;

#[repr(C)]
pub struct DataBlock {
    pub id: c_int,
    pub value: c_double,
    pub label: [c_char; 20],
}

fn safe_double_to_int(d: c_double) -> c_int {
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

fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result = base_value;

    match code {
        5 => {
            result += 50;
            result += 40;
            result += 30;
            result += 20;
            result += 10;
        }
        4 => {
            result += 40;
            result += 30;
            result += 20;
            result += 10;
        }
        3 => {
            result += 30;
            result += 20;
            result += 10;
        }
        2 => {
            result += 20;
            result += 10;
        }
        1 => {
            result += 10;
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

fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe {
        ptr::copy_nonoverlapping(src, dest, 1);
    }
}

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value * 2;
    let result = local_value + 100;
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1: c_int = a;
    let result_2: c_int = b;
    let result_3: c_int = c;
    let result_4: c_int = d;

    println!("result_1 = {}", result_1);
    println!("result_2 = {}", result_2);

    let temp1: c_double = a as c_double * 1.5;
    let temp2: c_double = b as c_double * 2.7;
    let temp3: c_double = c as c_double / 3.3;
    let temp4: c_double = ((d * d + a * a) as c_double).sqrt();

    let conv1: c_int = safe_double_to_int(temp1);
    let conv2: c_int = safe_double_to_int(temp2);
    let conv3: c_int = safe_double_to_int(temp3);
    let conv4: c_int = safe_double_to_int(temp4);

    println!("Converted values: {}, {}, {}, {}", conv1, conv2, conv3, conv4);

    let switch_result: c_int = process_with_fallthrough(a % 6, b);
    println!("Switch fall-through result: {}", switch_result);

    let mut source_block: DataBlock = unsafe { std::mem::zeroed() };
    source_block.id = a;
    source_block.value = temp1;
    let label_str = CString::new("Source").unwrap();
    let label_bytes = label_str.as_bytes_with_nul();
    let copy_len = label_bytes.len().min(19);
    unsafe {
        ptr::copy_nonoverlapping(
            label_bytes.as_ptr() as *const c_char,
            source_block.label.as_mut_ptr(),
            copy_len,
        );
        source_block.label[copy_len] = 0;
    }

    let mut dest_block: DataBlock = unsafe { std::mem::zeroed() };
    copy_data_block(&mut dest_block, &source_block);

    let label_cstr = unsafe {
        CStr::from_ptr(dest_block.label.as_ptr())
    };
    let label_str = label_cstr.to_str().unwrap_or("");
    println!(
        "Copied block: id={}, value={:.2}, label={}",
        dest_block.id, dest_block.value, label_str
    );

    let ptr_result: c_int = handle_pointer_operations(c);
    println!("Pointer operation result: {}", ptr_result);

    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += dest_block.id;

    let overflow_test: c_double = 1e15;
    let safe_conv: c_int = safe_double_to_int(overflow_test);
    println!("Overflow protected conversion: {}", safe_conv);

    let underflow_test: c_double = -1e15;
    let safe_conv2: c_int = safe_double_to_int(underflow_test);
    println!("Underflow protected conversion: {}", safe_conv2);

    let array1: [c_int; 5] = [a, b, c, d, a + b];
    let mut array2: [c_int; 5] = [0; 5];

    array2.copy_from_slice(&array1);

    print!("Array copied via memcpy: ");
    for i in 0..5 {
        print!("{} ", array2[i]);
        total += array2[i];
    }
    println!();

    total
}
