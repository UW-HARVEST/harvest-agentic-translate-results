#[repr(C)]
pub struct FooT {
    bitfield: u32,
    z: i32,
}

impl FooT {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        let bitfield = (x & 0x3) | ((y & 0x7) << 2) | ((b as u32) << 5);
        FooT { bitfield, z }
    }
    fn x(&self) -> u32 { self.bitfield & 0x3 }
    fn y(&self) -> u32 { (self.bitfield >> 2) & 0x7 }
    fn b(&self) -> bool { (self.bitfield >> 5) & 0x1 != 0 }
}

#[no_mangle]
pub extern "C" fn print_foo(foo: *const FooT) {
    let foo = unsafe { &*foo };
    println!("{} {} {} {}", foo.x(), foo.y(), foo.b() as i32, foo.z);
}

#[no_mangle]
pub extern "C" fn driver(x: u32, y: u32, b: bool, z: i32) {
    let foo = FooT::new(x, y, b, z);
    print_foo(&foo as *const FooT);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let x: u32 = tokens.get(0).map_or(0, |s| s.parse().unwrap_or(0));
    let y: u32 = tokens.get(1).map_or(0, |s| s.parse().unwrap_or(0));
    let b: i32 = tokens.get(2).map_or(0, |s| s.parse().unwrap_or(0));
    let z: i32 = tokens.get(3).map_or(0, |s| s.parse().unwrap_or(0));
    driver(x, y, b != 0, z);
    0
}
