use std::os::raw::c_float;

#[repr(C)]
pub enum cb_impairment {
    cbProtanopia,
    cbDeuteranopia,
    cbTritanopia,
}

fn Protanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R = *Red;
    let G = *Green;
    let B = *Blue;
    *Red = 0.17055699213417f32 * R + 0.82944301379913f32 * G + 2.91188E-9f32 * B;
    *Green = 0.17055699092998f32 * R + 0.82944300785005f32 * G - 5.98679E-10f32 * B;
    *Blue = -0.00451714424166f32 * R + 0.00451714427397f32 * G + B;
}

fn Deuteranopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R = *Red;
    let G = *Green;
    let B = *Blue;
    *Red = 0.33066007266046f32 * R + 0.66933992517563f32 * G + 3.559314E-9f32 * B;
    *Green = 0.33066007387760f32 * R + 0.66933992719147f32 * G - 1.758327E-9f32 * B;
    *Blue = -0.02785538261323f32 * R + 0.02785538252318f32 * G + B;
}

fn Tritanopia(Red: &mut f32, Green: &mut f32, Blue: &mut f32) {
    let R = *Red;
    let G = *Green;
    let B = *Blue;
    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}

#[unsafe(no_mangle)]
pub extern "C" fn colourblind(Impairment: cb_impairment, R: *mut c_float, G: *mut c_float, B: *mut c_float) {
    unsafe {
        let r = &mut *R;
        let g = &mut *G;
        let b = &mut *B;
        match Impairment {
            cb_impairment::cbProtanopia => Protanopia(r, g, b),
            cb_impairment::cbDeuteranopia => Deuteranopia(r, g, b),
            cb_impairment::cbTritanopia => Tritanopia(r, g, b),
        }
    }
}
