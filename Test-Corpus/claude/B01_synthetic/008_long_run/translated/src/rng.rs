//! Bit-exact reimplementation of glibc's `srand()`/`rand()`.
//!
//! glibc's `rand()` is `random()`, which uses the TYPE_3 additive-feedback
//! generator: a 31-word state (`rand_deg = 31`, `rand_sep = 3`).
//!
//! `srandom_r`: state[0] = seed (0 becomes 1), the remaining 30 words are
//! filled with the Lehmer generator `state[i] = 16807 * state[i-1] % 2147483647`
//! (computed via Schrage's method exactly as glibc does, in `long int`
//! arithmetic), then `10 * rand_deg == 310` outputs are discarded.
//!
//! `random_r`: `*fptr += (uint32_t) *rptr; result = (uint32_t)*fptr >> 1;`
//! with both pointers advancing cyclically through the state array.

const DEG: usize = 31;
const SEP: usize = 3;

pub struct GlibcRand {
    state: [i32; DEG],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    /// Equivalent to glibc `srand(seed)`.
    pub fn new(seed: u32) -> Self {
        // "We must make sure the seed is not 0.  Take arbitrarily 1 in this case."
        let seed = if seed == 0 { 1 } else { seed };

        let mut state = [0i32; DEG];
        state[0] = seed as i32;

        // word is an int32_t in glibc, promoted to `long int` (64-bit here) for
        // the arithmetic below.
        let mut word: i32 = seed as i32;
        for slot in state.iter_mut().skip(1) {
            let w = word as i64;
            let hi = w / 127773;
            let lo = w % 127773;
            let mut next = 16807 * lo - 2836 * hi;
            if next < 0 {
                next += 2147483647;
            }
            // Truncating store back into the int32_t state slot.
            word = next as i32;
            *slot = word;
        }

        let mut rng = GlibcRand {
            state,
            fptr: SEP,
            rptr: 0,
        };

        // kc *= 10; while (--kc >= 0) discard one output.
        for _ in 0..(10 * DEG) {
            rng.next_i32();
        }

        rng
    }

    /// Equivalent to glibc `rand()`.
    pub fn next_i32(&mut self) -> i32 {
        let val = (self.state[self.fptr] as u32).wrapping_add(self.state[self.rptr] as u32);
        self.state[self.fptr] = val as i32;

        // "Chucking least random bit." -- unsigned shift.
        let result = (val >> 1) as i32;

        self.fptr += 1;
        if self.fptr >= DEG {
            self.fptr = 0;
        }
        self.rptr += 1;
        if self.rptr >= DEG {
            self.rptr = 0;
        }

        result
    }
}
