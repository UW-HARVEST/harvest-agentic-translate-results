use std::os::raw::c_int;

pub fn printLine(line: &str) {
    println!("{}", line);
}

pub fn printIntLine(int_number: c_int) {
    println!("{}", int_number);
}

pub fn bad(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine("ERROR: Array index is negative.");
    }
}

fn goodG2B() {
    let data: c_int = 7;
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine("ERROR: Array index is negative.");
    }
}

fn goodB2G(data: c_int) {
    let mut buffer: [c_int; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            printIntLine(buffer[i]);
        }
    } else {
        printLine("ERROR: Array index is out-of-bounds");
    }
}

pub fn good(data: c_int) {
    goodG2B();
    goodB2G(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(goodData: c_int, badData: c_int) {
    printLine("Calling good()...");
    good(goodData);
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad(badData);
    printLine("Finished bad()");
}
