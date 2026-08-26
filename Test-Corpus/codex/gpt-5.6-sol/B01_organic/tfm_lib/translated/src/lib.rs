use std::ffi::c_int;

#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(value: f32) -> f32;
}

type Sqrtf = unsafe extern "C" fn(f32) -> f32;

static SQRTF: Sqrtf = sqrtf;

#[inline(never)]
fn add(left: f32, right: f32) -> f32 {
    left + right
}

#[inline(never)]
fn subtract(left: f32, right: f32) -> f32 {
    left - right
}

#[inline(never)]
fn multiply(left: f32, right: f32) -> f32 {
    left * right
}

#[inline(never)]
fn nonnegative(value: f32) -> f32 {
    if 0.0_f32 > value {
        0.0
    } else {
        value
    }
}

#[inline(never)]
fn square_root(value: f32) -> f32 {
    unsafe {
        let function = std::ptr::read_volatile(&SQRTF);
        function(value)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tfm(mut dest: *mut f32, mut src: *const f32, count: c_int) {
    let mut i = 0;

    while i < count {
        unsafe {
            if *src < *src.add(1) {
                let dx2 = *src;
                let dy2 = *src.add(1);
                let dxy = *src.add(2);
                let doubled_dx2 = add(dx2, dx2);
                let mixed = multiply(dy2, doubled_dx2);
                let partial = add(subtract(multiply(dy2, dy2), mixed), multiply(dx2, dx2));
                let dxy_term = multiply(multiply(dxy, 4.0_f32), dxy);
                let sqd = add(partial, dxy_term);
                let lambda = multiply(add(square_root(nonnegative(sqd)), add(dy2, dx2)), 0.5_f32);

                *dest = subtract(dx2, lambda);
                *dest.add(1) = dxy;
            } else {
                let dy2 = *src;
                let dx2 = *src.add(1);
                let dxy = *src.add(2);
                let doubled_dx2 = add(dx2, dx2);
                let mixed = multiply(doubled_dx2, dy2);
                let partial = add(subtract(multiply(dy2, dy2), mixed), multiply(dx2, dx2));
                let dxy_term = multiply(multiply(dxy, 4.0_f32), dxy);
                let sqd = add(dxy_term, partial);
                let lambda = multiply(add(square_root(nonnegative(sqd)), add(dy2, dx2)), 0.5_f32);

                *dest = dxy;
                *dest.add(1) = subtract(dx2, lambda);
            }

            src = src.add(3);
            dest = dest.add(2);
        }

        i += 1;
    }
}
