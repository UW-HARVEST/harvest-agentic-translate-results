//! Phase B - valid path differential tests.
//!
//! One test per row of `CONFIGS.md`; every row drives BOTH shared libraries
//! through their exported `process_strings` symbol with many randomised inputs
//! (fixed seeds) and asserts the results are identical.

mod common;

use common::*;

const N: usize = 200;

fn args(region: &mut Region, input_len: usize, ref_len: usize, operation: i32, flags: u32) -> Args {
    Args {
        input: region.input_ptr(),
        input_len,
        reference: region.ref_ptr(),
        ref_len,
        operation,
        flags,
    }
}

// ---------------------------------------------------------------------------
// row 1-5: operation 0 (validate_token)
// ---------------------------------------------------------------------------

#[test]
fn cfg_op0_terminated() {
    let rng = Rng::new(1);
    let mut region = Region::new();
    for _ in 0..N {
        let a = rand_bytes(&rng, rng.below(18), true);
        let b = rand_bytes(&rng, rng.below(18), true);
        region.place(&a, &b);
        let a_len = a.len();
        let b_len = b.len();
        let ar = args(&mut region, a_len, b_len, 0, rng.next_u64() as u32);
        diff(ar, "row1 op0 terminated");
    }
}

#[test]
fn cfg_op0_equal_strings() {
    let rng = Rng::new(2);
    let mut region = Region::new();
    for len in 0..17usize {
        for _ in 0..12 {
            let a = rand_bytes(&rng, len, true);
            region.place(&a, &a);
            let l = a.len();
            let ar = args(&mut region, l, l, 0, 0);
            let r = diff(ar, "row2 op0 equal");
            assert_eq!(r, 1, "equal NUL terminated strings must validate");
        }
    }
}

#[test]
fn cfg_op0_valid_ok_literals() {
    let rng = Rng::new(3);
    let mut region = Region::new();
    for lit in [b"VALID".as_slice(), b"OK", b"valid", b"ok", b"VALIDX", b"O"] {
        for _ in 0..20 {
            let mut a = lit.to_vec();
            a.push(0);
            let b = rand_bytes(&rng, rng.below(8), true);
            region.place(&a, &b);
            let (la, lb) = (a.len(), b.len());
            let ar = args(&mut region, la, lb, 0, 0);
            diff(ar, "row3 op0 literals");
        }
    }
}

#[test]
fn cfg_op0_unterminated_input() {
    let rng = Rng::new(4);
    let mut region = Region::new();
    for _ in 0..N {
        let a = rand_bytes(&rng, rng.below(24), false);
        let b = rand_bytes(&rng, rng.below(24), true);
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let ar = args(&mut region, la, lb, 0, 0);
        diff(ar, "row4 op0 unterminated input");
    }
    // and with a long non-zero tail after the input
    for _ in 0..N {
        let a = rand_bytes(&rng, 1 + rng.below(20), false);
        let b = rand_bytes(&rng, 1 + rng.below(20), true);
        region.place_dense(&a, &b, 300);
        let (la, lb) = (a.len(), b.len());
        let ar = args(&mut region, la, lb, 0, 0);
        diff(ar, "row4 op0 unterminated input, dense tail");
    }
}

#[test]
fn cfg_op0_unterminated_ref() {
    let rng = Rng::new(5);
    let mut region = Region::new();
    for _ in 0..N {
        let a = rand_bytes(&rng, rng.below(24), true);
        let b = rand_bytes(&rng, rng.below(24), false);
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let ar = args(&mut region, la, lb, 0, 0);
        diff(ar, "row5 op0 unterminated ref");
    }
    for _ in 0..N {
        // identical prefixes so the comparison really runs into the junk
        let common_len = 1 + rng.below(10);
        let a = rand_bytes(&rng, common_len, false);
        region.place(&a, &a);
        let l = a.len();
        let ar = args(&mut region, l, l, 0, 0);
        diff(ar, "row5 op0 both unterminated, equal data");
    }
}

// ---------------------------------------------------------------------------
// row 6-11: operation 1 (parse_command)
// ---------------------------------------------------------------------------

