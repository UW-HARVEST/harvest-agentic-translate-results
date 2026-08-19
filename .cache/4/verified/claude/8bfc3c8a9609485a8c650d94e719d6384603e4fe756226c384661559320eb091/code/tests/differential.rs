//! Phase B — valid-path differential tests.
//!
//! One row per line of `CONFIGS.md`.  Every row drives BOTH shared objects
//! (`c_src/build/libString_Slice.so` and `target/<profile>/libString_Slice.so`)
//! through their exported `slice` symbol with many randomised inputs and
//! compares the `int` return value, the exact bytes written to stdout, and the
//! caller's memory.
//!
//! Run with `cargo test --test differential` (custom harness: see Cargo.toml).

#[path = "harness/mod.rs"]
mod harness;

use harness::{cstr, rand_ascii, rand_bytes, Arg, Diff, Rng, Runner, SEED};

/// `strlen` of a NUL-terminated test buffer.
fn slen(content: &[u8]) -> i32 {
    content.iter().position(|&b| b == 0).unwrap_or(content.len()) as i32
}

/// A random C string, cycling through the interesting content classes.
fn rand_string(rng: &mut Rng, len: usize) -> Vec<u8> {
    match rng.below(3) {
        0 => rand_ascii(rng, len),
        1 => rand_bytes(rng, len),
        _ => {
            // UTF-8 text plus stray high bytes, truncated/padded to `len`.
            const POOL: &[&str] = &["héllo", "→", "世界", "🎉", "abc", "%s", "\u{7f}\u{1}"];
            let mut v = Vec::new();
            while v.len() < len {
                let s = POOL[rng.below(POOL.len() as u64) as usize];
                v.extend_from_slice(s.as_bytes());
            }
            v.truncate(len);
            // Never leave an interior NUL: the pool has none, but be explicit.
            for b in v.iter_mut() {
                if *b == 0 {
                    *b = b'?';
                }
            }
            v.push(0);
            v
        }
    }
}

