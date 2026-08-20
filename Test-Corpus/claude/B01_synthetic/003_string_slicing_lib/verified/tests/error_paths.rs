//! Phase C — error-path differential tests.
//!
//! One row per line of `ERRORS.md`.  Each row builds the exact invalid input,
//! calls BOTH shared objects through their exported `slice` symbol, and asserts
//!   * C and Rust return the same sentinel (`1`) and print the same message, and
//!   * that sentinel/message is the one `ERRORS.md` documents for the C code
//!     (`cmp_expect`), so a row cannot pass by "both failed somehow".
//!
//! Run with `cargo test --test error_paths` (custom harness: see Cargo.toml).

#[path = "harness/mod.rs"]
mod harness;

use harness::{
    cstr, null_string_status, rand_ascii, rand_bytes, wexitstatus, wif_signaled, wtermsig, Arg,
    Diff, Lib, Rng, Runner, SEED,
};

/// `Error: start is off the end of the string!\n`
const E1: &[u8] = b"Error: start is off the end of the string!\n";
/// `Error: stop is off the end of the string!\n`
const E2: &[u8] = b"Error: stop is off the end of the string!\n";
/// `Error: stop must come after start!\n`
const E3: &[u8] = b"Error: stop must come after start!\n";

fn slen(content: &[u8]) -> i32 {
    content.iter().position(|&b| b == 0).unwrap_or(content.len()) as i32
}

/// A handful of representative strings (incl. empty, 1 byte, long, raw bytes).
fn sample_strings(rng: &mut Rng) -> Vec<Vec<u8>> {
    vec![
        cstr(b""),
        cstr(b"a"),
        cstr(b"ab"),
        cstr(b"hello, world"),
        rand_ascii(rng, 17),
        rand_bytes(rng, 64),
        rand_bytes(rng, 300),
    ]
}

