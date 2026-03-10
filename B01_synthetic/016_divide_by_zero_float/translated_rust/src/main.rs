use std::io::BufRead;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Read up to 19 bytes (like fgets with buffer size 20) from stdin.
/// Returns None on EOF/error (matching fgets returning NULL).
fn fgets_line(stdin: &mut impl BufRead) -> Option<String> {
    let mut buf = [0u8; 19];
    let mut pos = 0;
    loop {
        if pos >= 19 {
            break;
        }
        let mut one = [0u8; 1];
        match stdin.read(&mut one) {
            Ok(0) => {
                if pos == 0 {
                    return None;
                }
                break;
            }
            Ok(_) => {
                buf[pos] = one[0];
                pos += 1;
                if one[0] == b'\n' {
                    break;
                }
            }
            Err(_) => {
                if pos == 0 {
                    return None;
                }
                break;
            }
        }
    }
    Some(String::from_utf8_lossy(&buf[..pos]).into_owned())
}

fn bad(stdin: &mut impl BufRead) {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_line(stdin) {
        data = input.trim_end_matches('\n').parse::<f64>().unwrap_or(0.0) as f32;
    } else {
        print_line("fgets() failed.");
    }
    let result = (100.0_f64 / data as f64) as i32;
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = (100.0_f64 / data as f64) as i32;
    print_int_line(result);
}

fn good_b2g(stdin: &mut impl BufRead) {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_line(stdin) {
        data = input.trim_end_matches('\n').parse::<f64>().unwrap_or(0.0) as f32;
    } else {
        print_line("fgets() failed.");
    }
    if (data as f64).abs() > 0.000001 {
        let result = (100.0_f64 / data as f64) as i32;
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(stdin: &mut impl BufRead) {
    good_g2b();
    good_b2g(stdin);
}

fn main() {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    print_line("Calling good()...");
    good(&mut reader);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut reader);
    print_line("Finished bad()");
}
