use std::ffi::c_int;

fn safe_double_to_int(d: f64) -> c_int {
    if d > i32::MAX as f64 {
        i32::MAX
    } else if d < i32::MIN as f64 {
        i32::MIN
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

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value * 2;
    local_value + 100
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    // PRINT_VAR(result_1) -> printf("result_1" " = %d\n", result_1)
    print!("result_1 = {}\n", result_1);
    print!("result_2 = {}\n", result_2);

    let temp1 = a as f64 * 1.5;
    let temp2 = b as f64 * 2.7;
    let temp3 = c as f64 / 3.3;
    let temp4 = ((d * d + a * a) as f64).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    print!("Converted values: {}, {}, {}, {}\n", conv1, conv2, conv3, conv4);

    let switch_result = process_with_fallthrough(a % 6, b);
    print!("Switch fall-through result: {}\n", switch_result);

    let source_id = a;
    let source_value = temp1;
    let source_label = "Source";

    print!(
        "Copied block: id={}, value={:.2}, label={}\n",
        source_id, source_value, source_label
    );

    let ptr_result = handle_pointer_operations(c);
    print!("Pointer operation result: {}\n", ptr_result);

    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += source_id;

    let overflow_test: f64 = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    print!("Overflow protected conversion: {}\n", safe_conv);

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    print!("Underflow protected conversion: {}\n", safe_conv2);

    let array2 = [a, b, c, d, a + b];

    print!("Array copied via memcpy: ");
    for i in 0..5 {
        print!("{} ", array2[i]);
        total += array2[i];
    }
    print!("\n");

    total
}
