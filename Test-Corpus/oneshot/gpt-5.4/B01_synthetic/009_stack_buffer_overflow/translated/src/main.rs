use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn read_input_data() -> Option<i32> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => Some(input.trim_end_matches(['\r', '\n']).parse::<i32>().unwrap_or(0)),
        Err(_) => {
            print_line("fgets() failed.");
            None
        }
    }
}

fn bad() {
    let mut data: i32 = -1;
    if let Some(value) = read_input_data() {
        data = value;
    }
    let mut buffer = [0i32; 10];
    if data >= 0 {
        let index = data as usize;
        if index < buffer.len() {
            buffer[index] = 1;
            for value in buffer {
                print_int_line(value);
            }
        } else {
            panic!("index out of bounds");
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_g2b() {
    let data: i32 = 7;
    let mut buffer = [0i32; 10];
    if data >= 0 {
        let index = data as usize;
        buffer[index] = 1;
        for value in buffer {
            print_int_line(value);
        }
    } else {
        print_line("ERROR: Array index is negative.");
    }
}

fn good_b2g() {
    let mut data: i32 = -1;
    if let Some(value) = read_input_data() {
        data = value;
    }
    let mut buffer = [0i32; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for value in buffer {
            print_int_line(value);
        }
    } else {
        print_line("ERROR: Array index is out-of-bounds");
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
