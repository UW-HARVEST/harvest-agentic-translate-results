use std::io::{self, Write};
use std::os::raw::{c_char, c_int};
use std::slice;

#[repr(C)]
struct House {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: f64,
}

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_hex(bytes: &[u8]) {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    writeln!(output).unwrap();
}

fn driver(floors: c_int) {
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };

    // House is fully initialized and repr(C), matching the source object layout.
    let raw = unsafe {
        slice::from_raw_parts(
            (&house as *const House).cast::<u8>(),
            std::mem::size_of::<House>(),
        )
    };
    print_hex(raw);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }
    driver(x);
}
