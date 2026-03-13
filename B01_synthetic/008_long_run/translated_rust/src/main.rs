use std::num::Wrapping;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

// glibc TYPE_3 PRNG state
struct GlibcRand {
    table: [i32; 31],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    fn new(seed: u32) -> Self {
        let mut table = [0i32; 31];
        // glibc treats seed 0 as seed 1
        let seed = if seed == 0 { 1 } else { seed };
        table[0] = seed as i32;
        for i in 1..31 {
            let prev = table[i - 1] as i64;
            table[i] = ((16807 * prev) % 2147483647) as i32;
            if table[i] < 0 {
                table[i] = (table[i] as i64 + 2147483647) as i32;
            }
        }
        // "warm up" with 310 iterations (glibc does 3*31 + 31*10 - 31 = 310 calls conceptually,
        // but actually it runs the feedback loop 310 times)
        let mut fptr: usize = 3;
        let mut rptr: usize = 0;
        for _ in 0..310 {
            let val = Wrapping(table[fptr]) + Wrapping(table[rptr]);
            table[fptr] = val.0;
            fptr += 1;
            if fptr >= 31 {
                fptr = 0;
            }
            rptr += 1;
            if rptr >= 31 {
                rptr = 0;
            }
        }
        GlibcRand { table, fptr, rptr }
    }

    fn next(&mut self) -> i32 {
        let val = Wrapping(self.table[self.fptr]) + Wrapping(self.table[self.rptr]);
        self.table[self.fptr] = val.0;
        let result = (val.0 as u32 >> 1) as i32;
        self.fptr += 1;
        if self.fptr >= 31 {
            self.fptr = 0;
        }
        self.rptr += 1;
        if self.rptr >= 31 {
            self.rptr = 0;
        }
        result
    }
}

fn perform_expensive_operations(array: &mut [i32; ARRAY_SIZE]) {
    for i in 0..ARRAY_SIZE {
        let mut x = Wrapping(array[i]);
        for _ in 0..100 {
            x = x * Wrapping(3) + Wrapping(7);
            x = x ^ (Wrapping(x.0 >> 3));
            x = x - (Wrapping(x.0 << 1));
            // Division and modulo on i32 truncate toward zero in both C and Rust
            x = Wrapping(x.0 / 2 + x.0 % 7);
        }
        array[i] = x.0;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args[0]);
        std::process::exit(1);
    }

    let seed: u32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid seed: '{}'", args[1]);
            std::process::exit(1);
        }
    };

    let mut rng = GlibcRand::new(seed);
    let mut array = [0i32; ARRAY_SIZE];
    for i in 0..ARRAY_SIZE {
        array[i] = rng.next();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    println!("{}", xor_result);
}
