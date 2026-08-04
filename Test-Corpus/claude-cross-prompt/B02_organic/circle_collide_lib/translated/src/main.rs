use std::io::{self, Read, Write};

mod collide;
use collide::circle_collide;

/// Mimics C's `scanf("%f %f %f", ...)` for reading three floats from stdin
/// across whitespace (including newlines).
fn read_three_floats() -> Option<(f32, f32, f32)> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let mut iter = buf.split_ascii_whitespace();
    let x: f32 = iter.next()?.parse().ok()?;
    let y: f32 = iter.next()?.parse().ok()?;
    let r: f32 = iter.next()?.parse().ok()?;
    Some((x, y, r))
}

fn main() {
    let (x, y, r) = match read_three_floats() {
        Some(v) => v,
        None => return,
    };
    let result = circle_collide(x, y, r);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", result);
}