#[test]
fn cfg_op1_exact_commands() {
    let rng = Rng::new(6);
    let mut region = Region::new();
    for cmd in COMMANDS {
        for extra in [0usize, 1, 2, 5] {
            let mut a = cmd.to_vec();
            a.push(0);
            let b = rand_bytes(&rng, rng.below(6), true);
            region.place(&a, &b);
            let l = cmd.len() + extra;
            let lb = b.len();
            let ar = args(&mut region, l, lb, 1, 0);
            diff(ar, "row6 op1 exact command");
        }
    }
    // lower case / partial commands must not match
    for cmd in [b"start".as_slice(), b"STAR", b"STOPP", b"RESUM", b"RESETS"] {
        let mut a = cmd.to_vec();
        a.push(0);
        region.place(&a, &[]);
        let l = a.len();
        let ar = args(&mut region, l, 0, 1, 0);
        diff(ar, "row6 op1 near-miss command");
    }
}

#[test]
fn cfg_op1_space_terminated() {
    let rng = Rng::new(7);
    let mut region = Region::new();
    for cmd in COMMANDS {
        for tail in [b" ".as_slice(), b" x", b"  ", b"\0 ", b"x ", b"\t"] {
            let mut a = cmd.to_vec();
            a.extend_from_slice(tail);
            a.push(0);
            let b = rand_bytes(&rng, rng.below(4), true);
            region.place(&a, &b);
            let (la, lb) = (a.len(), b.len());
            let ar = args(&mut region, la, lb, 1, 0);
            diff(ar, "row7 op1 space terminated");
        }
    }
}

#[test]
fn cfg_op1_short_buf_size() {
    let rng = Rng::new(8);
    let mut region = Region::new();
    for cmd in COMMANDS {
        for len in 0..cmd.len() {
            let mut a = cmd.to_vec();
            a.push(0);
            let b = rand_bytes(&rng, rng.below(4), true);
            region.place(&a, &b);
            let lb = b.len();
            let ar = args(&mut region, len, lb, 1, 0);
            diff(ar, "row8 op1 buf_size < cmd_len");
        }
    }
}

#[test]
fn cfg_op1_len_boundaries() {
    let rng = Rng::new(9);
    let mut region = Region::new();
    for len in [0usize, 1, 2, 3, 4, 5, 6, 7] {
        for _ in 0..20 {
            let terminated = rng.bool();
            let a = if rng.bool() {
                let mut v = rng.pick(&LITERALS).to_vec();
                if terminated {
                    v.push(0);
                }
                v
            } else {
                rand_bytes(&rng, len, terminated)
            };
            let b = rand_bytes(&rng, rng.below(4), true);
            region.place(&a, &b);
            let lb = b.len();
            let ar = args(&mut region, len, lb, 1, 0);
            diff(ar, "row9 op1 length boundaries");
        }
    }
}

#[test]
fn cfg_op1_admin() {
    let mut region = Region::new();
    for (data, len) in [
        (b"ADMIN\0".as_slice(), 5usize),
        (b"ADMIN\0", 6),
        (b"ADMIN\0", 0),
        (b"ADMIN ", 5),
        (b"admin\0", 5),
        (b"ADMI\0", 5),
        (b"ADMINX\0", 6),
    ] {
        region.place(data, &[]);
        let ar = args(&mut region, len, 0, 1, 0);
        diff(ar, "row10 op1 ADMIN");
    }
}

#[test]
fn cfg_op1_len_vs_data() {
    let rng = Rng::new(11);
    let mut region = Region::new();
    for _ in 0..N {
        let mut a = rng.pick(&LITERALS).to_vec();
        if rng.bool() {
            a.push(0);
        }
        let claimed = rng.below(12);
        let b = rand_bytes(&rng, rng.below(4), true);
        region.place(&a, &b);
        let lb = b.len();
        let ar = args(&mut region, claimed, lb, 1, 0);
        diff(ar, "row11 op1 len vs data");
    }
}

// ---------------------------------------------------------------------------
// row 12-18: operation 2 (compare_prefix)
// ---------------------------------------------------------------------------

