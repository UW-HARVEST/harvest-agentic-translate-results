//! Phase B rows 63-65: the public driver `str_put`, compared through its
//! stdout, byte for byte.
mod common;
use common::*;

fn run_both(num: i32) -> (Vec<u8>, Vec<u8>) {
    let l = libs();
    seed_both(0x31415926);
    let c = unsafe { capture_stdout("c", || (l.c.str_put)(num)) };
    seed_both(0x31415926);
    let r = unsafe { capture_stdout("r", || (l.r.str_put)(num)) };
    (c, r)
}

#[track_caller]
fn assert_same_stdout(num: i32) {
    let (c, r) = run_both(num);
    assert_eq!(
        c,
        r,
        "str_put({num}) stdout differs\n  C   ={:?}\n  RUST={:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // and it must actually be the expected line
    assert_eq!(
        c,
        format!("a {num}\n").into_bytes(),
        "str_put({num}) unexpected output: {:?}",
        String::from_utf8_lossy(&c)
    );
}

/// Rows 63-65 live in a SINGLE test function on purpose: `capture_stdout`
/// redirects the process-wide fd 1, so a second test running concurrently would
/// have libtest's own progress output land inside the capture.
#[test]
fn rows63_65_str_put_stdout() {
    let _s = session(0x31415926);
    // row 63
    assert_same_stdout(0);

    // row 64
    for &n in &[
        1i32, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 129, 255,
        256, 511, 512, 1000, 4095, 4096, 10000,
    ] {
        assert_same_stdout(n);
    }

    // row 64 (negative / extreme)
    for &n in &[-1i32, -2, -1000, i32::MIN] {
        assert_same_stdout(n);
    }

    // row 64 (randomised)
    let mut rng = Rng::with(64);
    for _ in 0..120 {
        let n = (rng.below(3000)) as i32;
        assert_same_stdout(n);
    }
    // negative randoms
    for _ in 0..40 {
        let n = -((rng.below(1 << 30)) as i32);
        assert_same_stdout(n);
    }

    // row 65: repeated calls in one process -- the global `stbds_hash_seed`
    // evolves, so the whole sequence must match, not just individual calls.
    let l = libs();
    let mut rng = Rng::with(65);
    let nums: Vec<i32> = (0..200).map(|_| rng.below(600) as i32).collect();

    for &start_seed in &[0x31415926usize, 0, 1, usize::MAX, 0xfeed_face_dead_beef] {
        seed_both(start_seed);
        let c = unsafe {
            capture_stdout("cseq", || {
                for &n in &nums {
                    (l.c.str_put)(n);
                }
            })
        };
        seed_both(start_seed);
        let r = unsafe {
            capture_stdout("rseq", || {
                for &n in &nums {
                    (l.r.str_put)(n);
                }
            })
        };
        assert_eq!(c, r, "repeated str_put sequence differs (seed={start_seed:#x})");
        let expect: String = nums.iter().map(|n| format!("a {n}\n")).collect();
        assert_eq!(c, expect.into_bytes());
    }

    // `strkey` shares the file-static 256-byte buffer with `str_put`; make sure
    // the interleaving of the two is identical too.
    let mut rng = Rng::with(651);
    seed_both(0x31415926);
    let c = unsafe {
        capture_stdout("cmix", || {
            let mut g = Rng::with(651);
            for _ in 0..100 {
                let n = g.below(50) as i32;
                let p = (l.c.strkey)(n);
                let s = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
                print_bytes(&s);
                (l.c.str_put)(n);
            }
        })
    };
    seed_both(0x31415926);
    let r = unsafe {
        capture_stdout("rmix", || {
            let mut g = Rng::with(651);
            for _ in 0..100 {
                let n = g.below(50) as i32;
                let p = (l.r.strkey)(n);
                let s = std::ffi::CStr::from_ptr(p).to_bytes().to_vec();
                print_bytes(&s);
                (l.r.str_put)(n);
            }
        })
    };
    let _ = rng.next_u64();
    assert_eq!(c, r, "interleaved strkey/str_put output differs");
}

fn print_bytes(b: &[u8]) {
    use std::io::Write;
    let mut out = std::io::stdout();
    let _ = out.write_all(b);
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}
