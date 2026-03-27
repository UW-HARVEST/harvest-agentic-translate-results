use std::env;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(i: *mut i32) -> *mut Test {
    i as *mut Test
}

fn find_container_of_b(i: *mut i32) -> *mut Test {
    unsafe { (i as *mut u8).sub(4) as *mut Test }
}

/// Mimics C atoi: parse leading decimal digits, return 0 on failure.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    if s.is_empty() {
        return 0;
    }
    let (s, neg) = if let Some(rest) = s.strip_prefix('-') {
        (rest, true)
    } else if let Some(rest) = s.strip_prefix('+') {
        (rest, false)
    } else {
        (s, false)
    };
    let mut val: i32 = 0;
    for c in s.chars() {
        if let Some(d) = c.to_digit(10) {
            val = val.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let mut t = Test { a: 0, b: 0 };
    t.a = a;
    t.b = b;

    unsafe {
        let ra = (*find_container_of_a(&mut t.a)).a;
        let rb = (*find_container_of_b(&mut t.b)).b;
        println!("{}", ra + rb);
    }
}
