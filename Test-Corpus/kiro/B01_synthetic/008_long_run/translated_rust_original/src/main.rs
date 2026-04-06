use std::env;
use std::process;

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

// glibc TYPE_3 PRNG state
const DEG: usize = 31;
const SEP: usize = 3;

struct GlibcRand {
    state: [i32; DEG],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    fn srand(seed: u32) -> Self {
        let mut state = [0i32; DEG];
        let seed = if seed == 0 { 1 } else { seed as i32 };
        state[0] = seed;
        for i in 1..DEG {
            let val = 16807i64 * state[i - 1] as i64 % 2147483647;
            state[i] = val as i32;
        }
        let mut rng = GlibcRand { state, fptr: SEP, rptr: 0 };
        for _ in 0..(10 * DEG) {
            rng.rand();
        }
        rng
    }

    fn rand(&mut self) -> i32 {
        let val = self.state[self.fptr].wrapping_add(self.state[self.rptr]);
        self.state[self.fptr] = val;
        let result = (val as u32 >> 1) as i32;
        self.fptr += 1;
        if self.fptr >= DEG { self.fptr = 0; }
        self.rptr += 1;
        if self.rptr >= DEG { self.rptr = 0; }
        result
    }
}

fn perform_expensive_operations(array: &mut [i32; ARRAY_SIZE]) {
    for i in 0..ARRAY_SIZE {
        let mut x = array[i];
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x = x ^ (x >> 3);
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = x / 2 + x % 7;
        }
        array[i] = x;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <seed>", args[0]);
        process::exit(1);
    }

    let seed: u32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid seed: '{}'", args[1]);
            process::exit(1);
        }
    };

    let mut rng = GlibcRand::srand(seed);

    let mut array = Box::new([0i32; ARRAY_SIZE]);
    for i in 0..ARRAY_SIZE {
        array[i] = rng.rand();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut *array);
    }

    let mut xor_result: i32 = 0;
    for i in 0..ARRAY_SIZE {
        xor_result ^= array[i];
    }

    println!("{}", xor_result);
}
