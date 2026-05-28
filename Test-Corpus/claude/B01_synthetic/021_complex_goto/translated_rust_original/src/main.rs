use std::io::Read;

fn parse_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches scanf %d behavior)
    while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return None;
    }
    let start = *pos;
    if bytes[*pos] == b'-' || bytes[*pos] == b'+' {
        *pos += 1;
    }
    let digit_start = *pos;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos == digit_start {
        // No digits read — rewind position so scanf-style "matching failure" leaves
        // the input pointer at the offending character (after whitespace skipping).
        *pos = start;
        return None;
    }
    std::str::from_utf8(&bytes[start..*pos]).ok()?.parse().ok()
}

fn foo(mut x: i32, mut y: i32) {
    // Outer translation of: while (x > 0 || y > 0)
    'outer: while x > 0 || y > 0 {
        println!("loop");

        // if (x == 1 && y == 4) goto label2;
        let mut skip_label1 = x == 1 && y == 4;

        // Inner loop allows `goto label1` to be modeled as `continue`.
        loop {
            // label1:
            if !skip_label1 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            // After the first pass, label1 is no longer skipped — subsequent
            // `goto label1` from below must execute the label1 block.
            skip_label1 = false;

            // label2:
            if y == 0 {
                // `continue;` in the original C re-evaluates the while condition.
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // goto label1 — restart the inner loop.
                continue;
            }
            // Fall through to the bottom of the while body — re-check condition.
            break;
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut input = String::new();
    // Ignore read errors — leaves x and y at their default zero values, just
    // like scanf would if the stream were empty.
    let _ = std::io::stdin().read_to_string(&mut input);
    let bytes = input.as_bytes();
    let mut pos: usize = 0;

    if let Some(v) = parse_int(bytes, &mut pos) {
        x = v;
        if let Some(v) = parse_int(bytes, &mut pos) {
            y = v;
        }
    }

    foo(x, y);
}
