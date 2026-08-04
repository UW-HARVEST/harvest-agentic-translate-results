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
    BM_START.with(|s| {
        let elapsed = match *s.borrow() {
            Some(start) => now.duration_since(start),
            None => std::time::Duration::ZERO,
        };
        let secs = elapsed.as_secs();
        let nsec = elapsed.subsec_nanos();
        println!("{}: {}.{:09}s", msg, secs, nsec);
        *s.borrow_mut() = Some(now);
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

    // Randomize order using a simple Fisher-Yates shuffle backed by `rand`.
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let m = keys.len();
    if m > 2 {
        for i in (2..m).rev() {
            let j = rng.gen_range(0..i);
            keys.swap(i, j);
        }
    }

    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
