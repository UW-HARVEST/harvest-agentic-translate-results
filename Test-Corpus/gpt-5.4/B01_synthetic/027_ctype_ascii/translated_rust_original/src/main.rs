use std::io::{self, Read};

fn isalnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn isalpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn islower(c: char) -> bool {
    c.is_ascii_lowercase()
}

fn isupper(c: char) -> bool {
    c.is_ascii_uppercase()
}

fn isdigit(c: char) -> bool {
    c.is_ascii_digit()
}

fn isxdigit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn iscntrl(c: char) -> bool {
    c.is_ascii_control()
}

fn isgraph(c: char) -> bool {
    c.is_ascii_graphic()
}

fn isspace(c: char) -> bool {
    c.is_ascii_whitespace()
}

fn isblank(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

fn isprint(c: char) -> bool {
    c.is_ascii_graphic() || c == ' '
}

fn ispunct(c: char) -> bool {
    c.is_ascii_punctuation()
}

fn tolower(c: char) -> char {
    c.to_ascii_lowercase()
}

fn toupper(c: char) -> char {
    c.to_ascii_uppercase()
}

fn driver(c: char) {
    println!("alphanumeric: {}", if isalnum(c) { 1 } else { 0 });
    println!("alphabetic: {}", if isalpha(c) { 1 } else { 0 });
    println!("lowercase: {}", if islower(c) { 1 } else { 0 });
    println!("uppercase: {}", if isupper(c) { 1 } else { 0 });
    println!("digit: {}", if isdigit(c) { 1 } else { 0 });
    println!("hexadecimal: {}", if isxdigit(c) { 1 } else { 0 });
    println!("control: {}", if iscntrl(c) { 1 } else { 0 });
    println!("graphical: {}", if isgraph(c) { 1 } else { 0 });
    println!("space: {}", if isspace(c) { 1 } else { 0 });
    println!("blank: {}", if isblank(c) { 1 } else { 0 });
    println!("printing: {}", if isprint(c) { 1 } else { 0 });
    println!("punctuation: {}", if ispunct(c) { 1 } else { 0 });
    println!("to lower: {}", tolower(c));
    println!("to upper: {}", toupper(c));
}

fn main() {
    let mut input = [0u8; 1];
    let c = match io::stdin().read(&mut input) {
        Ok(1) => input[0] as char,
        _ => '\0',
    };
    driver(c);
}
