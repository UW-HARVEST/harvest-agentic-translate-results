//! Faithful re-implementation of glibc's `srand()` / `rand()`.
//!
//! glibc's `rand()` is `random()`, which uses the default TYPE_3 additive
//! feedback generator: a 31-word state with separation 3.  The logic below
//! mirrors `__srandom_r` / `__random_r` from glibc's `stdlib/random_r.c`
//! step for step, including the truncation to 32-bit words, so the produced
//! stream is bit-identical to the C program's.

/// Degree of the TYPE_3 polynomial (number of state words).
const RAND_DEG: usize = 31;
/// Separation of the TYPE_3 polynomial (initial offset of the front pointer).
const RAND_SEP: usize = 3;

pub struct GlibcRand {
    state: [i32; RAND_DEG],
    /// Index of glibc's `fptr`.
    fptr: usize,
    /// Index of glibc's `rptr`.
    rptr: usize,
}

impl GlibcRand {
    /// Equivalent of `srand(seed)`.
    pub fn new(seed: u32) -> Self {
        // "We must make sure the seed is not 0. Take arbitrarily 1 in this case."
        let seed = if seed == 0 { 1 } else { seed };

        let mut state = [0i32; RAND_DEG];
        state[0] = seed as i32;

        // state[i] = (16807 * state[i - 1]) % 2147483647, computed the way
        // glibc does it so that 31-bit overflow is avoided.  `word` is an
        // `int32_t` in glibc, and the multiply/subtract happens in `long int`
        // before being truncated back down, so do exactly that here.
        let mut word: i32 = seed as i32;
        for i in 1..RAND_DEG {
            let hi: i64 = (word as i64) / 127773;
            let lo: i64 = (word as i64) % 127773;
            word = (16807 * lo - 2836 * hi) as i32;
            if word < 0 {
                word = word.wrapping_add(2147483647);
            }
            state[i] = word;
        }

        let mut rng = GlibcRand {
            state,
            fptr: RAND_SEP,
            rptr: 0,
        };

        // glibc discards `10 * RAND_DEG` outputs to warm the state up.
        for _ in 0..(RAND_DEG * 10) {
            rng.next_i32();
        }

        rng
    }

    /// Equivalent of `rand()`.
    pub fn next_i32(&mut self) -> i32 {
        // `val = *fptr += (uint32_t) *rptr;` -- unsigned wrap-around addition
        // stored back into the (signed) state word.
        let val: u32 = (self.state[self.fptr] as u32).wrapping_add(self.state[self.rptr] as u32);
        self.state[self.fptr] = val as i32;

        // "Chucking least random bit."
        let result = (val >> 1) as i32;

        // Advance the two pointers exactly the way glibc does: the front
        // pointer is bumped first and, if it wrapped, the rear pointer is
        // bumped without its own wrap check (the two can never wrap together).
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
