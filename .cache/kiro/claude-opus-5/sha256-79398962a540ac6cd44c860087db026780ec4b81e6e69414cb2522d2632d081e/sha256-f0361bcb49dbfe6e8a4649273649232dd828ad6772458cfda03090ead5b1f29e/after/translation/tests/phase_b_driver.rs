//! Phase B rows 53–57 — the top-level driver (`strkey`, `sh_puts`).
//!
//! `sh_puts` is the only symbol declared in `c_src/include/lib.h`. It writes to
//! stdout via libc `printf`, so its output is captured at the fd level and
//! compared byte for byte — this is the project's stand-in for a driver binary
//! (`c_src/CMakeLists.txt` has no `add_executable`).

mod common;
use common::*;
use std::ffi::c_char;

unsafe fn read_cstr(p: *const c_char) -> Vec<u8> {
    let mut out = Vec::new();
    let mut q = p as *const u8;
    while *q != 0 {
        out.push(*q);
        q = q.add(1);
    }
    out
}

/// CONFIGS row 53 — `strkey` over boundary + random `int`s.
#[test]
fn cfg_53_strkey() {
    run(1, |c, r| unsafe {
        let mut fixed: Vec<i32> = vec![
            0,
            1,
            2,
            9,
            10,
            11,
            99,
            100,
            999,
            1000,
            12345,
            1_000_000_000,
            i32::MAX,
            -1,
            -9,
            -10,
            -99,
            -12345,
            -1_000_000_000,
            i32::MIN,
            i32::MIN + 1,
        ];
        let mut rng = Rng::new(0x5eed_1_3333);
        for _ in 0..256 {
            fixed.push(rng.next_u32() as i32);
        }
        for n in fixed {
            let pc = (c.strkey)(n);
            let sc = read_cstr(pc);
            let pr = (r.strkey)(n);
            let sr = read_cstr(pr);
            assert_eq!(
                String::from_utf8_lossy(&sc),
                String::from_utf8_lossy(&sr),
                "strkey({})",
                n
            );
            assert_eq!(sc, format!("test_{}", n).into_bytes(), "strkey({}) value", n);
            // the C returns the address of its file-static 256-byte buffer:
            // repeated calls must return the same pointer in both libraries.
            let pc2 = (c.strkey)(n);
            let pr2 = (r.strkey)(n);
            assert_eq!(pc, pc2, "C strkey must return a stable static buffer");
            assert_eq!(pr, pr2, "Rust strkey must return a stable static buffer");
        }
    });
}

fn sh_puts_diff(num: i32) {
    run_locked(0x3141_5926, |c, r| unsafe {
        let oc = capture_stdout("c", || (c.sh_puts)(num));
        let or_ = capture_stdout("r", || (r.sh_puts)(num));
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or_),
            "sh_puts({}) stdout differs",
            num
        );
    });
}

/// CONFIGS row 54 — `sh_puts` stdout for a spread of positive `num`.
#[test]
fn cfg_54_sh_puts_stdout() {
    for num in [1i32, 2, 3, 7, 8, 9, 15, 16, 17, 63, 64, 65, 100, 255, 256, 1000] {
        sh_puts_diff(num);
    }
}

/// CONFIGS row 55 — `sh_puts` with non-positive `num` (loop body never runs).
#[test]
fn cfg_55_sh_puts_nonpositive() {
    for num in [0i32, -1, -2, -100, -1_000_000, i32::MIN, i32::MIN + 1] {
        sh_puts_diff(num);
    }
}

/// CONFIGS row 56 — repeated calls in one process: the file-static
/// `stbds_hash_seed` advances and `buffer` is reused, so a *sequence* of calls
/// must produce identical cumulative output.
#[test]
fn cfg_56_sh_puts_repeated() {
    run_locked(0x3141_5926, |c, r| unsafe {
        let nums = [3i32, 0, 1, 40, 2, 600, 5, -7, 9];
        let oc = capture_stdout("c", || {
            for n in nums {
                (c.sh_puts)(n);
            }
        });
        let or_ = capture_stdout("r", || {
            for n in nums {
                (r.sh_puts)(n);
            }
        });
        assert_eq!(
            String::from_utf8_lossy(&oc),
            String::from_utf8_lossy(&or_),
            "sh_puts sequence {:?} stdout differs",
            nums
        );
        assert_eq!(
            oc.iter().filter(|&&b| b == b'\n').count(),
            nums.len(),
            "expected one line per sh_puts call"
        );
    });
}

/// CONFIGS row 57 — `num` large enough to cross several arena block sizes.
#[test]
fn cfg_57_sh_puts_large() {
    for num in [2000i32, 5000, 20000] {
        sh_puts_diff(num);
    }
}

/// Extra: randomized `num` sweep.
#[test]
fn cfg_54b_sh_puts_random() {
    run_locked(0x3141_5926, |c, r| unsafe {
        let mut rng = Rng::new(0x5eed_1_4444);
        for _ in 0..60 {
            let num = (rng.next_u32() % 3000) as i32 - 500;
            let oc = capture_stdout("c", || (c.sh_puts)(num));
            let or_ = capture_stdout("r", || (r.sh_puts)(num));
            assert_eq!(
                String::from_utf8_lossy(&oc),
                String::from_utf8_lossy(&or_),
                "sh_puts({}) stdout differs",
                num
            );
        }
    });
}
