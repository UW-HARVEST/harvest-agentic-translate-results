use std::ffi::c_char;

fn is_ascii_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_ascii_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_ascii_lower(c: u8) -> bool {
    c.is_ascii_lowercase()
}

fn is_ascii_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

fn is_ascii_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_ascii_hexdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn is_ascii_cntrl(c: u8) -> bool {
    c.is_ascii_control()
}

fn is_ascii_graph(c: u8) -> bool {
    c.is_ascii_graphic()
}

fn is_ascii_space(c: u8) -> bool {
    c.is_ascii_whitespace()
}

fn is_ascii_blank(c: u8) -> bool {
    matches!(c, b' ' | b'\t')
}

fn is_ascii_print(c: u8) -> bool {
    matches!(c, 0x20..=0x7e)
}

fn is_ascii_punct(c: u8) -> bool {
    c.is_ascii_punctuation()
}

fn to_ascii_lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn to_ascii_upper(c: u8) -> u8 {
    c.to_ascii_uppercase()
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(c: c_char) {
    let b = c as u8;

    println!("alphanumeric: {}", if is_ascii_alnum(b) { 1 } else { 0 });
    println!("alphabetic: {}", if is_ascii_alpha(b) { 1 } else { 0 });
    println!("lowercase: {}", if is_ascii_lower(b) { 1 } else { 0 });
    println!("uppercase: {}", if is_ascii_upper(b) { 1 } else { 0 });
    println!("digit: {}", if is_ascii_digit(b) { 1 } else { 0 });
    println!("hexadecimal: {}", if is_ascii_hexdigit(b) { 1 } else { 0 });
    println!("control: {}", if is_ascii_cntrl(b) { 1 } else { 0 });
    println!("graphical: {}", if is_ascii_graph(b) { 1 } else { 0 });
    println!("space: {}", if is_ascii_space(b) { 1 } else { 0 });
    println!("blank: {}", if is_ascii_blank(b) { 1 } else { 0 });
    println!("printing: {}", if is_ascii_print(b) { 1 } else { 0 });
    println!("punctuation: {}", if is_ascii_punct(b) { 1 } else { 0 });
    println!("to lower: {}", to_ascii_lower(b) as char);
    println!("to upper: {}", to_ascii_upper(b) as char);
}
