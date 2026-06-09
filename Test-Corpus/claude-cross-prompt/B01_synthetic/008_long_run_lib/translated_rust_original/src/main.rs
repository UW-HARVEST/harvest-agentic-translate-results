// Translation of c_src/src/long.c to Rust.
// Produces byte-identical output to the original C program for the same input.

use std::io::{self, Read, Write, BufWriter};

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: i32 = 2000;

// ---------------------------------------------------------------------------
// glibc-compatible rand()/srand() implementation (TYPE_3 random).
//
// glibc's default rand() uses the additive feedback generator with
// rand_deg = 31 and rand_sep = 3. The state is 31 32-bit words.
// On srand(seed):
//   state[0] = (seed == 0 ? 1 : seed)
//   for i=1..30: state[i] = (16807 * state[i-1]) mod (2^31 - 1)
//      (computed via Schrage's method as in glibc)
//   fptr = &state[3]; rptr = &state[0];
//   then 310 warm-up calls to random() are made (results discarded).
// On rand():
//   val = state[fptr] = state[fptr] + state[rptr]    (32-bit wrap)
//   result = val >> 1   (treated as unsigned shift -> nonnegative int)
//   advance fptr/rptr modulo 31.
// ---------------------------------------------------------------------------

const RAND_DEG: usize = 31;
const RAND_SEP: usize = 3;

struct GlibcRand {
    state: [i32; RAND_DEG],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    fn new() -> Self {
        GlibcRand {
            state: [0; RAND_DEG],
            fptr: RAND_SEP,
            rptr: 0,
        }
    }

    fn srand(&mut self, seed: u32) {
        let mut seed = seed;
        if seed == 0 {
            seed = 1;
        }
        // state[0] = seed; reinterpret unsigned -> signed bit-for-bit.
        self.state[0] = seed as i32;
        let mut word: i32 = seed as i32;
        for i in 1..RAND_DEG {
            // word = (16807 * word) mod (2^31 - 1) using Schrage's method.
            let hi: i32 = word / 127773;
            let lo: i32 = word % 127773;
            let mut w: i32 = 16807i32.wrapping_mul(lo).wrapping_sub(2836i32.wrapping_mul(hi));
            if w < 0 {
                w = w.wrapping_add(2147483647);
            }
            self.state[i] = w;
            word = w;
        }
        self.fptr = RAND_SEP;
        self.rptr = 0;
        // Warm up: 31 * 10 = 310 discarded calls.
        let kc = RAND_DEG * 10;
        for _ in 0..kc {
            let _ = self.rand();
        }
    }

    fn rand(&mut self) -> i32 {
        // val = *fptr += (uint32_t) *rptr  (32-bit unsigned arithmetic)
        let f = self.state[self.fptr] as u32;
        let r = self.state[self.rptr] as u32;
        let val = f.wrapping_add(r);
        self.state[self.fptr] = val as i32;
        // result = val >> 1   (logical shift, then interpreted as nonnegative int)
        let result = (val >> 1) as i32;

        // Advance pointers modulo RAND_DEG.
        self.fptr += 1;
        if self.fptr >= RAND_DEG {
            self.fptr = 0;
            self.rptr += 1;
        } else {
            self.rptr += 1;
            if self.rptr >= RAND_DEG {
                self.rptr = 0;
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Workload, translated literally from the C source.
// ---------------------------------------------------------------------------

fn perform_expensive_operations(array: &mut [i32]) {
    for i in 0..ARRAY_SIZE {
        let mut x: i32 = array[i];
        for _ in 0..100 {
            // x = x * 3 + 7;
            x = x.wrapping_mul(3).wrapping_add(7);
            // x = x ^ (x >> 3);   (arithmetic shift right for signed in Rust)
            x ^= x >> 3;
            // x = x - (x << 1);
            x = x.wrapping_sub(x.wrapping_shl(1));
            // x = x / 2 + x % 7;  (C's signed div/mod truncates toward zero,
            // same as Rust's / and % on i32)
            x = (x / 2).wrapping_add(x % 7);
        }
        array[i] = x;
    }
}

fn long_exec(seed: u32) {
    let mut rng = GlibcRand::new();
    rng.srand(seed);

    // Heap-allocate to avoid blowing the stack with a 1 MiB array.
    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];

    for i in 0..ARRAY_SIZE {
        array[i] = rng.rand();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    // printf("%d\n", xor_result);
    writeln!(out, "{}", xor_result).expect("write failed");
}

// ---------------------------------------------------------------------------
// Entry point.
//
// The C source provides a library function `long_exec(unsigned int seed)`
// with no main(); to make the translated artifact an executable we read a
// single unsigned int seed from stdin in scanf-style fashion (whitespace
// separated, may span newlines).
// ---------------------------------------------------------------------------

fn read_uint_from_stdin() -> u32 {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("failed to read stdin");
    // scanf("%u", ...) skips leading whitespace and parses the longest digit
    // run. Replicate that behavior.
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if start == i {
        // No digits found; default to 0 (matches uninitialized scanf failure).
        return 0;
    }
    // Parse as u64 to allow values that wrap into u32 the way C's %u would.
    let s = &input[start..i];
    let parsed: u64 = s.parse().unwrap_or(0);
    parsed as u32
}

fn main() {
    let seed = read_uint_from_stdin();
    long_exec(seed);
}
