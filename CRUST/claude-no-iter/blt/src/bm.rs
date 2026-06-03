use std::io::{self, BufRead};
use std::sync::Mutex;
use std::time::Instant;

static BM_START: Mutex<Option<Instant>> = Mutex::new(None);

/// Initializes the bm subsystem.
pub fn bm_init() {
    let mut guard = BM_START.lock().expect("bm_init lock");
    *guard = Some(Instant::now());
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    let mut guard = BM_START.lock().expect("bm_report lock");
    let start = guard.unwrap_or(now);
    let elapsed = now.duration_since(start);
    println!(
        "{}: {}.{:09}s",
        msg,
        elapsed.as_secs(),
        elapsed.subsec_nanos()
    );
    *guard = Some(Instant::now());
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    use rand::Rng;

    let stdin = io::stdin();
    let mut lines: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(s) => lines.push(s),
            Err(_) => break,
        }
    }

    // Randomize order of array (mirroring the C code's shuffle).
    let mut rng = rand::rng();
    let m = lines.len();
    if m > 2 {
        for i in (2..m).rev() {
            let j = rng.random_range(0..i);
            lines.swap(i, j);
        }
    }

    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
