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
    let now = Instant::now();
    let elapsed = BM_START.with(|s| {
        let mut b = s.borrow_mut();
        let started = b.unwrap_or(now);
        let elapsed = now.duration_since(started);
        *b = Some(now);
        elapsed
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
    let stdin = io::stdin();
    let mut lines: Vec<String> = Vec::new();
    let locked = stdin.lock();
    for line in locked.lines() {
        match line {
            Ok(s) => lines.push(s),
            Err(_) => break,
        }
    }

    // Randomize order of array (mirror C: for i in (2..m).rev(), swap with random index in [0, i)).
    use rand::Rng;
    let m = lines.len();
    if m > 2 {
        let mut rng = rand::rng();
        for i in (2..m).rev() {
            let j = rng.random_range(0..i);
            lines.swap(i, j);
        }
    }

    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    cb(&refs, m as i32);
}
