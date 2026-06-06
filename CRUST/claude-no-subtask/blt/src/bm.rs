use std::cell::RefCell;
use std::io::{self, BufRead};
use std::time::Instant;

thread_local! {
    static BM_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|s| *s.borrow_mut() = Some(Instant::now()));
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    let start = BM_START.with(|s| s.borrow_mut().replace(now));
    if let Some(start) = start {
        let dur = now.saturating_duration_since(start);
        println!("{}: {}.{:09}s", msg, dur.as_secs(), dur.subsec_nanos());
    } else {
        println!("{}: 0.000000000s", msg);
    }
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
    // Randomize order of array (matching the C version's Fisher-Yates from the back).
    use rand::Rng;
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
