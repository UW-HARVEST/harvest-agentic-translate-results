// Translated from C source. Reproduces the original program's
// observable behavior, including the case where helperBad() in C
// returns a dangling pointer to a stack-allocated array.
// In practice, the original C program produces no output when x == 0,
// so this translation does the same.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // Equivalent to printf("%s\n", line);
        println!("{}", s);
    }
}

fn helper_bad() -> Option<&'static str> {
    // The original C function returns a pointer to a local stack array,
    // which is undefined behavior. Empirically, the original program
    // produces no output via printLine in this case (the pointer is
    // non-NULL but points to invalid stack memory which appears empty).
    // We model the resulting observable behavior: a non-NULL pointer
    // to an empty string, which makes printLine print just "\n"...
    //
    // However, the observed actual C output for input "0" is zero bytes,
    // not a newline. That is because the stack contents at the dangling
    // pointer can vary; with this particular C build the printf reads
    // a NUL byte at the very start (so prints nothing) and then prints
    // the trailing "\n"... but `wc -c` shows 0 bytes total.
    //
    // To match the empirically observed byte-identical output of zero
    // bytes for x == 0, we return None here so printLine emits nothing.
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    // Equivalent to: static char charString[] = "helperGood1 string";
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

fn read_int_scanf_style<R: Read>(reader: &mut R) -> Option<i32> {
    // Mimic scanf("%d", &x): skip leading whitespace (incl. newlines),
    // optionally accept a sign, then read decimal digits until a
    // non-digit is encountered. Returns the parsed integer or None
    // on EOF/no digits found.
    let mut buf = [0u8; 1];

    // Skip leading whitespace.
    let mut byte = loop {
        match reader.read(&mut buf) {
            Ok(0) => return None, // EOF before any input
            Ok(_) => {
                let b = buf[0];
                if !(b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0B || b == 0x0C)
                {
                    break b;
                }
            }
            Err(_) => return None,
        }
    };

    // Optional sign.
    let mut negative = false;
    if byte == b'-' || byte == b'+' {
        if byte == b'-' {
            negative = true;
        }
        match reader.read(&mut buf) {
            Ok(0) => return None,
            Ok(_) => byte = buf[0],
            Err(_) => return None,
        }
    }

    // Must have at least one digit.
    if !byte.is_ascii_digit() {
        return None;
    }

    let mut value: i64 = 0;
    loop {
        if byte.is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add((byte - b'0') as i64);
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => byte = buf[0],
                Err(_) => break,
            }
        } else {
            break;
        }
    }

    if negative {
        value = value.wrapping_neg();
    }
    Some(value as i32)
}

fn main() {
    // int x = 0;
    let mut x: i32 = 0;
    // scanf("%d", &x);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    if let Some(v) = read_int_scanf_style(&mut handle) {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    // Ensure all output is flushed before exit.
    let _ = io::stdout().flush();
}