#[test]
fn cfg_op2_loose_prefix() {
    let rng = Rng::new(12);
    let mut region = Region::new();
    for _ in 0..N {
        let plen = rng.below(12);
        let prefix = rand_bytes(&rng, plen, true);
        let mut s = prefix[..plen].to_vec();
        if rng.bool() {
            s.extend(rand_bytes(&rng, rng.below(8), false));
        } else {
            s = rand_bytes(&rng, plen + rng.below(4), false);
        }
        s.push(0);
        region.place(&s, &prefix);
        let (ls, lp) = (s.len(), prefix.len());
        let ar = args(&mut region, ls, lp, 2, 0);
        diff(ar, "row12 op2 loose prefix");
    }
}

#[test]
fn cfg_op2_loose_empty_prefix() {
    let rng = Rng::new(13);
    let mut region = Region::new();
    for _ in 0..40 {
        let s = rand_bytes(&rng, rng.below(10), true);
        region.place(&s, &[0u8]);
        let ls = s.len();
        let ar = args(&mut region, ls, 1, 2, 0);
        let r = diff(ar, "row13 op2 empty prefix");
        assert_eq!(r, 1, "strncmp with n = 0 always matches");
    }
}

#[test]
fn cfg_op2_exact_equal() {
    let rng = Rng::new(14);
    let mut region = Region::new();
    for _ in 0..N {
        let p = rand_bytes(&rng, rng.below(20), true);
        region.place(&p, &p);
        let l = p.len();
        let ar = args(&mut region, l, l, 2, 1 | (rng.next_u64() as u32 & 0xFFFF_FFFC));
        let r = diff(ar, "row14 op2 exact equal");
        assert_eq!(r, 1);
    }
}

#[test]
fn cfg_op2_exact_variations() {
    let rng = Rng::new(15);
    let mut region = Region::new();
    for (i, var) in VARIATIONS.iter().enumerate() {
        for plen in [0usize, 1, 3, 10, 20] {
            let prefix = rand_bytes(&rng, plen, true);
            let mut s = prefix[..plen].to_vec();
            s.extend_from_slice(var);
            s.push(0);
            region.place(&s, &prefix);
            let (ls, lp) = (s.len(), prefix.len());
            let ar = args(&mut region, ls, lp, 2, 1);
            let r = diff(ar, "row15 op2 exact variation");
            assert_eq!(r, 2 + i as i32, "variation {i} must be reported");
        }
    }
}

#[test]
fn cfg_op2_exact_truncation() {
    let rng = Rng::new(16);
    let mut region = Region::new();
    for plen in 50..75usize {
        for var in VARIATIONS.iter() {
            let prefix = rand_bytes(&rng, plen, true);
            // str = prefix + variation, truncated the way the C code truncates
            let mut s = prefix[..plen].to_vec();
            s.extend_from_slice(var);
            s.truncate(63);
            s.push(0);
            region.place(&s, &prefix);
            let (ls, lp) = (s.len(), prefix.len());
            let ar = args(&mut region, ls, lp, 2, 1);
            diff(ar, "row16 op2 exact truncation");
            // and the untruncated string
            let mut s2 = prefix[..plen].to_vec();
            s2.extend_from_slice(var);
            s2.push(0);
            region.place(&s2, &prefix);
            let (ls2, lp2) = (s2.len(), prefix.len());
            let ar = args(&mut region, ls2, lp2, 2, 1);
            diff(ar, "row16 op2 exact truncation (long)");
        }
    }
}

#[test]
fn cfg_op2_flag_bit1_ignored() {
    let rng = Rng::new(17);
    let mut region = Region::new();
    for flags in [0u32, 1, 2, 3, 0xFFFF_FFFE, 0xFFFF_FFFF, 0x8000_0001] {
        for _ in 0..30 {
            let p = rand_bytes(&rng, rng.below(12), true);
            let mut s = p[..p.len() - 1].to_vec();
            if rng.bool() {
                s.extend_from_slice(rng.pick(&VARIATIONS));
            }
            s.push(0);
            region.place(&s, &p);
            let (ls, lp) = (s.len(), p.len());
            let ar = args(&mut region, ls, lp, 2, flags);
            diff(ar, "row17 op2 flag bits");
        }
    }
}

