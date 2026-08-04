



pub type cb_impairment = ::core::ffi::c_uint;
pub const cbTritanopia: cb_impairment = 2;
pub const cbDeuteranopia: cb_impairment = 1;
pub const cbProtanopia: cb_impairment = 0;
fn Protanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = 0.17055699213417f32 * r + 0.82944301379913f32 * g + 2.91188E-9f32 * b;
    *green = 0.17055699092998f32 * r + 0.82944300785005f32 * g - 5.98679E-10f32 * b;
    *blue = -0.00451714424166f32 * r + 0.00451714427397f32 * g + b;
}

fn Deuteranopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;
    *red = 0.33066007266046f32 * r + 0.66933992517563f32 * g + 3.559314E-9f32 * b;
    *green = 0.33066007387760f32 * r + 0.66933992719147f32 * g - 1.758327E-9f32 * b;
    *blue = -0.02785538261323f32 * r + 0.02785538252318f32 * g + b;
}

fn Tritanopia(red: &mut f32, green: &mut f32, blue: &mut f32) {
    let r = *red;
    let g = *green;
    let b = *blue;

    *red = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    *green = -4.486e-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    *blue = 3.1113e-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
}

#[no_mangle]
pub fn colourblind(impairment: cb_impairment, r: &mut f32, g: &mut f32, b: &mut f32) {
    match impairment {
        0 => Protanopia(r, g, b),
        1 => Deuteranopia(r, g, b),
        2 => Tritanopia(r, g, b),
        _ => {}
    }
}

