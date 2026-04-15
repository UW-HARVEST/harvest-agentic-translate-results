use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn main() {
    let mut data: i32 = -1;
    {
        let mut input_buffer = String::new();
        if io::stdin().read_line(&mut input_buffer).is_ok() && !input_buffer.is_empty() {
            data = input_buffer.trim().parse().unwrap_or(0);
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let source = "A".repeat(99);
        let mut dest = String::new();
        if data < 100 {
            let data_usize = data as usize;
            if data_usize <= source.len() {
                dest.push_str(&source[..data_usize]);
            } else {
                dest.push_str(&source);
            }
        }
        print_line(&dest);
    }
}
