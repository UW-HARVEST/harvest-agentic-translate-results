use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: f32, mut exp_q2: c_int) -> f32 {
    const G_EXPFRAC: [f32; 4] = [
        9.31322575e-10_f32,
        7.83145814e-10_f32,
        6.58544508e-10_f32,
        5.53767716e-10_f32,
    ];
    loop {
        let e: c_int = if (30 * 4) > exp_q2 { exp_q2 } else { 30 * 4 };
        // Match C behavior: `1 << 30 >> (e >> 2)`. In Rust the equivalent of
        // the platform shift (which on x86 masks the count to 5 bits) is
        // wrapping_shr after reinterpreting the signed shift amount as u32.
        let shift_amount = (e >> 2) as u32;
        let int_factor = (1_i32 << 30).wrapping_shr(shift_amount);
        let idx = (e & 3) as usize;
        y *= G_EXPFRAC[idx] * (int_factor as f32);
        exp_q2 -= e;
        if exp_q2 <= 0 {
            break;
        }
    }
    y
}
