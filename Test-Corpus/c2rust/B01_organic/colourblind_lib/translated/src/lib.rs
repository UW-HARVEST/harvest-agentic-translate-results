pub type cb_impairment = ::core::ffi::c_uint;
pub const cbTritanopia: cb_impairment = 2;
pub const cbDeuteranopia: cb_impairment = 1;
pub const cbProtanopia: cb_impairment = 0;
unsafe extern "C" fn Protanopia(
    mut Red: *mut ::core::ffi::c_float,
    mut Green: *mut ::core::ffi::c_float,
    mut Blue: *mut ::core::ffi::c_float,
) {
    let mut R: ::core::ffi::c_float = *Red;
    let mut G: ::core::ffi::c_float = *Green;
    let mut B: ::core::ffi::c_float = *Blue;
    *Red = 0.17055699213417f32 * R + 0.82944301379913f32 * G + 2.91188E-9f32 * B;
    *Green = 0.17055699092998f32 * R + 0.82944300785005f32 * G - 5.98679E-10f32 * B;
    *Blue = -0.00451714424166f32 * R + 0.00451714427397f32 * G + B;
}
unsafe extern "C" fn Deuteranopia(
    mut Red: *mut ::core::ffi::c_float,
    mut Green: *mut ::core::ffi::c_float,
    mut Blue: *mut ::core::ffi::c_float,
) {
    let mut R: ::core::ffi::c_float = *Red;
    let mut G: ::core::ffi::c_float = *Green;
    let mut B: ::core::ffi::c_float = *Blue;
    *Red = 0.33066007266046f32 * R + 0.66933992517563f32 * G + 3.559314E-9f32 * B;
    *Green = 0.33066007387760f32 * R + 0.66933992719147f32 * G - 1.758327E-9f32 * B;
    *Blue = -0.02785538261323f32 * R + 0.02785538252318f32 * G + B;
}
unsafe extern "C" fn Tritanopia(
    mut Red: *mut ::core::ffi::c_float,
    mut Green: *mut ::core::ffi::c_float,
    mut Blue: *mut ::core::ffi::c_float,
) {
    let mut R: ::core::ffi::c_float = *Red;
    let mut G: ::core::ffi::c_float = *Green;
    let mut B: ::core::ffi::c_float = *Blue;
    *Red = R + 0.12739886310880f32 * G - 0.12739886341072f32 * B;
    *Green = -4.486E-11f32 * R + 0.87390929928361f32 * G + 0.12609070101523f32 * B;
    *Blue = 3.1113E-10f32 * R + 0.87390929725848f32 * G + 0.12609070067115f32 * B;
}
#[no_mangle]
pub unsafe extern "C" fn colourblind(
    mut Impairment: cb_impairment,
    mut R: *mut ::core::ffi::c_float,
    mut G: *mut ::core::ffi::c_float,
    mut B: *mut ::core::ffi::c_float,
) {
    match Impairment as ::core::ffi::c_uint {
        0 => {
            Protanopia(R, G, B);
        }
        1 => {
            Deuteranopia(R, G, B);
        }
        2 => {
            Tritanopia(R, G, B);
        }
        _ => {}
    };
}
