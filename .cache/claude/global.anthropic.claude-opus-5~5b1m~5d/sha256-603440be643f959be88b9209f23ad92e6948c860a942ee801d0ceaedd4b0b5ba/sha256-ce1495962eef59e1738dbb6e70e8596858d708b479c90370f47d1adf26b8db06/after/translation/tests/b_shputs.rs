//! Phase B — CONFIGS.md rows C69..C74: `sh_puts`, the only function declared in
//! the public header `c_src/include/lib.h`.
//!
//! It is the fully composed pipeline:
//!
//! ```c
//! for (i=0; i < num; ++i) stralloc(&sa, strkey(i));   // arena block ladder
//! strreset(&sa);
//! sh_new_arena(strmap);                               // shmode_func(_, SH_ARENA)
//! shputs(strmap, s);                                  // hmput_key + temp_key
//! assert(*strmap[0].key=='a' && strmap[0].key!=s.key && strmap[0].value==s.value);
//! for (int z=0; z < shlen(strmap); ++z)
//!     printf("%s %d\n", strmap[z], strmap[z].value);  // <- ABI quirk, C74
//! shfree(strmap);
//! ```
//!
//! `strmap[z]` is a 16-byte `struct { char *key; int value; }` passed to a
//! variadic function.  Under the SysV AMD64 ABI both of its eightbytes classify
//! as INTEGER, so it consumes the next two integer argument registers: `%s`
//! consumes `.key` and `%d` consumes `.value`.  The explicit third argument
//! (`strmap[z].value`) is never consumed by the format string.  The only way to
//! verify this faithfully is to compare the bytes that actually reach stdout.

mod common;
use common::*;

fn run(num: i32) -> (Vec<u8>, Vec<u8>) {
    let (c, r) = both();
    sync_seed(DEFAULT_SEED);
    let cout = capture_stdout(|| unsafe { (c.sh_puts)(num) });
    sync_seed(DEFAULT_SEED);
    let rout = capture_stdout(|| unsafe { (r.sh_puts)(num) });
    (cout, rout)
}

#[track_caller]
fn check(num: i32) {
    let (cout, rout) = run(num);
    assert_eq!(
        cout,
        rout,
        "sh_puts({num}) stdout differs:\n  C   : {}\n  Rust: {}",
        show(&cout),
        show(&rout)
    );
    // and it really is the one line the C source implies
    assert_eq!(
        cout,
        format!("a {num}\n").into_bytes(),
        "sh_puts({num}) unexpected output {}",
        show(&cout)
    );
}

// ---------------------------------------------------------------------------
// C69 — num = 0,1,2,3
// C74 — the printf ABI quirk (verified by every single call)
// ---------------------------------------------------------------------------
#[test]
fn c69_c74_small_num() {
    let _g = lock();
    for num in 0..=8 {
        check(num);
    }
}

// ---------------------------------------------------------------------------
// C70 — the first 512-byte arena block boundary.
//       `strkey(i)` yields "test_<i>", i.e. 6..10 bytes incl. NUL, so the first
//       512-byte block runs out somewhere around i == 70.
// ---------------------------------------------------------------------------
#[test]
fn c70_arena_block_boundary() {
    let _g = lock();
    for num in 25..=40 {
        check(num);
    }
    for num in 60..=85 {
        check(num);
    }
}

// ---------------------------------------------------------------------------
// C71 — several steps up the arena block ladder (512,512,1024,1024,2048,...)
// ---------------------------------------------------------------------------
#[test]
fn c71_many_arena_blocks() {
    let _g = lock();
    for num in [100, 128, 200, 256, 500, 512, 1000, 1024, 2000, 5000, 12345] {
        check(num);
    }
}

// ---------------------------------------------------------------------------
// C72 — num < 0: the stralloc loop never runs
// R33/R34
// ---------------------------------------------------------------------------
#[test]
fn c72_negative_num() {
    let _g = lock();
    for num in [-1, -2, -9, -10, -100, -999_999, i32::MIN + 1, i32::MIN] {
        check(num);
    }
}

// ---------------------------------------------------------------------------
// C73 — repeated and interleaved calls: no dependence on leftover global state
//       (the `stbds_hash_seed` static, the `strkey` static buffer, the arena)
// ---------------------------------------------------------------------------
#[test]
fn c73_repeated_and_interleaved() {
    let _g = lock();
    let (c, r) = both();

    // (a) the same value many times must give the same bytes every time
    for _ in 0..5 {
        check(7);
    }

    // (b) drive the two libraries alternately WITHOUT resyncing the seed: the
    //     printed output must not depend on the hash seed at all.
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..60 {
        let num = (rng.below(3000) as i32) - 1000;
        let first_c = rng.next_u64() & 1 == 0;
        let (a, b) = unsafe {
            if first_c {
                (
                    capture_stdout(|| (c.sh_puts)(num)),
                    capture_stdout(|| (r.sh_puts)(num)),
                )
            } else {
                (
                    capture_stdout(|| (r.sh_puts)(num)),
                    capture_stdout(|| (c.sh_puts)(num)),
                )
            }
        };
        assert_eq!(a, b, "interleaved sh_puts({num}): {} vs {}", show(&a), show(&b));
        assert_eq!(a, format!("a {num}\n").into_bytes());
    }

    // (c) a long back-to-back run in ONE capture, so libc's stdio buffering and
    //     the ordering of the two libraries' writes are compared too.
    let cout = capture_stdout(|| unsafe {
        for n in 0..25 {
            (c.sh_puts)(n);
        }
    });
    let rout = capture_stdout(|| unsafe {
        for n in 0..25 {
            (r.sh_puts)(n);
        }
    });
    assert_eq!(cout, rout, "batched sh_puts:\n{}\n{}", show(&cout), show(&rout));
    let want: String = (0..25).map(|n| format!("a {n}\n")).collect();
    assert_eq!(cout, want.into_bytes());
}

/// Randomized sweep over `num`.
#[test]
fn c69_c72_randomized_num() {
    let _g = lock();
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..400 {
        let num = match rng.below(5) {
            0 => rng.next_u64() as i32 % 50,
            1 => -(rng.below(1_000_000) as i32),
            2 => rng.below(4096) as i32,
            3 => (rng.next_u64() as i32).saturating_abs() % 3,
            _ => (rng.next_u64() % 2048) as i32,
        };
        check(num);
    }
    // the two documented extremes of `int`
    check(i32::MIN);
    // (i32::MAX would loop 2^31 times in the C original; not exercised.)
}

/// `strkey` is the helper `sh_puts` feeds the arena with; check the exact
/// strings that the loop produces, for both libraries, at the boundaries where
/// the decimal width changes (which is what shifts the arena block boundary).
#[test]
fn c70_strkey_widths_inside_shputs_range() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for n in [0, 1, 9, 10, 99, 100, 999, 1000, 9999, 10000, 12344, 12345] {
            let cs = cstr_opt((c.strkey)(n));
            let rs = cstr_opt((r.strkey)(n));
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("{:?}", format!("test_{n}")));
        }
    }
}
