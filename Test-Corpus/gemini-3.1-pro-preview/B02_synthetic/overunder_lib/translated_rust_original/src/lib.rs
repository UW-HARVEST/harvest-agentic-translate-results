use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DataBlock {
    pub id: c_int,
    pub value: f64,
    pub label: [c_char; 20],
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

fn copy_data_block(dest: &mut DataBlock, src: &DataBlock) {
    *dest = *src;
}

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value * 2;
    let ptr = &local_value;
    *ptr + 100
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total = 0;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    println!("result_1 = {}", result_1);
    println!("result_2 = {}", result_2);

    let temp1 = a as f64 * 1.5;
    let temp2 = b as f64 * 2.7;
    let temp3 = c as f64 / 3.3;
    let temp4 = ((d * d + a * a) as f64).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    println!("Converted values: {}, {}, {}, {}", conv1, conv2, conv3, conv4);

    let switch_result = process_with_fallthrough(a % 6, b);
    println!("Switch fall-through result: {}", switch_result);

    let mut source_block = DataBlock {
        id: a,
        value: temp1,
        label: [0; 20],
    };
    
    let src_bytes = b"Source";
    let len = src_bytes.len().min(19);
    for i in 0..len {
        source_block.label[i] = src_bytes[i] as c_char;
    }
    source_block.label[19] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    copy_data_block(&mut dest_block, &source_block);

    let label_cstr = unsafe { CStr::from_ptr(dest_block.label.as_ptr()) };
    println!("Copied block: id={}, value={:.2}, label={}", 
             dest_block.id, dest_block.value, label_cstr.to_string_lossy());

    let ptr_result = handle_pointer_operations(c);
    println!("Pointer operation result: {}", ptr_result);

    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += dest_block.id;

    let overflow_test = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    println!("Overflow protected conversion: {}", safe_conv);

    let underflow_test = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
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
