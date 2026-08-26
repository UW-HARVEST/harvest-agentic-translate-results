use std::io::{self, Read};

struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 3,
        y: y & 7,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut tokens = input.split_whitespace();

    let mut x: u32 = 0;
    let mut y: u32 = 0;
    let mut b: i32 = 0;
    let mut z: i32 = 0;

    if let Some(s) = tokens.next() {
        if let Ok(val) = s.parse() {
            x = val;
        }
    }
    if let Some(s) = tokens.next() {
        if let Ok(val) = s.parse() {
            y = val;
        }
    }
    if let Some(s) = tokens.next() {
        if let Ok(val) = s.parse() {
            b = val;
        }
    }
    if let Some(s) = tokens.next() {
        if let Ok(val) = s.parse() {
            z = val;
        }
    }

    driver(x, y, b != 0, z);
}
