use std::env;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

fn find_container_of_a(i: *const i32, t: &Test) -> &Test {
    t
}

fn find_container_of_b(i: *const i32, t: &Test) -> &Test {
    t
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let a: i32 = args[1].parse().unwrap();
    let b: i32 = args[2].parse().unwrap();

    let t = Test { a, b };

    println!("{}", t.a + t.b);
}
