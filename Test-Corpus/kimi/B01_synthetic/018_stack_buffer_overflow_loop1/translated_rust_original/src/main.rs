use std::io;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let mut data: [i32; 10] = [0; 10];
    {
        let source: [i32; 10] = [0; 10];
        for i in 0..10 {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

fn good() {
    let mut data: Box<[i32]> = vec![0; 10].into_boxed_slice();
    {
        let source: [i32; 10] = [0; 10];
        for i in 0..10 {
            data[i] = source[i];
        }
        print_int_line(data[0]);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
