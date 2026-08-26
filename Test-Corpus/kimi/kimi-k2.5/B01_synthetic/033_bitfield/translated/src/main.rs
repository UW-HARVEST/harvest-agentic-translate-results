use std::io;

struct Foo {
    x: u8,
    y: u8,
    b: bool,
    z: i32,
}

fn print_foo(foo: &Foo) {
    println!("{} {} {} {}", foo.x, foo.y, foo.b as i32, foo.z);
}

fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = Foo {
        x: (x & 0x3) as u8,
        y: (y & 0x7) as u8,
        b,
        z,
    };
    print_foo(&foo);
}

fn main() {
    let mut input = String::new();
    
    io::stdin().read_line(&mut input).unwrap();
    let x: u32 = input.trim().parse().unwrap();
    input.clear();
    
    io::stdin().read_line(&mut input).unwrap();
    let y: u32 = input.trim().parse().unwrap();
    input.clear();
    
    io::stdin().read_line(&mut input).unwrap();
    let b: i32 = input.trim().parse().unwrap();
    input.clear();
    
    io::stdin().read_line(&mut input).unwrap();
    let z: i32 = input.trim().parse().unwrap();
    
    driver(x, y, b != 0, z);
}