use std::io::{self, Read, Write};

fn print_hex(p: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for b in p {
        write!(out, "{:02x}", b).unwrap();
    }
    writeln!(out).unwrap();
}

fn driver(x: i32) {
    // Match C: char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    // sizeof(int) is 4 on the target platforms; copy the bytes in
    // native (little-endian on x86_64) order to mirror C's memcpy.
    let raw = x.to_ne_bytes();
    print_hex(&raw);
}

fn main() {
    // Read all of stdin and parse whitespace-separated integers,
    // mirroring scanf("%d", ...) which skips whitespace including
    // newlines.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    for tok in input.split_ascii_whitespace() {
        match tok.parse::<i32>() {
            Ok(v) => driver(v),
            Err(_) => {
                // scanf would stop on first parse failure; emulate that.
                break;
            }
        }
    }
}
