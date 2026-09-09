//! Phase B — CONFIGS.md sections G (pack) and H (unpack).
//!
//! `json_vpack_ex` / `json_vunpack_ex` are the low-level entry points; because a
//! `va_list` cannot portably be synthesised from Rust, they are driven through
//! `json_pack_ex` / `json_unpack_ex` (which forward straight to them) and
//! through the flagless `json_pack` / `json_unpack` wrappers.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};

const R_ORD: usize = JSON_ENCODE_ANY;
const R_SORT: usize = JSON_ENCODE_ANY | JSON_SORT_KEYS;

/// Compare a pack result: the value (or NULL) plus the whole `json_error_t`.
fn cmp_pack(cj: json_ptr, rj: json_ptr, ce: &json_error_t, re: &json_error_t, ctx: &str) {
    unsafe {
        let p = pair();
        assert_eq!(
            cj.is_null(),
            rj.is_null(),
            "{ctx}: NULL-ness (C err {:?} / RUST err {:?})",
            String::from_utf8_lossy(&ce.snap().text),
            String::from_utf8_lossy(&re.snap().text)
        );
        assert_eq!(ce.snap(), re.snap(), "{ctx}: json_error_t");
        if !cj.is_null() {
            assert_eq!(typeof_json(cj), typeof_json(rj), "{ctx}: root type");
            for f in [R_ORD, R_SORT] {
                let cd = p.c.dumps(cj, f);
                let rd = p.r.dumps(rj, f);
                assert_eq!(
                    cd,
                    rd,
                    "{ctx}: dumps(0x{f:x}):\n  C   = {}\n  RUST= {}",
                    show(&cd),
                    show(&rd)
                );
            }
        }
        p.c.json_decref(cj);
        p.r.json_decref(rj);
    }
}

/// Every pack flag word worth trying. `pack` never reads `flags`, so all of
/// these must produce identical output — which is itself worth asserting.
const PACK_FLAGS: [usize; 5] = [0, JSON_VALIDATE_ONLY, JSON_STRICT, 3, usize::MAX];

// ===========================================================================
// G — pack
// ===========================================================================

/// G1, G13, G14: every no-argument / single-argument format char, and the
/// cosmetic separators, under every flag word.
#[test]
fn g1_g13_g14_simple_formats() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();

        // No varargs at all.
        for fmt in [
            "n", "[]", "{}", "[n]", "[n,n]", "{s:n}", " n ", "\tn\n", "[ n , n ]",
            "{ s : n , s : n }", "[[[n]]]", "{s:{s:{s:n}}}", "[,n]", "[n,]", "[:n]",
        ] {
            let f = cs(fmt);
            for &fl in &PACK_FLAGS {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let ka = cs("a");
                let kb = cs("b");
                let kc = cs("c");
                // Supply enough key args for the widest format above.
                let cj = cpk(&mut ce, fl, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr());
                let rj = rpk(&mut re, fl, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr());
                cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, flags 0x{fl:x})"));
            }
        }
    }
}

/// G2: `b` with 0 and assorted nonzero ints.
#[test]
fn g2_pack_bool() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();
        for fmt in ["b", "[b]", "[b,b]", "{s:b}"] {
            let f = cs(fmt);
            for v in [0i32, 1, -1, 2, i32::MAX, i32::MIN, 0x100] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let ka = cs("k");
                let cj = cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), v, v);
                let rj = rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), v, v);
                let _ = (cj, rj);
                // The key arg position differs per format, so re-issue precisely:
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), v),
                    )
                } else if fmt == "[b,b]" {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), v, !v),
                        rpk(&mut re, 0, f.as_ptr(), v, !v),
                    )
                } else {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), v),
                    )
                };
                cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, b={v})"));
            }
        }
    }
}

/// G3: `i` (consumes an `int`) and `I` (consumes a `json_int_t`) at their
/// boundaries, plus randomized values.
#[test]
fn g3_pack_integers() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();
        let mut rng = Rng::new(0x63_63);

        let mut i32s: Vec<i32> = vec![0, 1, -1, i32::MAX, i32::MIN, 42, -42];
        let mut i64s: Vec<i64> = vec![0, 1, -1, i64::MAX, i64::MIN, i32::MAX as i64 + 1, i32::MIN as i64 - 1];
        for _ in 0..300 {
            i32s.push(rng.next_u64() as i32);
            i64s.push(rng.next_u64() as i64);
        }

        for fmt in ["i", "[i]", "[i,i]", "{s:i}"] {
            let f = cs(fmt);
            for &v in &i32s {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let ka = cs("k");
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), v),
                    )
                } else if fmt == "[i,i]" {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), v, v.wrapping_neg()),
                        rpk(&mut re, 0, f.as_ptr(), v, v.wrapping_neg()),
                    )
                } else {
                    (cpk(&mut ce, 0, f.as_ptr(), v), rpk(&mut re, 0, f.as_ptr(), v))
                };
                cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, i={v})"));
            }
        }
        for fmt in ["I", "[I]", "[I,I]", "{s:I}"] {
            let f = cs(fmt);
            for &v in &i64s {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let ka = cs("k");
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), v),
                    )
                } else if fmt == "[I,I]" {
                    (
                        cpk(&mut ce, 0, f.as_ptr(), v, v.wrapping_neg()),
                        rpk(&mut re, 0, f.as_ptr(), v, v.wrapping_neg()),
                    )
                } else {
                    (cpk(&mut ce, 0, f.as_ptr(), v), rpk(&mut re, 0, f.as_ptr(), v))
                };
                cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, I={v})"));
            }
        }
        // Mixed i/I in one format, so vararg alignment is checked.
        let f = cs("[i,I,i,I]");
        for _ in 0..200 {
            let a = rng.next_u64() as i32;
            let b = rng.next_u64() as i64;
            let c = rng.next_u64() as i32;
            let d = rng.next_u64() as i64;
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), a, b, c, d);
            let rj = rpk(&mut re, 0, f.as_ptr(), a, b, c, d);
            cmp_pack(cj, rj, &ce, &re, &format!("pack([i,I,i,I], {a},{b},{c},{d})"));
        }
    }
}

