use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn my_pow(base: f64, exponent: f64) -> f64 {
    let result = base.powf(exponent);

    if result.is_nan() {
        let _ = writeln!(
            io::stderr(),
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base,
            exponent
        );
        return -1.0;
    }

    if result.is_infinite() || (result == 0.0 && base != 0.0 && exponent.is_finite()) {
        let _ = writeln!(
            io::stderr(),
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base,
            exponent
        );
        return -1.0;
    }

    result
}
