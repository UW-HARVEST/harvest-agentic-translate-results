use std::ffi::c_int;

#[cfg(target_arch = "x86_64")]
use std::arch::asm;

#[cfg(not(target_arch = "x86_64"))]
#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(x: f32) -> f32;
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn c_max_zero(value: f32) -> f32 {
    let bits = value.to_bits();
    let is_nan = (bits & 0x7f80_0000) == 0x7f80_0000 && (bits & 0x007f_ffff) != 0;
    let is_negative_nonzero = (bits & 0x8000_0000) != 0 && (bits << 1) != 0;

    if is_nan {
        value
    } else if is_negative_nonzero {
        0.0_f32
    } else {
        value
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn tfm_step(dest: *mut f32, src: *const f32) {
    let zero = 0.0_f32;
    let half = 0.5_f32;
    let four = 4.0_f32;

    asm!(
        "movss xmm4, dword ptr [{src} + 8]",
        "movaps xmm9, xmm4",
        "movss xmm2, dword ptr [{src}]",
        "movss xmm1, dword ptr [{src} + 4]",
        "mulss xmm9, {four}",
        "movaps xmm3, xmm2",
        "ucomiss xmm1, xmm2",
        "movaps xmm0, xmm1",
        "mulss xmm3, xmm2",
        "movaps xmm8, xmm2",
        "mulss xmm0, xmm1",
        "addss xmm8, xmm1",
        "mulss xmm9, xmm4",
        "ja 2f",
        "movaps xmm10, xmm1",
        "addss xmm10, xmm1",
        "mulss xmm2, xmm10",
        "subss xmm3, xmm2",
        "pxor xmm2, xmm2",
        "addss xmm3, xmm0",
        "addss xmm3, xmm9",
        "ucomiss {zero}, xmm3",
        "ja 3f",
        "sqrtss xmm2, xmm3",
        "3:",
        "addss xmm2, xmm8",
        "movss dword ptr [{dest}], xmm4",
        "mulss xmm2, {half}",
        "subss xmm1, xmm2",
        "movss dword ptr [{dest} + 4], xmm1",
        "jmp 4f",
        "2:",
        "movaps xmm10, xmm2",
        "addss xmm10, xmm2",
        "mulss xmm1, xmm10",
        "subss xmm0, xmm1",
        "pxor xmm1, xmm1",
        "addss xmm3, xmm0",
        "addss xmm3, xmm9",
        "ucomiss {zero}, xmm3",
        "ja 5f",
        "sqrtss xmm1, xmm3",
        "5:",
        "addss xmm1, xmm8",
        "movss dword ptr [{dest} + 4], xmm4",
        "mulss xmm1, {half}",
        "subss xmm2, xmm1",
        "movss dword ptr [{dest}], xmm2",
        "4:",
        src = in(reg) src,
        dest = in(reg) dest,
        zero = in(xmm_reg) zero,
        half = in(xmm_reg) half,
        four = in(xmm_reg) four,
        out("xmm0") _,
        out("xmm1") _,
        out("xmm2") _,
        out("xmm3") _,
        out("xmm4") _,
        out("xmm8") _,
        out("xmm9") _,
        out("xmm10") _,
        options(nostack),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(mut dest: *mut f32, mut src: *const f32, count: c_int) {
    let mut i: c_int = 0;
    while i < count {
        #[cfg(target_arch = "x86_64")]
        tfm_step(dest, src);

        #[cfg(not(target_arch = "x86_64"))]
        {
            if *src.add(0) < *src.add(1) {
                let dx2 = *src.add(0);
                let dy2 = *src.add(1);
                let dxy = *src.add(2);
                let sqd =
                    (dy2 * dy2) - (2.0_f32 * dx2 * dy2) + (dx2 * dx2) + (4.0_f32 * dxy * dxy);
                let lambda = 0.5_f32 * (dy2 + dx2 + sqrtf(c_max_zero(sqd)));
                *dest.add(0) = dx2 - lambda;
                *dest.add(1) = dxy;
            } else {
                let dy2 = *src.add(0);
                let dx2 = *src.add(1);
                let dxy = *src.add(2);
                let sqd =
                    (dy2 * dy2) - (2.0_f32 * dx2 * dy2) + (dx2 * dx2) + (4.0_f32 * dxy * dxy);
                let lambda = 0.5_f32 * (dy2 + dx2 + sqrtf(c_max_zero(sqd)));
                *dest.add(0) = dxy;
                *dest.add(1) = dx2 - lambda;
            }
        }

        src = src.add(3);
        dest = dest.add(2);
        i += 1;
    }
}
