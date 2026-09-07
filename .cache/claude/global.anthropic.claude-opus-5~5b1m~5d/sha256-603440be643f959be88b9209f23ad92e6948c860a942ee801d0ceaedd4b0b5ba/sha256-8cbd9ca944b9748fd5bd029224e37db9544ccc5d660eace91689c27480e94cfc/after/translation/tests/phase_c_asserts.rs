//! Phase C, live-`STBDS_ASSERT` rows (A1–A8 of `ERRORS.md`).
//!
//! The C `.so` is built without `-DNDEBUG` (it imports `__assert_fail`), so a
//! failing `STBDS_ASSERT` prints glibc's assertion message to stderr and raises
//! `SIGABRT`.  At least one of those assertions is REACHABLE from the public API,
//! so the Rust must abort in exactly the same way.
//!
//! Because the abort kills the process, each scenario is run in a CHILD process
//! (this same test binary re-executed with `DIFF_ABORT_SCENARIO` set); the parent
//! then compares the child's termination signal, exit code and stderr.

mod common;
use common::*;
use std::ffi::c_void;
use std::process::{Command, Stdio};

/// Scenario A5: `stbds_hmdel_key` with `mode == 2` on a populated map, deleting a
/// NON-tail element.  `mode >= STBDS_HM_STRING` so the slot is found by string
/// hash, but `mode == STBDS_HM_STRING` is FALSE, so the post-`memmove` re-lookup
/// (lib.c:842-845) passes `(char*)a + elemsize*old_index + keyoffset` as the key
/// while still hashing it AS A STRING -> the pointer bytes are hashed, the lookup
/// misses, and `STBDS_ASSERT(slot >= 0)` at lib.c:846 fires.
fn scenario_a5(which: &str) {
    let s = session(0x31415926);
    let elemsize = 16usize;
    let api = if which == "C" { s.c } else { s.r };
    unsafe {
        let mut map = (api.shmode_func)(elemsize, SH_DEFAULT);
        // 8 distinct keys, kept alive for the whole scenario (SH_DEFAULT stores
        // the caller's pointer verbatim).
        let keys: Vec<CStrBuf> = (0..8)
            .map(|i| CStrBuf::new(format!("assert-key-{i}").as_bytes()))
            .collect();
        for k in &keys {
            map = (api.hmput_key)(map, elemsize, k.ptr() as *mut c_void, 8, 1);
        }
        eprintln!("[{which}] populated, deleting a non-tail element with mode=2");
        // Delete the FIRST element: old_index (0) != final_index (7) -> memmove
        // -> re-lookup -> assert.
        let _ = (api.hmdel_key)(map, elemsize, keys[0].ptr() as *mut c_void, 8, 0, 2);
        eprintln!("[{which}] NO ABORT");
    }
}

fn run_child(scenario: &str, which: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .env("DIFF_ABORT_SCENARIO", scenario)
        .env("DIFF_ABORT_WHICH", which)
        .arg("--exact")
        .arg("child_entry_point")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn child");
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    (
        signal,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// The child entry point.  Ignored unless `DIFF_ABORT_SCENARIO` is set, so a
/// normal `cargo test` run never executes it directly.
#[test]
fn child_entry_point() {
    let Ok(scenario) = std::env::var("DIFF_ABORT_SCENARIO") else {
        return;
    };
    let which = std::env::var("DIFF_ABORT_WHICH").unwrap();
    match scenario.as_str() {
        "A5" => scenario_a5(&which),
        other => panic!("unknown scenario {other}"),
    }
}

/// Extract the `<file>:<line>: <func>: Assertion `<text>' failed.` tail of a
/// glibc assertion message, dropping the leading program-name and directory.
fn assertion_tail(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|l| {
        l.find("Assertion `").map(|i| {
            // keep "<basename>:<line>: <func>: Assertion `...' failed."
            let head = &l[..i];
            let base = head.rfind('/').map(|p| &head[p + 1..]).unwrap_or(head);
            format!("{}{}", base.trim_start_matches(|c: char| c != 's'), &l[i..])
        })
    })
}

fn assertion_core(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|l| {
        let i = l.find("lib.c:")?;
        let j = l.find("Assertion `")?;
        Some(format!("{}{}", &l[i..], &l[j..]).replace(&l[j..], &l[j..]))
            .map(|_| format!("{}", &l[i..]))
    })
}

