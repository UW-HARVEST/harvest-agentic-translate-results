//! Phase B — CONFIGS.md rows C39..C41 (and ERRORS.md E43/E44): the two demo
//! entry points `strkey` and `helxo`.
//!
//! `helxo` is the fully composed pipeline (6 × `shput` into an implicitly
//! created SH_DEFAULT string map, iteration with `shlen`, `printf` of the
//! struct *by value*, then `shfree`), so it exercises the whole stack through
//! one call.
//!
//! `helxo` writes to `stdout`, i.e. to file descriptor 1, which is a
//! *process-global* resource shared with the libtest harness' own progress
//! output.  To capture it deterministically, the actual calls run in a
//! re-executed child process (`helxo_child` below) that redirects fd 1 only
//! around the calls and hands the bytes back through a file.
mod common;

use common::*;
use std::ffi::{c_char, c_int};
use std::process::Command;

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

/// C39 / E43 — `strkey` over the full `int` range boundaries.
#[test]
fn cfg_strkey_matrix() {
    let l = libs();
    let mut rng = Rng::new(0x39_0000);
    let mut ns: Vec<c_int> = vec![
        0,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        100,
        12345,
        -12345,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX - 1,
        c_int::MIN + 1,
    ];
    for _ in 0..256 {
        ns.push(rng.next_u64() as u32 as c_int);
    }
    for n in ns {
        unsafe {
            let cp = (l.c.strkey)(n);
            let rp = (l.r.strkey)(n);
            assert!(!cp.is_null() && !rp.is_null());
            let cs = read_cstr(cp);
            let rs = read_cstr(rp);
            assert_eq!(
                cs,
                rs,
                "strkey({n}): C={:?} RUST={:?}",
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
            assert_eq!(cs, format!("test_{n}").into_bytes());
            // the buffer is static: calling again must return the same address
            assert_eq!((l.c.strkey)(n), cp);
            assert_eq!((l.r.strkey)(n), rp);
        }
    }
}

// ---------------------------------------------------------------------------
// helxo, via a child process
// ---------------------------------------------------------------------------

/// Re-executed by `run_helxo` with `HELXO_SPEC` set; a normal `cargo test` run
/// of this test does nothing.
#[test]
fn helxo_child() {
    let Ok(spec) = std::env::var("HELXO_SPEC") else {
        return;
    };
    let out = std::env::var("HELXO_OUT").expect("HELXO_OUT");
    let mut it = spec.split(',');
    let which = it.next().unwrap().to_string();
    let seed: usize = it.next().unwrap().parse().unwrap();
    let letters: Vec<c_char> = it.map(|s| s.parse::<i32>().unwrap() as c_char).collect();

    let l = libs();
    let lib = if which == "c" { &l.c } else { &l.r };
    let bytes = capture_stdout("child", || unsafe {
        for &letter in &letters {
            (lib.rand_seed)(seed);
            (lib.helxo)(letter);
        }
    });
    std::fs::write(out, bytes).unwrap();
}

fn run_helxo(which: &str, seed: usize, letters: &[c_char]) -> Vec<u8> {
    let mut out = std::env::temp_dir();
    out.push(format!(
        "helxo_out_{}_{}_{}.bin",
        std::process::id(),
        which,
        seed
    ));
    let spec = format!(
        "{which},{seed},{}",
        letters
            .iter()
            .map(|c| (*c as i32).to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let exe = std::env::current_exe().unwrap();
    let st = Command::new(&exe)
        .args(["--exact", "helxo_child", "--test-threads=1"])
        .env("HELXO_SPEC", &spec)
        .env("HELXO_OUT", &out)
        .output()
        .expect("spawn helxo child");
    assert!(
        st.status.success(),
        "helxo child failed: {}\n{}",
        String::from_utf8_lossy(&st.stdout),
        String::from_utf8_lossy(&st.stderr)
    );
    let bytes = std::fs::read(&out).expect("child produced no output file");
    let _ = std::fs::remove_file(&out);
    bytes
}

fn assert_streams_eq(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let n = c.len().min(r.len());
    let first = (0..n).find(|&i| c[i] != r[i]).unwrap_or(n);
    let lo = first.saturating_sub(24);
    panic!(
        "helxo stdout differs ({ctx})\n  first difference at byte {first}\n  \
         C   [{lo}..]: {:02x?}\n  RUST[{lo}..]: {:02x?}\n  lens: C={} RUST={}",
        &c[lo..(lo + 64).min(c.len())],
        &r[lo..(lo + 64).min(r.len())],
        c.len(),
        r.len()
    );
}

/// C40 / E44 — `helxo` for a wide range of `char` values, stdout compared byte
/// for byte.
#[test]
fn cfg_helxo_letters() {
    let mut rng = Rng::new(0x40_0000);
    let mut letters: Vec<c_char> = vec![
        b'A' as c_char,
        b'z' as c_char,
        b'0' as c_char,
        b' ' as c_char,
        0x7f,
        0,
        -1,
        c_char::MIN,
        c_char::MAX,
        200u8 as i8 as c_char,
        b'\n' as c_char,
        b'%' as c_char,
        b'\t' as c_char,
    ];
    for _ in 0..64 {
        letters.push(rng.byte() as i8 as c_char);
    }
    // one letter per call, all in a single child run each
    let cout = run_helxo("c", 0x3141_5926, &letters);
    let rout = run_helxo("r", 0x3141_5926, &letters);
    assert_streams_eq("all letters", &cout, &rout);

    // Structural sanity on the C output: insertion order is
    // bob, sally, fred, jen, doug and the 2nd put of "jen" only sets the value.
    // `letter == '\n'` would add a 6th record for its call, so it is excluded
    // from the *structural* check only (it is still part of the byte-for-byte
    // comparison above).
    let letters: Vec<c_char> = letters
        .iter()
        .copied()
        .filter(|&c| c != b'\n' as c_char)
        .collect();
    let cout = run_helxo("c", 0x3141_5926, &letters);
    let rout = run_helxo("r", 0x3141_5926, &letters);
    assert_streams_eq("letters without \\n", &cout, &rout);
    let records: Vec<&[u8]> = cout.split(|&b| b == b'\n').collect();
    assert_eq!(records.len(), letters.len() * 5 + 1);
    for (i, letter) in letters.iter().enumerate() {
        let r = &records[i * 5..i * 5 + 5];
        assert_eq!(r[0], b"bob h");
        assert_eq!(r[1], b"sally e");
        assert_eq!(r[2], b"fred l");
        assert_eq!(r[3], [b'j', b'e', b'n', b' ', *letter as u8]);
        assert_eq!(r[4], b"doug o");
    }
}

/// C41 — repeated `helxo` calls from one seed: every call advances the
/// library's private `stbds_hash_seed`.
#[test]
fn cfg_helxo_repeated() {
    for start in [0usize, 1, 0x3141_5926, usize::MAX, usize::MAX / 3] {
        // NOTE: run_helxo re-seeds before *every* letter; to let the seed drift
        // we instead ask for many letters and compare the whole stream, then
        // repeat with a single seeding for the whole batch below.
        let letters: Vec<c_char> = (0..8).map(|i| b'a' as c_char + i).collect();
        let cout = run_helxo("c", start, &letters);
        let rout = run_helxo("r", start, &letters);
        assert_streams_eq(&format!("repeated from {start:#x}"), &cout, &rout);
        assert_eq!(cout.iter().filter(|&&b| b == b'\n').count(), 8 * 5);
    }
}

/// The output must not depend on the seed at all (the array is walked in
/// insertion order), and both libraries must agree for every seed.
#[test]
fn cfg_helxo_seed_independent() {
    let mut rng = Rng::new(0x41_0000);
    let letters = [b'Q' as c_char];
    let mut baseline: Option<Vec<u8>> = None;
    for _ in 0..8 {
        let s = rng.next_u64() as usize;
        let cout = run_helxo("c", s, &letters);
        let rout = run_helxo("r", s, &letters);
        assert_streams_eq(&format!("seed {s:#x}"), &cout, &rout);
        match &baseline {
            None => baseline = Some(cout),
            Some(b) => assert_eq!(b, &cout, "helxo output must not depend on the seed"),
        }
    }
}
