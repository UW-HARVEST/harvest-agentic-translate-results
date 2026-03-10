use std::io::{self, Read};

fn driver(f: f64) {
    let bits = f.to_bits();
    // Match C's printf("%llx %a %.4f\n", u.x, f, f)
    // Use libc printf for exact format compatibility
    unsafe {
        libc::printf(
            b"%llx %a %.4f\n\0".as_ptr() as *const libc::c_char,
            bits as libc::c_ulonglong,
            f,
            f,
        );
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    // scanf("%lf", &f) skips whitespace and parses a double
    let f: f64 = input.split_whitespace().next().unwrap().parse().unwrap();
    driver(f);
}
