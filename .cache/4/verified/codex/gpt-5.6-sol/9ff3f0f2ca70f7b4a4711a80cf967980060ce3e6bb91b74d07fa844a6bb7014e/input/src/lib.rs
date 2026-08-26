use std::ffi::{c_float, c_int};

const EXP_FRAC: [c_float; 4] = [
    c_float::from_bits(0x3080_0000),
    c_float::from_bits(0x3057_44fd),
    c_float::from_bits(0x3035_04f3),
    c_float::from_bits(0x3018_37f0),
];

#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(mut y: c_float, mut exp_q2: c_int) -> c_float {
    loop {
        let e = exp_q2.min(30 * 4);
        let scale = (1_i32 << 30).wrapping_shr((e >> 2) as u32);
        y *= EXP_FRAC[(e & 3) as usize] * scale as c_float;
        exp_q2 -= e;

        if exp_q2 <= 0 {
            return y;
        }
    }
}
