use std::cell::RefCell;
use std::io::{self, BufRead};
use std::time::Instant;

thread_local! {
    static BM_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|s| {
        *s.borrow_mut() = Some(Instant::now());
    });
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let elapsed = BM_START.with(|s| {
        let start = s.borrow().unwrap_or_else(Instant::now);
        start.elapsed()
    });
    println!(
        "{}: {}.{:09}s",
        msg,
        elapsed.as_secs(),
        elapsed.subsec_nanos()
    );
    BM_START.with(|s| {
        *s.borrow_mut() = Some(Instant::now());
    });
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    use rand::Rng;
    let stdin = io::stdin();
    let mut keys: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(s) => keys.push(s),
            Err(_) => {
                eprintln!("getline error");
                std::process::exit(1);
            }
        }
    }
    // Randomize order of array (Fisher-Yates style — same as the C version).
    let mut rng = rand::thread_rng();
    if keys.len() > 2 {
        for i in (2..keys.len()).rev() {
            let j = rng.gen_range(0..i);
            keys.swap(i, j);
        }
    }
    let m = keys.len() as i32;
    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    cb(&refs, m);
}
