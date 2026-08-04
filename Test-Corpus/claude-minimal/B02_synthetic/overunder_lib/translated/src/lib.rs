// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

#[derive(Clone, Copy)]
struct DataBlock {
    id: i32,
    value: f64,
    label: [u8; 20],
}

fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        // Match the C order: NaN comparisons against INT_MAX/INT_MIN are false,
        // so it would fall through to the isnan check.
        return 0;
    }
    if d > i32::MAX as f64 {
        i32::MAX
    } else if d < i32::MIN as f64 {
        i32::MIN
    } else {
        d as i32
    }
}

fn process_with_fallthrough(code: i32, base_value: i32) -> i32 {
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

fn handle_pointer_operations(value: i32) -> i32 {
    let local_value = value.wrapping_mul(2);
    let ptr: &i32 = &local_value;
    *ptr + 100
}

fn print_label(label: &[u8]) -> String {
    let end = label.iter().position(|&c| c == 0).unwrap_or(label.len());
    String::from_utf8_lossy(&label[..end]).into_owned()
}

#[no_mangle]
pub extern "C" fn overunder(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let mut total: i32 = 0;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    println!("result_1 = {}", result_1);
    println!("result_2 = {}", result_2);

    let temp1: f64 = a as f64 * 1.5;
    let temp2: f64 = b as f64 * 2.7;
    let temp3: f64 = c as f64 / 3.3;
    let dd = (d as i64).wrapping_mul(d as i64) as i32 as i64;
    let aa = (a as i64).wrapping_mul(a as i64) as i32 as i64;
    // Mimic C's int*int arithmetic with wrapping then convert to double
    let d_sq = (d.wrapping_mul(d)) as f64;
    let a_sq = (a.wrapping_mul(a)) as f64;
    let _ = dd;
    let _ = aa;
    let temp4: f64 = (d_sq + a_sq).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    println!(
        "Converted values: {}, {}, {}, {}",
        conv1, conv2, conv3, conv4
    );

    // C uses `a % 6` which is truncated modulo (same as Rust's `%`).
    let switch_result = process_with_fallthrough(a % 6, b);
    println!("Switch fall-through result: {}", switch_result);

    let mut source_block = DataBlock {
        id: a,
        value: temp1,
        label: [0u8; 20],
    };
    let label_str = b"Source";
    let copy_len = label_str.len().min(source_block.label.len() - 1);
    source_block.label[..copy_len].copy_from_slice(&label_str[..copy_len]);
    source_block.label[source_block.label.len() - 1] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0u8; 20],
    };
    copy_data_block(&mut dest_block, &source_block);

    println!(
        "Copied block: id={}, value={:.2}, label={}",
        dest_block.id,
        dest_block.value,
        print_label(&dest_block.label)
    );

    let ptr_result = handle_pointer_operations(c);
    println!("Pointer operation result: {}", ptr_result);

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
    println!("Overflow protected conversion: {}", safe_conv);

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    println!("Underflow protected conversion: {}", safe_conv2);

    let array1: [i32; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [i32; 5] = [0; 5];

    array2.copy_from_slice(&array1);

    print!("Array copied via memcpy: ");
    for i in 0..5 {
        print!("{} ", array2[i]);
        total = total.wrapping_add(array2[i]);
    }
    println!();

    total
}