#[test]
fn cfg_op2_unterminated() {
    let rng = Rng::new(18);
    let mut region = Region::new();
    for exact in [0u32, 1u32] {
        for _ in 0..N {
            let a = rand_bytes(&rng, rng.below(20), rng.bool());
            let b = rand_bytes(&rng, rng.below(20), rng.bool());
            region.place(&a, &b);
            let (la, lb) = (a.len(), b.len());
            let ar = args(&mut region, la, lb, 2, exact);
            diff(ar, "row18 op2 unterminated");
        }
        for _ in 0..N {
            let n = 1 + rng.below(10);
            let a = rand_bytes(&rng, n, false);
            region.place(&a, &a);
            let l = a.len();
            let ar = args(&mut region, l, l, 2, exact);
            diff(ar, "row18 op2 unterminated equal data");
        }
        for _ in 0..50 {
            let n = 1 + rng.below(10);
            let a = rand_bytes(&rng, n, false);
            region.place_dense(&a, &a, 200);
            let l = a.len();
            let ar = args(&mut region, l, l, 2, exact);
            diff(ar, "row18 op2 unterminated dense tail");
        }
    }
}

// ---------------------------------------------------------------------------
// row 19-25: operation 3 (find_delimiter)
// ---------------------------------------------------------------------------

#[test]
fn cfg_op3_delim_found() {
    let rng = Rng::new(19);
    let mut region = Region::new();
    for _ in 0..N {
        let len = 1 + rng.below(24);
        let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        let delim = rng.byte() | 1;
        let pos = rng.below(len);
        data[pos] = delim;
        let reference = vec![delim, rng.byte(), 0];
        region.place(&data, &reference);
        let (ld, lr) = (data.len(), reference.len());
        let ar = args(&mut region, ld, lr, 3, 0);
        diff(ar, "row19 op3 delimiter found");
    }
}

#[test]
fn cfg_op3_default_colon() {
    let rng = Rng::new(20);
    let mut region = Region::new();
    for _ in 0..N {
        let len = rng.below(20);
        let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        if rng.bool() && len > 0 {
            let p = rng.below(len);
            data[p] = b':';
        }
        let reference = rand_bytes(&rng, rng.below(4), true);
        region.place(&data, &reference);
        let ld = data.len();
        // ref_len == 0 -> the delimiter falls back to ':'
        let ar = args(&mut region, ld, 0, 3, 0);
        diff(ar, "row20 op3 default colon");
    }
}

#[test]
fn cfg_op3_nul_before_delim() {
    let rng = Rng::new(21);
    let mut region = Region::new();
    for _ in 0..N {
        let len = 2 + rng.below(20);
        let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        let nul = rng.below(len);
        data[nul] = 0;
        let delim = rng.pick(&[b'|', b':', b'x']);
        if nul + 1 < len {
            data[len - 1] = delim;
        }
        let reference = vec![delim, 0];
        region.place(&data, &reference);
        let (ld, lr) = (data.len(), reference.len());
        let ar = args(&mut region, ld, lr, 3, 0);
        diff(ar, "row21 op3 NUL before delimiter");
    }
}

#[test]
fn cfg_op3_special_patterns() {
    let mut region = Region::new();
    for (data, delim) in [
        (b"NONE\0".as_slice(), b'|'),
        (b"NONE\0", b':'),
        (b"EMPTY\0", b':'),
        (b"EMPTY\0", b'|'),
        (b"NONE", b'|'),
        (b"EMPTY", b':'),
        (b"NONEX\0", b'|'),
        (b"EMPTYX\0", b':'),
        (b"NON\0", b'|'),
        (b"EMPT\0", b':'),
    ] {
        for len in [0usize, 1, 4, 5, 6] {
            let reference = [delim, 0u8];
            region.place(data, &reference);
            let ar = args(&mut region, len, 2, 3, 0);
            diff(ar, "row22 op3 special patterns");
        }
    }
}

#[test]
fn cfg_op3_nul_delimiter() {
    let rng = Rng::new(23);
    let mut region = Region::new();
    for _ in 0..N {
        let len = rng.below(20);
        let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
        if rng.bool() && len > 0 {
            let p = rng.below(len);
            data[p] = 0;
        }
        let reference = [0u8, rng.byte()];
        region.place(&data, &reference);
        let ld = data.len();
        let ar = args(&mut region, ld, 2, 3, 0);
        diff(ar, "row23 op3 NUL delimiter");
    }
}

