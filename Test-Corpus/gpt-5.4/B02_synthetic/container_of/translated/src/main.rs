use memoffset::offset_of;
use std::env;
use std::process;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(i: *const i32) -> *const Test {
    let offset = offset_of!(Test, a);
    ((i as *const u8).wrapping_sub(offset)) as *const Test
}

fn find_container_of_b(i: *const i32) -> *const Test {
    let offset = offset_of!(Test, b);
    ((i as *const u8).wrapping_sub(offset)) as *const Test
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        process::exit(1);
    }

    let a: i32 = args[1].parse().unwrap_or(0);
    let b: i32 = args[2].parse().unwrap_or(0);

    let t = Test { a, b };

    let sum = unsafe { (*find_container_of_a(&t.a)).a + (*find_container_of_b(&t.b)).b };
    println!("{}", sum);
}
