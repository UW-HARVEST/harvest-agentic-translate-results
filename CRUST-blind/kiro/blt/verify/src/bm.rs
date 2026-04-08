use std::io::{self, BufRead};
use std::time::Instant;

use rand::seq::SliceRandom;
use rand::rng;

thread_local! {
    static BM_START: std::cell::RefCell<Option<Instant>> = std::cell::RefCell::new(None);
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|s| *s.borrow_mut() = Some(Instant::now()));
}
/// Reports a message.
pub fn bm_report(msg: &str) {
    BM_START.with(|s| {
        let now = Instant::now();
        if let Some(start) = *s.borrow() {
            let dur = now.duration_since(start);
            println!("{}: {}.{:09}s", msg, dur.as_secs(), dur.subsec_nanos());
        }
        *s.borrow_mut() = Some(Instant::now());
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
    let mut rng = rng();
    keys.shuffle(&mut rng);
    let m = keys.len() as i32;
    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m);
}
