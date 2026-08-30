// Translation of c_src/src/main.c
//
// Takes two arguments, a base and an exponent, and prints base^exponent.
// Behaviour (including the quirks of the original, such as accepting an empty
// argument as 0) is reproduced byte for byte.

mod cfmt;
mod cpow;
mod strtod;

use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

use cfmt::format_f2;
use cpow::{pow_with_errno, Errno};

fn stderr_bytes(parts: &[&[u8]]) {
    let stderr = std::io::stderr();
    let mut out = stderr.lock();
    for p in parts {
        let _ = out.write_all(p);
    }
    let _ = out.flush();
}

fn stdout_bytes(parts: &[&[u8]]) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for p in parts {
        let _ = out.write_all(p);
    }
    let _ = out.flush();
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not do.  Without restoring the default disposition, a `driver`
/// whose stdout is a closed pipe would swallow the write error and exit 0 where
/// the C program is killed by `SIGPIPE` (shell status 141).
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() -> ExitCode {
    restore_default_sigpipe();

    // argv as raw bytes, exactly as C sees it.
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();

    if argv.len() != 3 {
        // fprintf(stderr, "Usage: %s base exponent\n", argv[0]);
        let progname: &[u8] = match argv.first() {
            Some(a) => a,
            None => b"(null)",
        };
        stderr_bytes(&[b"Usage: ", progname, b" base exponent\n"]);
        return ExitCode::from(1);
    }

    // Convert base
    let base_arg = &argv[1];
    let base_conv = strtod::strtod(base_arg);
    if base_conv.erange {
        stderr_bytes(&[
            b"Range error while converting base '",
            base_arg,
            b"'\n",
        ]);
        return ExitCode::from(1);
    } else if base_conv.consumed != base_arg.len() {
        stderr_bytes(&[
            b"Invalid numeric input for base: '",
            base_arg,
            b"'\n",
        ]);
        return ExitCode::from(1);
    }
    let base = base_conv.value;

    // Convert exponent
    let exponent_arg = &argv[2];
    let exponent_conv = strtod::strtod(exponent_arg);
    if exponent_conv.erange {
        stderr_bytes(&[
            b"Range error while converting exponent '",
            exponent_arg,
            b"'\n",
        ]);
        return ExitCode::from(1);
    } else if exponent_conv.consumed != exponent_arg.len() {
        stderr_bytes(&[
            b"Invalid numeric input for exponent: '",
            exponent_arg,
            b"'\n",
        ]);
        return ExitCode::from(1);
    }
    let exponent = exponent_conv.value;

    // Calculate power
    let (result, errno) = pow_with_errno(base, exponent);
    if errno == Errno::Edom {
        let msg = format!(
            "Domain error: pow({}, {}) is undefined in the real number domain.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_bytes(&[msg.as_bytes()]);
        return ExitCode::from(1);
    } else if errno == Errno::Erange {
        let msg = format!(
            "Range error: pow({}, {}) caused overflow or underflow.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_bytes(&[msg.as_bytes()]);
        return ExitCode::from(1);
    }

    let msg = format!("Result: {}\n", format_f2(result));
    stdout_bytes(&[msg.as_bytes()]);
    ExitCode::from(0)
}

#[cfg(test)]
mod tests {
    use super::cfmt::format_f2;
    use super::cpow::{pow_with_errno, Errno};
    use super::strtod::strtod;

    /// Expected values taken from glibc 2.34 on x86-64.
    fn check(input: &str, bits: u64, consumed: usize, erange: bool) {
        let r = strtod(input.as_bytes());
        assert_eq!(
            r.value.to_bits(),
            bits,
            "value for {:?}: got {:016x}",
            input,
            r.value.to_bits()
        );
        assert_eq!(r.consumed, consumed, "consumed for {:?}", input);
        assert_eq!(r.erange, erange, "erange for {:?}", input);
    }

    #[test]
    fn strtod_basics() {
        check("", 0x0000000000000000, 0, false);
        check(" ", 0x0000000000000000, 0, false);
        check("abc", 0x0000000000000000, 0, false);
        check("--3", 0x0000000000000000, 0, false);
        check(".", 0x0000000000000000, 0, false);
        check("+", 0x0000000000000000, 0, false);
        check("  +5", 0x4014000000000000, 4, false);
        check("5 ", 0x4014000000000000, 1, false);
        check(".5", 0x3fe0000000000000, 2, false);
        check("5.", 0x4014000000000000, 2, false);
        check("1e", 0x3ff0000000000000, 1, false);
        check("1e+", 0x3ff0000000000000, 1, false);
        check("1e5x", 0x40f86a0000000000, 3, false);
        check("  \t\n 3.5", 0x400c000000000000, 8, false);
    }

    #[test]
    fn strtod_hex_and_specials() {
        check("0x10", 0x4030000000000000, 4, false);
        check("0x1p4", 0x4030000000000000, 5, false);
        check("0X1P+2", 0x4010000000000000, 6, false);
        check("0x1.8p1", 0x4008000000000000, 7, false);
        check("0x", 0x0000000000000000, 1, false);
        check("0xp3", 0x0000000000000000, 1, false);
        check("inf", 0x7ff0000000000000, 3, false);
        check("INFINITY", 0x7ff0000000000000, 8, false);
        check("infinityx", 0x7ff0000000000000, 8, false);
        check("na", 0x0000000000000000, 0, false);
        assert!(strtod(b"nan").value.is_nan());
        assert_eq!(strtod(b"nan").consumed, 3);
        assert_eq!(strtod(b"nan(123)").consumed, 8);
        assert_eq!(strtod(b"nan(12").consumed, 3);
        assert!(strtod(b"-nan").value.is_sign_negative());
    }

    #[test]
    fn strtod_range_errors() {
        check("1e309", 0x7ff0000000000000, 5, true);
        check("1e999abc", 0x7ff0000000000000, 5, true);
        check("-1e400", 0xfff0000000000000, 6, true);
        check("1e308", 0x7fe1ccf385ebc8a0, 5, false);
        check("1e-320", 0x00000000000007e8, 6, true);
        check("5e-324", 0x0000000000000001, 6, true);
        check("2e-324", 0x0000000000000000, 6, true);
        check("1e-400", 0x0000000000000000, 6, true);
        check("0e999999999999999999999", 0x0000000000000000, 23, false);
        check("1e-999999999999999999999", 0x0000000000000000, 24, true);
        check("1e+999999999999999999999", 0x7ff0000000000000, 24, true);
        // Tininess is detected after rounding, and requires inexactness.
        check("2.2250738585072014e-308", 0x0010000000000000, 23, false);
        check("2.2250738585072013e-308", 0x0010000000000000, 23, false);
        check("2.2250738585072012e-308", 0x0010000000000000, 23, true);
        check("2.225073858507201e-308", 0x000fffffffffffff, 22, true);
        check("0x1p-1023", 0x0008000000000000, 9, false); // exact subnormal
        check("0x1p-1074", 0x0000000000000001, 9, false); // exact subnormal
        check("0x1.8p-1074", 0x0000000000000002, 11, true); // inexact, ties to even
        check("0x1p-1075", 0x0000000000000000, 9, true);
        check("0x1.FFFFFFFFFFFFF8p-1023", 0x0010000000000000, 24, false);
        check("0x1.FFFFFFFFFFFFF7p-1023", 0x0010000000000000, 24, true);
        check("0x1.fffffffffffffp-1023", 0x0010000000000000, 23, true);
        check("0x1p-1022", 0x0010000000000000, 9, false);
        check("0x1.fffffffffffffp1023", 0x7fefffffffffffff, 22, false);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_f2(1024.0), "1024.00");
        assert_eq!(format_f2(-0.0), "-0.00");
        assert_eq!(format_f2(0.125), "0.12"); // ties to even
        assert_eq!(format_f2(0.375), "0.38");
        assert_eq!(format_f2(f64::INFINITY), "inf");
        assert_eq!(format_f2(f64::NEG_INFINITY), "-inf");
        assert_eq!(format_f2(f64::from_bits(0x7ff8000000000000)), "nan");
        assert_eq!(format_f2(f64::from_bits(0xfff8000000000000)), "-nan");
    }

    #[test]
    fn pow_errno_rules() {
        assert!(pow_with_errno(-2.0, 0.5).1 == Errno::Edom);
        assert!(pow_with_errno(-3.0, 3.5).1 == Errno::Edom);
        assert!(pow_with_errno(-2.0, 2.0).1 == Errno::None);
        assert!(pow_with_errno(0.0, -1.0).1 == Errno::Erange);
        assert!(pow_with_errno(-0.0, -0.5).1 == Errno::Erange);
        assert!(pow_with_errno(0.0, f64::NEG_INFINITY).1 == Errno::None);
        assert!(pow_with_errno(0.0, 1.0).1 == Errno::None);
        assert!(pow_with_errno(10.0, 400.0).1 == Errno::Erange);
        assert!(pow_with_errno(10.0, -400.0).1 == Errno::Erange);
        // A subnormal (but non-zero) result is not a range error for glibc.
        assert!(pow_with_errno(2.0, -1074.0).1 == Errno::None);
        assert!(pow_with_errno(10.0, -323.0).1 == Errno::None);
        assert!(pow_with_errno(10.0, -324.0).1 == Errno::Erange);
        assert!(pow_with_errno(f64::INFINITY, 2.0).1 == Errno::None);
        assert!(pow_with_errno(-1.0, f64::INFINITY).1 == Errno::None);
        assert!(pow_with_errno(f64::NAN, 2.0).1 == Errno::None);
        assert!(pow_with_errno(2.0, 0.0).1 == Errno::None);
        assert_eq!(pow_with_errno(2.0, 10.0).0, 1024.0);
    }
}
