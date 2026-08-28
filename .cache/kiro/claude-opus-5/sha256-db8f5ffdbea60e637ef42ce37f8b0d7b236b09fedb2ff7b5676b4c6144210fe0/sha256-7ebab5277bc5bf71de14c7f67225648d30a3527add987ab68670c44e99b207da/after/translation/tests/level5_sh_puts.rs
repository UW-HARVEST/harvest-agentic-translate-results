//! Level 5: `sh_puts` - the only function declared in `include/lib.h`.
//!
//! It exercises `stbds_stralloc`/`stbds_strreset`, `sh_new_arena`, `shputs`,
//! `shlen`, `shfree`, the three internal `STBDS_ASSERT`s, and finally prints
//! through libc `printf`. The comparison is on the raw bytes written to fd 1.
//!
//! Note the printf call in the C source:
//!
//! ```c
//! printf("%s %d\n", strmap[z], strmap[z].value);
//! ```
//!
//! `strmap[z]` is a 16-byte two-eightbyte struct, so under the SysV AMD64 ABI
//! it consumes two vararg slots: `%s` reads the key pointer and `%d` reads the
//! eightbyte holding `value`. The explicit third argument is never consumed.
//! The Rust translation has to reproduce that register layout, which is exactly
//! what this test pins down.

mod harness;

use harness::*;

fn run_pair(num: i32) -> (Vec<u8>, Vec<u8>) {
    let p = pair();
    unsafe {
        // keep the seed sequence in lockstep
        p.c.rand_seed(0x31415926);
        p.rs.rand_seed(0x31415926);
        let c_out = capture_stdout("c", || p.c.sh_puts(num));
        let rs_out = capture_stdout("rs", || p.rs.sh_puts(num));
        (c_out, rs_out)
    }
}

fn check(num: i32) {
    let (c_out, rs_out) = run_pair(num);
    assert_eq!(
        c_out,
        rs_out,
        "sh_puts({}) stdout differs\n C: {:?}\nRS: {:?}",
        num,
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rs_out)
    );
}

#[test]
fn sh_puts_small_values() {
    let _g = shared_lock();
    for num in -5..40 {
        check(num);
    }
}

#[test]
fn sh_puts_around_arena_block_growth() {
    let _g = shared_lock();
    // "test_<n>" is 7-12 bytes, so 512-byte blocks turn over every ~60 keys;
    // these counts land on and around several block boundaries.
    for num in [
        50, 60, 63, 64, 65, 70, 100, 127, 128, 129, 200, 255, 256, 257, 511, 512, 513, 1000,
    ] {
        check(num);
    }
}

#[test]
fn sh_puts_large_values() {
    let _g = shared_lock();
    for num in [2000, 5000, 12345, 65536] {
        check(num);
    }
}

#[test]
fn sh_puts_negative_and_extremes() {
    let _g = shared_lock();
    // num <= 0 skips the stralloc loop entirely; num is also the stored value,
    // so it must round-trip through the %d conversion. i32::MAX is left out on
    // purpose: the loop would run 2^31 times.
    for num in [0, -1, -42, -12345, i32::MIN, i32::MIN + 1] {
        check(num);
    }
}

/// The printed text must actually be `"a <num>\n"`; if both libraries were
/// silently printing nothing the equality test above would still pass.
#[test]
fn sh_puts_output_is_the_expected_line() {
    let _g = shared_lock();
    for num in [0, 1, 7, 1234, -9] {
        let (c_out, rs_out) = run_pair(num);
        let expected = format!("a {}\n", num);
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            expected,
            "C sh_puts({}) output",
            num
        );
        assert_eq!(
            String::from_utf8_lossy(&rs_out),
            expected,
            "Rust sh_puts({}) output",
            num
        );
    }
}

/// Repeated calls must stay identical: `sh_puts` leaks nothing into
/// library-global state that would change later output (it frees the map and
/// resets the arena), but the global hash seed does advance.
#[test]
fn sh_puts_repeated_calls_match() {
    let _g = shared_lock();
    let p = pair();
    unsafe {
        p.c.rand_seed(0xDEAD_BEEF);
        p.rs.rand_seed(0xDEAD_BEEF);
        for round in 0..25 {
            let num = round * 7;
            let c_out = capture_stdout("c", || p.c.sh_puts(num));
            let rs_out = capture_stdout("rs", || p.rs.sh_puts(num));
            assert_eq!(
                c_out, rs_out,
                "round {} (num {}) stdout differs: {:?} vs {:?}",
                round,
                num,
                String::from_utf8_lossy(&c_out),
                String::from_utf8_lossy(&rs_out)
            );
        }
    }
}