fn main() {
    let mut run = Runner::new("Phase C — ERRORS.md error-path differential");

    // ---------------------------------------------------------------- row 1
    // *start_ptr == len + 1 (one step past the valid range).
    run.row("err-01 start = len+1 -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 101);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for stop in [Arg::Null, Arg::Val(0), Arg::Val(l), Arg::Val(l + 1)] {
                d.cmp_expect(
                    &format!("len={l} stop={stop:?}"),
                    &s,
                    Arg::Val(l + 1),
                    stop,
                    1,
                    E1,
                );
            }
        }
    });

    // ---------------------------------------------------------------- row 2
    // *start_ptr == -1: negative converts to a huge size_t -> E1, not a
    // Python-style "from the end" index.
    run.row("err-02 start = -1 -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 102);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for stop in [Arg::Null, Arg::Val(l), Arg::Val(-1)] {
                d.cmp_expect(&format!("len={l} stop={stop:?}"), &s, Arg::Val(-1), stop, 1, E1);
            }
        }
    });

    // ---------------------------------------------------------------- row 3
    run.row("err-03 start = INT_MIN -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 103);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Val(i32::MIN), Arg::Null, 1, E1);
            d.cmp_expect(
                &format!("len={l} +stop"),
                &s,
                Arg::Val(i32::MIN),
                Arg::Val(l),
                1,
                E1,
            );
        }
    });

    // ---------------------------------------------------------------- row 4
    run.row("err-04 start = INT_MAX -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 104);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Val(i32::MAX), Arg::Null, 1, E1);
            d.cmp_expect(
                &format!("len={l} +stop"),
                &s,
                Arg::Val(i32::MAX),
                Arg::Val(l),
                1,
                E1,
            );
        }
    });

    // ---------------------------------------------------------------- row 5
    // Randomised start in (len, INT_MAX].
    run.row("err-05 random start > len -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 105);
        for i in 0..150 {
            let len = rng.range(0, 64) as usize;
            let s = rand_bytes(&mut rng, len);
            let st = rng.range(len as i64 + 1, i32::MAX as i64) as i32;
            let stop = match rng.below(3) {
                0 => Arg::Null,
                1 => Arg::Val(rng.range(-4, len as i64 + 4) as i32),
                _ => Arg::Val(rng.range(i32::MIN as i64, i32::MAX as i64) as i32),
            };
            d.cmp_expect(
                &format!("i={i} len={len} start={st} stop={stop:?}"),
                &s,
                Arg::Val(st),
                stop,
                1,
                E1,
            );
        }
    });

    // ---------------------------------------------------------------- row 6
    // Randomised start in [INT_MIN, -1].
    run.row("err-06 random negative start -> E1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 106);
        for i in 0..150 {
            let len = rng.range(0, 64) as usize;
            let s = rand_bytes(&mut rng, len);
            let st = rng.range(i32::MIN as i64, -1) as i32;
            let stop = match rng.below(3) {
                0 => Arg::Null,
                1 => Arg::Val(rng.range(0, len as i64) as i32),
                _ => Arg::Val(rng.range(i32::MIN as i64, i32::MAX as i64) as i32),
            };
            d.cmp_expect(
                &format!("i={i} len={len} start={st} stop={stop:?}"),
                &s,
                Arg::Val(st),
                stop,
                1,
                E1,
            );
        }
    });

    // ---------------------------------------------------------------- row 7
    // start_ptr = NULL, *stop_ptr == len + 1.
    run.row("err-07 start-NULL, stop = len+1 -> E2", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 107);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Null, Arg::Val(l + 1), 1, E2);
        }
    });

    // ---------------------------------------------------------------- row 8
    // Valid start, *stop_ptr == len + 1.
    run.row("err-08 valid start, stop = len+1 -> E2", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 108);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for st in [0, l / 2, l] {
                d.cmp_expect(
                    &format!("len={l} start={st}"),
                    &s,
                    Arg::Val(st),
                    Arg::Val(l + 1),
                    1,
                    E2,
                );
            }
        }
    });

    // ---------------------------------------------------------------- row 9
    // *stop_ptr == -1: the `stop > len` check runs BEFORE `stop <= start`,
    // so the message is E2 and never E3.
    run.row("err-09 stop = -1 -> E2 (not E3)", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 109);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for st in [Arg::Null, Arg::Val(0), Arg::Val(l)] {
                d.cmp_expect(
                    &format!("len={l} start={st:?}"),
                    &s,
                    st,
                    Arg::Val(-1),
                    1,
                    E2,
                );
            }
        }
    });

    // --------------------------------------------------------------- row 10
    run.row("err-10 stop = INT_MIN -> E2", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 110);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Null, Arg::Val(i32::MIN), 1, E2);
            d.cmp_expect(
                &format!("len={l} start=0"),
                &s,
                Arg::Val(0),
                Arg::Val(i32::MIN),
                1,
                E2,
            );
        }
    });

    // --------------------------------------------------------------- row 11
    run.row("err-11 stop = INT_MAX -> E2", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 111);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Null, Arg::Val(i32::MAX), 1, E2);
            d.cmp_expect(
                &format!("len={l} start=0"),
                &s,
                Arg::Val(0),
                Arg::Val(i32::MAX),
                1,
                E2,
            );
        }
    });

    // --------------------------------------------------------------- row 12
    // Randomised out-of-range stop with a valid (or absent) start.
    run.row("err-12 random out-of-range stop -> E2", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 112);
        for i in 0..200 {
            let len = rng.range(0, 64) as usize;
            let s = rand_bytes(&mut rng, len);
            let sp = if rng.below(2) == 0 {
                rng.range(len as i64 + 1, i32::MAX as i64) as i32
            } else {
                rng.range(i32::MIN as i64, -1) as i32
            };
            let st = if rng.below(2) == 0 {
                Arg::Null
            } else {
                Arg::Val(rng.range(0, len as i64) as i32)
            };
            d.cmp_expect(
                &format!("i={i} len={len} start={st:?} stop={sp}"),
                &s,
                st,
                Arg::Val(sp),
                1,
                E2,
            );
        }
    });

    // --------------------------------------------------------------- row 13
    // stop == start (the `<=` boundary).
    run.row("err-13 stop == start -> E3", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 113);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for v in 0..=l {
                d.cmp_expect(
                    &format!("len={l} v={v}"),
                    &s,
                    Arg::Val(v),
                    Arg::Val(v),
                    1,
                    E3,
                );
            }
        }
    });

    // --------------------------------------------------------------- row 14
    // 0 <= stop < start <= len.
    run.row("err-14 stop < start -> E3", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 114);
        for i in 0..200 {
            let len = rng.range(1, 96) as usize;
            let s = rand_bytes(&mut rng, len);
            let st = rng.range(1, len as i64) as i32;
            let sp = rng.range(0, (st - 1) as i64) as i32;
            d.cmp_expect(
                &format!("i={i} len={len} start={st} stop={sp}"),
                &s,
                Arg::Val(st),
                Arg::Val(sp),
                1,
                E3,
            );
        }
    });

    // --------------------------------------------------------------- row 15
    // Default start (start_ptr = NULL -> 0) with *stop_ptr == 0.
    run.row("err-15 start-NULL, stop = 0 -> E3", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 115);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            d.cmp_expect(&format!("len={l}"), &s, Arg::Null, Arg::Val(0), 1, E3);
        }
    });

    // --------------------------------------------------------------- row 16
    // Aliased pointers: start_ptr == stop_ptr, so stop == start -> E3.
    run.raw_row("err-16 aliased start_ptr == stop_ptr -> E3", |c, r| {
        let mut sess = harness::Session::new();
        let mut results = Vec::new();
        for lib in [c, r] {
            let mut per_lib = Vec::new();
            for v in [0i32, 1, 3] {
                let mut buf = cstr(b"hello");
                let mut cell: std::ffi::c_int = v;
                let p = buf.as_mut_ptr() as *mut std::ffi::c_char;
                let cp: *mut std::ffi::c_int = &mut cell;
                let f = lib.slice;
                let (ret, out) = sess.call(|| unsafe { f(p, cp, cp) });
                per_lib.push((v, ret, out, cell));
            }
            results.push(per_lib);
        }
        drop(sess);
        let (a, b) = (&results[0], &results[1]);
        if a != b {
            return Err(format!("C {a:?} != Rust {b:?}"));
        }
        for (v, ret, out, cell) in a {
            if *ret != 1 || out.as_slice() != E3 || *cell != *v {
                return Err(format!(
                    "C behaviour not as documented for v={v}: ret={ret} out={out:?} cell={cell}"
                ));
            }
        }
        Ok(format!("{} aliased cases", a.len()))
    });

    // --------------------------------------------------------------- row 17
    // Both indices out of range: `start` is validated first, so only E1 is
    // printed and E2 is never reached.
    run.row("err-17 both out of range -> E1 wins", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 117);
        for s in sample_strings(&mut rng) {
            let l = slen(&s);
            for (st, sp) in [
                (l + 1, l + 1),
                (l + 1, -1),
                (-1, l + 5),
                (i32::MIN, i32::MAX),
                (i32::MAX, i32::MIN),
            ] {
                d.cmp_expect(
                    &format!("len={l} start={st} stop={sp}"),
                    &s,
                    Arg::Val(st),
                    Arg::Val(sp),
                    1,
                    E1,
                );
            }
        }
    });

    // --------------------------------------------------------------- row 18
    // An invalid start must short-circuit before `*stop_ptr` is read: pass an
    // unreadable stop_ptr and require a clean E1 with no fault.
    run.row("err-18 invalid start short-circuits stop deref", |d: &mut Diff| {
        let s = cstr(b"hello");
        let l = slen(&s);
        for wild in [0x1usize, 0x8, 0xdead_0000_0000_0000] {
            d.cmp_expect(
                &format!("wild stop_ptr {wild:#x}"),
                &s,
                Arg::Val(l + 1),
                Arg::Wild(wild),
                1,
                E1,
            );
            d.cmp_expect(
                &format!("wild stop_ptr {wild:#x} (neg start)"),
                &s,
                Arg::Val(-7),
                Arg::Wild(wild),
                1,
                E1,
            );
        }
    });

    // --------------------------------------------------------------- row 19
    run.row("err-19 empty string, start = 1 -> E1", |d: &mut Diff| {
        let s = cstr(b"");
        for stop in [Arg::Null, Arg::Val(0), Arg::Val(1), Arg::Val(-1)] {
            d.cmp_expect(&format!("stop={stop:?}"), &s, Arg::Val(1), stop, 1, E1);
        }
    });

    // --------------------------------------------------------------- row 20
    run.row("err-20 empty string, start-NULL stop = 0 -> E3", |d: &mut Diff| {
        d.cmp_expect("empty", &cstr(b""), Arg::Null, Arg::Val(0), 1, E3);
    });

    // --------------------------------------------------------------- row 21
    run.row("err-21 empty string, start = 0 stop = 0 -> E3", |d: &mut Diff| {
        d.cmp_expect("empty", &cstr(b""), Arg::Val(0), Arg::Val(0), 1, E3);
    });

    // --------------------------------------------------------------- row 22
    // No minimum-length rejection: an empty string with both defaults succeeds
    // and prints just the newline.
    run.row("err-22 empty string, both NULL -> success", |d: &mut Diff| {
        d.cmp_expect("empty", &cstr(b""), Arg::Null, Arg::Null, 0, b"\n");
        d.cmp_expect("empty start=0", &cstr(b""), Arg::Val(0), Arg::Null, 0, b"\n");
    });

    // --------------------------------------------------------------- row 23
    // The "out-of-range enum value" analogue: sweep every index in
    // [-3, len+3] (plus NULL) for both parameters and all lengths 0..=6.
    run.row("err-23 exhaustive out-of-domain sweep", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 123);
        for len in 0..=6i32 {
            let s = rand_bytes(&mut rng, len as usize);
            let mut vals = vec![Arg::Null];
            for v in -3..=(len + 3) {
                vals.push(Arg::Val(v));
            }
            for st in &vals {
                for sp in &vals {
                    d.cmp(&format!("len={len} {st:?}/{sp:?}"), &s, *st, *sp);
                }
            }
        }
    });

    // --------------------------------------------------------------- row 24
    // mystr == NULL: unchecked strlen(NULL). Both libraries must fault the
    // same way; compared in forked children so the test survives.
    run.raw_row("err-24 mystr = NULL faults identically", |c: &Lib, r: &Lib| {
        let cs = null_string_status(c);
        let rs = null_string_status(r);
        let describe = |st: i32| {
            if wif_signaled(st) {
                format!("signal {}", wtermsig(st))
            } else {
                format!("exit {}", wexitstatus(st))
            }
        };
        if wif_signaled(cs) != wif_signaled(rs) || wtermsig(cs) != wtermsig(rs) {
            return Err(format!(
                "C child {} but Rust child {}",
                describe(cs),
                describe(rs)
            ));
        }
        if !wif_signaled(cs) {
            return Err(format!(
                "expected the C child to be killed by a signal, got {}",
                describe(cs)
            ));
        }
        Ok(format!("both children killed by {}", describe(cs)))
    });

    // --------------------------------------------------------------- row 26
    // Unwritable stdout (fd 1 closed).  The C code ignores printf's result, so
    // both libraries must return their usual sentinel and leave errno in the
    // same state.  Deliberately the last row: it perturbs process-wide stdio.
    run.raw_row("err-26 unwritable stdout is ignored", |c: &Lib, r: &Lib| {
        let cases: [(&str, Arg, Arg, i32); 4] = [
            ("success path", Arg::Val(0), Arg::Val(5), 0),
            ("E1 path", Arg::Val(99), Arg::Null, 1),
            ("E2 path", Arg::Null, Arg::Val(99), 1),
            ("E3 path", Arg::Val(2), Arg::Val(1), 1),
        ];
        let s = cstr(b"hello");
        let mut notes = Vec::new();
        for (label, st, sp, want_ret) in cases {
            let a = harness::call_with_unwritable_stdout(c, &s, st, sp);
            let b = harness::call_with_unwritable_stdout(r, &s, st, sp);
            if a.0 != b.0 || a.1 != b.1 {
                return Err(format!(
                    "{label}: C (ret={}, errno={}, fflush={}) != Rust (ret={}, errno={}, fflush={})",
                    a.0, a.1, a.2, b.0, b.1, b.2
                ));
            }
            if a.0 != want_ret {
                return Err(format!(
                    "{label}: C returned {} with stdout closed, expected {want_ret}",
                    a.0
                ));
            }
            notes.push(format!("{label}: ret={} errno={}", a.0, a.1));
        }
        Ok(notes.join(", "))
    });

    run.finish();
}
