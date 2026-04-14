#[repr(C)]
#[derive(Copy, Clone)]
pub enum cb_impairment {
    cbProtanopia,
    cbDeuteranopia,
    cbTritanopia,
}

fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    unsafe {
        let r = *red;
        let g = *green;
        let b = *blue;
        *red = 0.17055699213417f32 * r + 0.82944301379913f32 * g + 2.91188e-9f32 * b;
        *green = 0.17055699092998f32 * r + 0.82944300785005f32 * g - 5.98679e-10f32 * b;
        *blue = -0.00451714424166f32 * r + 0.00451714427397f32 * g + b;
    }
}

fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    unsafe {
        let r = *red;
        let g = *green;
        let b = *blue;
        *red = 0.33066007266046f32 * r + 0.66933992517563f32 * g + 3.559314e-9f32 * b;
        *green = 0.33066007387760f32 * r + 0.66933992719147f32 * g - 1.758327e-9f32 * b;
        *blue = -0.02785538261323f32 * r + 0.02785538252318f32 * g + b;
    }
}

fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    unsafe {
        let r = *red;
        let g = *green;
        let b = *blue;
        *red = r + 0.12739886310880f32 * g - 0.12739886341072f32 * b;
        *green = -4.486e-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b;
        *blue = 3.1113e-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn colourblind(impairment: cb_impairment, r: *mut f32, g: *mut f32, b: *mut f32) {
    match impairment {
        cb_impairment::cbProtanopia => protanopia(r, g, b),
        cb_impairment::cbDeuteranopia => deuteranopia(r, g, b),
        cb_impairment::cbTritanopia => tritanopia(r, g, b),
    }
}
