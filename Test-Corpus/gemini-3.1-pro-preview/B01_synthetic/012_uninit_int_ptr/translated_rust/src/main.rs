use std::io;

fn print_int_ptr_line(int_number: &i32) {
    println!("{}", int_number);
}

fn bad() {
    let data: *const i32 = std::ptr::null();
    unsafe {
        print_int_ptr_line(&*data);
    }
}

fn good() {
    let data = 5;
    let data_addr = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(parsed) = input.trim().parse::<i32>() {
            x = parsed;
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
