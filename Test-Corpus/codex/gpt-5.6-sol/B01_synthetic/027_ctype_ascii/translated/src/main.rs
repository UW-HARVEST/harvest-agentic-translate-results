use std::io::{self, Read, Write};

fn ctype_value(condition: bool, value: i32) -> i32 {
    if condition {
        value
    } else {
        0
    }
}

fn main() {
    let mut input = [0xff];
    let _ = io::stdin().read(&mut input);
    let c = input[0];

    let mut output = Vec::new();
    let _ = writeln!(
        output,
        "alphanumeric: {}",
        ctype_value(c.is_ascii_alphanumeric(), 8)
    );
    let _ = writeln!(
        output,
        "alphabetic: {}",
        ctype_value(c.is_ascii_alphabetic(), 1024)
    );
    let _ = writeln!(
        output,
        "lowercase: {}",
        ctype_value(c.is_ascii_lowercase(), 512)
    );
    let _ = writeln!(
        output,
        "uppercase: {}",
        ctype_value(c.is_ascii_uppercase(), 256)
    );
    let _ = writeln!(
        output,
        "digit: {}",
        ctype_value(c.is_ascii_digit(), 2048)
    );
    let _ = writeln!(
        output,
        "hexadecimal: {}",
        ctype_value(c.is_ascii_hexdigit(), 4096)
    );
    let _ = writeln!(
        output,
        "control: {}",
        ctype_value(c.is_ascii_control(), 2)
    );
    let _ = writeln!(
        output,
        "graphical: {}",
        ctype_value(c.is_ascii_graphic(), 32768)
    );
    let _ = writeln!(
        output,
        "space: {}",
        ctype_value(matches!(c, b'\t'..=b'\r' | b' '), 8192)
    );
    let _ = writeln!(
        output,
        "blank: {}",
        ctype_value(matches!(c, b'\t' | b' '), 1)
    );
    let _ = writeln!(
        output,
        "printing: {}",
        ctype_value(c.is_ascii_graphic() || c == b' ', 16384)
    );
    let _ = writeln!(
        output,
        "punctuation: {}",
        ctype_value(c.is_ascii_punctuation(), 4)
    );

    output.extend_from_slice(b"to lower: ");
    output.push(c.to_ascii_lowercase());
    output.push(b'\n');
    output.extend_from_slice(b"to upper: ");
    output.push(c.to_ascii_uppercase());
    output.push(b'\n');

    let _ = io::stdout().write_all(&output);
}
