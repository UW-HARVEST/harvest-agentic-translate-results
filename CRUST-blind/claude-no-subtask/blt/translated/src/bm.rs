use std::cell::RefCell;
use std::io::{self, BufRead};
use std::time::Instant;

use rand::Rng;

thread_local! {
    static BM_TIME: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_TIME.with(|t| {
        *t.borrow_mut() = Some(Instant::now());
    });
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    let elapsed = BM_TIME.with(|t| {
        let prev = t.borrow().clone();
        *t.borrow_mut() = Some(now);
        prev
    });
    let dur = match elapsed {
        Some(prev) => now.duration_since(prev),
        None => std::time::Duration::ZERO,
    };
    let secs = dur.as_secs();
    let nanos = dur.subsec_nanos();
    println!("{}: {}.{:09}s", msg, secs, nanos);
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    let stdin = io::stdin();
    let mut keys: Vec<String> = Vec::new();
    for line_res in stdin.lock().lines() {
        match line_res {
            Ok(line) => keys.push(line),
            Err(_) => break,
        }
    }
    // Randomize order of array (Fisher-Yates style, mimicking C code).
    let mut rng = rand::rng();
    let m = keys.len();
    if m > 2 {
        for i in (2..m).rev() {
            let j = rng.random_range(0..i);
            keys.swap(i, j);
        }
    }
    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
