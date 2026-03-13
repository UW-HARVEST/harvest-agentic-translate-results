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
        x: x & 0x3,       // 2-bit unsigned
        y: y & 0x7,       // 3-bit unsigned
        b: b as i32,      // 1-bit bool printed as %d
        z,
    };
    print_foo(&foo);
}

fn scanf_tokens() -> Vec<String> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    input.split_whitespace().map(String::from).collect()
}

fn main() {
    let tokens = scanf_tokens();
    let x: u32 = tokens[0].parse().unwrap();
    let y: u32 = tokens[1].parse().unwrap();
    let b: i32 = tokens[2].parse().unwrap();
    let z: i32 = tokens[3].parse().unwrap();
    driver(x, y, b != 0, z);
}