#[test]
fn cfg_op3_len_boundaries() {
    let rng = Rng::new(24);
    let mut region = Region::new();
    for len in [1usize, 2, 1023, 1024] {
        for place in 0..4 {
            let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
            let delim = b'%';
            match place {
                0 => data[0] = delim,
                1 => data[len - 1] = delim,
                2 => {
                    if len > 2 {
                        data[len / 2] = delim;
                    }
                }
                _ => {}
            }
            let reference = [delim, 0u8];
            region.place(&data, &reference);
            let ld = data.len();
            let ar = args(&mut region, ld, 2, 3, 0);
            diff(ar, "row24 op3 length boundaries");
        }
    }
}

#[test]
fn cfg_op3_high_bit_delim() {
    let rng = Rng::new(25);
    let mut region = Region::new();
    for delim in [0x80u8, 0xFF, 0xC3, 0x7F] {
        for _ in 0..30 {
            let len = 1 + rng.below(16);
            let mut data: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
            if rng.bool() {
                let p = rng.below(len);
                data[p] = delim;
            }
            let reference = [delim, 0u8];
            region.place(&data, &reference);
            let ld = data.len();
            let ar = args(&mut region, ld, 2, 3, 0);
            diff(ar, "row25 op3 high bit delimiter");
        }
    }
}

// ---------------------------------------------------------------------------
// row 26-35: operation 4 (match_pattern)
// ---------------------------------------------------------------------------

#[test]
fn cfg_op4_cs_exact() {
    let rng = Rng::new(26);
    let mut region = Region::new();
    for _ in 0..N {
        let p = rand_bytes(&rng, rng.below(16), true);
        region.place(&p, &p);
        let l = p.len();
        let ar = args(&mut region, l, l, 4, 2);
        let r = diff_forked(ar, "row26 op4 cs exact");
        assert_eq!(r, Outcome::Value(1));
    }
}

#[test]
fn cfg_op4_cs_wildcards() {
    let rng = Rng::new(27);
    let mut region = Region::new();
    for shape in 0..3usize {
        for plen in [1usize, 2, 5, 20] {
            let pattern = rand_bytes(&rng, plen, true);
            let body = &pattern[..plen];
            let mut text = Vec::new();
            if shape == 0 || shape == 2 {
                text.push(b'*');
            }
            text.extend_from_slice(body);
            if shape == 0 || shape == 1 {
                text.push(b'*');
            }
            text.push(0);
            region.place(&text, &pattern);
            let (lt, lp) = (text.len(), pattern.len());
            let ar = args(&mut region, lt, lp, 4, 2);
            let r = diff_forked(ar, "row27 op4 cs wildcards");
            let expect = match shape {
                0 => 2,
                1 => 3,
                _ => 4,
            };
            assert_eq!(r, Outcome::Value(expect), "shape {shape}");
        }
    }
}

#[test]
fn cfg_op4_cs_substring() {
    let rng = Rng::new(28);
    let mut region = Region::new();
    for _ in 0..N {
        let plen = 1 + rng.below(6);
        let pattern = rand_bytes(&rng, plen, true);
        let body = pattern[..plen].to_vec();
        let lead = rng.below(8);
        let mut text: Vec<u8> = (0..lead).map(|_| (rng.byte() | 1) & 0x7F).collect();
        // make sure the pattern does not occur earlier by accident: the check
        // is done by the reference implementation anyway, both must agree
        text.extend_from_slice(&body);
        text.extend(rand_bytes(&rng, rng.below(5), false));
        text.push(0);
        region.place(&text, &pattern);
        let (lt, lp) = (text.len(), pattern.len());
        let ar = args(&mut region, lt, lp, 4, 2);
        diff_forked(ar, "row28 op4 cs substring");
    }
}

