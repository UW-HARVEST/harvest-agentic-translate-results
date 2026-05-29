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
        let mut start = cell.borrow_mut();
        let prev = start.unwrap_or(now);
        *start = Some(now);
        now.duration_since(prev)
    });
    println!("{}: {}.{:09}s", msg, elapsed.as_secs(), elapsed.subsec_nanos());
}

/// Reads keys from stdin and calls the provided callback with a slice of
/// keys (in randomized order) and the number of keys read.
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
    // Randomize order using rand crate.
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    keys.shuffle(&mut rng);

    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    let m = refs.len() as i32;
    cb(&refs, m);
}
