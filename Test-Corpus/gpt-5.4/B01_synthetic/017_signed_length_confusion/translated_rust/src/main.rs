use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn main() {
    let mut data: i32 = -1;
    {
        let mut input_buffer = String::new();
        if io::stdin().read_line(&mut input_buffer).is_ok() {
            let truncated: String = input_buffer.chars().take(13).collect();
            data = truncated.trim_end_matches(['\n', '\r']).parse::<i32>().unwrap_or(0);
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let source = "A".repeat(99);
        let mut dest = String::new();
        if data < 100 {
            let count = if data < 0 { 0 } else { data as usize };
            dest = source.chars().take(count).collect();
        }
        print_line(&dest);
    }
}
