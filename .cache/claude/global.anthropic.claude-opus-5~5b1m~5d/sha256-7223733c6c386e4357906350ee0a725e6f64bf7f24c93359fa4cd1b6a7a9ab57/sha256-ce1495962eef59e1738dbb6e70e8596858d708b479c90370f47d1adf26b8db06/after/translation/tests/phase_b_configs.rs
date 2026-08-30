//! Phase B — valid-path differential tests, one sub-test per `CONFIGS.md` row.
//!
//! Every row drives the *only* public entry point, `slice`, through both `.so`s
//! and compares the return code, the exact bytes written to `stdout`, and the
//! post-call state of all three arguments. Rows use many randomized inputs from
//! a fixed-seed PRNG so they cover value-dependent behaviour, not one lucky
//! sample.

mod common;

use common::*;

/// Iterations per randomized row. Kept modest because every call performs a
/// `pipe`/`dup2`/`read` cycle for each of the two implementations.
const N: usize = 200;
/// Iterations for rows whose payloads are kilobytes long.
const N_BIG: usize = 24;

/// All four `(start_ptr, stop_ptr)` presence combinations, as used by rows that
/// sweep a payload shape across the A1×A2 axes.
fn bound_combos(len: usize) -> Vec<(Option<i32>, Option<i32>)> {
    let l = len as i32;
    let mut v = vec![(None, None), (Some(0), None)];
    if len > 0 {
        v.push((None, Some(l)));
        v.push((Some(0), Some(l)));
        v.push((Some(l), None));
        if len > 1 {
            v.push((Some(l / 2), None));
            v.push((None, Some(l / 2 + 1)));
            v.push((Some(l / 2), Some(l)));
            v.push((Some(0), Some(l / 2 + 1)));
        }
    }
    v
}

