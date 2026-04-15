use std::env;
use std::mem::offset_of;

#[repr(C)]
pub struct Test {
    pub a: i32,
    pub b: i32,
}

pub fn find_container_of_a(i: *mut i32) -> *mut Test {
    unsafe { (i as *mut u8).sub(offset_of!(Test, a)) as *mut Test }
}

pub fn find_container_of_b(i: *mut i32) -> *mut Test {
    unsafe { (i as *mut u8).sub(offset_of!(Test, b)) as *mut Test }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let a: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let b: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut t = Test { a, b };

    unsafe {
        println!("{}", (*find_container_of_a(&mut t.a)).a + (*find_container_of_b(&mut t.b)).b);
    }
}
