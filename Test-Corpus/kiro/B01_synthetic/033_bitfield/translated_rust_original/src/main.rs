use std::io::{self, Read};

struct Foo {
    x: u32,
    y: u32,
    b: i32,
    z: i32,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: x & 0x3,
        y: y & 0x7,
        b: b as i32,
        z,
    };
    print_foo(&foo);
}

/// Read whitespace-delimited tokens from stdin (matching scanf behavior)
fn scanf_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let tokens: Vec<&str> = scanf_tokens(&input);

    let x: u32 = tokens.get(0).map_or(0, |s| s.parse().unwrap_or(0));
    let y: u32 = tokens.get(1).map_or(0, |s| s.parse().unwrap_or(0));
    let b: i32 = tokens.get(2).map_or(0, |s| s.parse().unwrap_or(0));
    let z: i32 = tokens.get(3).map_or(0, |s| s.parse().unwrap_or(0));

    driver(x, y, b != 0, z);
}
