//! `CONFIGS.md` rows 35 and 36 — broad randomized differential sweeps.
//!
//! Every case runs in a `fork()`ed grandchild with `alarm(1)` armed, so the
//! transcript records, per case, whether the library returned, aborted (with
//! the exact `assert()` message), crashed, or looped forever.  The C and Rust
//! transcripts must be identical line for line.

mod common;
use common::*;

define_child_runner!();

fn summarize(tag: &str, lines: &[String]) {
    let ok = lines.iter().filter(|l| l.contains(" OK ")).count();
    let ab = lines.iter().filter(|l| l.contains(" SIG")).count();
    let hg = lines.iter().filter(|l| l.contains("HANG")).count();
    let mut asserts: std::collections::BTreeMap<String, usize> = Default::default();
    for l in lines {
        if let Some(p) = l.find("lib.c:") {
            let n: String = l[p + 6..].chars().take_while(|c| c.is_ascii_digit()).collect();
            *asserts.entry(n).or_default() += 1;
        }
    }
    eprintln!(
        "[{tag}] {} cases: {ok} returned, {ab} died by signal, {hg} hung; asserts by line: {asserts:?}",
        lines.len()
    );
}

/// Row 35 — completely random byte strings as `in`.
#[test]
fn cfg35_random_bytes_sweep() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF1);
    let mut cases = Vec::new();
    for _ in 0..6000 {
        let n = rng.range(0, 64);
        let data = rng.bytes(n);
        cases.push(
            Case::new("cfg35", &data)
                .align(rng.below(4))
                .out(rng.range(0, 256)),
        );
    }
    let lines = diff_cases_in_children("cfg35", &cases);
    summarize("cfg35", &lines);
    assert_eq!(lines.len(), cases.len());
}

/// Row 35b — random bytes with a *plausible* block header, so more of the
/// decoder is reached instead of failing in the first three bits.
#[test]
fn cfg35b_seeded_header_sweep() {
    let mut rng = Rng::new(0x0F0F_0F0F_1111_2222);
    let mut cases = Vec::new();
    for btype in 0..3u32 {
        for _ in 0..1500 {
            let n = rng.range(1, 48);
            let mut w = BitWriter::new();
            w.bits(1, 1);
            w.bits(btype, 2);
            for _ in 0..n {
                w.bits(rng.byte() as u32, 8);
            }
            let s = w.finish();
            cases.push(
                Case::new(&format!("cfg35b/b{btype}"), &s)
                    .align(rng.below(4))
                    .out(rng.range(0, 256)),
            );
        }
    }
    let lines = diff_cases_in_children("cfg35b", &cases);
    summarize("cfg35b", &lines);
}

/// Row 36 — structure-aware corruption: take a *valid* stream and flip bits.
#[test]
fn cfg36_bitflip_sweep() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let mut cases = Vec::new();
    for _ in 0..2500 {
        let n = rng.range(1, 300);
        let plain = if rng.below(2) == 0 {
            rng.bytes(n)
        } else {
            rng.repetitive(n)
        };
        let level = (rng.below(10)) as u32;
        let mut s = deflate_ref(&plain, level);
        if s.is_empty() {
            continue;
        }
        let flips = rng.range(1, 8);
        for _ in 0..flips {
            let byte = rng.below(s.len());
            let bit = rng.below(8);
            s[byte] ^= 1 << bit;
        }
        cases.push(
            Case::new("cfg36", &s)
                .align(rng.below(4))
                .out(rng.range(0, n + 64)),
        );
    }
    let lines = diff_cases_in_children("cfg36", &cases);
    summarize("cfg36", &lines);
}

/// Row 36b — byte-level truncation of valid streams (every prefix length).
#[test]
fn cfg36b_truncation_sweep() {
    let mut rng = Rng::new(0x5555_AAAA_3333_9999);
    let mut cases = Vec::new();
    for _ in 0..300 {
        let n = rng.range(1, 120);
        let plain = rng.repetitive(n);
        let s = deflate_ref(&plain, (rng.below(10)) as u32);
        for cut in 0..s.len().min(24) {
            let t = &s[..s.len() - cut];
            cases.push(
                Case::new("cfg36b", t)
                    .align(rng.below(4))
                    .out(rng.range(0, n + 32)),
            );
        }
    }
    let lines = diff_cases_in_children("cfg36b", &cases);
    summarize("cfg36b", &lines);
}

