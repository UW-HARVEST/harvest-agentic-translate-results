use std::io::{self, BufRead};

fn print_line(line: &str) {
    println!("{}", line);
}

fn main() {
    let mut data: i32 = -1;
    {
        let stdin = io::stdin();
        let mut input_buffer = String::new();
        if stdin.lock().read_line(&mut input_buffer).is_ok() {
            if let Ok(val) = input_buffer.trim().parse::<i32>() {
                data = val;
            }
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let source: Vec<u8> = vec![b'A'; 99];
        let mut dest = String::new();
        if data < 100 && data >= 0 {
            dest = String::from_utf8_lossy(&source[..data as usize]).to_string();
        }
        print_line(&dest);
    }
}
