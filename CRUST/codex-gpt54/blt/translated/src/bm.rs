use rand::seq::SliceRandom;
use std::cell::RefCell;
use std::io::{self, BufRead};
use std::process::exit;
use std::time::Instant;

thread_local! {
    static BM_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Initializes the bm subsystem.
pub fn bm_init() {
    BM_START.with(|start| {
        *start.borrow_mut() = Some(Instant::now());
    });
}
/// Reports a message.
pub fn bm_report(msg: &str) {
    BM_START.with(|start| {
        let mut start_ref = start.borrow_mut();
        let now = Instant::now();
        let elapsed = start_ref
            .as_ref()
            .map(|instant| instant.elapsed())
            .unwrap_or_default();
        println!(
            "{msg}: {}.{:09}s",
            elapsed.as_secs(),
            elapsed.subsec_nanos()
        );
        *start_ref = Some(now);
    });
}
/// Reads keys and calls the provided callback with a slice of keys and an integer.
/// The callback receives a slice of string slices (`&[&str]`) and an `i32`.
pub fn bm_read_keys<F>(mut cb: F)
where
    F: FnMut(&[&str], i32),
{
    let stdin = io::stdin();
    let mut owned_keys = Vec::new();
    for line in stdin.lock().lines() {
        match line {
            Ok(s) => owned_keys.push(s),
            Err(err) => {
                eprintln!("getline failed: {err}");
                exit(1);
            }
        }
    }

    let mut rng = rand::rng();
    owned_keys.shuffle(&mut rng);

    let key_refs = owned_keys.iter().map(String::as_str).collect::<Vec<_>>();
    cb(&key_refs, key_refs.len() as i32);
}