/// Argument-space sweep: `in_bytes` / `out_bytes` values that do not match the
/// buffer, including negatives, zero and NULL `out`.
#[test]
fn cfg36c_argument_sweep() {
    let mut rng = Rng::new(0x7777_1111_2222_3333);
    let mut cases = Vec::new();
    for _ in 0..2000 {
        let n = rng.range(0, 40);
        let plain = rng.repetitive(n);
        let s = deflate_ref(&plain, 6);
        let ib = match rng.below(6) {
            0 => 0i32,
            1 => -(rng.range(1, 8) as i32),
            2 => (s.len() as i32).saturating_sub(rng.range(1, 4) as i32),
            3 => s.len() as i32 + rng.range(1, 4) as i32,
            _ => s.len() as i32,
        };
        let ob = match rng.below(6) {
            0 => 0i32,
            1 => -(rng.range(1, 64) as i32),
            2 => rng.range(1, n.max(1)) as i32,
            3 => 1 << 20,
            _ => (n + rng.below(8)) as i32,
        };
        let alloc = if ob > 0 { (ob as usize).min(1 << 20) } else { 64 };
        let mut c = Case::new("cfg36c", &s)
            .align(rng.below(4))
            .in_bytes(ib)
            .out_raw(ob, alloc);
        if rng.below(12) == 0 && null_out_allowed() {
            c = c.null_out();
        }
        cases.push(c);
    }
    let lines = diff_cases_in_children("cfg36c", &cases);
    summarize("cfg36c", &lines);
}

/// Out-of-range "enum"-like values crossing the FFI boundary: the exported
/// tables are the only enums the API has, and they accept any `uint8_t`.
#[test]
fn cfg36d_poisoned_table_sweep() {
    let mut rng = Rng::new(0x9999_8888_7777_6666);
    let mut cases = Vec::new();
    let base = {
        let mut w = BitWriter::new();
        let data: Vec<u8> = (0..32u8).collect();
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match(10, 4));
        write_fixed_block(&mut w, true, &toks);
        w.finish()
    };
    for _ in 0..1200 {
        let mut c = Case::new("cfg36d", &base).align(rng.below(4)).out(256);
        match rng.below(4) {
            0 => c = c.poke_fixed(rng.below(320), rng.byte()),
            1 => c = c.poke_len_extra(rng.below(31), rng.byte()),
            2 => c = c.poke_dist_extra(rng.below(32), rng.byte()),
            _ => {
                c = c
                    .poke_fixed(rng.below(320), rng.byte())
                    .poke_len_extra(rng.below(31), rng.byte())
                    .poke_dist_extra(rng.below(32), rng.byte());
            }
        }
        cases.push(c);
    }
    let lines = diff_cases_in_children("cfg36d", &cases);
    summarize("cfg36d", &lines);
}

// ---------------------------------------------------------------------------
// Targeted: `cp_dynamic`'s unchecked `lens[]` writes
// ---------------------------------------------------------------------------
//
// `cp_dynamic` declares `uint8_t lens[288 + 32]` and fills it from the
// code-length stream *with no bound check*: symbols 16/17/18 advance `n` by up
// to 138 at a time, so `n` can run well past 319 and the writes land on the
// neighbouring locals of `cp_dynamic`'s own stack frame -- including `nlit`,
// `ndst` and the loop counter `n` itself, which is what makes some streams loop
// forever.  The Rust translation reproduces gcc -O0's frame layout to match
// that behaviour, so this is the single most delicate part of the translation
// and gets its own dense sweep.

fn overshoot_case(rng: &mut Rng, label: &str) -> Case {
    // A complete 2-bit code over the code-length symbols {0, 16, 17, 18}, all of
    // which live in the first four permutation slots, so HCLEN = 4 suffices.
    let mut cl_lens = [0u8; 19];
    for sym in [0usize, 16, 17, 18] {
        cl_lens[sym] = 2;
    }
    let nlit = rng.range(257, 288);
    let ndst = rng.range(1, 32);
    let mut w = BitWriter::new();
    w.bits(1, 1);
    w.bits(2, 2);
    w.bits((nlit - 257) as u32, 5);
    w.bits((ndst - 1) as u32, 5);
    w.bits(0, 4); // HCLEN = 4
    let perm = pristine_tables().permutation_order;
    for i in 0..4 {
        w.bits(cl_lens[perm[i] as usize] as u32, 3);
    }
    let clc = canonical(&cl_lens);
    // Emit far more code lengths than `nlit + ndst` asks for, using long runs so
    // that `n` overshoots the end of `lens[]` by a controlled amount.
    let items = rng.range(1, 40);
    for _ in 0..items {
        let sym = [0usize, 16, 17, 18][rng.below(4)];
        let (c, l) = clc[sym];
        w.huff(c, l as u32);
        match sym {
            16 => w.bits(rng.next_u32() & 3, 2),
            17 => w.bits(rng.next_u32() & 7, 3),
            18 => w.bits(rng.next_u32() & 127, 7),
            _ => {}
        }
    }
    // trailing bits so the reader does not run dry immediately
    for _ in 0..rng.range(0, 24) {
        w.bits(rng.byte() as u32, 8);
    }
    let s = w.finish();
    Case::new(label, &s).align(rng.below(4)).out(rng.range(0, 512))
}