/// G4: `f` with randomized finite doubles, plus the NaN/Inf rejections.
#[test]
fn g4_pack_real() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();
        let mut rng = Rng::new(0x64_64);
        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324,
            f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        ];
        for _ in 0..500 {
            vals.push(rng.finite_f64());
        }
        for fmt in ["f", "[f]", "[f,f]", "{s:f}", "[i,f,I,f]"] {
            let f = cs(fmt);
            for &v in &vals {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                let ka = cs("k");
                let (cj, rj) = match fmt {
                    "{s:f}" => (
                        cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), v),
                        rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), v),
                    ),
                    "[f,f]" => (
                        cpk(&mut ce, 0, f.as_ptr(), v, -v),
                        rpk(&mut re, 0, f.as_ptr(), v, -v),
                    ),
                    "[i,f,I,f]" => (
                        cpk(&mut ce, 0, f.as_ptr(), 7i32, v, 9i64, -v),
                        rpk(&mut re, 0, f.as_ptr(), 7i32, v, 9i64, -v),
                    ),
                    _ => (cpk(&mut ce, 0, f.as_ptr(), v), rpk(&mut re, 0, f.as_ptr(), v)),
                };
                cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, f={v:?})"));
            }
        }
    }
}

/// G5..G9, G16: the `s` family — plain, `#`, `%`, `+` (with per-piece `#`/`%`),
/// `?` and `*`, over ASCII, non-ASCII and invalid UTF-8.
#[test]
fn g5_g9_g16_pack_strings() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();

        let payloads: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello".to_vec(),
            "é".as_bytes().to_vec(),
            "€".as_bytes().to_vec(),
            "😀".as_bytes().to_vec(),
            "aé€😀z".as_bytes().to_vec(),
            vec![0x80],
            vec![0xC0, 0x80],
            vec![0xFF],
            vec![0xED, 0xA0, 0x80],
            vec![0xC3], // truncated 2-byte
            vec![b'x'; 100],
        ];

        // plain `s`, and the `?`/`*` variants, with a real pointer and NULL.
        for fmt in ["s", "[s]", "{s:s}", "s?", "[s?]", "{s:s?}", "s*", "[s*]", "{s:s*}"] {
            let f = cs(fmt);
            for pl in &payloads {
                let z = raw_z(pl);
                for use_null in [false, true] {
                    let arg: *const c_char = if use_null {
                        std::ptr::null()
                    } else {
                        z.as_ptr() as *const c_char
                    };
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let (cj, rj) = if fmt.starts_with('{') {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), arg),
                            rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), arg),
                        )
                    } else {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), arg),
                            rpk(&mut re, 0, f.as_ptr(), arg),
                        )
                    };
                    cmp_pack(
                        cj,
                        rj,
                        &ce,
                        &re,
                        &format!("pack({fmt:?}, {pl:02x?}, null={use_null})"),
                    );
                }
            }
        }

        // G5: `s#` takes an extra `int` length.
        for fmt in ["s#", "[s#]", "{s:s#}"] {
            let f = cs(fmt);
            for pl in &payloads {
                let z = raw_z(pl);
                for len in -1i32..=(pl.len() as i32 + 1) {
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let a = z.as_ptr() as *const c_char;
                    let (cj, rj) = if fmt.starts_with('{') {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), a, len),
                            rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), a, len),
                        )
                    } else {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), a, len),
                            rpk(&mut re, 0, f.as_ptr(), a, len),
                        )
                    };
                    cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, {pl:02x?}, len {len})"));
                }
            }
        }

        // G6: `s%` takes an extra `size_t` length.
        for fmt in ["s%", "[s%]", "{s:s%}"] {
            let f = cs(fmt);
            for pl in &payloads {
                let z = raw_z(pl);
                for len in 0..=pl.len() + 1 {
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let a = z.as_ptr() as *const c_char;
                    let (cj, rj) = if fmt.starts_with('{') {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), a, len),
                            rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), a, len),
                        )
                    } else {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), a, len),
                            rpk(&mut re, 0, f.as_ptr(), a, len),
                        )
                    };
                    cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}, {pl:02x?}, len {len})"));
                }
            }
        }

        // G7: `s+` concatenation, 2/3/5 pieces, with and without per-piece
        // lengths. Also exercises the "invalid UTF-8 only after joining" case
        // (a split multi-byte sequence).
        let pieces: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"bc".to_vec(),
            vec![0xC3],       // first half of "é"
            vec![0xA9],       // second half of "é"
            "€".as_bytes().to_vec(),
            vec![b'z'; 40],
        ];
        for fmt in ["s+", "s++", "s++++", "[s+]", "{s:s+}"] {
            let f = cs(fmt);
            let n_pieces = 1 + fmt.matches('+').count();
            for i in 0..pieces.len() {
                for j in 0..pieces.len() {
                    let zs: Vec<Vec<u8>> = (0..n_pieces)
                        .map(|q| raw_z(&pieces[(i + q * 3 + j) % pieces.len()]))
                        .collect();
                    let ptrs: Vec<*const c_char> =
                        zs.iter().map(|z| z.as_ptr() as *const c_char).collect();
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let ctx = format!("pack({fmt:?}, pieces {i},{j})");
                    macro_rules! call {
                        ($sym:expr, $err:expr) => {
                            match (fmt.starts_with('{'), n_pieces) {
                                (true, 2) => $sym($err, 0, f.as_ptr(), ka.as_ptr(), ptrs[0], ptrs[1]),
                                (false, 2) => $sym($err, 0, f.as_ptr(), ptrs[0], ptrs[1]),
                                (false, 3) => $sym($err, 0, f.as_ptr(), ptrs[0], ptrs[1], ptrs[2]),
                                (false, 5) => $sym(
                                    $err, 0, f.as_ptr(), ptrs[0], ptrs[1], ptrs[2], ptrs[3], ptrs[4],
                                ),
                                _ => $sym($err, 0, f.as_ptr(), ka.as_ptr(), ptrs[0], ptrs[1]),
                            }
                        };
                    }
                    let cj = call!(cpk, &mut ce);
                    let rj = call!(rpk, &mut re);
                    cmp_pack(cj, rj, &ce, &re, &ctx);
                }
            }
        }
        // `s+#` / `s+%`: per-piece explicit lengths.
        for (fmt, lens) in [("s#+#", true), ("s%+%", false)] {
            let f = cs(fmt);
            for i in 0..pieces.len() {
                for j in 0..pieces.len() {
                    let z0 = raw_z(&pieces[i]);
                    let z1 = raw_z(&pieces[j]);
                    let a0 = z0.as_ptr() as *const c_char;
                    let a1 = z1.as_ptr() as *const c_char;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ctx = format!("pack({fmt:?}, {i},{j})");
                    let (cj, rj) = if lens {
                        let l0 = pieces[i].len() as c_int;
                        let l1 = pieces[j].len() as c_int;
                        (
                            cpk(&mut ce, 0, f.as_ptr(), a0, l0, a1, l1),
                            rpk(&mut re, 0, f.as_ptr(), a0, l0, a1, l1),
                        )
                    } else {
                        let l0 = pieces[i].len();
                        let l1 = pieces[j].len();
                        (
                            cpk(&mut ce, 0, f.as_ptr(), a0, l0, a1, l1),
                            rpk(&mut re, 0, f.as_ptr(), a0, l0, a1, l1),
                        )
                    };
                    cmp_pack(cj, rj, &ce, &re, &ctx);
                }
            }
        }
        // G-N4: optional combined with a length/concat modifier is rejected.
        for fmt in ["s?#", "s?%", "s?+", "s*#", "s*%", "s*+", "[s?#]", "{s:s*+}"] {
            let f = cs(fmt);
            let z = raw_z(b"x");
            let a = z.as_ptr() as *const c_char;
            let ka = cs("k");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let (cj, rj) = if fmt.starts_with('{') {
                (
                    cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), a, a),
                    rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), a, a),
                )
            } else {
                (
                    cpk(&mut ce, 0, f.as_ptr(), a, a),
                    rpk(&mut re, 0, f.as_ptr(), a, a),
                )
            };
            cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?}) optional+modifier"));
        }
        // Non-ASCII object keys via `s` in key position.
        for key in ["é", "€", "😀", "", "k"] {
            let f = cs("{s:i}");
            let kk = cs(key);
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), kk.as_ptr(), 1i32);
            let rj = rpk(&mut re, 0, f.as_ptr(), kk.as_ptr(), 1i32);
            cmp_pack(cj, rj, &ce, &re, &format!("pack({{s:i}}, key {key:?})"));
        }
        // Invalid-UTF-8 object key.
        for bad in [vec![0x80u8, 0], vec![0xFFu8, 0], vec![0xC3u8, 0]] {
            let f = cs("{s:i}");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(&mut ce, 0, f.as_ptr(), bad.as_ptr() as *const c_char, 1i32);
            let rj = rpk(&mut re, 0, f.as_ptr(), bad.as_ptr() as *const c_char, 1i32);
            cmp_pack(cj, rj, &ce, &re, &format!("pack({{s:i}}, bad key {bad:02x?})"));
        }
        // NULL object key.
        let f = cs("{s:i}");
        let mut ce = json_error_t::default();
        let mut re = json_error_t::default();
        let cj = cpk(&mut ce, 0, f.as_ptr(), std::ptr::null::<c_char>(), 1i32);
        let rj = rpk(&mut re, 0, f.as_ptr(), std::ptr::null::<c_char>(), 1i32);
        cmp_pack(cj, rj, &ce, &re, "pack({s:i}, NULL key)");
    }
}

