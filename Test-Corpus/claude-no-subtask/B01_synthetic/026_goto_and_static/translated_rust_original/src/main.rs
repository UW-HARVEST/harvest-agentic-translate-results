use std::io::{self, Read, Write};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if x != 1 {
        let _ = writeln!(out, "Error: x != 1");
        result = 1;
        let _ = writeln!(out, "Operation failed");
        return result;
    }

    let y_val = unsafe { Y };
    if y_val != 2 {
        let _ = writeln!(out, "Error: x == 1 but y != 2");
        result = 2;
        let _ = writeln!(out, "Operation failed");
        return result;
    }

    if z != 3 {
        let _ = writeln!(out, "Error: x == 1 and y == 2, but z != 3");
        result = 3;
        let _ = writeln!(out, "Operation failed");
        return result;
    }

    let _ = writeln!(out, "Ok!");
    result
}

fn read_three_ints() -> (i32, i32, i32) {
    // Mimic scanf("%d %d %d", ...) by reading whitespace-separated integers from stdin.
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut iter = input.split_ascii_whitespace();
    let x: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let z: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (x, y, z)
}

fn main() {
    // Note: in original C, x and z are local vars, y is a static global.
    // scanf populates &x, &y (the static), &z. We honor that ordering.
    let (x, sy, z) = read_three_ints();
    unsafe { Y = sy; }

    let result = multi_stage(x, z);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "Result: {}", result);
}
