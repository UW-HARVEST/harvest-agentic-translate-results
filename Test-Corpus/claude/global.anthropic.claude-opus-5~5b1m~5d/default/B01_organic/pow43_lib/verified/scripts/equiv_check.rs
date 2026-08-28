// Determines whether performing the `frac` division in f64 (a plausible
// mistranslation) is observable in `pow43`'s *output*, and if so at which x.
fn poly(frac: f32) -> f32 { 1.0f32 + frac * ((4.0f32/3.0f32) + frac * (2.0f32/9.0f32)) }

fn main() {
    let mut frac_diff = 0u64;
    let mut poly_diff = 0u64;
    let mut examples: Vec<i64> = Vec::new();
    let mut x_in: i64 = 129;
    while x_in <= i32::MAX as i64 {
        let mut x = x_in as i32;
        if x < 1024 { x = x.wrapping_shl(3); }
        let sign = x.wrapping_mul(2) & 64;
        let num = (x & 63).wrapping_sub(sign);
        let den = (x & !63).wrapping_add(sign);
        let f_ref = (num as f32) / (den as f32);
        let f_64  = ((num as f64) / (den as f64)) as f32;
        if f_ref.to_bits() != f_64.to_bits() {
            frac_diff += 1;
            if poly(f_ref).to_bits() != poly(f_64).to_bits() {
                poly_diff += 1;
                if examples.len() < 12 { examples.push(x_in); }
            }
        }
        x_in += 1;
    }
    println!("frac differs for {frac_diff} x; poly (observable) differs for {poly_diff} x");
    println!("first observable x: {examples:?}");
}
