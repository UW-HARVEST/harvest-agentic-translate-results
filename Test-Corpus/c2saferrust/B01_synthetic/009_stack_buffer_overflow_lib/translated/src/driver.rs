






extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(intNumber: i32) {
    println!("{}", intNumber);
}

#[no_mangle]
pub fn bad(data: i32) {
    let mut buffer = [0i32; 10];

    if data < 0 {
        printLine("ERROR: Array index is negative.");
        return;
    }

    let index = data as usize;
    if index >= buffer.len() {
        printLine("ERROR: Array index is out-of-bounds");
        return;
    }

    buffer[index] = 1;

    for &value in &buffer {
        printIntLine(value);
    }
}

fn goodG2B() {
    let data: i32 = 7;
    let mut buffer = [0i32; 10];

    if data >= 0 {
        buffer[data as usize] = 1;
        for &value in &buffer {
            printIntLine(value);
        }
    } else {
        printLine("ERROR: Array index is negative.");
    }
}

fn goodB2G(data: i32) {
    let mut buffer = [0i32; 10];

    if (0..10).contains(&data) {
        buffer[data as usize] = 1;
        for value in buffer {
            printIntLine(value);
        }
    } else {
        printLine("ERROR: Array index is out-of-bounds");
    }
}

#[no_mangle]
pub fn good(data: i32) {
    goodG2B();
    goodB2G(data);
}

#[no_mangle]
pub fn driver(good_data: i32, bad_data: i32) {
    printLine("Calling good()...");
    good(good_data);
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad(bad_data);
    printLine("Finished bad()");
}

