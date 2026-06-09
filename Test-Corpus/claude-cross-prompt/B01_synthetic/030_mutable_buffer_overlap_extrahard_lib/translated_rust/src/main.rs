use std::io::{self, Read, Write, BufWriter};

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn inner<W: Write>(out: &mut [i32], len: usize, w: &mut W) {
    // fma_array(out, out, out, out, len) — equivalent to out[i] = out[i]*out[i] + out[i]
    let snapshot: Vec<i32> = out[..len].to_vec();
    fma_array(out, &snapshot, &snapshot, &snapshot, len);
    for i in 0..len {
        writeln!(w, "{}", out[i]).unwrap();
    }
}

fn driver<W: Write>(data: &[i32], len: usize, w: &mut W) {
    let mut out: Vec<i32> = data[..len].to_vec();
    inner(&mut out, len, w);
}

fn main() {
    // Read all input from stdin and tokenize like scanf("%d") does
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_ascii_whitespace();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // First token: length
    let len: i32 = match tokens.next() {
        Some(t) => t.parse().unwrap_or(0),
        None => 0,
    };

    if len <= 0 {
        return;
    }

    let len_usize = len as usize;
    let mut data: Vec<i32> = Vec::with_capacity(len_usize);
    for _ in 0..len_usize {
        let v: i32 = match tokens.next() {
            Some(t) => t.parse().unwrap_or(0),
            None => 0,
        };
        data.push(v);
    }

    driver(&data, len_usize, &mut out);
}
