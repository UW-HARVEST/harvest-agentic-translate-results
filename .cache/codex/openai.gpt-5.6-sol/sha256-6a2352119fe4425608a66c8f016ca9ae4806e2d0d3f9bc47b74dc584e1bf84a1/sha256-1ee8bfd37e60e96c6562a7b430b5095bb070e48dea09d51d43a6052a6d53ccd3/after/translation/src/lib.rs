use std::ffi::{c_float, c_int};

const CB_PROTANOPIA: c_int = 0;
const CB_DEUTERANOPIA: c_int = 1;
const CB_TRITANOPIA: c_int = 2;

// Keep each operation opaque to LLVM so NaN signs and payloads follow the C
// implementation's exact sequence instead of being algebraically rewritten.
#[inline(never)]
fn multiply(lhs: c_float, rhs: c_float) -> c_float {
    lhs * rhs
}

#[inline(never)]
fn add(lhs: c_float, rhs: c_float) -> c_float {
    lhs + rhs
}

#[inline(never)]
fn subtract(lhs: c_float, rhs: c_float) -> c_float {
    lhs - rhs
}

unsafe fn protanopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    unsafe {
        *red = add(
            multiply(2.91188E-9_f32, b),
            add(
                multiply(0.17055699213417_f32, r),
                multiply(0.82944301379913_f32, g),
            ),
        );
        *green = subtract(
            add(
                multiply(0.82944300785005_f32, g),
                multiply(0.17055699092998_f32, r),
            ),
            multiply(5.98679E-10_f32, b),
        );
        *blue = add(
            add(
                multiply(0.00451714427397_f32, g),
                multiply(-0.00451714424166_f32, r),
            ),
            b,
        );
    }
}

unsafe fn deuteranopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    unsafe {
        *red = add(
            multiply(3.559314E-9_f32, b),
            add(
                multiply(0.33066007266046_f32, r),
                multiply(0.66933992517563_f32, g),
            ),
        );
        *green = subtract(
            add(
                multiply(0.66933992719147_f32, g),
                multiply(0.33066007387760_f32, r),
            ),
            multiply(1.758327E-9_f32, b),
        );
        *blue = add(
            add(
                multiply(0.02785538252318_f32, g),
                multiply(-0.02785538261323_f32, r),
            ),
            b,
        );
    }
}

unsafe fn tritanopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    unsafe {
        *red = subtract(
            add(multiply(0.12739886310880_f32, g), r),
            multiply(0.12739886341072_f32, b),
        );
        *green = add(
            multiply(0.12609070101523_f32, b),
            add(
                multiply(-4.486E-11_f32, r),
                multiply(0.87390929928361_f32, g),
            ),
        );
        *blue = add(
            multiply(0.12609070067115_f32, b),
            add(
                multiply(3.1113E-10_f32, r),
                multiply(0.87390929725848_f32, g),
            ),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    impairment: c_int,
    red: *mut c_float,
    green: *mut c_float,
    blue: *mut c_float,
) {
    match impairment {
        CB_PROTANOPIA => unsafe { protanopia(red, green, blue) },
        CB_DEUTERANOPIA => unsafe { deuteranopia(red, green, blue) },
        CB_TRITANOPIA => unsafe { tritanopia(red, green, blue) },
        _ => {}
    }
}
