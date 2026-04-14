use std::os::raw::c_uint;
use std::sync::{Mutex, OnceLock};

const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: usize = 2000;

static ARRAY: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();

fn array_storage() -> &'static Mutex<Vec<i32>> {
    ARRAY.get_or_init(|| Mutex::new(vec![0; ARRAY_SIZE]))
}

struct CPrng {
    state: u32,
}

impl CPrng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn rand(&mut self) -> i32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.state / 65536) % 32768) as i32
    }
}

fn perform_expensive_operations(array: &mut [i32]) {
    for x in array.iter_mut() {
        let mut v = *x;
        for _ in 0..100 {
            v = v.wrapping_mul(3).wrapping_add(7);
            v ^= v >> 3;
            v = v.wrapping_sub(v.wrapping_shl(1));
            v = v / 2 + v % 7;
        }
        *x = v;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn long_exec(seed: c_uint) {
    let mut prng = CPrng::new(seed);
    let mut array = array_storage().lock().unwrap();

    for x in array.iter_mut() {
        *x = prng.rand();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let xor_result = array.iter().copied().fold(0i32, |acc, x| acc ^ x);
    println!("{}", xor_result);
}