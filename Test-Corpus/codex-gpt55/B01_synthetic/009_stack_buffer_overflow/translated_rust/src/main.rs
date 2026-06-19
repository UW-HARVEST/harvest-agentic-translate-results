use std::io::{self, Read};
use std::os::raw::{c_char, c_int};

extern "C" {
    fn atoi(nptr: *const c_char) -> c_int;
}

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

struct FgetsReader<R> {
    input: R,
}

impl<R: Read> FgetsReader<R> {
    fn fgets_14(&mut self) -> Option<[c_char; 14]> {
        let mut input_buffer = [0 as c_char; 14];
        let mut written = 0usize;
        while written < 13 {
            let mut byte_buffer = [0u8; 1];
            match self.input.read(&mut byte_buffer) {
                Ok(0) => break,
                Ok(_) => {}
                Err(_) => return None,
            }
            let byte = byte_buffer[0];
            input_buffer[written] = byte as c_char;
            written += 1;
            if byte == b'\n' {
                break;
            }
        }

        if written == 0 {
            None
        } else {
            Some(input_buffer)
        }
    }
}

fn atoi_buffer(input_buffer: &[c_char; 14]) -> c_int {
    unsafe { atoi(input_buffer.as_ptr()) }
}

fn bad_sink(data: c_int) {
    let mut buffer = [0 as c_int; 10];
    if data >= 0 {
        unsafe {
            *buffer.as_mut_ptr().offset(data as isize) = 1;
        }
        for value in buffer {
            print_int_line(value);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_sink(data: c_int) {
    let mut buffer = [0 as c_int; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for value in buffer {
            print_int_line(value);
        }
    } else {
        print_line("ERROR: Array index is out-of-bounds");
    }
}

fn bad<R: Read>(reader: &mut FgetsReader<R>) {
    let mut data = -1;
    if let Some(input_buffer) = reader.fgets_14() {
        data = atoi_buffer(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }
    bad_sink(data);
}

fn good_g2b() {
    let data = 7;
    bad_sink(data);
}

fn good_b2g<R: Read>(reader: &mut FgetsReader<R>) {
    let mut data = -1;
    if let Some(input_buffer) = reader.fgets_14() {
        data = atoi_buffer(&input_buffer);
    } else {
        print_line("fgets() failed.");
    }
    good_sink(data);
}

fn good<R: Read>(reader: &mut FgetsReader<R>) {
    good_g2b();
    good_b2g(reader);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = FgetsReader {
        input: stdin.lock(),
    };

    print_line("Calling good()...");
    good(&mut reader);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut reader);
    print_line("Finished bad()");
}
