use std::env;
use std::mem;

#[repr(C)]
struct Test {
    a: i32,
    b: i32,
}

/// Safety: `ptr` must point to the `a` field of a `Test` struct that is still
/// alive for the duration of the returned reference.
unsafe fn find_container_of_a(ptr: *const i32) -> *mut Test {
    let offset = mem::offset_of!(Test, a);
    (ptr as *const u8).wrapping_sub(offset) as *mut Test
}

/// Safety: `ptr` must point to the `b` field of a `Test` struct that is still
/// alive for the duration of the returned reference.
unsafe fn find_container_of_b(ptr: *const i32) -> *mut Test {
    let offset = mem::offset_of!(Test, b);
    (ptr as *const u8).wrapping_sub(offset) as *mut Test
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Mimic atoi: parse, defaulting to 0 on failure.
    let a: i32 = args
        .get(1)
        .map(|s| atoi(s))
        .unwrap_or(0);
    let b: i32 = args
        .get(2)
        .map(|s| atoi(s))
        .unwrap_or(0);

    // memset(&t, 0, sizeof(t)) followed by assigning a and b
    let mut t: Test = unsafe { mem::zeroed() };
    t.a = a;
    t.b = b;

    unsafe {
        let a_ptr: *const i32 = &t.a;
        let b_ptr: *const i32 = &t.b;

        let from_a = find_container_of_a(a_ptr);
        let from_b = find_container_of_b(b_ptr);

        println!("{}", (*from_a).a + (*from_b).b);
    }
}

/// Mimic C's atoi: parses leading optional sign and digits, returns 0 on
/// failure or empty input. Stops at first non-digit character.
fn atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip leading whitespace as C's atoi does
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i32);
        i += 1;
    }
    result.wrapping_mul(sign)
}
