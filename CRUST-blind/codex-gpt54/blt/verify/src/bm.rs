use rand::Rng;
use std::io::{self, BufRead};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

fn timer() -> &'static Mutex<Option<Instant>> {
    static TIMER: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    TIMER.get_or_init(|| Mutex::new(None))
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    if let Ok(mut guard) = timer().lock() {
        *guard = Some(Instant::now());
    }
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    if let Ok(mut guard) = timer().lock() {
        let now = Instant::now();
        let start = guard.replace(now).unwrap_or(now);
        let elapsed = now.saturating_duration_since(start);
        println!("{msg}: {}.{:09}s", elapsed.as_secs(), elapsed.subsec_nanos());
    }
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    let stdin = io::stdin();
    let mut keys: Vec<String> = stdin
        .lock()
        .lines()
        .map_while(Result::ok)
        .map(|line| line.trim_end_matches('\n').to_owned())
        .collect();

    let mut rng = rand::rng();
    for i in (2..keys.len()).rev() {
        let j = rng.random_range(0..i);
        keys.swap(i, j);
    }

    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    cb(&refs, refs.len() as i32);
}
