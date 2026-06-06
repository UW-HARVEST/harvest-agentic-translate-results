use std::cell::RefCell;
use std::io::{self, BufRead};
use std::time::Instant;

thread_local! {
    static BM_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|cell| {
        *cell.borrow_mut() = Some(Instant::now());
    });
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    let elapsed = BM_START.with(|cell| {
        let start = cell.borrow().unwrap_or(now);
        now.duration_since(start)
    });
    let secs = elapsed.as_secs();
    let nanos = elapsed.subsec_nanos();
    println!("{}: {}.{:09}s", msg, secs, nanos);
    BM_START.with(|cell| {
        *cell.borrow_mut() = Some(Instant::now());
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

    // Randomize the order. Mirrors the C loop which only shuffles indices
    // [2..m-1] effectively:
    //   for (int i = m-1; i>1; i--) { int j = random() % i; swap(key[i], key[j]); }
    let m = keys.len();
    if m > 2 {
        for i in (2..m).rev() {
            let j = (rand::random::<u32>() as usize) % i;
            keys.swap(i, j);
        }
    }

    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
