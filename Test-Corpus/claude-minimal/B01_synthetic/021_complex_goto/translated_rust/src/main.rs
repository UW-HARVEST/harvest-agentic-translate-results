use std::io::Read;

fn foo(mut x: i32, mut y: i32) {
    'outer: while x > 0 || y > 0 {
        println!("loop");

        // `goto label2` is modeled as skipping the label1 block on the
        // first inner iteration when this condition holds.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // `goto label1` -> restart the inner loop.
                continue;
            }
            break;
        }
    }
}

fn main() {
    let mut input = String::new();
    // Mimic C's scanf("%d %d", ...) by reading all of stdin and grabbing
    // the first two integer tokens (whitespace-separated).
    std::io::stdin().read_to_string(&mut input).ok();

    let mut tokens = input.split_whitespace();
    let x: i32 = tokens
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let y: i32 = tokens
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    foo(x, y);
}