#[test]
fn cfg_op4_cs_empty_pattern() {
    let rng = Rng::new(29);
    let mut region = Region::new();
    for _ in 0..40 {
        let text = rand_bytes(&rng, rng.below(10), true);
        region.place(&text, &[0u8]);
        let lt = text.len();
        let ar = args(&mut region, lt, 1, 4, 2);
        diff_forked(ar, "row29 op4 cs empty pattern");
    }
}

#[test]
fn cfg_op4_cs_wildcard_truncation() {
    let rng = Rng::new(30);
    let mut region = Region::new();
    for plen in 58..70usize {
        let pattern = rand_bytes(&rng, plen, true);
        let body = pattern[..plen].to_vec();
        for shape in 0..3usize {
            let mut text = Vec::new();
            if shape == 0 || shape == 2 {
                text.push(b'*');
            }
            text.extend_from_slice(&body);
            if shape == 0 || shape == 1 {
                text.push(b'*');
            }
            text.truncate(63);
            text.push(0);
            region.place(&text, &pattern);
            let (lt, lp) = (text.len(), pattern.len());
            let ar = args(&mut region, lt, lp, 4, 2);
            diff_forked(ar, "row30 op4 cs wildcard truncation");
        }
        // exact pattern as text, plus the pattern truncated to 63 chars
        let mut t = body.clone();
        t.truncate(63);
        t.push(0);
        region.place(&t, &pattern);
        let (lt, lp) = (t.len(), pattern.len());
        let ar = args(&mut region, lt, lp, 4, 2);
        diff_forked(ar, "row30 op4 cs truncated text");
    }
}

#[test]
fn cfg_op4_ci_exact() {
    let rng = Rng::new(31);
    let mut region = Region::new();
    for _ in 0..N {
        let p = rand_bytes(&rng, rng.below(16), true);
        region.place(&p, &p);
        let l = p.len();
        let ar = args(&mut region, l, l, 4, 0);
        let r = diff(ar, "row31 op4 ci exact");
        assert_eq!(r, 1);
    }
}

#[test]
fn cfg_op4_ci_prefix() {
    let rng = Rng::new(32);
    let mut region = Region::new();
    for _ in 0..N {
        let plen = rng.below(10);
        let pattern = rand_bytes(&rng, plen, true);
        let mut text = pattern[..plen].to_vec();
        text.extend(rand_bytes(&rng, 1 + rng.below(6), false));
        text.push(0);
        region.place(&text, &pattern);
        let (lt, lp) = (text.len(), pattern.len());
        let ar = args(&mut region, lt, lp, 4, 0);
        let r = diff(ar, "row32 op4 ci prefix");
        assert_eq!(r, 5, "longer text with matching prefix");
    }
}

#[test]
fn cfg_op4_ci_equal_len() {
    let rng = Rng::new(33);
    let mut region = Region::new();
    let boundary = [b'@', b'[', b'`', b'{', b'A', b'Z', b'a', b'z', 0x80, 0xFF];
    for _ in 0..N {
        let len = 1 + rng.below(12);
        let mut a: Vec<u8> = (0..len)
            .map(|_| {
                if rng.bool() {
                    rng.pick(&boundary)
                } else {
                    rng.byte() | 1
                }
            })
            .collect();
        let mut b = a.clone();
        // flip the case of some letters
        for i in 0..len {
            if rng.bool() {
                if b[i].is_ascii_lowercase() {
                    b[i] -= 32;
                } else if b[i].is_ascii_uppercase() {
                    b[i] += 32;
                }
            }
        }
        if rng.below(4) == 0 {
            let i = rng.below(len);
            a[i] = a[i].wrapping_add(1) | 1;
        }
        a.push(0);
        b.push(0);
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let ar = args(&mut region, la, lb, 4, 0);
        diff(ar, "row33 op4 ci equal length");
    }
}

#[test]
fn cfg_op4_ci_no_match() {
    let rng = Rng::new(34);
    let mut region = Region::new();
    for _ in 0..N {
        let len = 1 + rng.below(12);
        let a: Vec<u8> = (0..len).map(|_| b'a' + (rng.byte() % 26)).collect();
        let b: Vec<u8> = (0..len).map(|_| b'0' + (rng.byte() % 10)).collect();
        let mut a = a;
        let mut b = b;
        a.push(0);
        b.push(0);
        region.place(&a, &b);
        let (la, lb) = (a.len(), b.len());
        let ar = args(&mut region, la, lb, 4, 0);
        let r = diff(ar, "row34 op4 ci no match");
        assert_eq!(r, 0);
    }
}

