use std::io::{self, Read, Write};

fn max_size_frame(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    // Translation of the C expression. Use wrapping arithmetic to mirror
    // unsigned 32-bit integer wraparound semantics from C.
    let ne2: u32 = if channels != 2 { 1 } else { 0 };
    let eq2: u32 = if channels == 2 { 1 } else { 0 };
    let nz32: u32 = if bitdepth != 32 { 1 } else { 0 };

    let term1 = blocksize
        .wrapping_mul(bitdepth)
        .wrapping_mul(channels.wrapping_mul(ne2));
    let term2 = blocksize.wrapping_mul(bitdepth).wrapping_mul(eq2);
    let term3 = blocksize
        .wrapping_mul(bitdepth.wrapping_add(nz32))
        .wrapping_mul(eq2);

    let inner = term1
        .wrapping_add(term2)
        .wrapping_add(term3)
        .wrapping_add(7);

    18u32
        .wrapping_add(channels)
        .wrapping_add(inner / 8)
}

fn main() {
    // Read all of stdin and parse three unsigned 32-bit integers, mirroring
    // the behavior of scanf("%u %u %u", ...) which reads across whitespace
    // (including newlines).
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return;
    }

    let mut iter = input.split_ascii_whitespace();
    let parse_next = |iter: &mut std::str::SplitAsciiWhitespace| -> Option<u32> {
        iter.next().and_then(|s| s.parse::<u32>().ok())
    };

    let blocksize = match parse_next(&mut iter) {
        Some(v) => v,
        None => return,
    };
    let channels = match parse_next(&mut iter) {
        Some(v) => v,
        None => return,
    };
    let bitdepth = match parse_next(&mut iter) {
        Some(v) => v,
        None => return,
    };

    let result = max_size_frame(blocksize, channels, bitdepth);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", result);
}
