use std::io::{self, BufRead, Write};

use driver::process_decisions;

const MAX_INPUT_SIZE: usize = 1024;

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut input_buffer = String::with_capacity(MAX_INPUT_SIZE);

    input_buffer.clear();
    if reader.read_line(&mut input_buffer).unwrap_or(0) == 0 {
        let _ = writeln!(io::stderr(), "Error reading operation");
        std::process::exit(1);
    }
    let operation = input_buffer.trim_end().parse::<i32>().unwrap_or(0);

    input_buffer.clear();
    if reader.read_line(&mut input_buffer).unwrap_or(0) == 0 {
        let _ = writeln!(io::stderr(), "Error reading parameter");
        std::process::exit(1);
    }
    let param = input_buffer.trim_end().parse::<i32>().unwrap_or(0);

    input_buffer.clear();
    if reader.read_line(&mut input_buffer).unwrap_or(0) == 0 {
        let _ = writeln!(io::stderr(), "Error reading decision string");
        std::process::exit(1);
    }

    if input_buffer.ends_with('\n') {
        input_buffer.pop();
        if input_buffer.ends_with('\r') {
            input_buffer.pop();
        }
    }

    if input_buffer.len() > MAX_INPUT_SIZE {
        input_buffer.truncate(MAX_INPUT_SIZE);
    }

    let len = input_buffer.len();
    let result = process_decisions(input_buffer.as_mut_str(), len, operation, param);

    println!("{}", result);
}
