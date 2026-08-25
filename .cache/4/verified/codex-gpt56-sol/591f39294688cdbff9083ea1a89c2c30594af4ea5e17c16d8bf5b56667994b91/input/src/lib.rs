use std::ffi::c_int;

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn multiply(mut left: f32, right: f32) -> f32 {
    unsafe {
        std::arch::asm!(
            "mulss {left}, {right}",
            left = inout(xmm_reg) left,
            right = in(xmm_reg) right,
            options(nomem, nostack)
        );
    }
    left
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn add(mut left: f32, right: f32) -> f32 {
    unsafe {
        std::arch::asm!(
            "addss {left}, {right}",
            left = inout(xmm_reg) left,
            right = in(xmm_reg) right,
            options(nomem, nostack)
        );
    }
    left
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn subtract(mut left: f32, right: f32) -> f32 {
    unsafe {
        std::arch::asm!(
            "subss {left}, {right}",
            left = inout(xmm_reg) left,
            right = in(xmm_reg) right,
            options(nomem, nostack)
        );
    }
    left
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn multiply(left: f32, right: f32) -> f32 {
    left * right
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn add(left: f32, right: f32) -> f32 {
    left + right
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn subtract(left: f32, right: f32) -> f32 {
    left - right
}

unsafe fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };
    let rg = add(
        multiply(r, 0.17055699213417_f32),
        multiply(g, 0.82944301379913_f32),
    );
    let red_out = add(multiply(b, 2.91188E-9_f32), rg);
    let green_out = subtract(rg, multiply(b, 5.98679E-10_f32));
    let blue_out = add(
        add(
            multiply(r, -0.00451714424166_f32),
            multiply(g, 0.00451714427397_f32),
        ),
        b,
    );

    unsafe {
        *red = red_out;
        *green = green_out;
        *blue = blue_out;
    }
}

unsafe fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };
    let rg = add(
        multiply(r, 0.33066007266046_f32),
        multiply(g, 0.66933992517563_f32),
    );
    let red_out = add(multiply(b, 3.559314E-9_f32), rg);
    let green_out = subtract(rg, multiply(b, 1.758327E-9_f32));
    let blue_out = add(
        add(
            multiply(r, -0.02785538261323_f32),
            multiply(g, 0.02785538252318_f32),
        ),
        b,
    );

    unsafe {
        *red = red_out;
        *green = green_out;
        *blue = blue_out;
    }
}

unsafe fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (*red, *green, *blue) };
    let red_out = subtract(
        add(multiply(g, 0.12739886310880_f32), r),
        multiply(b, 0.12739886341072_f32),
    );
    let green_g = multiply(g, 0.87390929928361_f32);
    let green_b = multiply(b, 0.12609070101523_f32);
    let green_out = add(add(multiply(r, -4.486E-11_f32), green_g), green_b);
    let blue_out = add(add(multiply(r, 3.1113E-10_f32), green_g), green_b);

    unsafe {
        *red = red_out;
        *green = green_out;
        *blue = blue_out;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    impairment: c_int,
    red: *mut f32,
    green: *mut f32,
    blue: *mut f32,
) {
    match impairment {
        0 => unsafe { protanopia(red, green, blue) },
        1 => unsafe { deuteranopia(red, green, blue) },
        2 => unsafe { tritanopia(red, green, blue) },
        _ => {}
    }
}
