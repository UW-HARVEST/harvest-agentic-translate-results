
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn driver(x: i32, y: i32) {
    let result = x | !y;
    println!("{}", result);
}

fn read_int<R: std::io::BufRead>(reader: &mut R, buffer: &mut String, pos: &mut usize) -> Option<i32> {
    loop {
        // Skip whitespace in the buffer
        while *pos < buffer.len() {
            let c = buffer.as_bytes()[*pos];
            if c.is_ascii_whitespace() {
                *pos += 1;
            } else {
                break;
            }
        }
        if *pos >= buffer.len() {
            buffer.clear();
            *pos = 0;
            let mut chunk = String::new();
            match reader.read_line(&mut chunk) {
                Ok(0) => return None,
                Ok(_) => {
                    buffer.push_str(&chunk);
                    continue;
                }
                Err(_) => return None,
            }
        }
        // Read token
        let start = *pos;
        while *pos < buffer.len() && !buffer.as_bytes()[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        let token = &buffer[start..*pos];
        return token.parse::<i32>().ok();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let mut pos: usize = 0;

    let x = read_int(&mut handle, &mut buffer, &mut pos).unwrap_or(0);
    let y = read_int(&mut handle, &mut buffer, &mut pos).unwrap_or(0);

    driver(x, y);
    0
}