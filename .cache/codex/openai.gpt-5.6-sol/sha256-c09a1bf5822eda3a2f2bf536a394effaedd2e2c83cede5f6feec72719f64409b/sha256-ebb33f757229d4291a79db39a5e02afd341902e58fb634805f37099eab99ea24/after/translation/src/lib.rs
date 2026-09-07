use std::ffi::{c_float, c_int};

const EXP_FRAC: [f32; 4] = [
    f32::from_bits(0x3080_0000),
    f32::from_bits(0x3057_44fd),
    f32::from_bits(0x3035_04f3),
    f32::from_bits(0x3018_37f0),
];

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: c_float, mut exp_q2: c_int) -> c_float {
    loop {
        let e = exp_q2.min(30 * 4);
        let scale = (1_i32 << 30).wrapping_shr((e >> 2) as u32) as f32;
        y *= EXP_FRAC[(e & 3) as usize] * scale;
        exp_q2 -= e;

        if exp_q2 <= 0 {
            return y;
        }
    }
}
