use std::io::{self, Read};

#[repr(C)]
pub struct foo_t {
    _bitfield: u32,
    pub z: i32,
}

impl foo_t {
    pub fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        let mut bf: u32 = 0;
        bf |= x & 0x3;
        bf |= (y & 0x7) << 2;
        bf |= (b as u32) << 5;
        foo_t { _bitfield: bf, z }
    }
    pub fn x(&self) -> u32 {
        self._bitfield & 0x3
    }
    pub fn y(&self) -> u32 {
        (self._bitfield >> 2) & 0x7
    }
    pub fn b(&self) -> bool {
        ((self._bitfield >> 5) & 0x1) != 0
    }
}

#[no_mangle]
pub extern "C" fn print_foo(foo: *const foo_t) {
    let foo = unsafe { &*foo };
    println!("{} {} {} {}", foo.x(), foo.y(), foo.b() as i32, foo.z);
}

#[no_mangle]
pub extern "C" fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = foo_t::new(x, y, b, z);
    print_foo(&foo as *const foo_t);
}

fn scan_tokens(input: &str) -> Vec<&str> {
    input.split_whitespace().collect()
}

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let tokens: Vec<&str> = scan_tokens(&input);

    let x: u32 = tokens[0].parse().unwrap();
    let y: u32 = tokens[1].parse().unwrap();
    let b: i32 = tokens[2].parse().unwrap();
    let z: i32 = tokens[3].parse().unwrap();

    driver(x, y, b != 0, z);
    0
}
