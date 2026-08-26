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

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn load(pointer: *const f32) -> f32 {
    let value: f32;
    unsafe {
        std::arch::asm!(
            "movss {value}, dword ptr [{pointer}]",
            value = lateout(xmm_reg) value,
            pointer = in(reg) pointer,
            options(readonly, nostack, preserves_flags)
        );
    }
    value
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

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn load(pointer: *const f32) -> f32 {
    unsafe { *pointer }
}

unsafe fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (load(red), load(green), load(blue)) };
    let red_rg = add(
        multiply(r, 0.17055699213417_f32),
        multiply(0.82944301379913_f32, g),
    );
    let red_out = add(multiply(2.91188E-9_f32, b), red_rg);
    let green_rg = add(
        multiply(0.82944300785005_f32, g),
        multiply(r, 0.17055699092998_f32),
    );
    let green_out = subtract(green_rg, multiply(5.98679E-10_f32, b));
    let blue_rg = add(
        multiply(0.00451714427397_f32, g),
        multiply(r, -0.00451714424166_f32),
    );
    let blue_out = add(blue_rg, b);

    unsafe {
        *red = red_out;
        *green = green_out;
        *blue = blue_out;
    }
}

unsafe fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (load(red), load(green), load(blue)) };
    let red_rg = add(
        multiply(r, 0.33066007266046_f32),
        multiply(0.66933992517563_f32, g),
    );
    let red_out = add(multiply(3.559314E-9_f32, b), red_rg);
    let green_rg = add(
        multiply(0.66933992719147_f32, g),
        multiply(r, 0.33066007387760_f32),
    );
    let green_out = subtract(green_rg, multiply(1.758327E-9_f32, b));
    let blue_rg = add(
        multiply(0.02785538252318_f32, g),
        multiply(r, -0.02785538261323_f32),
    );
    let blue_out = add(blue_rg, b);

    unsafe {
        *red = red_out;
        *green = green_out;
        *blue = blue_out;
    }
}

unsafe fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = unsafe { (load(red), load(green), load(blue)) };
    let red_out = subtract(
        add(multiply(0.12739886310880_f32, g), r),
        multiply(0.12739886341072_f32, b),
    );
    let green_rg = add(
        multiply(r, -4.486E-11_f32),
        multiply(0.87390929928361_f32, g),
    );
    let green_out = add(multiply(0.12609070101523_f32, b), green_rg);
    let blue_rg = add(
        multiply(r, 3.1113E-10_f32),
        multiply(0.87390929725848_f32, g),
    );
    let blue_out = add(multiply(0.12609070067115_f32, b), blue_rg);

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
