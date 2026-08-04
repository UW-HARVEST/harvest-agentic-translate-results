use std::ffi::{c_float, c_int};

const CB_PROTANOPIA: c_int = 0;
const CB_DEUTERANOPIA: c_int = 1;
const CB_TRITANOPIA: c_int = 2;

unsafe fn protanopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let r = unsafe { *red };
    let g = unsafe { *green };
    let b = unsafe { *blue };

    unsafe {
        *red = 0.17055699213417f32 * r + 0.82944301379913f32 * g + 2.91188E-9f32 * b;
        *green = 0.17055699092998f32 * r + 0.82944300785005f32 * g - 5.98679E-10f32 * b;
        *blue = -0.00451714424166f32 * r + 0.00451714427397f32 * g + b;
    }
}

unsafe fn deuteranopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let r = unsafe { *red };
    let g = unsafe { *green };
    let b = unsafe { *blue };

    unsafe {
        *red = 0.33066007266046f32 * r + 0.66933992517563f32 * g + 3.559314E-9f32 * b;
        *green = 0.33066007387760f32 * r + 0.66933992719147f32 * g - 1.758327E-9f32 * b;
        *blue = -0.02785538261323f32 * r + 0.02785538252318f32 * g + b;
    }
}

unsafe fn tritanopia(red: *mut c_float, green: *mut c_float, blue: *mut c_float) {
    let r = unsafe { *red };
    let g = unsafe { *green };
    let b = unsafe { *blue };

    unsafe {
        *red = r + 0.12739886310880f32 * g - 0.12739886341072f32 * b;
        *green = -4.486E-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b;
        *blue = 3.1113E-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    impairment: c_int,
    r: *mut c_float,
    g: *mut c_float,
    b: *mut c_float,
) {
    match impairment {
        CB_PROTANOPIA => unsafe { protanopia(r, g, b) },
        CB_DEUTERANOPIA => unsafe { deuteranopia(r, g, b) },
        CB_TRITANOPIA => unsafe { tritanopia(r, g, b) },
        _ => {}
    }
}
