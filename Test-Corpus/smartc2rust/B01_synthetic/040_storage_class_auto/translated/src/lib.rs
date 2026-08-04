
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn rust_driver(x: i32) {
    let y = 2 * x + 300;
    println!("{}", y);
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let mut input = String::new();
    let x = if std::io::stdin().read_line(&mut input).is_ok() {
        input.trim().parse::<i32>().unwrap_or(0)
    } else {
        0
    };
    rust_driver(x);
    0
}