/// G10, G11: `O` (increfs) and `o` (steals), plus their `?`/`*` variants with
/// NULL. Refcounts are compared on both sides.
#[test]
fn g10_g11_pack_objects() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();

        let nodes: Vec<Node> = vec![
            Node::Null,
            Node::True,
            Node::Int(5),
            Node::Real(1.5),
            Node::Str(b"s".to_vec()),
            Node::Arr(vec![Node::Int(1)]),
            Node::Obj(vec![(b"a".to_vec(), Node::Int(1))]),
        ];

        // `O`: the caller keeps its reference, so refcount must go up by one.
        for fmt in ["O", "[O]", "{s:O}", "O?", "[O?]", "O*", "[O*]", "{s:O*}"] {
            let f = cs(fmt);
            for node in &nodes {
                for use_null in [false, true] {
                    let cv = if use_null {
                        std::ptr::null_mut()
                    } else {
                        node.build(&p.c)
                    };
                    let rv = if use_null {
                        std::ptr::null_mut()
                    } else {
                        node.build(&p.r)
                    };
                    let crc0 = if cv.is_null() { 0 } else { refcount_json(cv) };
                    let rrc0 = if rv.is_null() { 0 } else { refcount_json(rv) };
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let (cj, rj) = if fmt.starts_with('{') {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), cv),
                            rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), rv),
                        )
                    } else {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), cv),
                            rpk(&mut re, 0, f.as_ptr(), rv),
                        )
                    };
                    let ctx = format!("pack({fmt:?}, {node:?}, null={use_null})");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: error");
                    assert_eq!(cj.is_null(), rj.is_null(), "{ctx}: NULL-ness");
                    if !cj.is_null() {
                        for fl in [R_ORD, R_SORT] {
                            assert_eq!(p.c.dumps(cj, fl), p.r.dumps(rj, fl), "{ctx}: dumps");
                        }
                    }
                    if !cv.is_null() {
                        assert_eq!(
                            refcount_json(cv).wrapping_sub(crc0),
                            refcount_json(rv).wrapping_sub(rrc0),
                            "{ctx}: refcount delta on the packed value"
                        );
                    }
                    p.c.json_decref(cj);
                    p.r.json_decref(rj);
                    p.c.json_decref(cv);
                    p.r.json_decref(rv);
                }
            }
        }

        // `o`: the reference is stolen, so the caller must NOT decref it.
        for fmt in ["o", "[o]", "{s:o}", "o?", "[o?]", "o*", "[o*]", "{s:o*}"] {
            let f = cs(fmt);
            for node in &nodes {
                for use_null in [false, true] {
                    let cv = if use_null {
                        std::ptr::null_mut()
                    } else {
                        node.build(&p.c)
                    };
                    let rv = if use_null {
                        std::ptr::null_mut()
                    } else {
                        node.build(&p.r)
                    };
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let ka = cs("k");
                    let (cj, rj) = if fmt.starts_with('{') {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), cv),
                            rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), rv),
                        )
                    } else {
                        (
                            cpk(&mut ce, 0, f.as_ptr(), cv),
                            rpk(&mut re, 0, f.as_ptr(), rv),
                        )
                    };
                    cmp_pack(
                        cj,
                        rj,
                        &ce,
                        &re,
                        &format!("pack({fmt:?}, {node:?}, null={use_null})"),
                    );
                }
            }
        }
    }
}

