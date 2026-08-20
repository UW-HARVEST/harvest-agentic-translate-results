//! Phase B — CONFIGS.md rows 46 & 47: `sh_puts`, the only entry point declared
//! in `c_src/include/lib.h`.  Its whole observable behaviour is what it writes
//! to `stdout`, so both libraries' output is captured and compared
//! byte-for-byte.

mod common;
use common::*;

fn run(num: i32) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = pair();
    sync_seed(0x3141_5926);
    let out_c = capture_stdout(|| unsafe { (c.sh_puts)(num) });
    sync_seed(0x3141_5926);
    let out_r = capture_stdout(|| unsafe { (r.sh_puts)(num) });
    (out_c, out_r)
}

// ---------------------------------------------------------------- row 46
#[test]
fn sh_puts_matrix() {
    let _g = lock();
    for num in [
        i32::MIN,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        7,
        8,
        9,
        63,
        64,
        65,
        511,
        512,
        513,
        1000,
        5000,
    ] {
        let (a, b) = run(num);
        assert_eq!(
            a,
            b,
            "sh_puts({num}) stdout diverged\n C = {:?}\n R = {:?}",
            String::from_utf8_lossy(&a),
            String::from_utf8_lossy(&b)
        );
        assert_eq!(
            a,
            format!("a {num}\n").into_bytes(),
            "sh_puts({num}) unexpected output"
        );
    }
}

#[test]
fn sh_puts_random() {
    let _g = lock();
    let mut rng = Rng::new(0x4601);
    for _ in 0..60 {
        let num = (rng.next_u32() % 2000) as i32 - 1000;
        let (a, b) = run(num);
        assert_eq!(a, b, "sh_puts({num}) diverged");
    }
    // and a handful of large i32 values (num <= 0 short-circuits the loop)
    for num in [i32::MAX, i32::MIN, -i32::MAX] {
        if num > 100_000 {
            continue; // would allocate a huge arena; the loop body is linear
        }
        let (a, b) = run(num);
        assert_eq!(a, b, "sh_puts({num}) diverged");
    }
}

// ---------------------------------------------------------------- row 47
#[test]
fn sh_puts_repeated() {
    let _g = lock();
    // The global `stbds_hash_seed` advances on every call (a fresh
    // `make_hash_index`) and `strkey`'s static buffer is reused, so a *sequence*
    // of calls in one process is a distinct configuration from a single call.
    let (c, r) = pair();
    sync_seed(0x3141_5926);
    let out_c = capture_stdout(|| unsafe {
        for i in 0..50 {
            (c.sh_puts)(i * 3);
        }
    });
    sync_seed(0x3141_5926);
    let out_r = capture_stdout(|| unsafe {
        for i in 0..50 {
            (r.sh_puts)(i * 3);
        }
    });
    assert_eq!(
        out_c,
        out_r,
        "repeated sh_puts diverged\n C = {:?}\n R = {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r)
    );
    let expect: Vec<u8> = (0..50).flat_map(|i| format!("a {}\n", i * 3).into_bytes()).collect();
    assert_eq!(out_c, expect);

    // interleaving the two libraries must also be identical (they share the
    // process's stdio but keep separate global seeds and static buffers)
    sync_seed(1);
    let inter = capture_stdout(|| unsafe {
        for i in 0..20 {
            (c.sh_puts)(i);
            (r.sh_puts)(i);
        }
    });
    let expect: Vec<u8> = (0..20)
        .flat_map(|i| format!("a {i}\na {i}\n").into_bytes())
        .collect();
    assert_eq!(inter, expect, "interleaved sh_puts diverged");
}

/// `sh_puts` must leave nothing behind that breaks a following library call.
#[test]
fn sh_puts_then_map_ops() {
    let _g = lock();
    sync_seed(0x777);
    let (c, r) = pair();
    let out = capture_stdout(|| unsafe {
        (c.sh_puts)(17);
        (r.sh_puts)(17);
    });
    assert_eq!(out, b"a 17\na 17\n".to_vec());
    // both libraries advanced their global seed once; re-sync and keep going
    sync_seed(0x778);
    let mut m = Dual::new(16, false);
    for i in 0..50i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
        m.check(&format!("post-sh_puts put {i}"));
    }
    m.free();
}
