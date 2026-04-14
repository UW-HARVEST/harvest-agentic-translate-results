use std::io::{self, Read};

#[derive(Clone, Copy)]
struct Foo {
    x: u32,
    y: u32,
    b: bool,
    z: i32,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x & 0b11, foo.y & 0b111, if foo.b { 1 } else { 0 }, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 0b11,
        y: y & 0b111,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut it = input.split_whitespace();

    let x: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: u32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let b: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let z: i32 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    driver(x, y, b != 0, z);
}