/// G12, G15: nested containers to depth 4, empty containers, and an object
/// large enough to cross a rehash threshold.
#[test]
fn g12_g15_pack_nesting_and_size() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_ex_sym();
        let rpk = p.r.json_pack_ex_sym();

        for fmt in [
            "[[[[i]]]]",
            "{s:{s:{s:{s:i}}}}",
            "[{s:[{s:i}]}]",
            "[[],{},[[]],[{}]]",
            "{s:[],s:{}}",
        ] {
            let f = cs(fmt);
            let ka = cs("a");
            let kb = cs("b");
            let kc = cs("c");
            let kd = cs("d");
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let cj = cpk(
                &mut ce, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr(), kd.as_ptr(), 7i32,
            );
            let rj = rpk(
                &mut re, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr(), kd.as_ptr(), 7i32,
            );
            let _ = (cj, rj);
            // The key/value interleaving differs per format; re-issue exactly.
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let (cj, rj) = match fmt {
                "[[[[i]]]]" => (
                    cpk(&mut ce, 0, f.as_ptr(), 7i32),
                    rpk(&mut re, 0, f.as_ptr(), 7i32),
                ),
                "{s:{s:{s:{s:i}}}}" => (
                    cpk(
                        &mut ce, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr(),
                        kd.as_ptr(), 7i32,
                    ),
                    rpk(
                        &mut re, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), kc.as_ptr(),
                        kd.as_ptr(), 7i32,
                    ),
                ),
                "[{s:[{s:i}]}]" => (
                    cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), 7i32),
                    rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr(), 7i32),
                ),
                "[[],{},[[]],[{}]]" => {
                    (cpk(&mut ce, 0, f.as_ptr()), rpk(&mut re, 0, f.as_ptr()))
                }
                _ => (
                    cpk(&mut ce, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr()),
                    rpk(&mut re, 0, f.as_ptr(), ka.as_ptr(), kb.as_ptr()),
                ),
            };
            cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?})"));
        }

        // G15: a packed object crossing rehash thresholds (9, 17, 33 keys).
        for n in [1usize, 8, 9, 16, 17, 32, 33] {
            let fmt = format!("{{{}}}", vec!["s:i"; n].join(","));
            let f = cs(&fmt);
            let keys: Vec<std::ffi::CString> = (0..n).map(|i| cs(&format!("k{i:03}"))).collect();
            // Build the vararg list by chunking: use a fixed-arity dispatch for
            // the supported sizes.
            let mut ce = json_error_t::default();
            let mut re = json_error_t::default();
            let kp: Vec<*const c_char> = keys.iter().map(|c| c.as_ptr()).collect();
            macro_rules! packn {
                ($sym:expr, $err:expr) => {
                    match n {
                        1 => $sym($err, 0, f.as_ptr(), kp[0], 0i32),
                        8 => $sym(
                            $err, 0, f.as_ptr(), kp[0], 0i32, kp[1], 1i32, kp[2], 2i32, kp[3],
                            3i32, kp[4], 4i32, kp[5], 5i32, kp[6], 6i32, kp[7], 7i32,
                        ),
                        9 => $sym(
                            $err, 0, f.as_ptr(), kp[0], 0i32, kp[1], 1i32, kp[2], 2i32, kp[3],
                            3i32, kp[4], 4i32, kp[5], 5i32, kp[6], 6i32, kp[7], 7i32, kp[8],
                            8i32,
                        ),
                        16 => $sym(
                            $err, 0, f.as_ptr(), kp[0], 0i32, kp[1], 1i32, kp[2], 2i32, kp[3],
                            3i32, kp[4], 4i32, kp[5], 5i32, kp[6], 6i32, kp[7], 7i32, kp[8],
                            8i32, kp[9], 9i32, kp[10], 10i32, kp[11], 11i32, kp[12], 12i32,
                            kp[13], 13i32, kp[14], 14i32, kp[15], 15i32,
                        ),
                        17 => $sym(
                            $err, 0, f.as_ptr(), kp[0], 0i32, kp[1], 1i32, kp[2], 2i32, kp[3],
                            3i32, kp[4], 4i32, kp[5], 5i32, kp[6], 6i32, kp[7], 7i32, kp[8],
                            8i32, kp[9], 9i32, kp[10], 10i32, kp[11], 11i32, kp[12], 12i32,
                            kp[13], 13i32, kp[14], 14i32, kp[15], 15i32, kp[16], 16i32,
                        ),
                        _ => std::ptr::null_mut(),
                    }
                };
            }
            let cj = packn!(cpk, &mut ce);
            let rj = packn!(rpk, &mut re);
            if cj.is_null() && rj.is_null() && (n == 32 || n == 33) {
                continue; // not covered by the fixed-arity dispatch
            }
            cmp_pack(cj, rj, &ce, &re, &format!("pack({fmt:?})"));
        }
    }
}

// ===========================================================================
// H — unpack
// ===========================================================================

/// Roots to unpack against, one per json_type plus containers.
fn roots() -> Vec<Node> {
    vec![
        Node::Null,
        Node::True,
        Node::False,
        Node::Int(0),
        Node::Int(42),
        Node::Int(-1),
        Node::Int(i64::MAX),
        Node::Int(i64::MIN),
        Node::Real(0.0),
        Node::Real(1.5),
        Node::Real(-2.5),
        Node::Str(b"".to_vec()),
        Node::Str(b"hello".to_vec()),
        Node::Str("é€😀".as_bytes().to_vec()),
        Node::StrNoCheck(b"a\0b".to_vec()),
        Node::Arr(vec![]),
        Node::Arr(vec![Node::Int(1)]),
        Node::Arr(vec![Node::Int(1), Node::Int(2)]),
        Node::Arr(vec![Node::Int(1), Node::Int(2), Node::Int(3)]),
        Node::Obj(vec![]),
        Node::Obj(vec![(b"a".to_vec(), Node::Int(1))]),
        Node::Obj(vec![(b"a".to_vec(), Node::Int(1)), (b"b".to_vec(), Node::Int(2))]),
        Node::Obj(vec![
            (b"a".to_vec(), Node::Int(1)),
            (b"b".to_vec(), Node::Int(2)),
            (b"c".to_vec(), Node::Int(3)),
        ]),
    ]
}

