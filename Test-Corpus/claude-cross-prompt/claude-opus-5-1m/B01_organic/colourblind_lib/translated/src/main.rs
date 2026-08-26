// Translation of c_src/src/lib.c to Rust.
//
// The original C source is a library (it exposes a single function
// `colourblind` that simulates one of three forms of colour-blindness on an
// RGB triple).  There is no `main`, no stdin reading, and no printf output in
// the original code, so this binary's `main` is intentionally empty: it
// produces no output for any input, which matches what the original library
// would produce when invoked with no driver code.

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CbImpairment {
    Protanopia = 0,
    Deuteranopia = 1,
    Tritanopia = 2,
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

#[allow(dead_code)]
pub fn colourblind(impairment: CbImpairment, r: &mut f32, g: &mut f32, b: &mut f32) {
    match impairment {
        CbImpairment::Protanopia => protanopia(r, g, b),
        CbImpairment::Deuteranopia => deuteranopia(r, g, b),
        CbImpairment::Tritanopia => tritanopia(r, g, b),
    }
}

fn main() {
    // The original C source defines only library functions and no `main`,
    // so no output is produced.
}
