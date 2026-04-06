use std::time::Instant;
use std::io::{self, BufRead};
use rand::seq::SliceRandom;

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
        let start = s.borrow().unwrap();
        let elapsed = start.elapsed();
        let secs = elapsed.as_secs() as i64;
        let nanos = elapsed.subsec_nanos() as i64;
        println!("{}: {}.{:09}s", msg, secs, nanos);
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
        let line = line.expect("getline");
        keys.push(line);
    }
    let mut rng = rand::rng();
    keys.shuffle(&mut rng);
    let m = keys.len() as i32;
    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m);
}
