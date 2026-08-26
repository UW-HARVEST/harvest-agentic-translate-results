//! Translation of `c_src/src/main.c`.
//!
//! The binary intentionally declares the modules itself instead of depending on
//! the library target: the library exports a `#[no_mangle] extern "C" fn main`
//! for symbol parity with the C shared object, which would collide with the
//! binary's own `main` symbol at link time.

mod cfmt;
mod cscan;
mod driver;
mod stb_perlin;
mod tables;

fn main() {
    std::process::exit(driver::c_main());
}

#[cfg(test)]
mod tests {
    use super::cfmt;
    use super::cscan::Scanner;
    use super::driver::inner;

    /// Expected strings captured from the compiled C program.
    fn output_for(input: &str) -> String {
        let mut scanner = Scanner::new(std::io::Cursor::new(input.as_bytes().to_vec()));
        let which = scanner.scan_int().unwrap();
        let x = scanner.scan_float().unwrap();
        let y = scanner.scan_float().unwrap();
        let z = scanner.scan_float().unwrap();
        let x_wrap = scanner.scan_int().unwrap();
        let y_wrap = scanner.scan_int().unwrap();
        let z_wrap = scanner.scan_int().unwrap();
        let seed = scanner.scan_int().unwrap();
        let lacunarity = scanner.scan_float().unwrap();
        let gain = scanner.scan_float().unwrap();
        let offset = scanner.scan_float().unwrap();
        let octaves = scanner.scan_int().unwrap();
        let res = inner(
            which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
        );
        cfmt::format_g(f64::from(res), 9)
    }

    #[test]
    fn matches_reference_outputs() {
        assert_eq!(output_for("0 0.5 0.5 0.5 0 0 0 0 2 0.5 1 6"), "-0.5");
        assert_eq!(
            output_for("1 1.5 2.25 3.125 4 8 16 42 2.0 0.5 1.0 6"),
            "0.222685531"
        );
        assert_eq!(
            output_for("2 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 6"),
            "0.678124607"
        );
        assert_eq!(
            output_for("3 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 6"),
            "0.120202541"
        );
        assert_eq!(
            output_for("4 0.25 0.5 0.75 0 0 0 0 2.0 0.5 1.0 6"),
            "0.254797459"
        );
        assert_eq!(output_for("5 1.5 2.5 3.5 3 5 7 42 2 0.5 1 6"), "-0.25");
        // `which` outside 0..=5 returns NAN.
        assert_eq!(output_for("6 1 2 3 4 5 6 7 8 9 10 11"), "nan");
    }

    #[test]
    fn formats_like_printf_percent_point_9g() {
        assert_eq!(cfmt::format_g(0.0, 9), "0");
        assert_eq!(cfmt::format_g(-0.0, 9), "-0");
        assert_eq!(cfmt::format_g(1.0, 9), "1");
        assert_eq!(cfmt::format_g(123456789.0, 9), "123456789");
        assert_eq!(cfmt::format_g(1234567890.0, 9), "1.23456789e+09");
        assert_eq!(cfmt::format_g(0.000123456789, 9), "0.000123456789");
        assert_eq!(cfmt::format_g(1e-5, 9), "1e-05");
        assert_eq!(
            cfmt::format_g(f64::from(6.1035156e-05f32), 9),
            "6.10351562e-05"
        );
        assert_eq!(cfmt::format_g(f64::INFINITY, 9), "inf");
        assert_eq!(cfmt::format_g(f64::NEG_INFINITY, 9), "-inf");
        // A quiet NaN keeps its sign when the `float` argument is promoted.
        assert_eq!(
            cfmt::format_g(f64::from_bits(0x7ff8_0000_0000_0000), 9),
            "nan"
        );
        assert_eq!(
            cfmt::format_g(f64::from_bits(0xfff8_0000_0000_0000), 9),
            "-nan"
        );
    }

    #[test]
    fn scans_like_scanf() {
        let mut s = Scanner::new(std::io::Cursor::new(b"  12\t-3.5\n1e ".to_vec()));
        assert_eq!(s.scan_int(), Some(12));
        assert_eq!(s.scan_float(), Some(-3.5));
        // glibc consumes the exponent marker even without digits.
        assert_eq!(s.scan_float(), Some(1.0));
        assert_eq!(s.scan_int(), None);

        // `strtol` saturation followed by truncation to `int`.
        let mut s = Scanner::new(std::io::Cursor::new(b"99999999999999999999999".to_vec()));
        assert_eq!(s.scan_int(), Some(-1));
    }
}
