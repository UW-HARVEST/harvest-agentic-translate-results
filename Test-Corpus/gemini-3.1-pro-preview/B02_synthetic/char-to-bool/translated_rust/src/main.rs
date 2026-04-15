use std::io::{self, BufRead};

fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    let operation_str = match lines.next() {
        Some(Ok(line)) => line,
        _ => {
            eprintln!("Error reading operation");
            std::process::exit(1);
        }
    };
    let operation: i32 = operation_str.trim().parse().unwrap_or(0);

    let param_str = match lines.next() {
        Some(Ok(line)) => line,
        _ => {
            eprintln!("Error reading parameter");
            std::process::exit(1);
        }
    };
    let param: i32 = param_str.trim().parse().unwrap_or(0);

    let decision_string = match lines.next() {
        Some(Ok(line)) => line,
        _ => {
            eprintln!("Error reading decision string");
            std::process::exit(1);
        }
    };

    let result = driver::process_decisions(decision_string.as_bytes(), operation, param);

    println!("{}", result);
}
