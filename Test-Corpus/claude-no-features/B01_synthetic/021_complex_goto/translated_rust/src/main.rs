use std::io::Read;

fn foo(mut x: i32, mut y: i32) {
    'outer: while x > 0 || y > 0 {
        println!("loop");

        // If x == 1 && y == 4, skip the label1 block (goto label2).
        let mut do_label1 = !(x == 1 && y == 4);

        loop {
            if do_label1 {
                // label1
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }

            // label2
            if y == 0 {
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // goto label1 — re-enter the label1 block
                do_label1 = true;
                continue;
            }
            break;
        }
    }
}

/// Mimic C's `scanf("%d", ...)`:
/// - skip leading whitespace
/// - optional sign
/// - read decimal digits
/// - return parsed value and advance the cursor (only on success)
fn scanf_int(bytes: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    let start = *pos;
    let mut i = *pos;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        // No digits — match failure; do not advance past sign per scanf behavior
        // (Actually scanf may push back the sign on no-digits; safest: don't advance)
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..i]).ok()?;
    let v = s.parse::<i64>().ok()?;
    *pos = i;
    // C scanf with %d on overflow is UB; saturate to int range
    Some(v as i32)
}

fn main() {
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let mut pos: usize = 0;
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    if let Some(v) = scanf_int(&input, &mut pos) {
        x = v;
        if let Some(v2) = scanf_int(&input, &mut pos) {
            y = v2;
        }
    }
    foo(x, y);
}
