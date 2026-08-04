
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read, Write};

struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

impl Foo {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        // Emulate bitfield truncation: x is 2 bits, y is 3 bits, b is 1 bit
        Foo {
            x: x & 0x3,
            y: y & 0x7,
            b,
            z,
        }
    }

    fn print(&self) {
        let b_int = self.b as i32;
        println!("{} {} {} {}", self.x, self.y, b_int, self.z);
    }
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo::new(x, y, b, z);
    foo.print();
}

fn read_all_stdin() -> String {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    buf
}

fn run() -> io::Result<()> {
    let input = read_all_stdin();
    let mut tokens = input.split_ascii_whitespace();

    let x: u32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
    let y: u32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
    let b: i32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
    let z: i32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);

    driver(x, y, b != 0, z);
    io::stdout().flush()?;
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    match run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}