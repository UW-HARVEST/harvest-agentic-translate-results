use std::ffi::c_int;

static G_EXPFRAC: [f32; 4] = [
    9.31322575e-10_f32,
    7.83145814e-10_f32,
    6.58544508e-10_f32,
    5.53767716e-10_f32,
];

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    loop {
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        // Replicate C: g_expfrac[e & 3] * (1 << 30 >> (e >> 2))
        // The shift count is e >> 2 (arithmetic shift on signed int).
        // Use wrapping shifts to match typical x86 behavior (count masked
        // to low 5 bits) and avoid panics on edge inputs.
        let shift_count: u32 = (e >> 2) as u32;
        let int_factor: c_int = (1_i32)
            .wrapping_shl(30)
            .wrapping_shr(shift_count);
        let idx: usize = (e & 3) as usize;
        y *= G_EXPFRAC[idx] * (int_factor as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
