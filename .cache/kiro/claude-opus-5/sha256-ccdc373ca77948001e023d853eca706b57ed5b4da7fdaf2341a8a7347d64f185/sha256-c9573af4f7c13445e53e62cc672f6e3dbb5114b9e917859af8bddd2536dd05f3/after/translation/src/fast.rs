//! Exact accelerator for `f^n` where `f` is the C inner-loop body
//! (`crate::kernel_step`).
//!
//! `long_exec` calls `perform_expensive_operations()` 2000 times, and that
//! function applies `f` 100 times to every element independently.  So the whole
//! nested loop is nothing but `f^200000` applied element-wise.
//!
//! `f` maps `i32 -> i32`, i.e. it is a self-map of a finite set, so every orbit
//! is eventually periodic: `v, f(v), ...` runs down a "tail" of length `mu` and
//! then cycles with period `lambda`.  Once the cycle is known, `f^n(v)` for any
//! `n >= mu` is a single modular index into the cycle.  Everything here is exact
//! function-iteration algebra -- no approximation -- so the resulting array (and
//! therefore the printed XOR) is bit-identical to running the naive nested loop.
//!
//! Orbits started from different array elements coalesce, so a memo of
//! "how this value relates to its cycle" makes the whole pass cost roughly the
//! number of *distinct* values visited instead of `262144 * 200000` steps.
//!
//! Every escape hatch in here falls back to plain iteration, which is why the
//! result cannot depend on the heuristics: they only decide how much work is
//! saved, never what is computed.

use crate::{kernel_iterate, kernel_step};
use std::collections::HashMap;
use std::ffi::c_int;
use std::hash::{BuildHasherDefault, Hasher};

/// Size of the membership filter in bits (2^25 bits = 4 MiB).
const FILTER_BITS: u32 = 25;
const FILTER_WORDS: usize = 1usize << (FILTER_BITS - 5);

/// Upper bound on memoised tail entries, to keep peak memory bounded.
const MEMO_CAP: usize = 6 << 20;

/// Only every `STRIDE`-th value of a walked tail is memoised; a later walk then
/// overshoots the first already-known value by at most `STRIDE` steps.
const STRIDE: usize = 16;

/// Refuse to materialise absurdly long cycles (keeps worst-case memory bounded).
const MAX_CYCLE: u64 = 8 << 20;

/// Cycle discovery costs a few times the orbit length, so it only pays off when
/// `n` is large.  Below this we just iterate.
const LEARN_MIN_N: u32 = 8192;

/// Hard cap on cycle-discovery attempts, so a pathological input can never turn
/// this into more work than the naive loop.
const MAX_LEARN_ATTEMPTS: u32 = 64;

/// Cheap multiplicative hasher; keys are already well-mixed integers.
#[derive(Default)]
struct IntHasher(u64);

impl Hasher for IntHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.0 = (i as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0 ^ (self.0 >> 29)
    }
}

type IntMap<K, V> = HashMap<K, V, BuildHasherDefault<IntHasher>>;

/// For a value `y`: for every `t >= dist`, `f^t(y) == cycles[cycle][(off + t) % len]`.
#[derive(Clone, Copy)]
struct Entry {
    cycle: u32,
    off: u32,
    dist: u32,
}

struct Solver {
    cycles: Vec<Vec<c_int>>,
    memo: IntMap<c_int, Entry>,
    filter: Vec<u32>,
    path: Vec<c_int>,
    learn_attempts: u32,
}