#[test]
fn phase_b_configs() {
    silence_panic_hook();
    let mut s = Suite::new("phase_b_configs");

    // -- C1: both bounds absent, random ASCII ------------------------------
    s.row("C1 c1_both_null_ascii", || {
        let mut rng = Rng::new(0xC1);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            assert_same_str(&format!("C1/#{i}/len={len}"), &p, None, None);
        }
    });

    // -- C2: both absent, empty string (precision 0 ⇒ bare newline) --------
    s.row("C2 c2_both_null_empty", || {
        let out = assert_same_str("C2/empty", b"", None, None);
        assert_eq!(out.ret, 0, "C2: empty string must succeed");
        assert_eq!(out.stdout, b"\n", "C2: expected a bare newline");
    });

    // -- C3: both absent, len == 1, every possible byte --------------------
    s.row("C3 c3_both_null_single_char", || {
        for b in 1u8..=255 {
            assert_same_str(&format!("C3/byte=0x{b:02x}"), &[b], None, None);
        }
    });

    // -- C4: both absent, multi-KiB payloads over the full byte range ------
    s.row("C4 c4_both_null_large", || {
        let mut rng = Rng::new(0xC4);
        for i in 0..N_BIG {
            let len = rng.range(1024, 8192);
            let p = rng.payload_any(len);
            let out = assert_same_str(&format!("C4/#{i}/len={len}"), &p, None, None);
            assert_eq!(out.stdout.len(), len + 1, "C4: whole string + newline");
        }
    });

    // -- C5: start only, random in [0, len] (includes the len boundary) ----
    s.row("C5 c5_start_only_random", || {
        let mut rng = Rng::new(0xC5);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(0, len) as i32;
            let out = assert_same_str(&format!("C5/#{i}/len={len}/start={start}"), &p, Some(start), None);
            assert_eq!(out.ret, 0, "C5: start in [0,len] must be accepted");
        }
    });

    // -- C6: explicit start 0 behaves exactly like a NULL start ------------
    s.row("C6 c6_start_zero_vs_null", || {
        let mut rng = Rng::new(0xC6);
        for i in 0..N {
            let len = rng.range(0, 48);
            let p = rng.payload(len, ASCII);
            let explicit = assert_same_str(&format!("C6/#{i}/explicit"), &p, Some(0), None);
            let implicit = assert_same_str(&format!("C6/#{i}/implicit"), &p, None, None);
            assert_eq!(
                explicit.stdout, implicit.stdout,
                "C6/#{i}: start=0 and start=NULL must print the same"
            );
            assert_eq!(explicit.ret, implicit.ret, "C6/#{i}: same return code");
        }
    });

    // -- C7: start == len exactly (accepted boundary, empty output) --------
    s.row("C7 c7_start_at_len", || {
        let mut rng = Rng::new(0xC7);
        for i in 0..N {
            let len = rng.range(0, 64);
            let p = rng.payload(len, ASCII);
            let out = assert_same_str(&format!("C7/#{i}/len={len}"), &p, Some(len as i32), None);
            assert_eq!(out.ret, 0, "C7: start == len is accepted (check is `>`)");
            assert_eq!(out.stdout, b"\n", "C7: zero-width slice prints a newline");
        }
    });

    // -- C8: stop only, random in [1, len] --------------------------------
    s.row("C8 c8_stop_only_random", || {
        let mut rng = Rng::new(0xC8);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let stop = rng.range(1, len) as i32;
            let out = assert_same_str(&format!("C8/#{i}/len={len}/stop={stop}"), &p, None, Some(stop));
            assert_eq!(out.ret, 0, "C8: stop in [1,len] with NULL start is accepted");
        }
    });

    // -- C9: stop == len exactly (accepted boundary ⇒ whole string) --------
    s.row("C9 c9_stop_at_len", || {
        let mut rng = Rng::new(0xC9);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let out = assert_same_str(&format!("C9/#{i}/len={len}"), &p, None, Some(len as i32));
            assert_eq!(out.ret, 0, "C9: stop == len is accepted");
            assert_eq!(out.stdout.len(), len + 1, "C9: whole string + newline");
        }
    });

    // -- C10: both bounds set, random interior window ---------------------
    s.row("C10 c10_both_set_random", || {
        let mut rng = Rng::new(0xC10);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(0, len - 1);
            let stop = rng.range(start + 1, len);
            let out = assert_same_str(
                &format!("C10/#{i}/len={len}/{start}..{stop}"),
                &p,
                Some(start as i32),
                Some(stop as i32),
            );
            assert_eq!(out.ret, 0, "C10: 0<=start<stop<=len is accepted");
            assert_eq!(out.stdout.len(), stop - start + 1, "C10: width + newline");
        }
    });

    // -- C11: minimal non-empty width (stop == start + 1) -----------------
    s.row("C11 c11_both_set_width_one", || {
        let mut rng = Rng::new(0xC11);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(0, len - 1);
            let out = assert_same_str(
                &format!("C11/#{i}/len={len}/at={start}"),
                &p,
                Some(start as i32),
                Some(start as i32 + 1),
            );
            assert_eq!(out.ret, 0);
            assert_eq!(
                out.stdout,
                vec![p[start], b'\n'],
                "C11: must print exactly one character"
            );
        }
    });

    // -- C12: maximal width via explicit bounds (0 .. len) ----------------
    s.row("C12 c12_both_set_full_range", || {
        let mut rng = Rng::new(0xC12);
        for i in 0..N {
            let len = rng.range(1, 64);
            let p = rng.payload(len, ASCII);
            let out = assert_same_str(
                &format!("C12/#{i}/len={len}"),
                &p,
                Some(0),
                Some(len as i32),
            );
            let mut expect = p.clone();
            expect.push(b'\n');
            assert_eq!(out.stdout, expect, "C12: explicit full range == whole string");
        }
    });

    // -- C13: both bounds set on a multi-KiB payload ----------------------
    s.row("C13 c13_both_set_large", || {
        let mut rng = Rng::new(0xC13);
        for i in 0..N_BIG {
            let len = rng.range(1024, 8192);
            let p = rng.payload_any(len);
            let start = rng.range(0, len - 1);
            let stop = rng.range(start + 1, len);
            let out = assert_same_str(
                &format!("C13/#{i}/len={len}/{start}..{stop}"),
                &p,
                Some(start as i32),
                Some(stop as i32),
            );
            assert_eq!(out.stdout.len(), stop - start + 1);
        }
    });

    // -- C14: payload full of format-specifier bytes ----------------------
    // The payload is an *argument* to `%.*s`, never a format string; if either
    // side ever passed it as a format, output would diverge (or crash).
    s.row("C14 c14_percent_payload", || {
        let fixed: [&[u8]; 6] = [
            b"%s",
            b"%n%n%n",
            b"%%",
            b"100%% sure",
            b"%.*s",
            b"a%sb%nc%dd",
        ];
        for (k, p) in fixed.iter().enumerate() {
            for (j, (a, b)) in bound_combos(p.len()).into_iter().enumerate() {
                assert_same_str(&format!("C14/fixed{k}/combo{j}"), p, a, b);
            }
        }
        let mut rng = Rng::new(0xC14);
        for i in 0..N {
            let len = rng.range(1, 40);
            let p = rng.payload(len, PERCENTS);
            let combos = bound_combos(len);
            let (a, b) = combos[rng.below(combos.len())];
            assert_same_str(&format!("C14/rand#{i}/len={len}"), &p, a, b);
        }
    });

    // -- C15: high bytes 0x80..=0xFF (invalid UTF-8) ----------------------
    s.row("C15 c15_high_bytes", || {
        let all = high_bytes();
        for (j, (a, b)) in bound_combos(all.len()).into_iter().enumerate() {
            assert_same_str(&format!("C15/all/combo{j}"), &all, a, b);
        }
        // A lone continuation byte, a truncated multi-byte sequence, and an
        // over-long form: all must pass through untouched.
        let tricky: [&[u8]; 5] = [
            &[0x80],
            &[0xff],
            &[0xc3],             // truncated 2-byte lead
            &[0xe2, 0x82],       // truncated 3-byte sequence
            &[0xf0, 0x9f, 0x92], // truncated 4-byte sequence
        ];
        for (k, p) in tricky.iter().enumerate() {
            for (j, (a, b)) in bound_combos(p.len()).into_iter().enumerate() {
                assert_same_str(&format!("C15/tricky{k}/combo{j}"), p, a, b);
            }
        }
        let mut rng = Rng::new(0xC15);
        for i in 0..N {
            let len = rng.range(1, 40);
            let p: Vec<u8> = (0..len).map(|_| 0x80 + (rng.next_u32() % 128) as u8).collect();
            let combos = bound_combos(len);
            let (a, b) = combos[rng.below(combos.len())];
            assert_same_str(&format!("C15/rand#{i}"), &p, a, b);
        }
    });

    // -- C16: embedded newlines and other control bytes -------------------
    s.row("C16 c16_control_bytes", || {
        let fixed: [&[u8]; 4] = [b"a\nb", b"\r\n", b"tab\there", b"\x1b[31mred\x1b[0m"];
        for (k, p) in fixed.iter().enumerate() {
            for (j, (a, b)) in bound_combos(p.len()).into_iter().enumerate() {
                assert_same_str(&format!("C16/fixed{k}/combo{j}"), p, a, b);
            }
        }
        let mut rng = Rng::new(0xC16);
        for i in 0..N {
            let len = rng.range(1, 40);
            let p = rng.payload(len, CONTROLS);
            let combos = bound_combos(len);
            let (a, b) = combos[rng.below(combos.len())];
            assert_same_str(&format!("C16/rand#{i}"), &p, a, b);
        }
    });

    // -- C17: window ending at the NUL vs strictly before it --------------
    // `%.*s` stops at the precision *or* the terminator, whichever comes
    // first; both cases must agree.
    s.row("C17 c17_window_vs_terminator", || {
        let mut rng = Rng::new(0xC17);
        for i in 0..N {
            let len = rng.range(2, 64);
            let p = rng.payload(len, ASCII);
            let start = rng.range(0, len - 1) as i32;
            // Ends exactly at the terminator.
            let at_end = assert_same_str(
                &format!("C17/#{i}/at_end"),
                &p,
                Some(start),
                Some(len as i32),
            );
            assert_eq!(at_end.stdout.len(), len - start as usize + 1);
            // Ends strictly before the terminator.
            if (start as usize) + 1 < len {
                let before = assert_same_str(
                    &format!("C17/#{i}/before_end"),
                    &p,
                    Some(start),
                    Some(len as i32 - 1),
                );
                assert_eq!(before.stdout.len(), len - 1 - start as usize + 1);
            }
            // Start-only form, which derives stop from the terminator.
            assert_same_str(&format!("C17/#{i}/derived"), &p, Some(start), None);
        }
    });

    // -- C18: statelessness across interleaved calls ----------------------
    s.row("C18 c18_stateless_repeat", || {
        let mut rng = Rng::new(0xC18);
        for i in 0..40 {
            let len = rng.range(4, 48);
            let p = rng.payload(len, ASCII);
            let l = len as i32;
            let script: [(Option<i32>, Option<i32>); 8] = [
                (None, None),
                (Some(1), None),
                (None, Some(l)),
                (Some(1), Some(l)),
                (Some(l), None),          // zero-width
                (Some(l), Some(l)),       // rejected (stop <= start)
                (Some(l + 1), None),      // rejected (start past end)
                (None, None),             // back to the start: no residue
            ];
            let mut firsts = Vec::new();
            for pass in 0..3 {
                for (k, (a, b)) in script.iter().enumerate() {
                    let out =
                        assert_same_str(&format!("C18/#{i}/pass{pass}/step{k}"), &p, *a, *b);
                    if pass == 0 {
                        firsts.push(out);
                    } else {
                        assert_eq!(
                            firsts[k], out,
                            "C18/#{i}/step{k}: repeated call differed ⇒ hidden state"
                        );
                    }
                }
            }
        }
    });

    // -- C19: exhaustive small sweep over len × start × stop × NULLs ------
    s.row("C19 c19_exhaustive_small", || {
        let mut rng = Rng::new(0xC19);
        for len in 0usize..=24 {
            let p = rng.payload(len, ASCII);
            let mut starts: Vec<Option<i32>> = vec![None];
            let mut stops: Vec<Option<i32>> = vec![None];
            for v in 0..=len {
                starts.push(Some(v as i32));
                stops.push(Some(v as i32));
            }
            for a in &starts {
                for b in &stops {
                    assert_same_str(&format!("C19/len={len}/{a:?}/{b:?}"), &p, *a, *b);
                }
            }
        }
    });

    // -- C20: bytes after the NUL terminator must be ignored --------------
    s.row("C20 c20_bytes_after_nul", || {
        // The two regions use disjoint alphabets, so "did anything past the
        // terminator leak?" is a simple membership test rather than a
        // substring search that short random markers could satisfy by chance.
        const VISIBLE_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz ";
        const HIDDEN_ALPHABET: &[u8] = b"0123456789";

        let mut rng = Rng::new(0xC20);
        for i in 0..N {
            let len = rng.range(0, 32);
            let visible = rng.payload(len, VISIBLE_ALPHABET);
            let hidden_len = rng.range(1, 32);
            let hidden = rng.payload(hidden_len, HIDDEN_ALPHABET);

            let mut buf = visible.clone();
            buf.push(0);
            buf.extend_from_slice(&hidden);
            buf.push(0); // keep the allocation itself terminated

            let combos = bound_combos(len);
            for (j, (a, b)) in combos.into_iter().enumerate() {
                let out = assert_same(&format!("C20/#{i}/combo{j}"), &buf, a, b);
                assert!(
                    !out.stdout.iter().any(|b| HIDDEN_ALPHABET.contains(b)),
                    "C20/#{i}/combo{j}: bytes past the NUL leaked into the output: {:?}",
                    String::from_utf8_lossy(&out.stdout)
                );
            }
        }
    });

    s.finish();
}