fn main() {
    let mut run = Runner::new("Phase B — CONFIGS.md valid-path differential");

    // ---------------------------------------------------------------- row 1
    // start_ptr = NULL, stop_ptr = NULL, len == 0.
    run.row("cfg-01 both-NULL len=0", |d: &mut Diff| {
        d.cmp_v("empty", &cstr(b""), None, None);
    });

    // ---------------------------------------------------------------- row 2
    // start_ptr = NULL, stop_ptr = NULL, len == 1 — every possible byte.
    run.row("cfg-02 both-NULL len=1 (all 255 bytes)", |d: &mut Diff| {
        for b in 1u16..=255 {
            d.cmp_v(&format!("byte {b:#04x}"), &cstr(&[b as u8]), None, None);
        }
    });

    // ---------------------------------------------------------------- row 3
    // start_ptr = NULL, stop_ptr = NULL, random len 2..=64, printable ASCII.
    run.row("cfg-03 both-NULL len=2..64 ascii", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 3);
        for i in 0..200 {
            let len = rng.range(2, 64) as usize;
            let s = rand_ascii(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, None, None);
        }
    });

    // ---------------------------------------------------------------- row 4
    // start_ptr = NULL, stop_ptr = NULL, large strings of arbitrary bytes.
    run.row("cfg-04 both-NULL len=256..4096 raw bytes", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 4);
        for i in 0..40 {
            let len = rng.range(256, 4096) as usize;
            let s = rand_bytes(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, None, None);
        }
    });

    // ---------------------------------------------------------------- row 5
    // Content that looks like printf conversion specifiers.
    run.row("cfg-05 printf-specifier payloads", |d: &mut Diff| {
        const FIXED: &[&[u8]] = &[
            b"%s",
            b"%d %i %u %x",
            b"%n",
            b"%%",
            b"100%% done",
            b"%p%p%p%p%p%p%p%p",
            b"%.*s",
            b"%99999999d",
            b"%s%n%s%n",
            b"a%sb%nc",
        ];
        for (i, f) in FIXED.iter().enumerate() {
            let s = cstr(f);
            let l = slen(&s);
            d.cmp_v(&format!("fixed {i} both-NULL"), &s, None, None);
            d.cmp_v(&format!("fixed {i} explicit"), &s, Some(0), Some(l));
            if l >= 2 {
                d.cmp_v(&format!("fixed {i} inner"), &s, Some(1), Some(l - 1));
            }
        }
        // Randomised concatenations of specifier tokens.
        let mut rng = Rng::new(SEED ^ 5);
        const TOK: &[&[u8]] = &[b"%s", b"%n", b"%d", b"%%", b"%.*s", b"x", b" ", b"%c"];
        for i in 0..50 {
            let mut v = Vec::new();
            for _ in 0..rng.range(1, 12) {
                v.extend_from_slice(TOK[rng.below(TOK.len() as u64) as usize]);
            }
            let s = cstr(&v);
            let l = slen(&s);
            let st = rng.range(0, l as i64) as i32;
            let sp = if st < l { Some(rng.range((st + 1) as i64, l as i64) as i32) } else { None };
            d.cmp_v(&format!("rand {i}"), &s, Some(st), sp);
        }
    });

    // ---------------------------------------------------------------- row 6
    // Buffer physically longer than strlen: an embedded NUL followed by junk.
    run.row("cfg-06 buffer longer than strlen (embedded NUL)", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 6);
        for i in 0..50 {
            let visible = rng.range(0, 32) as i32;
            let hidden = rng.range(1, 32) as usize;
            let mut buf = rand_bytes(&mut rng, visible as usize); // ends with the NUL
            buf.extend((0..hidden).map(|_| rng.byte_nonzero()));
            buf.push(0);
            let tag = format!("i={i} visible={visible} hidden={hidden}");
            // Defaults: only the bytes before the embedded NUL are visible.
            d.cmp_v(&format!("{tag} NULL/NULL"), &buf, None, None);
            // Indices are bounded by strlen, not by the allocation: everything
            // above `visible` must be rejected even though the memory is there.
            for st in [0, visible, visible + 1, visible + hidden as i32] {
                d.cmp_v(&format!("{tag} {st}/NULL"), &buf, Some(st), None);
                d.cmp_v(
                    &format!("{tag} {st}/{}", visible + 1),
                    &buf,
                    Some(st),
                    Some(visible + 1),
                );
            }
            if visible > 0 {
                d.cmp_v(&format!("{tag} 0/{visible}"), &buf, Some(0), Some(visible));
            }
        }
    });

    // ---------------------------------------------------------------- row 7
    // start_ptr = &0, stop_ptr = NULL.
    run.row("cfg-07 start=0 stop-NULL", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 7);
        for i in 0..100 {
            let len = rng.range(0, 80) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, Some(0), None);
        }
    });

    // ---------------------------------------------------------------- row 8
    // start_ptr = &len (accepted boundary) -> zero-width slice.
    run.row("cfg-08 start=len stop-NULL (width 0)", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 8);
        for i in 0..100 {
            let len = rng.range(0, 80) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, Some(len as i32), None);
        }
    });

    // ---------------------------------------------------------------- row 9
    // start_ptr = &(len-1): last byte only.
    run.row("cfg-09 start=len-1 stop-NULL", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 9);
        for i in 0..100 {
            let len = rng.range(1, 80) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, Some(len as i32 - 1), None);
        }
    });

    // --------------------------------------------------------------- row 10
    // Random start in [0, len], stop_ptr = NULL.
    run.row("cfg-10 random start, stop-NULL", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 10);
        for i in 0..300 {
            let len = rng.range(0, 128) as usize;
            let s = rand_string(&mut rng, len);
            let st = rng.range(0, len as i64) as i32;
            d.cmp_v(&format!("i={i} len={len} start={st}"), &s, Some(st), None);
        }
    });

    // --------------------------------------------------------------- row 11
    // start_ptr = NULL, stop = len (accepted boundary).
    run.row("cfg-11 start-NULL stop=len", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 11);
        for i in 0..100 {
            let len = rng.range(1, 80) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, None, Some(len as i32));
        }
    });

    // --------------------------------------------------------------- row 12
    // start_ptr = NULL, stop = 1 (minimal valid stop against default start 0).
    run.row("cfg-12 start-NULL stop=1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 12);
        for i in 0..100 {
            let len = rng.range(1, 80) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, None, Some(1));
        }
    });

    // --------------------------------------------------------------- row 13
    // start_ptr = NULL, random stop in [1, len].
    run.row("cfg-13 start-NULL random stop", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 13);
        for i in 0..300 {
            let len = rng.range(1, 128) as usize;
            let s = rand_string(&mut rng, len);
            let sp = rng.range(1, len as i64) as i32;
            d.cmp_v(&format!("i={i} len={len} stop={sp}"), &s, None, Some(sp));
        }
    });

    // --------------------------------------------------------------- row 14
    // Both non-NULL, full range.
    run.row("cfg-14 start=0 stop=len", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 14);
        for i in 0..100 {
            let len = rng.range(1, 96) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(&format!("i={i} len={len}"), &s, Some(0), Some(len as i32));
        }
    });

    // --------------------------------------------------------------- row 15
    // Both non-NULL, single trailing byte.
    run.row("cfg-15 start=len-1 stop=len", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 15);
        for i in 0..100 {
            let len = rng.range(1, 96) as usize;
            let s = rand_string(&mut rng, len);
            d.cmp_v(
                &format!("i={i} len={len}"),
                &s,
                Some(len as i32 - 1),
                Some(len as i32),
            );
        }
    });

    // --------------------------------------------------------------- row 16
    // Both non-NULL, minimal window stop = start + 1.
    run.row("cfg-16 stop=start+1", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 16);
        for i in 0..300 {
            let len = rng.range(1, 128) as usize;
            let s = rand_string(&mut rng, len);
            let st = rng.range(0, len as i64 - 1) as i32;
            d.cmp_v(
                &format!("i={i} len={len} start={st}"),
                &s,
                Some(st),
                Some(st + 1),
            );
        }
    });

    // --------------------------------------------------------------- row 17
    // Both non-NULL, random valid window.
    run.row("cfg-17 random 0<=start<stop<=len", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 17);
        for i in 0..400 {
            let len = rng.range(1, 200) as usize;
            let s = rand_string(&mut rng, len);
            let st = rng.range(0, len as i64 - 1) as i32;
            let sp = rng.range((st + 1) as i64, len as i64) as i32;
            d.cmp_v(
                &format!("i={i} len={len} start={st} stop={sp}"),
                &s,
                Some(st),
                Some(sp),
            );
        }
    });

    // --------------------------------------------------------------- row 18
    // len == 1, start = 0, stop = 1 — smallest non-empty slice, every byte.
    run.row("cfg-18 len=1 start=0 stop=1 (all bytes)", |d: &mut Diff| {
        for b in 1u16..=255 {
            d.cmp_v(
                &format!("byte {b:#04x}"),
                &cstr(&[b as u8]),
                Some(0),
                Some(1),
            );
        }
    });

    // --------------------------------------------------------------- row 19
    // UTF-8 text cut mid-codepoint: every valid (start, stop) pair.
    run.row("cfg-19 UTF-8 sliced mid-codepoint", |d: &mut Diff| {
        const TEXTS: &[&str] = &[
            "héllo wörld",
            "→←↑↓",
            "世界こんにちは",
            "🎉🎊🥳",
            "aé→世🎉z",
        ];
        for (t, text) in TEXTS.iter().enumerate() {
            let s = cstr(text.as_bytes());
            let l = slen(&s);
            for st in 0..=l {
                for sp in (st + 1)..=l {
                    d.cmp_v(&format!("t={t} {st}..{sp}"), &s, Some(st), Some(sp));
                }
                d.cmp_v(&format!("t={t} {st}..NULL"), &s, Some(st), None);
            }
            d.cmp_v(&format!("t={t} NULL..NULL"), &s, None, None);
        }
    });

    // --------------------------------------------------------------- row 20
    // String of every byte value 0x01..=0xFF, sliced on 16-byte boundaries.
    run.row("cfg-20 all byte values, 16-byte slices", |d: &mut Diff| {
        let all: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
        let s = cstr(&all);
        let l = slen(&s); // 255
        let mut bounds: Vec<i32> = (0..=l).step_by(16).collect();
        if *bounds.last().unwrap() != l {
            bounds.push(l);
        }
        for &st in &bounds {
            for &sp in &bounds {
                if sp > st {
                    d.cmp_v(&format!("{st}..{sp}"), &s, Some(st), Some(sp));
                }
            }
            d.cmp_v(&format!("{st}..NULL"), &s, Some(st), None);
            d.cmp_v(&format!("NULL..{st}"), &s, None, Some(st).filter(|v| *v > 0));
        }
    });

    // --------------------------------------------------------------- row 21
    // Exhaustive valid domain: len 0..=8 x 4 pointer combos x all valid
    // (start, stop), with several random contents per length.
    run.row("cfg-21 exhaustive valid cross-product len<=8", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 21);
        for len in 0..=8i32 {
            for rep in 0..3 {
                let s = rand_string(&mut rng, len as usize);
                let tag = format!("len={len} rep={rep}");
                // combo 1: both NULL
                d.cmp_v(&format!("{tag} NULL/NULL"), &s, None, None);
                // combo 2: start only
                for st in 0..=len {
                    d.cmp_v(&format!("{tag} {st}/NULL"), &s, Some(st), None);
                }
                // combo 3: stop only (valid means stop > 0)
                for sp in 1..=len {
                    d.cmp_v(&format!("{tag} NULL/{sp}"), &s, None, Some(sp));
                }
                // combo 4: both, valid window
                for st in 0..=len {
                    for sp in (st + 1)..=len {
                        d.cmp_v(&format!("{tag} {st}/{sp}"), &s, Some(st), Some(sp));
                    }
                }
            }
        }
    });

    // --------------------------------------------------------------- row 22
    // Statelessness: replay a long randomised transcript against C, then the
    // identical transcript against Rust, and compare the whole recording.
    run.row("cfg-22 400-call transcript replay", |d: &mut Diff| {
        let mut rng = Rng::new(SEED ^ 22);
        let mut cases: Vec<(Vec<u8>, Arg, Arg)> = Vec::new();
        for _ in 0..400 {
            let len = rng.range(0, 64) as usize;
            let s = rand_string(&mut rng, len);
            let l = len as i64;
            // Mix valid and invalid indices so the transcript exercises both
            // the printing path and all three rejection paths.
            let mk = |rng: &mut Rng| -> Arg {
                match rng.below(6) {
                    0 => Arg::Null,
                    1 => Arg::Val(rng.range(-4, l + 4) as i32),
                    2 => Arg::Val(rng.range(0, l.max(0)) as i32),
                    3 => Arg::Val(l as i32),
                    4 => Arg::Val(rng.range(i32::MIN as i64, i32::MAX as i64) as i32),
                    _ => Arg::Val(0),
                }
            };
            let a = mk(&mut rng);
            let b = mk(&mut rng);
            cases.push((s, a, b));
        }
        d.transcript("mixed", &cases);
    });

    run.finish();
}