#[inline(always)]
fn filter_index(x: c_int) -> usize {
    let h = (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    ((h >> (64 - FILTER_BITS)) as usize) & ((1usize << FILTER_BITS) - 1)
}

#[inline]
fn clamp_u32(v: u64) -> u32 {
    if v > u32::MAX as u64 {
        u32::MAX
    } else {
        v as u32
    }
}

impl Solver {
    fn new() -> Self {
        Solver {
            cycles: Vec::new(),
            memo: IntMap::default(),
            filter: vec![0u32; FILTER_WORDS],
            path: Vec::new(),
            learn_attempts: 0,
        }
    }

    #[inline(always)]
    fn maybe_known(&self, x: c_int) -> bool {
        let i = filter_index(x);
        (self.filter[i >> 5] >> (i & 31)) & 1 != 0
    }

    #[inline(always)]
    fn lookup(&self, x: c_int) -> Option<Entry> {
        if self.maybe_known(x) {
            self.memo.get(&x).copied()
        } else {
            None
        }
    }

    /// `force` bypasses the size cap; used for cycle members, which must stay
    /// resident or later walks cannot short-circuit at all.
    fn remember(&mut self, x: c_int, e: Entry, force: bool) {
        if !force && self.memo.len() >= MEMO_CAP {
            return;
        }
        self.memo.entry(x).or_insert(e);
        let i = filter_index(x);
        self.filter[i >> 5] |= 1u32 << (i & 31);
    }

    /// Brent's cycle finding: returns `(mu, lambda)` for the orbit of `v`.
    fn brent(v: c_int) -> (u64, u64) {
        let mut power: u64 = 1;
        let mut lam: u64 = 1;
        let mut tortoise = v;
        let mut hare = kernel_step(v);
        while tortoise != hare {
            if power == lam {
                tortoise = hare;
                power *= 2;
                lam = 0;
            }
            hare = kernel_step(hare);
            lam += 1;
        }
        let mut tortoise = v;
        let mut hare = v;
        for _ in 0..lam {
            hare = kernel_step(hare);
        }
        let mut mu: u64 = 0;
        while tortoise != hare {
            tortoise = kernel_step(tortoise);
            hare = kernel_step(hare);
            mu += 1;
        }
        (mu, lam)
    }

    /// Discover the cycle reached from `v` and memoise it plus a subsample of
    /// the tail leading into it.  Re-uses an already known cycle if this orbit
    /// funnels into one.
    fn learn(&mut self, v: c_int) {
        self.learn_attempts += 1;
        let (mu, lam) = Self::brent(v);

        let mut entry_val = v;
        for _ in 0..mu {
            entry_val = kernel_step(entry_val);
        }

        // Position of `entry_val` on its cycle, creating the cycle if new.
        // `entry_val` is `f^mu(v)`, so it genuinely lies on a cycle; if a memo
        // entry exists at all then its `off` is that cycle position (both sides
        // of the memo identity are periodic in `t`), so the cycle can be reused
        // rather than duplicated.
        let (ci, entry_pos, len) = match self.lookup(entry_val) {
            Some(e) => {
                let len = self.cycles[e.cycle as usize].len() as u64;
                (e.cycle, e.off as u64, len)
            }
            None => {
                if lam > MAX_CYCLE {
                    return;
                }
                let mut vals = Vec::with_capacity(lam as usize);
                let mut y = entry_val;
                for _ in 0..lam {
                    vals.push(y);
                    y = kernel_step(y);
                }
                let ci = self.cycles.len() as u32;
                self.cycles.push(vals);
                let len = self.cycles[ci as usize].len();
                for pos in 0..len {
                    let val = self.cycles[ci as usize][pos];
                    self.remember(
                        val,
                        Entry {
                            cycle: ci,
                            off: pos as u32,
                            dist: 0,
                        },
                        true,
                    );
                }
                (ci, 0u64, len as u64)
            }
        };

        // Tail: y_j = f^j(v) reaches cycle position `entry_pos` after mu - j steps.
        let mut y = v;
        let mut j: u64 = 0;
        while j < mu {
            if (j as usize) % STRIDE == 0 {
                let back = mu - j;
                let off = ((entry_pos + len - (back % len)) % len) as u32;
                self.remember(
                    y,
                    Entry {
                        cycle: ci,
                        off,
                        dist: clamp_u32(back),
                    },
                    false,
                );
            }
            y = kernel_step(y);
            j += 1;
        }
    }

    /// Exact `f^n(v)`.
    fn resolve(&mut self, v: c_int, n: u32) -> c_int {
        let mut path = std::mem::take(&mut self.path);
        path.clear();

        let mut x = v;
        let mut k: u32 = 0;
        let hit = loop {
            if let Some(e) = self.lookup(x) {
                break Some(e);
            }
            if k == n {
                break None;
            }
            path.push(x);
            x = kernel_step(x);
            k += 1;
        };

        let answer = match hit {
            None => {
                // Walked the full n steps, so `x` is already the exact answer.
                // Learn this basin so that later elements are cheap.
                if n >= LEARN_MIN_N && self.learn_attempts < MAX_LEARN_ATTEMPTS {
                    self.learn(v);
                }
                x
            }
            Some(e) => {
                let len = self.cycles[e.cycle as usize].len() as u64;
                let total_dist = k as u64 + e.dist as u64;
                let ans = if (n as u64) >= total_dist {
                    let idx = ((e.off as u64 + n as u64 - k as u64) % len) as usize;
                    self.cycles[e.cycle as usize][idx]
                } else {
                    kernel_iterate(v, n)
                };
                // Memoise a subsample of the freshly walked tail.
                let mut j = 0usize;
                while j < path.len() {
                    let y = path[j];
                    let back = (k as u64) - (j as u64);
                    let off = ((e.off as u64 + len - (back % len)) % len) as u32;
                    let dist = clamp_u32(back + e.dist as u64);
                    self.remember(
                        y,
                        Entry {
                            cycle: e.cycle,
                            off,
                            dist,
                        },
                        false,
                    );
                    j += STRIDE;
                }
                ans
            }
        };

        self.path = path;
        answer
    }
}

/// Apply `f^n` to every element of `arr`, exactly as `n / 100` calls to
/// `perform_expensive_operations()` would.
pub fn apply_iterations(arr: &mut [c_int], n: u32) {
    if n == 0 {
        return;
    }
    if n < LEARN_MIN_N {
        // Cheap enough that the bookkeeping would cost more than it saves.
        for slot in arr.iter_mut() {
            *slot = kernel_iterate(*slot, n);
        }
        return;
    }
    let mut solver = Solver::new();
    for slot in arr.iter_mut() {
        *slot = solver.resolve(*slot, n);
    }
    debug_report(&solver, n);
}

#[cfg(feature = "debug-stats")]
fn debug_report(solver: &Solver, n: u32) {
    let cyc_vals: usize = solver.cycles.iter().map(|c| c.len()).sum();
    let mut lens: Vec<usize> = solver.cycles.iter().map(|c| c.len()).collect();
    lens.sort_unstable();
    eprintln!(
        "DBG n={} learn_attempts={} cycles={} cyc_vals={} memo={} cycle_lens={:?}",
        n,
        solver.learn_attempts,
        solver.cycles.len(),
        cyc_vals,
        solver.memo.len(),
        lens
    );
}

#[cfg(not(feature = "debug-stats"))]
#[inline(always)]
fn debug_report(_solver: &Solver, _n: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lcg(state: &mut u64) -> c_int {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 33) as u32 as i32
    }

    #[test]
    fn fast_matches_naive_small_n() {
        let mut st = 12345u64;
        let vals: Vec<c_int> = (0..2000).map(|_| lcg(&mut st)).collect();
        for &n in &[1u32, 2, 3, 7, 13, 100, 137, 1000, 4096, 8192, 8193, 12345] {
            let mut a = vals.clone();
            apply_iterations(&mut a, n);
            for (i, &v) in vals.iter().enumerate() {
                assert_eq!(a[i], kernel_iterate(v, n), "n={} v={}", n, v);
            }
        }
    }

    #[test]
    fn fast_matches_naive_large_n() {
        let mut st = 999u64;
        let vals: Vec<c_int> = (0..64)
            .map(|_| lcg(&mut st))
            .chain([0, -1, 1, i32::MIN, i32::MAX, 7, -7])
            .collect();
        for &n in &[200000u32, 199999, 200001, 65536] {
            let mut a = vals.clone();
            apply_iterations(&mut a, n);
            for (i, &v) in vals.iter().enumerate() {
                assert_eq!(a[i], kernel_iterate(v, n), "n={} v={}", n, v);
            }
        }
    }
}
