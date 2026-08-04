use std::cell::Cell;
use std::io::{self, BufRead};
use std::time::Instant;

thread_local! {
    /// Stores the last benchmark instant (mirrors `bm_tp[0]` in C).
    static BM_TP: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_TP.with(|t| t.set(Some(Instant::now())));
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    BM_TP.with(|t| {
        if let Some(start) = t.get() {
            let elapsed = now.duration_since(start);
            println!(
                "{}: {}.{:09}s",
                msg,
                elapsed.as_secs(),
                elapsed.subsec_nanos()
            );
        }
        t.set(Some(now));
    });
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    let stdin = io::stdin();
    let mut keys: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(s) => keys.push(s),
            Err(_) => break,
        }
    }

    // Randomize order of array (mirroring the loop in bm.c).
    // Uses a simple LCG seeded from the current time so that we don't
    // require an external rng to be plumbed through.
    let mut state: u64 = Instant::now().elapsed().as_nanos() as u64 ^ 0x9E3779B97F4A7C15;
    if state == 0 {
        state = 1;
    }
    let m = keys.len();
    let mut i = m;
    while i > 2 {
        i -= 1;
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = ((state >> 33) as usize) % i;
        keys.swap(i, j);
    }

    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
