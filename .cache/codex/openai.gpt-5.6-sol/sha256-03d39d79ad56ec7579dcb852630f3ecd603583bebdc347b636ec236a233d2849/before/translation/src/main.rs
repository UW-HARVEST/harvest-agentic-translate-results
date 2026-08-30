use std::ffi::{c_char, c_int};
use std::io::{self, BufWriter, Write};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn read_x() -> i32 {
    let mut x: c_int = 0;

    // Use libc for the input boundary so conversion and whitespace handling
    // remain identical to scanf("%d", &x).
    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }

    x
}

fn driver<W: Write>(x: i32, output: &mut W) -> io::Result<()> {
    let mut i = 0_i32;
    let mut j = 0_i32;

    while i < x {
        writeln!(output, "{i} {j}")?;
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }

    Ok(())
}

fn main() {
    let x = read_x();
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let _ = driver(x, &mut output);
}
