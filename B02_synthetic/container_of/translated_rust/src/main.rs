use std::env;

/// Mimics C atoi: parse leading optional sign + digits, return 0 on failure.
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('+') => { chars.next(); false }
        Some('-') => { chars.next(); true }
        _ => false,
    };
    let mut val: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            val = val.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(t: &Test) -> &Test { t }
fn find_container_of_b(t: &Test) -> &Test { t }

fn main() {
    let args: Vec<String> = env::args().collect();
    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let t = Test { a, b };

    let sum = find_container_of_a(&t).a.wrapping_add(find_container_of_b(&t).b);
    println!("{}", sum);
}
