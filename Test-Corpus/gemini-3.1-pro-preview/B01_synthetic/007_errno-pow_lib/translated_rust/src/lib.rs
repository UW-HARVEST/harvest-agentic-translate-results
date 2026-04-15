use std::os::raw::c_double;

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: c_double, exponent: c_double) -> c_double {
    let result = base.powf(exponent);
    if result.is_nan() {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        -1.0
    } else if result.is_infinite() || (result == 0.0 && base != 0.0 && !base.is_infinite()) {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        -1.0
    } else {
        result
    }
}
