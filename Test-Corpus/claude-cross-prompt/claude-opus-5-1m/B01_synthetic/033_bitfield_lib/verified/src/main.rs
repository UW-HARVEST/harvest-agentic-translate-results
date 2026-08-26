use std::io::{self, Read};

struct Foo {
    x: u32, // bitfield: unsigned int : 2
    y: u32, // bitfield: unsigned int : 3
    b: bool, // bitfield: bool : 1
    z: i32,
}

fn print_foo(foo: &Foo) {
    // Match C printf("%u %u %d %d\n", foo->x, foo->y, foo->b, foo->z);
    let b_int: i32 = if foo.b { 1 } else { 0 };
    println!("{} {} {} {}", foo.x, foo.y, b_int, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    // Emulate C bitfield truncation behavior (GCC/Clang).
    // x : 2 bits, y : 3 bits, b : 1 bit (bool stays 0 or 1), z : full int
    let foo = Foo {
        x: x & 0x3,
        y: y & 0x7,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    // Read all of stdin and parse whitespace-separated tokens (scanf-like).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        std::process::exit(1);
    }
    let mut iter = input.split_ascii_whitespace();

    // Read x as unsigned int (matches C scanf "%u" wrap-around semantics on signed input).
    let x_tok = match iter.next() {
        Some(t) => t,
        None => std::process::exit(1),
    };
    let x: u32 = match x_tok.parse::<u32>() {
        Ok(v) => v,
        Err(_) => match x_tok.parse::<i64>() {
            Ok(v) => v as u32,
            Err(_) => std::process::exit(1),
        },
    };

    let y_tok = match iter.next() {
        Some(t) => t,
        None => std::process::exit(1),
    };
    let y: u32 = match y_tok.parse::<u32>() {
        Ok(v) => v,
        Err(_) => match y_tok.parse::<i64>() {
            Ok(v) => v as u32,
            Err(_) => std::process::exit(1),
        },
    };

    let b_tok = match iter.next() {
        Some(t) => t,
        None => std::process::exit(1),
    };
    let b_val: i32 = match b_tok.parse::<i32>() {
        Ok(v) => v,
        Err(_) => std::process::exit(1),
    };
    let b: bool = b_val != 0;

    let z_tok = match iter.next() {
        Some(t) => t,
        None => std::process::exit(1),
    };
    let z: i32 = match z_tok.parse::<i32>() {
        Ok(v) => v,
        Err(_) => match z_tok.parse::<i64>() {
            Ok(v) => v as i32,
            Err(_) => std::process::exit(1),
        },
    };

    driver(x, y, b, z);
}
