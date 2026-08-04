use std::io;

fn print_int_ptr_line(int_number: &i32) {
    println!("{}", int_number);
}

fn bad() {
    let data: &i32;
    print_int_ptr_line(data);
}

fn good() {
    let data: i32 = 5;
    let data_addr: &i32 = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: i32 = 0;
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    x = input.trim().parse().unwrap();

    if x != 0 {
        good();
    } else {
        bad();
    }
}
