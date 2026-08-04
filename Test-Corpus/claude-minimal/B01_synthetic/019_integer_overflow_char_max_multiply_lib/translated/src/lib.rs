// Translation of c_src/src/driver.c to Rust.
//
// The original C code uses `char`, which is typically signed on most
// platforms (equivalent to `i8`). `CHAR_MAX` corresponds to `i8::MAX` (127).
// The `bad()` function intentionally triggers signed overflow when computing
// `data * 2` with `data == CHAR_MAX`. In Rust, signed overflow panics in
// debug builds, so we use `wrapping_mul` to mirror the C behavior.

const CHAR_MAX: i8 = i8::MAX;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn print_hex_char_line(char_hex: i8) {
    // Print as a two-digit hex value, treating the byte as unsigned to
    // match the `%02x` format specifier in C.
    println!("{:02x}", char_hex as u8);
}

pub fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result: i8 = data.wrapping_mul(2);
        print_hex_char_line(result);
    }
}

fn good_b2g() {
    let mut data: i8;
    #[allow(unused_assignments)]
    {
        data = b' ' as i8;
    }
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result: i8 = data.wrapping_mul(2);
            print_hex_char_line(result);
        } else {
            print_line(Some("data value is too large to perform arithmetic safely."));
        }
    }
}

pub fn good() {
    good_g2b();
    good_b2g();
}

/// Public entry point matching the C `driver(int useGood)` function.
#[no_mangle]
pub extern "C" fn driver(use_good: std::os::raw::c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