// =========================================================== A5 / A6
#[test]
fn x1_assert_slot_ge_zero_aborts_identically() {
    if std::env::var("DIFF_ABORT_SCENARIO").is_ok() {
        return; // we ARE the child
    }
    let (csig, ccode, cerr) = run_child("A5", "C");
    let (rsig, rcode, rerr) = run_child("A5", "RUST");

    eprintln!("C   : signal={csig:?} code={ccode:?}\n{cerr}");
    eprintln!("RUST: signal={rsig:?} code={rcode:?}\n{rerr}");

    // Both must die by SIGABRT (6), not exit normally.
    assert_eq!(csig, Some(6), "C must abort via SIGABRT:\n{cerr}");
    assert_eq!(rsig, Some(6), "Rust must abort via SIGABRT:\n{rerr}");
    assert_eq!(csig, rsig, "termination signal differs");
    assert_eq!(ccode, rcode, "exit code differs");
    assert!(
        !cerr.contains("NO ABORT"),
        "C unexpectedly returned from hmdel_key"
    );
    assert!(
        !rerr.contains("NO ABORT"),
        "Rust unexpectedly returned from hmdel_key"
    );

    // Same assertion text, same line, same function (the leading absolute
    // directory of __FILE__ is a build-environment artefact and is stripped).
    let c_tail = assertion_core(&cerr).expect("no glibc assertion line from C");
    let r_tail = assertion_core(&rerr).expect("no glibc assertion line from Rust");
    assert_eq!(c_tail, r_tail, "assertion message differs\nC: {cerr}\nR: {rerr}");
    assert!(
        c_tail.contains("lib.c:846:") && c_tail.contains("stbds_hmdel_key"),
        "unexpected assertion site: {c_tail}"
    );
    assert!(
        c_tail.contains("Assertion `slot >= 0' failed."),
        "unexpected assertion text: {c_tail}"
    );
    let _ = assertion_tail(&cerr);
}

// =========================================================== A1/A2/A3/A4/A7/A8
/// The remaining assertions must NOT fire for any input the differential suite
/// drives.  Since the Rust asserts are now LIVE (they call the same
/// `__assert_fail`), the whole test suite passing is itself the proof: a spurious
/// Rust assertion would abort the run.  This test additionally hammers the two
/// assertions whose operands are easiest to push near their bounds.
#[test]
fn x2_remaining_assertions_do_not_fire() {
    let s = session(0x31415926);

    // A1: make_hash_index thresholds -- exercise every slot_count the API can
    // produce (8 .. 4096) by growing and shrinking a map.
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut d = Driver::lazy(&s, elemsize, keysize, KeyKind::Binary);
    let mut rng = Rng::new(0xA1);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..3200usize {
        let k = rng.bytes(keysize);
        if keys.contains(&k) {
            continue;
        }
        d.hmput(&format!("A1 put#{i}"), &k, &rng.bytes(elemsize), HM_BINARY);
        keys.push(k);
    }
    let peak = d.snap_c().table.unwrap().slot_count;
    assert!(peak >= 4096, "A1: expected slot_count >= 4096, got {peak}");
    d.check("A1 grown");
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.remove(i);
        d.hmdel("A1 drain", &k, 0, HM_BINARY);
    }
    d.check("A1 drained");
    assert_eq!(d.snap_c().table.unwrap().slot_count, 8, "A1: back to 8 slots");
    d.free();

    // A7: stralloc `len <= a->remaining` across every boundary combination.
    unsafe {
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        let mut rng = Rng::new(0xA7);
        for i in 0..3000usize {
            // lengths that straddle `remaining` and `blocksize` exactly
            let rem = ca.remaining;
            let len = match rng.below(6) {
                0 => rem.saturating_sub(1),
                1 => rem,
                2 => rem + 1,
                3 => 512,
                4 => 513,
                _ => rng.below(2000),
            };
            let body = vec![b'k'; len.min(4000)];
            let cs = CStrBuf::new(&body);
            let cp = (s.c.stralloc)(&mut ca, cs.ptr());
            let rp = (s.r.stralloc)(&mut ra, cs.ptr());
            assert_eq!(read_cstr(cp), body, "A7 #{i}: C content");
            assert_eq!(read_cstr(rp), body, "A7 #{i}: Rust content");
            assert_eq!(ca.remaining, ra.remaining, "A7 #{i}: remaining");
            assert_eq!(ca.block, ra.block, "A7 #{i}: block");
        }
        (s.c.strreset)(&mut ca);
        (s.r.strreset)(&mut ra);
    }

    // A8: sh_puts invariants over many `num`.
    for num in [0i32, 1, 2, 5, 17, 100, 999, 1234, -1, -50] {
        s.seed(0x31415926);
        let cout = capture_stdout(|| unsafe { (s.c.sh_puts)(num) });
        s.seed(0x31415926);
        let rout = capture_stdout(|| unsafe { (s.r.sh_puts)(num) });
        assert_eq!(cout, rout, "A8 sh_puts({num})");
        assert_eq!(String::from_utf8_lossy(&cout), format!("a {num}\n"));
    }
}
