use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    static BM_START: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|s| s.set(Some(Instant::now())));
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let elapsed = BM_START.with(|s| {
        let now = Instant::now();
        let prev = s.replace(Some(now));
        match prev {
            Some(t) => now.saturating_duration_since(t),
            None => Duration::ZERO,
        }
    });
    println!(
        "{}: {}.{:09}s",
        msg,
        elapsed.as_secs(),
        elapsed.subsec_nanos()
    );
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut lines: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => lines.push(l),
            Err(_) => break,
        }
    }

    // Randomize order to match the C behavior of `bm_read_keys`.
    if lines.len() > 2 {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        lines.shuffle(&mut rng);
    }

    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let m = refs.len() as i32;
    cb(&refs, m);
}