#[test]
fn cfg_op4_unterminated() {
    let rng = Rng::new(35);
    let mut region = Region::new();
    for flags in [0u32, 2u32] {
        for _ in 0..N {
            let a = rand_bytes(&rng, rng.below(20), rng.bool());
            let b = rand_bytes(&rng, rng.below(20), rng.bool());
            region.place(&a, &b);
            let (la, lb) = (a.len(), b.len());
            let ar = args(&mut region, la, lb, 4, flags);
            diff_forked(ar, "row35 op4 unterminated");
        }
        for _ in 0..60 {
            let n = 1 + rng.below(10);
            let a = rand_bytes(&rng, n, false);
            region.place(&a, &a);
            let l = a.len();
            let ar = args(&mut region, l, l, 4, flags);
            diff_forked(ar, "row35 op4 unterminated equal data");
        }
    }
}

// ---------------------------------------------------------------------------
// row 36-39: cross products
// ---------------------------------------------------------------------------

#[test]
fn cfg_flags_cross_product() {
    let rng = Rng::new(36);
    let mut region = Region::new();
    for op in [0i32, 1, 2, 3, 4] {
        for flags in [0u32, 1, 2, 3, 4, 0x8000_0000, 0xFFFF_FFFF] {
            for _ in 0..12 {
                let a = rand_bytes(&rng, rng.below(12), rng.bool());
                let b = rand_bytes(&rng, rng.below(12), true);
                region.place(&a, &b);
                let (la, lb) = (a.len(), b.len());
                let ar = args(&mut region, la, lb, op, flags);
                diff_auto(ar, "row36 flags cross product");
            }
        }
    }
}

#[test]
fn cfg_zero_lengths_all_ops() {
    let mut region = Region::new();
    for op in [0i32, 1, 2, 3, 4] {
        for flags in [0u32, 1, 2, 3] {
            // both buffers empty but non-NULL: the C code reads the junk that
            // follows them
            region.place(&[], &[]);
            let ar = args(&mut region, 0, 0, op, flags);
            diff_auto(ar, "row37 zero lengths");
            // empty NUL terminated strings
            region.place(&[0u8], &[0u8]);
            let ar = args(&mut region, 1, 1, op, flags);
            diff_auto(ar, "row37 empty strings");
        }
    }
}

#[test]
fn cfg_random_binary_all_ops() {
    let rng = Rng::new(38);
    let mut region = Region::new();
    for _ in 0..800 {
        let op = rng.pick(&[0i32, 1, 2, 3, 4]);
        let flags = rng.pick(&[0u32, 1, 2, 3, 0xFFFF_FFFF]);
        let la = rng.below(41);
        let lb = rng.below(41);
        let a: Vec<u8> = (0..la).map(|_| rng.byte()).collect();
        let b: Vec<u8> = (0..lb).map(|_| rng.byte()).collect();
        region.place(&a, &b);
        let ar = args(&mut region, la, lb, op, flags);
        diff_auto(ar, "row38 random binary");
    }
}

#[test]
fn cfg_long_buffers_all_ops() {
    let rng = Rng::new(39);
    let mut region = Region::new();
    for op in [0i32, 1, 2, 3, 4] {
        for flags in [0u32, 1, 2, 3] {
            for len in [1023usize, 1024] {
                for terminated in [false, true] {
                    let mut a: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
                    let mut b: Vec<u8> = (0..len).map(|_| rng.byte() | 1).collect();
                    if terminated {
                        a[len - 1] = 0;
                        b[len - 1] = 0;
                    }
                    region.place(&a, &b);
                    let ar = args(&mut region, len, len, op, flags);
                    diff_auto(ar, "row39 long buffers");
                    // identical long buffers
                    region.place(&a, &a);
                    let ar = args(&mut region, len, len, op, flags);
                    diff_auto(ar, "row39 identical long buffers");
                }
            }
        }
    }
}
