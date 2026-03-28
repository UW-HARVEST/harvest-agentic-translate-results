use std::io::{self, Read};

/// Format f32 exactly like C's printf("%.9g\n", (double)val)
fn print_g9(val: f32) {
    let dval = val as f64;
    let mut buf = [0u8; 64];
    let fmt = b"%.9g\n\0";
    unsafe {
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            fmt.as_ptr() as *const libc::c_char,
            dval,
        );
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let s = std::str::from_utf8(&buf[..len]).unwrap();
    print!("{}", s);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_whitespace();

    macro_rules! next_i32 {
        () => { tokens.next().unwrap().parse::<i32>().unwrap() }
    }
    macro_rules! next_f32 {
        () => { tokens.next().unwrap().parse::<f32>().unwrap() }
    }

    let which = next_i32!();
    let x = next_f32!();
    let y = next_f32!();
    let z = next_f32!();
    let x_wrap = next_i32!();
    let y_wrap = next_i32!();
    let z_wrap = next_i32!();
    let seed = next_i32!();
    let lacunarity = next_f32!();
    let gain = next_f32!();
    let offset = next_f32!();
    let octaves = next_i32!();

    let res = perlin_noise::inner(which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves);
    print_g9(res);
}
