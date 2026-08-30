mod perlin;

use std::ffi::{c_char, c_double, c_float, c_int};

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn main() {
    let mut which: c_int = 0;
    let mut x: c_float = 0.0;
    let mut y: c_float = 0.0;
    let mut z: c_float = 0.0;
    let mut x_wrap: c_int = 0;
    let mut y_wrap: c_int = 0;
    let mut z_wrap: c_int = 0;
    let mut seed: c_int = 0;
    let mut lacunarity: c_float = 0.0;
    let mut gain: c_float = 0.0;
    let mut offset: c_float = 0.0;
    let mut octaves: c_int = 0;

    unsafe {
        scanf(
            b"%d%f%f%f%d%d%d%d%f%f%f%d\0".as_ptr().cast(),
            &mut which,
            &mut x,
            &mut y,
            &mut z,
            &mut x_wrap,
            &mut y_wrap,
            &mut z_wrap,
            &mut seed,
            &mut lacunarity,
            &mut gain,
            &mut offset,
            &mut octaves,
        );
    }

    let result = perlin::inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    );

    unsafe {
        printf(b"%.9g\n\0".as_ptr().cast(), c_double::from(result));
    }
}