const UNPACK_FLAGS: [usize; 4] = [0, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT];

/// H1..H3, H13, H14: every scalar format char against every root, under every
/// flag combination. Because `JSON_VALIDATE_ONLY` consumes no value varargs,
/// each format is issued once with the value target and once without.
#[test]
fn h1_h3_h13_h14_scalar_unpack() {
    unsafe {
        let p = pair();
        let cun = p.c.json_unpack_ex_sym();
        let run = p.r.json_unpack_ex_sym();

        for node in roots() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            if cj.is_null() {
                continue;
            }
            for &fl in &UNPACK_FLAGS {
                let validate = fl & JSON_VALIDATE_ONLY != 0;

                // 'n' — never consumes a vararg (H17)
                {
                    let f = cs("n");
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let cr = cun(cj, &mut ce, fl, f.as_ptr());
                    let rr = run(rj, &mut re, fl, f.as_ptr());
                    assert_eq!(cr, rr, "unpack('n', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('n', {node:?}, 0x{fl:x}) err");
                }
                // 'i' -> int*
                {
                    let f = cs("i");
                    let mut cv: c_int = -12345;
                    let mut rv: c_int = -12345;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut c_int),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut c_int),
                        )
                    };
                    assert_eq!(cr, rr, "unpack('i', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('i', {node:?}, 0x{fl:x}) err");
                    assert_eq!(cv, rv, "unpack('i', {node:?}, 0x{fl:x}) target");
                }
                // 'I' -> json_int_t*
                {
                    let f = cs("I");
                    let mut cv: i64 = -12345;
                    let mut rv: i64 = -12345;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut i64),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut i64),
                        )
                    };
                    assert_eq!(cr, rr, "unpack('I', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('I', {node:?}, 0x{fl:x}) err");
                    assert_eq!(cv, rv, "unpack('I', {node:?}, 0x{fl:x}) target");
                }
                // 'b' -> int*
                {
                    let f = cs("b");
                    let mut cv: c_int = -12345;
                    let mut rv: c_int = -12345;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut c_int),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut c_int),
                        )
                    };
                    assert_eq!(cr, rr, "unpack('b', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('b', {node:?}, 0x{fl:x}) err");
                    assert_eq!(cv, rv, "unpack('b', {node:?}, 0x{fl:x}) target");
                }
                // 'f' (real only) and 'F' (real or integer) -> double*
                for spec in ["f", "F"] {
                    let f = cs(spec);
                    let mut cv: f64 = -12345.0;
                    let mut rv: f64 = -12345.0;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut f64),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut f64),
                        )
                    };
                    assert_eq!(cr, rr, "unpack({spec:?}, {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack({spec:?}, {node:?}) err");
                    assert_eq!(
                        cv.to_bits(),
                        rv.to_bits(),
                        "unpack({spec:?}, {node:?}) target"
                    );
                }
                // 's' -> const char**  (H4 adds 's%')
                {
                    let f = cs("s");
                    let mut cv: *const c_char = std::ptr::null();
                    let mut rv: *const c_char = std::ptr::null();
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut *const c_char),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut *const c_char),
                        )
                    };
                    assert_eq!(cr, rr, "unpack('s', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('s', {node:?}) err");
                    assert_eq!(
                        cstr_bytes(cv),
                        cstr_bytes(rv),
                        "unpack('s', {node:?}) target"
                    );
                }
                // 's%' -> const char**, size_t*
                {
                    let f = cs("s%");
                    let mut cv: *const c_char = std::ptr::null();
                    let mut rv: *const c_char = std::ptr::null();
                    let mut cl: usize = 0xDEAD;
                    let mut rl: usize = 0xDEAD;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(),
                                &mut cv as *mut *const c_char, &mut cl as *mut usize,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(),
                                &mut rv as *mut *const c_char, &mut rl as *mut usize,
                            ),
                        )
                    };
                    assert_eq!(cr, rr, "unpack('s%', {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack('s%', {node:?}) err");
                    assert_eq!(cl, rl, "unpack('s%', {node:?}) length");
                    if cr == 0 && !validate && !cv.is_null() {
                        assert_eq!(
                            std::slice::from_raw_parts(cv as *const u8, cl),
                            std::slice::from_raw_parts(rv as *const u8, rl),
                            "unpack('s%', {node:?}) bytes"
                        );
                    }
                }
                // 'o' (borrow) and 'O' (incref) -> json_t**  (H15)
                for spec in ["o", "O"] {
                    let f = cs(spec);
                    let mut cv: json_ptr = std::ptr::null_mut();
                    let mut rv: json_ptr = std::ptr::null_mut();
                    let crc0 = refcount_json(cj);
                    let rrc0 = refcount_json(rj);
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (cun(cj, &mut ce, fl, f.as_ptr()), run(rj, &mut re, fl, f.as_ptr()))
                    } else {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut cv as *mut json_ptr),
                            run(rj, &mut re, fl, f.as_ptr(), &mut rv as *mut json_ptr),
                        )
                    };
                    assert_eq!(cr, rr, "unpack({spec:?}, {node:?}, 0x{fl:x})");
                    assert_eq!(ce.snap(), re.snap(), "unpack({spec:?}, {node:?}) err");
                    assert_eq!(
                        cv.is_null(),
                        rv.is_null(),
                        "unpack({spec:?}, {node:?}) target NULL-ness"
                    );
                    assert_eq!(
                        refcount_json(cj).wrapping_sub(crc0),
                        refcount_json(rj).wrapping_sub(rrc0),
                        "unpack({spec:?}, {node:?}, 0x{fl:x}) refcount delta"
                    );
                    if spec == "O" && !cv.is_null() {
                        p.c.json_decref(cv);
                        p.r.json_decref(rv);
                    }
                }
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// H5..H12, H16, H18: container formats — object keys, optional keys, strict
/// mode, `!` / `*`, and nesting.
#[test]
fn h5_h18_container_unpack() {
    unsafe {
        let p = pair();
        let cun = p.c.json_unpack_ex_sym();
        let run = p.r.json_unpack_ex_sym();

        // Formats consuming exactly one key + one int target.
        let one_key: &[&str] = &[
            "{s:i}", "{s?i}", "{s:i!}", "{s:i*}", "{s?i!}", "{s?i*}", "{s:o}", "{s:O}", "{s:s}",
            "{s:n}", "{s:b}", "{s:f}", "{s:F}", "{s:I}", "{s:[]}", "{s:{}}",
        ];
        // Array formats consuming one int target.
        let one_val: &[&str] = &[
            "[i]", "[i!]", "[i*]", "[]", "[!]", "[*]", "[o]", "[O]", "[s]", "[n]", "[b]", "[f]",
            "[F]", "[I]", "[[]]", "[{}]", "[i,i]", "[i,i!]", "[i,i,i]",
        ];

        for node in roots() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            if cj.is_null() {
                continue;
            }
            for &fl in &UNPACK_FLAGS {
                let validate = fl & JSON_VALIDATE_ONLY != 0;
                let ka = cs("a");
                let kz = cs("zzz"); // absent key

                for fmt in one_key {
                    for key in [&ka, &kz] {
                        let f = cs(fmt);
                        let mut cv: i64 = -1;
                        let mut rv: i64 = -1;
                        let mut ce = json_error_t::default();
                        let mut re = json_error_t::default();
                        // Container-only value specs ("[]" / "{}") consume no target.
                        let no_target = validate || fmt.ends_with("[]}") || fmt.ends_with("{}}")
                            || fmt.contains(":n");
                        let (cr, rr) = if no_target {
                            (
                                cun(cj, &mut ce, fl, f.as_ptr(), key.as_ptr()),
                                run(rj, &mut re, fl, f.as_ptr(), key.as_ptr()),
                            )
                        } else {
                            (
                                cun(
                                    cj, &mut ce, fl, f.as_ptr(), key.as_ptr(),
                                    &mut cv as *mut i64,
                                ),
                                run(
                                    rj, &mut re, fl, f.as_ptr(), key.as_ptr(),
                                    &mut rv as *mut i64,
                                ),
                            )
                        };
                        let ctx = format!(
                            "unpack({fmt:?}, key {:?}, {node:?}, flags 0x{fl:x})",
                            key.to_str().unwrap()
                        );
                        assert_eq!(cr, rr, "{ctx}");
                        assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                    }
                }
                for fmt in one_val {
                    let f = cs(fmt);
                    let mut c1: i64 = -1;
                    let mut r1: i64 = -1;
                    let mut c2: i64 = -2;
                    let mut r2: i64 = -2;
                    let mut c3: i64 = -3;
                    let mut r3: i64 = -3;
                    let ntargets = if validate { 0 } else { fmt.matches('i').count()
                        + fmt.matches('o').count() + fmt.matches('O').count()
                        + fmt.matches('s').count() + fmt.matches('b').count()
                        + fmt.matches('f').count() + fmt.matches('F').count()
                        + fmt.matches('I').count() };
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = match ntargets {
                        0 => (
                            cun(cj, &mut ce, fl, f.as_ptr()),
                            run(rj, &mut re, fl, f.as_ptr()),
                        ),
                        1 => (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut c1 as *mut i64),
                            run(rj, &mut re, fl, f.as_ptr(), &mut r1 as *mut i64),
                        ),
                        2 => (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(), &mut c1 as *mut i64,
                                &mut c2 as *mut i64,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(), &mut r1 as *mut i64,
                                &mut r2 as *mut i64,
                            ),
                        ),
                        _ => (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(), &mut c1 as *mut i64,
                                &mut c2 as *mut i64, &mut c3 as *mut i64,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(), &mut r1 as *mut i64,
                                &mut r2 as *mut i64, &mut r3 as *mut i64,
                            ),
                        ),
                    };
                    let ctx = format!("unpack({fmt:?}, {node:?}, flags 0x{fl:x})");
                    assert_eq!(cr, rr, "{ctx}");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                    // Only integer specs write a comparable value into the
                    // shared targets; `o`/`O`/`s` write pointers, which are
                    // necessarily different between the two libraries.
                    let value_comparable = fmt.chars().all(|c| "[]!*,iI".contains(c));
                    if cr == 0 && !validate && value_comparable {
                        assert_eq!((c1, c2, c3), (r1, r2, r3), "{ctx}: targets");
                    }
                }
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// H9..H12, H18: strict / non-strict with objects and arrays whose lengths do
/// and do not match the format, including nested strictness and a local `*`
/// overriding `JSON_STRICT`, and the same key unpacked twice.
#[test]
fn h9_h12_h18_strictness() {
    unsafe {
        let p = pair();
        let cun = p.c.json_unpack_ex_sym();
        let run = p.r.json_unpack_ex_sym();

        let objs: Vec<Node> = vec![
            Node::Obj(vec![]),
            Node::Obj(vec![(b"a".to_vec(), Node::Int(1))]),
            Node::Obj(vec![(b"a".to_vec(), Node::Int(1)), (b"b".to_vec(), Node::Int(2))]),
            Node::Obj(vec![
                (b"a".to_vec(), Node::Int(1)),
                (b"b".to_vec(), Node::Int(2)),
                (b"c".to_vec(), Node::Int(3)),
            ]),
            Node::Obj(vec![(
                b"n".to_vec(),
                Node::Obj(vec![(b"x".to_vec(), Node::Int(1)), (b"y".to_vec(), Node::Int(2))]),
            )]),
        ];
        // H18: `{s:i s:i}` with the SAME key twice — key_set counts it once.
        let one_key_fmts = ["{s:i}", "{s:i!}", "{s:i*}", "{s?i}", "{s?i!}", "{s?i*}"];
        let two_key_fmts = ["{s:i,s:i}", "{s:i,s:i!}", "{s:i,s:i*}", "{s?i,s?i}", "{s?i,s?i!}"];

        for node in &objs {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for &fl in &UNPACK_FLAGS {
                let validate = fl & JSON_VALIDATE_ONLY != 0;
                for fmt in one_key_fmts {
                    for key in ["a", "b", "n", "zz"] {
                        let f = cs(fmt);
                        let kk = cs(key);
                        let mut cv: i64 = -1;
                        let mut rv: i64 = -1;
                        let mut ce = json_error_t::default();
                        let mut re = json_error_t::default();
                        let (cr, rr) = if validate {
                            (
                                cun(cj, &mut ce, fl, f.as_ptr(), kk.as_ptr()),
                                run(rj, &mut re, fl, f.as_ptr(), kk.as_ptr()),
                            )
                        } else {
                            (
                                cun(cj, &mut ce, fl, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64),
                                run(rj, &mut re, fl, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64),
                            )
                        };
                        let ctx =
                            format!("unpack({fmt:?}, key {key:?}, {node:?}, flags 0x{fl:x})");
                        assert_eq!(cr, rr, "{ctx}");
                        assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                    }
                }
                for fmt in two_key_fmts {
                    for (k1, k2) in [("a", "b"), ("a", "a"), ("a", "zz"), ("zz", "yy")] {
                        let f = cs(fmt);
                        let kk1 = cs(k1);
                        let kk2 = cs(k2);
                        let mut c1: i64 = -1;
                        let mut c2: i64 = -2;
                        let mut r1: i64 = -1;
                        let mut r2: i64 = -2;
                        let mut ce = json_error_t::default();
                        let mut re = json_error_t::default();
                        let (cr, rr) = if validate {
                            (
                                cun(cj, &mut ce, fl, f.as_ptr(), kk1.as_ptr(), kk2.as_ptr()),
                                run(rj, &mut re, fl, f.as_ptr(), kk1.as_ptr(), kk2.as_ptr()),
                            )
                        } else {
                            (
                                cun(
                                    cj, &mut ce, fl, f.as_ptr(), kk1.as_ptr(),
                                    &mut c1 as *mut i64, kk2.as_ptr(), &mut c2 as *mut i64,
                                ),
                                run(
                                    rj, &mut re, fl, f.as_ptr(), kk1.as_ptr(),
                                    &mut r1 as *mut i64, kk2.as_ptr(), &mut r2 as *mut i64,
                                ),
                            )
                        };
                        let ctx = format!(
                            "unpack({fmt:?}, keys {k1:?},{k2:?}, {node:?}, flags 0x{fl:x})"
                        );
                        assert_eq!(cr, rr, "{ctx}");
                        assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                        if cr == 0 && !validate {
                            assert_eq!((c1, c2), (r1, r2), "{ctx}: targets");
                        }
                    }
                }
                // H12: nested strictness, and H11: a local `*` overriding JSON_STRICT.
                for fmt in ["{s:{s:i}}", "{s:{s:i!}}", "{s:{s:i*}}", "{s:{s:i}!}", "{s:{s:i}*}"] {
                    let f = cs(fmt);
                    let ka = cs("n");
                    let kx = cs("x");
                    let mut cv: i64 = -1;
                    let mut rv: i64 = -1;
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = if validate {
                        (
                            cun(cj, &mut ce, fl, f.as_ptr(), ka.as_ptr(), kx.as_ptr()),
                            run(rj, &mut re, fl, f.as_ptr(), ka.as_ptr(), kx.as_ptr()),
                        )
                    } else {
                        (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(), ka.as_ptr(), kx.as_ptr(),
                                &mut cv as *mut i64,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(), ka.as_ptr(), kx.as_ptr(),
                                &mut rv as *mut i64,
                            ),
                        )
                    };
                    let ctx = format!("unpack({fmt:?}, {node:?}, flags 0x{fl:x})");
                    assert_eq!(cr, rr, "{ctx}");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                }
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }

        // H8/H16 array strictness: array longer/shorter than the format.
        let arrs: Vec<Node> = (0..6)
            .map(|n| Node::Arr((0..n).map(|i| Node::Int(i as i64)).collect()))
            .collect();
        for node in &arrs {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for &fl in &UNPACK_FLAGS {
                let validate = fl & JSON_VALIDATE_ONLY != 0;
                for fmt in [
                    "[]", "[!]", "[*]", "[i]", "[i!]", "[i*]", "[i,i]", "[i,i!]", "[i,i*]",
                    "[i,i,i]", "[i,i,i!]",
                ] {
                    let f = cs(fmt);
                    let n = if validate { 0 } else { fmt.matches('i').count() };
                    let mut c = [-1i64, -2, -3];
                    let mut r = [-1i64, -2, -3];
                    let mut ce = json_error_t::default();
                    let mut re = json_error_t::default();
                    let (cr, rr) = match n {
                        0 => (
                            cun(cj, &mut ce, fl, f.as_ptr()),
                            run(rj, &mut re, fl, f.as_ptr()),
                        ),
                        1 => (
                            cun(cj, &mut ce, fl, f.as_ptr(), &mut c[0] as *mut i64),
                            run(rj, &mut re, fl, f.as_ptr(), &mut r[0] as *mut i64),
                        ),
                        2 => (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(), &mut c[0] as *mut i64,
                                &mut c[1] as *mut i64,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(), &mut r[0] as *mut i64,
                                &mut r[1] as *mut i64,
                            ),
                        ),
                        _ => (
                            cun(
                                cj, &mut ce, fl, f.as_ptr(), &mut c[0] as *mut i64,
                                &mut c[1] as *mut i64, &mut c[2] as *mut i64,
                            ),
                            run(
                                rj, &mut re, fl, f.as_ptr(), &mut r[0] as *mut i64,
                                &mut r[1] as *mut i64, &mut r[2] as *mut i64,
                            ),
                        ),
                    };
                    let ctx = format!("unpack({fmt:?}, {node:?}, flags 0x{fl:x})");
                    assert_eq!(cr, rr, "{ctx}");
                    assert_eq!(ce.snap(), re.snap(), "{ctx}: err");
                    if cr == 0 && !validate {
                        assert_eq!(c, r, "{ctx}: targets");
                    }
                }
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// H19: pack a value, unpack it back, and compare — the composed pipeline.
#[test]
fn h19_pack_unpack_roundtrip() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_sym();
        let rpk = p.r.json_pack_sym();
        let cun = p.c.json_unpack_sym();
        let run = p.r.json_unpack_sym();
        let mut rng = Rng::new(0x419_419);

        for _ in 0..2000 {
            let a = rng.next_u64() as i32;
            let b = rng.next_u64() as i64;
            let d = rng.finite_f64();
            let sv = cs(&rng.ascii(20));
            let pf = cs("{s:i,s:I,s:f,s:s,s:[i,I],s:{s:b}}");
            let keys: Vec<_> = ["i", "I", "f", "s", "arr", "obj", "b"]
                .iter()
                .map(|k| cs(k))
                .collect();
            let kp: Vec<*const c_char> = keys.iter().map(|c| c.as_ptr()).collect();
            let cj = cpk(
                pf.as_ptr(), kp[0], a, kp[1], b, kp[2], d, kp[3], sv.as_ptr(), kp[4], a, b,
                kp[5], kp[6], 1i32,
            );
            let rj = rpk(
                pf.as_ptr(), kp[0], a, kp[1], b, kp[2], d, kp[3], sv.as_ptr(), kp[4], a, b,
                kp[5], kp[6], 1i32,
            );
            assert_eq!(cj.is_null(), rj.is_null(), "pack roundtrip NULL-ness");
            if cj.is_null() {
                continue;
            }
            assert_eq!(
                p.c.dumps(cj, R_SORT),
                p.r.dumps(rj, R_SORT),
                "pack roundtrip value"
            );

            // Unpack it back.
            let uf = cs("{s:i,s:I,s:F,s:s,s:[i,I],s:{s:b}}");
            let mut ci: c_int = 0;
            let mut ri: c_int = 0;
            let mut cbi: i64 = 0;
            let mut rbi: i64 = 0;
            let mut cd: f64 = 0.0;
            let mut rd: f64 = 0.0;
            let mut csp: *const c_char = std::ptr::null();
            let mut rsp: *const c_char = std::ptr::null();
            let mut ca1: c_int = 0;
            let mut ra1: c_int = 0;
            let mut ca2: i64 = 0;
            let mut ra2: i64 = 0;
            let mut cbb: c_int = 0;
            let mut rbb: c_int = 0;
            let cr = cun(
                cj, uf.as_ptr(), kp[0], &mut ci as *mut c_int, kp[1], &mut cbi as *mut i64,
                kp[2], &mut cd as *mut f64, kp[3], &mut csp as *mut *const c_char, kp[4],
                &mut ca1 as *mut c_int, &mut ca2 as *mut i64, kp[5], kp[6],
                &mut cbb as *mut c_int,
            );
            let rr = run(
                rj, uf.as_ptr(), kp[0], &mut ri as *mut c_int, kp[1], &mut rbi as *mut i64,
                kp[2], &mut rd as *mut f64, kp[3], &mut rsp as *mut *const c_char, kp[4],
                &mut ra1 as *mut c_int, &mut ra2 as *mut i64, kp[5], kp[6],
                &mut rbb as *mut c_int,
            );
            assert_eq!(cr, rr, "unpack roundtrip return");
            assert_eq!(ci, ri, "unpack 'i'");
            assert_eq!(cbi, rbi, "unpack 'I'");
            assert_eq!(cd.to_bits(), rd.to_bits(), "unpack 'F'");
            assert_eq!(cstr_bytes(csp), cstr_bytes(rsp), "unpack 's'");
            assert_eq!((ca1, ca2), (ra1, ra2), "unpack nested array");
            assert_eq!(cbb, rbb, "unpack nested bool");
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
    }
}

/// The flagless `json_pack` / `json_unpack` wrappers must behave exactly like
/// the `_ex` forms with `flags == 0` and `error == NULL`.
#[test]
fn h_flagless_wrappers() {
    unsafe {
        let p = pair();
        let cpk = p.c.json_pack_sym();
        let rpk = p.r.json_pack_sym();
        let cun = p.c.json_unpack_sym();
        let run = p.r.json_unpack_sym();

        for fmt in ["n", "[i]", "{s:i}", "[s]", "x", "", "[", "ii"] {
            let f = cs(fmt);
            let kk = cs("k");
            let sv = cs("v");
            let cj = cpk(f.as_ptr(), kk.as_ptr(), 1i32, sv.as_ptr());
            let rj = rpk(f.as_ptr(), kk.as_ptr(), 1i32, sv.as_ptr());
            assert_eq!(cj.is_null(), rj.is_null(), "json_pack({fmt:?}) NULL-ness");
            if !cj.is_null() {
                assert_eq!(
                    p.c.dumps(cj, R_SORT),
                    p.r.dumps(rj, R_SORT),
                    "json_pack({fmt:?}) value"
                );
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        for node in roots() {
            let cj = node.build(&p.c);
            let rj = node.build(&p.r);
            for fmt in ["n", "i", "s", "[i]", "{s:i}", "x", ""] {
                let f = cs(fmt);
                let kk = cs("a");
                let mut cv: i64 = 0;
                let mut rv: i64 = 0;
                let cr = cun(cj, f.as_ptr(), kk.as_ptr(), &mut cv as *mut i64);
                let rr = run(rj, f.as_ptr(), kk.as_ptr(), &mut rv as *mut i64);
                assert_eq!(cr, rr, "json_unpack({fmt:?}, {node:?})");
            }
            p.c.json_decref(cj);
            p.r.json_decref(rj);
        }
        // NULL root
        for fmt in ["n", "i", "[i]"] {
            let f = cs(fmt);
            let mut cv: i64 = 0;
            let mut rv: i64 = 0;
            let cr = cun(
                std::ptr::null_mut(),
                f.as_ptr(),
                &mut cv as *mut i64,
            );
            let rr = run(
                std::ptr::null_mut(),
                f.as_ptr(),
                &mut rv as *mut i64,
            );
            assert_eq!(cr, rr, "json_unpack(NULL, {fmt:?})");
        }
    }
}

#[allow(dead_code)]
fn _unused(_: *mut c_void) {}
