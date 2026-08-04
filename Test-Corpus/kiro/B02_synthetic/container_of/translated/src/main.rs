use std::mem::offset_of;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(i: *const i32) -> *const Test {
    unsafe { (i as *const u8).sub(offset_of!(Test, a)) as *const Test }
}

fn find_container_of_b(i: *const i32) -> *const Test {
    unsafe { (i as *const u8).sub(offset_of!(Test, b)) as *const Test }
}

/// Mimics C atoi: parse leading decimal integer, return 0 on failure.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut n: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            n = n.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { n.wrapping_neg() } else { n }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let mut t = Test { a: 0, b: 0 };
    t.a = a;
    t.b = b;

    unsafe {
        println!("{}", (*find_container_of_a(&t.a)).a + (*find_container_of_b(&t.b)).b);
    }
}
