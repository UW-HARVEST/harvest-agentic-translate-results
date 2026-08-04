#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum cb_impairment {
    cbProtanopia = 0,
    cbDeuteranopia = 1,
    cbTritanopia = 2,
}

fn protanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = 0.17055699213417_f32 * r + 0.82944301379913_f32 * g + 2.91188E-9_f32 * b;
    *green = 0.17055699092998_f32 * r + 0.82944300785005_f32 * g - 5.98679E-10_f32 * b;
    *blue = -0.00451714424166_f32 * r + 0.00451714427397_f32 * g + b;
}

fn deuteranopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = 0.33066007266046_f32 * r + 0.66933992517563_f32 * g + 3.559314E-9_f32 * b;
    *green = 0.33066007387760_f32 * r + 0.66933992719147_f32 * g - 1.758327E-9_f32 * b;
    *blue = -0.02785538261323_f32 * r + 0.02785538252318_f32 * g + b;
}

fn tritanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

/// # Safety
/// `r`, `g`, and `b` must be valid, non-null, properly-aligned pointers to `f32` values.
#[no_mangle]
pub unsafe extern "C" fn colourblind(
    impairment: cb_impairment,
    r: *mut f32,
    g: *mut f32,
    b: *mut f32,
) {
    let red = &mut *r;
    let green = &mut *g;
    let blue = &mut *b;
    match impairment {
        cb_impairment::cbProtanopia => protanopia(red, green, blue),
        cb_impairment::cbDeuteranopia => deuteranopia(red, green, blue),
        cb_impairment::cbTritanopia => tritanopia(red, green, blue),
    }
}
