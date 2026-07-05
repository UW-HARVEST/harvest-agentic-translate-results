






extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn convert_double_to_int(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f64 {
        i32::MAX
    } else if value <= i32::MIN as f64 {
        i32::MIN
    } else {
        value as i32
    }
}

#[no_mangle]
pub fn find_value_in_buffer(
    buffer: *const ::core::ffi::c_char,
    size: size_t,
    search_val: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if buffer.is_null() {
        return -1;
    }

    let target = search_val as u8;
    let bytes = unsafe { ::core::slice::from_raw_parts(buffer as *const u8, size as usize) };

    bytes.iter()
        .position(|&b| b == target)
        .map(|idx| idx as ::core::ffi::c_int)
        .unwrap_or(-1)
}

#[no_mangle]
pub fn process_negation(var1: i32) -> i32 {
    (var1 != 0) as i32
}

#[no_mangle]
pub fn create_numeric_buffer(buffer: &mut [i8], seed: i32) {
    for (i, byte) in buffer.iter_mut().enumerate() {
        *byte = ((seed + i as i32 * 7) % 256) as i8;
    }
}

#[no_mangle]
pub fn calculate_with_doubles(a: i32, b: i32, c: i32) -> f64 {
    let mut result = 0.0_f64;
    if b != 0 {
        result = a as f64 / b as f64;
    }
    result *= 10.0_f64.powf((c % 10) as f64);
    result
}

#[no_mangle]
pub fn doubleneg(
    mut param1: i32,
    mut param2: i32,
    mut param3: i32,
    mut param4: i32,
) -> i32 {
    let mut result: i32 = 0;
let mut buffer = [0i8; 256];
let mut i: i32 = 0;

println!("=== Starting foo() execution ===");
println!("Parameters: {}, {}, {}, {}", param1, param2, param3, param4);

println!();
println!("--- Integer Negation Test ---");
let mut negation_test: i32 = param1;
let mut negation_result: i32 = (negation_test != 0) as i32;
println!("Original value: {}", negation_test);
println!("After !!negation: {}", negation_result);
result += negation_result * 10;

let mut neg_p2: i32 = (param2 != 0) as i32;
let mut neg_p3: i32 = (param3 != 0) as i32;
let mut neg_p4: i32 = (param4 != 0) as i32;
println!("Double negation results: {}, {}, {}", neg_p2, neg_p3, neg_p4);
result += neg_p2 + neg_p3 + neg_p4;

println!();
println!("--- Double to Int Conversion Test ---");
let mut large_double: f64 = calculate_with_doubles(param1, param2, param3);
println!("Calculated double value: {:e}", large_double);

let mut converted_int: i32 = convert_double_to_int(large_double);
println!("Converted to int (may be UB): {}", converted_int);

let mut negative_large: f64 = -1.0f64 * 2.0f64.powi(40);
println!("Very large negative double: {:e}", negative_large);

let mut converted_neg: i32 = convert_double_to_int(negative_large);
println!("Converted to int (UB likely): {}", converted_neg);

result += converted_int % 1000 + converted_neg % 1000;

println!();
println!("--- Memchr Search Test ---");
create_numeric_buffer(&mut buffer, param1);

let mut search_values: [i32; 4] = [param2 % 256, param3 % 256, param4 % 256, 42];
let mut num_searches: i32 = search_values.len() as i32;

println!("Searching buffer for values...");
i = 0;
while i < num_searches {
    let mut pos: i32 = find_value_in_buffer(buffer.as_ptr(), 256usize, search_values[i as usize]);
    if pos >= 0 {
        println!("Found value {} at position {}", search_values[i as usize], pos);
        result += pos;
    } else {
        println!("Value {} not found", search_values[i as usize]);
    }
    i += 1;
}

let mut direct_search = buffer.iter().position(|&b| b == 100i8);
if let Some(offset) = direct_search {
    println!("Direct memchr found byte 100 at offset: {}", offset);
    result += offset as i32;
}

println!();
println!("--- Combined Feature Test ---");
i = 0;
while i < 10 {
    let mut search_byte: i32 = (param1 + i * param2) % 256;
    let mut found_flag: i32 = buffer
        .iter()
        .any(|&b| b == search_byte as i8) as i32;
    println!("Search {}: byte={}, found={}", i, search_byte, found_flag);
    result += found_flag;
    i += 1;
}

let mut infinity_val: f64 = f64::INFINITY;
let mut nan_val: f64 = f64::NAN;

println!();
println!("--- Special Double Values ---");
print!("Converting INFINITY to int: ");
let mut inf_as_int: i32 = convert_double_to_int(infinity_val);
println!("{} (undefined behavior)", inf_as_int);

print!("Converting NAN to int: ");
let mut nan_as_int: i32 = convert_double_to_int(nan_val);
println!("{} (undefined behavior)", nan_as_int);

println!();
println!("=== Final Result ===");


    println!("Accumulated result: {}", result);
    result
}

