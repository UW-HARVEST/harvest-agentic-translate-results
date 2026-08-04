use std::ffi::c_int;

static G_EXPFRAC: [f32; 4] = [
    9.31322575e-10f32,
    7.83145814e-10f32,
    6.58544508e-10f32,
    5.53767716e-10f32,
];

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    loop {
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        // Reproduce C: g_expfrac[e & 3] * (1 << 30 >> (e >> 2))
        let idx: usize = (e & 3) as usize;
        let factor: c_int = (1i32 << 30).wrapping_shr((e >> 2) as u32);
        y *= G_EXPFRAC[idx] * factor as f32;
        exp_q2 = exp_q2.wrapping_sub(e);
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