#[test]
fn cfg37_dynamic_frame_overflow_sweep() {
    let mut rng = Rng::new(0xF00D_1234_5678_9ABC);
    let mut cases = Vec::new();
    for i in 0..4000 {
        cases.push(overshoot_case(&mut rng, &format!("cfg37/{i}")));
    }
    let lines = diff_cases_in_children("cfg37", &cases);
    summarize("cfg37", &lines);
    // The point of the row is that the overshoot really is exercised: some of
    // these must hang (the clobbered `n` case) and some must abort.
    let hangs = lines.iter().filter(|l| l.contains("HANG")).count();
    assert!(
        hangs > 0,
        "no case reached the runaway `lens[]` write; the sweep is not exercising \
         cp_dynamic's frame overflow"
    );
}

/// Same shape, but with code-length alphabets that also use the *literal*
/// lengths 1..15, so `cp_build` gets partially-clobbered length vectors.
#[test]
fn cfg38_dynamic_mixed_codelength_sweep() {
    let mut rng = Rng::new(0xBEEF_0F0F_1234_5678);
    let mut cases = Vec::new();
    for i in 0..4000 {
        // full HCLEN so every code-length symbol is available
        let used: Vec<usize> = (0..19).collect();
        let cl_lens = cl_lengths_for(&used);
        let nlit = rng.range(257, 288);
        let ndst = rng.range(1, 32);
        let mut w = BitWriter::new();
        w.bits(1, 1);
        w.bits(2, 2);
        w.bits((nlit - 257) as u32, 5);
        w.bits((ndst - 1) as u32, 5);
        w.bits(15, 4); // HCLEN = 19
        let perm = pristine_tables().permutation_order;
        for k in 0..19 {
            w.bits(cl_lens[perm[k] as usize] as u32, 3);
        }
        let clc = canonical(&cl_lens);
        for _ in 0..rng.range(1, 90) {
            let sym = rng.below(19);
            let (c, l) = clc[sym];
            w.huff(c, l as u32);
            match sym {
                16 => w.bits(rng.next_u32() & 3, 2),
                17 => w.bits(rng.next_u32() & 7, 3),
                18 => w.bits(rng.next_u32() & 127, 7),
                _ => {}
            }
        }
        for _ in 0..rng.range(0, 32) {
            w.bits(rng.byte() as u32, 8);
        }
        let s = w.finish();
        cases.push(
            Case::new(&format!("cfg38/{i}"), &s)
                .align(rng.below(4))
                .out(rng.range(0, 512)),
        );
    }
    let lines = diff_cases_in_children("cfg38", &cases);
    summarize("cfg38", &lines);
}

/// Stored blocks whose `LEN` deliberately exceeds `out_bytes`, i.e. the
/// unchecked `memcpy` in `cp_stored`.  Both libraries must write the *same*
/// bytes past the end of the caller's buffer (the harness gives them
/// `OUT_SLACK` bytes of room so the overrun is observable, see common/mod.rs).
#[test]
fn cfg39_stored_overrun_sweep() {
    let mut rng = Rng::new(0x0BAD_C0DE_1234_5678);
    let mut cases = Vec::new();
    for i in 0..2000 {
        let payload_len = rng.range(0, 400);
        let payload = rng.bytes(payload_len);
        // LEN >= remaining bytes is required by the C's own check
        let len = payload_len + rng.below(4096);
        let mut w = BitWriter::new();
        write_stored_block_lens(&mut w, true, len as u16, !(len as u16), &payload);
        let s = w.finish();
        let out = rng.below(payload_len + 8);
        cases.push(
            Case::new(&format!("cfg39/{i}"), &s)
                .align(rng.below(4))
                .out(out),
        );
    }
    let lines = diff_cases_in_children("cfg39", &cases);
    summarize("cfg39", &lines);
    let oks = lines.iter().filter(|l| l.contains(" OK ")).count();
    assert!(oks > 100, "expected many accepted (over-copying) stored blocks");
}
