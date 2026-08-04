use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;

static BM_STATE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

fn bm_state() -> &'static Mutex<Option<Instant>> {
    BM_STATE.get_or_init(|| Mutex::new(None))
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    *bm_state().lock().unwrap() = Some(Instant::now());
}

/// Reports a message.
pub fn bm_report(msg: &str) {
    let now = Instant::now();
    let mut state = bm_state().lock().unwrap();
    if let Some(start) = *state {
        let elapsed = now.duration_since(start);
        println!(
            "{}: {}.{:09}s",
            msg,
            elapsed.as_secs(),
            elapsed.subsec_nanos()
        );
    }
    *state = Some(now);
}

/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut keys: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(s) => keys.push(s),
            Err(_) => break,
        }
    }
    // Randomize order, mirroring the C version (Fisher-Yates-like).
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();
    keys.shuffle(&mut rng);

    let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    let m = key_refs.len() as i32;
    cb(&key_refs, m);
}
