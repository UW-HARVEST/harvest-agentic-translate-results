// Phase B (valid paths, CONFIGS.md) + Phase C (error paths, ERRORS.md)
// differential tests: the C executable built from the pristine `c_src/` and the
// Rust executable are run as subprocesses with identical argv and stdin, and
// their stdout / stderr / exit status are compared byte-for-byte.
//
// The C program's entire public surface *is* its process interface
// (`c_src/CMakeLists.txt` builds an executable — see SYMBOLS.md), so these tests
// drive it exactly the way an external consumer does.  The individual
// lower-level C functions are additionally driven through the shared-library
// boundary in `differential_ffi.rs`.

mod support;

use support::*;

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

// Row 1: 0 records.
#[test]
fn p01_empty_input() {
    for (i, data) in [
        &b""[..],
        b"\n",
        b" ",
        b"   ",
        b"\n\n\n",
        b" \t\n\x0b\x0c\r ",
        b"\t",
        b"\r\n",
    ]
    .iter()
    .enumerate()
    {
        assert_same(&format!("p01/{}", i), &wildcards(), data);
    }
}

// Row 2: one well-formed record, randomized fields.
#[test]
fn p02_single_record_random() {
    let mut rng = Rng::new(0x0201);
    for i in 0..200 {
        let rec = gen_record(&mut rng);
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p02/{}", i), &wildcards(), &data);
    }
}

// Rows 3 + 4: two records, insertion at the head and at the tail.
#[test]
fn p03_two_records_orders() {
    let mut rng = Rng::new(0x0304);
    for i in 0..150 {
        let mut a = gen_record(&mut rng);
        let mut b = gen_record(&mut rng);
        let (x, y) = (rng.below(1000), rng.below(1000));
        let (lo, hi) = if x <= y { (x, y) } else { (y, x) };
        match i % 3 {
            0 => {
                a.time_stamp = format!("{}", hi).into_bytes();
                b.time_stamp = format!("{}", lo).into_bytes();
            }
            1 => {
                a.time_stamp = format!("{}", lo).into_bytes();
                b.time_stamp = format!("{}", hi).into_bytes();
            }
            _ => {
                a.time_stamp = format!("{}", lo).into_bytes();
                b.time_stamp = a.time_stamp.clone();
            }
        }
        let mut data = a.render(&mut rng, false);
        data.extend(b.render(&mut rng, false));
        assert_same(&format!("p03/{}", i), &wildcards(), &data);
    }
}

// Row 5: middle insertion.
#[test]
fn p04_middle_insertion() {
    let mut rng = Rng::new(0x0405);
    for i in 0..150 {
        let n = rng.range(3, 6);
        let mut stamps: Vec<usize> = (0..n).map(|_| rng.below(50)).collect();
        // guarantee at least one middle insertion: first low, second high, rest between
        stamps[0] = 10;
        stamps[1] = 40;
        for s in stamps.iter_mut().skip(2) {
            *s = rng.range(11, 39);
        }
        let mut data = Vec::new();
        for s in &stamps {
            let mut rec = gen_record(&mut rng);
            rec.time_stamp = format!("{}", s).into_bytes();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p04/{}", i), &wildcards(), &data);
    }
}

// Row 6: 0..40 random records.
#[test]
fn p05_many_random_records() {
    let mut rng = Rng::new(0x0506);
    for i in 0..120 {
        let n = rng.below(41);
        let mut data = Vec::new();
        for _ in 0..n {
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        assert_same(&format!("p05/{}", i), &wildcards(), &data);
    }
}

// Row 7: many ties (stable insertion order).
#[test]
fn p06_tie_stability() {
    let mut rng = Rng::new(0x0607);
    for i in 0..120 {
        let n = rng.below(41);
        let pool = rng.range(1, 3);
        let mut data = Vec::new();
        for k in 0..n {
            let mut rec = gen_record(&mut rng);
            rec.time_stamp = format!("{}", rng.below(pool)).into_bytes();
            // distinct luggage ids so nothing is superseded and every record prints
            rec.luggage_id = format!("L{}", k).into_bytes();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p06/{}", i), &wildcards(), &data);
    }
}

// Row 8: ascending / descending / shuffled streams of 100 records.
#[test]
fn p07_sorted_streams() {
    let mut rng = Rng::new(0x0708);
    for round in 0..6 {
        let n = 100usize;
        let mut order: Vec<usize> = (0..n).collect();
        match round % 3 {
            0 => {}
            1 => order.reverse(),
            _ => {
                for i in (1..n).rev() {
                    let j = rng.below(i + 1);
                    order.swap(i, j);
                }
            }
        }
        let mut data = Vec::new();
        for (k, s) in order.iter().enumerate() {
            let mut rec = gen_record(&mut rng);
            rec.time_stamp = format!("{}", s * 3).into_bytes();
            rec.luggage_id = format!("L{}", k % 17).into_bytes();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p07/{}", round), &wildcards(), &data);
    }
}

// Row 9: timestamp shapes (axis C).
#[test]
fn p08_timestamp_shapes() {
    let fixed: [&str; 22] = [
        "0",
        "1",
        "00000000000123",
        "+7",
        "-1",
        "-42",
        "2147483646",
        "2147483647",
        "2147483648",
        "4294967294",
        "4294967295",
        "4294967296",
        "4294967297",
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
        "99999999999999999999999999",
        "-99999999999999999999999999",
        "000000000000000000000000001",
    ];
    let mut rng = Rng::new(0x0809);
    for (i, ts) in fixed.iter().enumerate() {
        let mut rec = gen_record(&mut rng);
        rec.time_stamp = ts.as_bytes().to_vec();
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p08/fixed/{}", i), &wildcards(), &data);
    }
    // randomized: several records, each with a randomly shaped timestamp
    for i in 0..150 {
        let n = rng.range(1, 6);
        let mut data = Vec::new();
        for k in 0..n {
            let mut rec = gen_record(&mut rng);
            rec.luggage_id = format!("L{}", k).into_bytes();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p08/rand/{}", i), &wildcards(), &data);
    }
}

// Row 10: all fields shorter than their limit.
#[test]
fn p09_widths_short() {
    let mut rng = Rng::new(0x090a);
    for i in 0..120 {
        let mut rec = gen_record(&mut rng);
        rec.luggage_id = rng.token(ALNUM, 1, 7);
        rec.flight_id = rng.token(ALNUM, 1, 5);
        rec.departure = rng.token(UPPER, 1, 2);
        rec.arrival = rng.token(UPPER, 1, 2);
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p09/{}", i), &wildcards(), &data);
    }
}

// Row 11: every field exactly at its limit (8 / 6 / 3 / 3 / 80).
#[test]
fn p10_widths_exact() {
    let mut rng = Rng::new(0x0a0b);
    for i in 0..120 {
        let mut rec = gen_record(&mut rng);
        rec.luggage_id = gen_token(&mut rng, ALNUM, 8);
        rec.flight_id = gen_token(&mut rng, ALNUM, 6);
        rec.departure = gen_token(&mut rng, UPPER, 3);
        rec.arrival = gen_token(&mut rng, UPPER, 3);
        let mut comment = vec![b' '];
        comment.extend(gen_token(&mut rng, b"ABCabc 019", 79));
        rec.comment = comment; // exactly 80 comment characters
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p10/{}", i), &wildcards(), &data);
    }
}

// Row 12: every field longer than its limit → truncation and re-parse.
#[test]
fn p11_widths_over() {
    let mut rng = Rng::new(0x0b0c);
    for i in 0..150 {
        let mut rec = gen_record(&mut rng);
        rec.luggage_id = rng.token(ALNUM, 9, 14);
        rec.flight_id = rng.token(ALNUM, 7, 12);
        rec.departure = rng.token(UPPER, 4, 8);
        rec.arrival = rng.token(UPPER, 4, 8);
        let mut data = rec.render(&mut rng, false);
        if rng.flip() {
            // a following record makes the leaked remainder observable
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        assert_same(&format!("p11/{}", i), &wildcards(), &data);
    }
}

// Row 13: separator shapes.
#[test]
fn p12_separator_shapes() {
    let mut rng = Rng::new(0x0c0d);
    for i in 0..200 {
        let n = rng.range(1, 4);
        let mut data = Vec::new();
        for k in 0..n {
            let mut rec = gen_record(&mut rng);
            rec.luggage_id = format!("L{}", k).into_bytes();
            data.extend(rec.render(&mut rng, true));
        }
        assert_same(&format!("p12/{}", i), &wildcards(), &data);
    }
}

// Row 14: no separator between departure and arrival.
#[test]
fn p13_no_separator() {
    let mut rng = Rng::new(0x0d0e);
    for i in 0..120 {
        let mut rec = gen_record(&mut rng);
        let dep = rng.token(UPPER, 1, 5);
        let arr = rng.token(UPPER, 1, 5);
        let mut joined = dep.clone();
        joined.extend_from_slice(&arr);
        rec.departure = joined;
        rec.arrival = Vec::new(); // renders as "<dep><arr>" followed by the separator
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p13/{}", i), &wildcards(), &data);
    }
}

// Row 15: comment shapes.
#[test]
fn p14_comment_shapes() {
    let fixed: [&[u8]; 12] = [
        b"",
        b" ",
        b"   ",
        b"\tx",
        b" \t\t tabbed",
        b" ab\0cd",
        b" \xff\xfe\x80\x7f end",
        b" \r",
        b" comment with spaces",
        b" 80chars",
        b" -leading dash",
        b" trailing\r",
    ];
    let mut rng = Rng::new(0x0e0f);
    for (i, c) in fixed.iter().enumerate() {
        let mut rec = gen_record(&mut rng);
        rec.comment = c.to_vec();
        let mut data = rec.render(&mut rng, false);
        data.extend(gen_record(&mut rng).render(&mut rng, false));
        assert_same(&format!("p14/fixed/{}", i), &wildcards(), &data);
    }
    for i in 0..40 {
        let mut rec = gen_record(&mut rng);
        let mut c = vec![b' '];
        c.extend(std::iter::repeat(b'z').take(80 - 1 + i % 3));
        rec.comment = c;
        let mut data = rec.render(&mut rng, false);
        data.extend(gen_record(&mut rng).render(&mut rng, false));
        assert_same(&format!("p14/len/{}", i), &wildcards(), &data);
    }
    for i in 0..120 {
        let mut rec = gen_record(&mut rng);
        rec.comment = gen_comment(&mut rng);
        let data = rec.render(&mut rng, false);
        assert_same(&format!("p14/rand/{}", i), &wildcards(), &data);
    }
}

// Row 16: supersede structures from small pools.
#[test]
fn p15_supersede_pool() {
    let mut rng = Rng::new(0x0f10);
    for i in 0..200 {
        let lugs = rng.pool(1, 3, ALNUM, 4);
        let flights = rng.pool(1, 3, ALNUM, 3);
        let airports = rng.pool(1, 3, UPPER, 3);
        let n = rng.range(1, 10);
        let mut data = Vec::new();
        for _ in 0..n {
            let ts_pool = rng.range(1, 5);
            data.extend(
                gen_pool_record(&mut rng, &lugs, &flights, &airports, ts_pool)
                    .render(&mut rng, false),
            );
        }
        let words: Vec<Vec<u8>> = lugs
            .iter()
            .chain(flights.iter())
            .chain(airports.iter())
            .cloned()
            .collect();
        let filters = gen_filters(&mut rng, &words);
        assert_same(&format!("p15/{}", i), &filters, &data);
    }
}

// Row 17: same luggage id, same departure, chains of 2..5.
#[test]
fn p16_supersede_chain() {
    let mut rng = Rng::new(0x1011);
    for i in 0..150 {
        let lug = gen_field(&mut rng, ALNUM, 8);
        let dep = gen_field(&mut rng, UPPER, 3);
        let n = rng.range(2, 5);
        let mut data = Vec::new();
        for k in 0..n {
            let mut rec = gen_record(&mut rng);
            rec.time_stamp = format!("{}", k * 10 + rng.below(3)).into_bytes();
            rec.luggage_id = lug.clone();
            rec.departure = dep.clone();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p16/{}", i), &wildcards(), &data);
    }
}

// Row 18: same luggage id, different departures → the search stops at the first
// luggage-id match even if a later record would supersede.
#[test]
fn p17_supersede_stops() {
    let mut rng = Rng::new(0x1112);
    for i in 0..150 {
        let lug = gen_field(&mut rng, ALNUM, 8);
        let dep_a = b"AAA".to_vec();
        let dep_b = b"BBB".to_vec();
        let mut data = Vec::new();
        let deps = [dep_a.clone(), dep_b.clone(), dep_a.clone()];
        for (k, dep) in deps.iter().enumerate() {
            let mut rec = gen_record(&mut rng);
            rec.time_stamp = format!("{}", k * 5).into_bytes();
            rec.luggage_id = lug.clone();
            rec.departure = dep.clone();
            data.extend(rec.render(&mut rng, false));
        }
        assert_same(&format!("p17/{}", i), &wildcards(), &data);
    }
}

// Row 19: exact duplicates.
#[test]
fn p18_duplicates() {
    let mut rng = Rng::new(0x1213);
    for i in 0..120 {
        let rec = gen_record(&mut rng);
        let n = rng.range(2, 4);
        let line = rec.render(&mut rng, false);
        let mut data = Vec::new();
        for _ in 0..n {
            data.extend_from_slice(&line);
        }
        assert_same(&format!("p18/{}", i), &wildcards(), &data);
    }
}

// Rows 20 + 21: filter cross-product (wildcard vs. exact in every position).
#[test]
fn p19_filter_cross_product() {
    let mut rng = Rng::new(0x1314);
    for round in 0..8 {
        let lugs = gen_pool(&mut rng, 2, ALNUM, 5);
        let flights = gen_pool(&mut rng, 2, ALNUM, 4);
        let airports = gen_pool(&mut rng, 2, UPPER, 3);
        let mut recs = Vec::new();
        let mut data = Vec::new();
        for _ in 0..rng.range(2, 6) {
            let rec = gen_pool_record(&mut rng, &lugs, &flights, &airports, 6);
            data.extend(rec.render(&mut rng, false));
            recs.push(rec);
        }
        let probe = &recs[rng.below(recs.len())];
        let exact = [
            probe.luggage_id.clone(),
            probe.flight_id.clone(),
            probe.departure.clone(),
            probe.arrival.clone(),
        ];
        for mask in 0..16u32 {
            let filters: Vec<Vec<u8>> = (0..4)
                .map(|pos| {
                    if mask & (1 << pos) != 0 {
                        exact[pos].clone()
                    } else {
                        b"-".to_vec()
                    }
                })
                .collect();
            assert_same(&format!("p19/{}/{:04b}", round, mask), &filters, &data);
        }
    }
}

// Row 22: special filters (non-matching literal, empty string, "-" + suffix).
#[test]
fn p20_filter_special() {
    let data = b"1 L1 F1 AAA BBB one\n2 L2 F2 CCC DDD two\n3 L1 F3 XXX BBB three\n";
    let specials: [[&str; 4]; 14] = [
        ["ZZZZ", "-", "-", "-"],
        ["-", "ZZZZ", "-", "-"],
        ["-", "-", "ZZZ", "-"],
        ["-", "-", "-", "ZZZ"],
        ["", "-", "-", "-"],
        ["-", "", "-", "-"],
        ["-", "-", "", "-"],
        ["-", "-", "-", ""],
        ["", "", "", ""],
        ["-L1", "-", "-", "-"],
        ["--", "--", "--", "--"],
        ["-x", "-y", "-z", "-w"],
        ["l1", "-", "-", "-"],
        ["L1XXXXXXXXXXXXXXX", "-", "-", "-"],
    ];
    for (i, f) in specials.iter().enumerate() {
        assert_same(&format!("p20/{}", i), &argv(f), data);
    }
}

// Row 23: randomized filters over randomized record sets.
#[test]
fn p21_filter_random() {
    let mut rng = Rng::new(0x1516);
    for i in 0..200 {
        let n = rng.range(1, 8);
        let mut data = Vec::new();
        let mut words: Vec<Vec<u8>> = Vec::new();
        for _ in 0..n {
            let rec = gen_record(&mut rng);
            words.push(rec.luggage_id.clone());
            words.push(rec.flight_id.clone());
            words.push(rec.departure.clone());
            words.push(rec.arrival.clone());
            data.extend(rec.render(&mut rng, false));
        }
        let filters = gen_filters(&mut rng, &words);
        assert_same(&format!("p21/{}", i), &filters, &data);
    }
}

// Row 24: EOF at each of the six conversion points.
#[test]
fn p22_eof_positions() {
    let mut rng = Rng::new(0x1617);
    for i in 0..200 {
        // optional complete records first
        let mut data = Vec::new();
        for _ in 0..rng.below(3) {
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        let rec = gen_record(&mut rng);
        let mut tail: Vec<u8> = Vec::new();
        let cut = rng.below(7);
        if cut >= 1 {
            tail.extend_from_slice(&rec.time_stamp);
        }
        if cut >= 2 {
            tail.push(b' ');
            tail.extend_from_slice(&rec.luggage_id);
        }
        if cut >= 3 {
            tail.push(b' ');
            tail.extend_from_slice(&rec.flight_id);
        }
        if cut >= 4 {
            tail.push(b' ');
            tail.extend_from_slice(&rec.departure);
        }
        if cut >= 5 {
            tail.push(b' ');
            tail.extend_from_slice(&rec.arrival);
        }
        if cut >= 6 {
            tail.extend_from_slice(&rec.comment);
        }
        if rng.flip() {
            tail.extend_from_slice(&gen_sep(&mut rng));
        }
        data.extend(tail);
        assert_same(&format!("p22/{}/cut{}", i, cut), &wildcards(), &data);
    }
}

// Row 25: stream termination variants.
#[test]
fn p23_stream_termination() {
    let mut rng = Rng::new(0x1718);
    let tails: [&[u8]; 8] = [b"", b"\n", b"\n\n\n", b" ", b"   ", b"\t", b"\r\n", b" \n \n"];
    for i in 0..80 {
        let mut base = Vec::new();
        for _ in 0..rng.range(1, 3) {
            let mut line = gen_record(&mut rng).render(&mut rng, false);
            line.pop(); // strip the trailing newline of the last record
            base.extend(line);
            base.push(b'\n');
        }
        base.pop();
        for (k, t) in tails.iter().enumerate() {
            let mut data = base.clone();
            data.extend_from_slice(t);
            assert_same(&format!("p23/{}/{}", i, k), &wildcards(), &data);
        }
    }
}

// Row 26: fully random byte streams.
#[test]
fn p24_random_bytes() {
    let mut rng = Rng::new(0x1819);
    for i in 0..250 {
        let n = rng.below(201);
        let data: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        assert_same(&format!("p24/{}", i), &wildcards(), &data);
    }
}

// Row 27: random tokens from a small alphabet (hits the stale-buffer paths).
#[test]
fn p25_random_tokens() {
    let mut rng = Rng::new(0x191a);
    let pool: &[u8] = b"0123456789ABCXYZ abcz\n\t-+[]\x00";
    for i in 0..250 {
        let n = rng.below(120);
        let data: Vec<u8> = (0..n).map(|_| *rng.pick(pool)).collect();
        let words: Vec<Vec<u8>> = vec![b"ABC".to_vec(), b"XYZ".to_vec(), b"1".to_vec()];
        let filters = if rng.flip() {
            wildcards()
        } else {
            gen_filters(&mut rng, &words)
        };
        assert_same(&format!("p25/{}", i), &filters, &data);
    }
}

// Row 28: larger streams built from small pools.
#[test]
fn p26_scale_pool() {
    let mut rng = Rng::new(0x1a1b);
    for round in 0..8 {
        let lugs = rng.pool(2, 6, ALNUM, 4);
        let flights = rng.pool(1, 4, ALNUM, 3);
        let airports = rng.pool(2, 4, UPPER, 3);
        let n = rng.range(100, 300);
        let mut data = Vec::new();
        for _ in 0..n {
            let ts_pool = rng.range(2, 30);
            data.extend(
                gen_pool_record(&mut rng, &lugs, &flights, &airports, ts_pool)
                    .render(&mut rng, false),
            );
        }
        let words: Vec<Vec<u8>> = lugs
            .iter()
            .chain(flights.iter())
            .chain(airports.iter())
            .cloned()
            .collect();
        for k in 0..3 {
            let filters = if k == 0 {
                wildcards()
            } else {
                gen_filters(&mut rng, &words)
            };
            assert_same(&format!("p26/{}/{}", round, k), &filters, &data);
        }
    }
}

// Row 29: a byte that is not `isspace` in the C locale used as a separator.
#[test]
fn p27_high_byte_separator() {
    let mut rng = Rng::new(0x1b1c);
    let seps: [&[u8]; 5] = [b"\xa0", b"\x80", b"\xff", b"\x01", b"\x7f"];
    for i in 0..100 {
        let sep = *rng.pick(&seps);
        let rec = gen_record(&mut rng);
        let positions = 5;
        for p in 0..positions {
            let mut data = Vec::new();
            let parts: [&[u8]; 5] = [
                &rec.time_stamp,
                &rec.luggage_id,
                &rec.flight_id,
                &rec.departure,
                &rec.arrival,
            ];
            for (k, part) in parts.iter().enumerate() {
                data.extend_from_slice(part);
                if k + 1 < parts.len() {
                    if k == p {
                        data.extend_from_slice(sep);
                    } else {
                        data.push(b' ');
                    }
                }
            }
            data.extend_from_slice(&rec.comment);
            data.push(b'\n');
            assert_same(&format!("p27/{}/{}", i, p), &wildcards(), &data);
        }
    }
}

// Row 37: records created from a matching failure (stale field values).
#[test]
fn p28_stale_records() {
    let mut rng = Rng::new(0x1c1d);
    for i in 0..200 {
        let mut data = Vec::new();
        // one or more good records first, so the stale values are deterministic
        for _ in 0..rng.range(1, 3) {
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        // then a line that fails at a chosen conversion
        let rec = gen_record(&mut rng);
        let which = rng.below(5);
        let bad = *rng.pick(NOT_IN_SET);
        let mut line: Vec<u8> = Vec::new();
        if which == 0 {
            line.push(bad);
        } else {
            line.extend_from_slice(&rec.time_stamp);
        }
        line.push(b' ');
        if which == 1 {
            line.push(bad);
        } else {
            line.extend_from_slice(&rec.luggage_id);
        }
        line.push(b' ');
        if which == 2 {
            line.push(bad);
        } else {
            line.extend_from_slice(&rec.flight_id);
        }
        line.push(b' ');
        if which == 3 {
            line.push(bad);
        } else {
            line.extend_from_slice(&rec.departure);
        }
        line.push(b' ');
        if which == 4 {
            line.push(bad);
        } else {
            line.extend_from_slice(&rec.arrival);
        }
        line.extend_from_slice(&rec.comment);
        line.push(b'\n');
        data.extend(line);
        if rng.flip() {
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        assert_same(&format!("p28/{}/w{}", i, which), &wildcards(), &data);
    }
}

// Row 38: a >80 character comment leaking into the next iteration.
#[test]
fn p29_comment_leak() {
    let mut rng = Rng::new(0x1d1e);
    for i in 0..150 {
        let mut rec = gen_record(&mut rng);
        let extra = rng.range(1, 60);
        let mut comment = vec![b' '];
        comment.extend(gen_token(&mut rng, b"ABCabc 019-", 79 + extra));
        rec.comment = comment;
        let mut data = rec.render(&mut rng, false);
        for _ in 0..rng.range(1, 2) {
            data.extend(gen_record(&mut rng).render(&mut rng, false));
        }
        assert_same(&format!("p29/{}", i), &wildcards(), &data);
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

const ARGC_ERROR: &[u8] = b"Command line error: 4 arguments expected\n";

/// Rows 1–6: `argc != 5`.
#[test]
fn e01_argc_wrong() {
    let all: [&[&str]; 8] = [
        &[],
        &["-"],
        &["-", "-"],
        &["-", "-", "-"],
        &["-", "-", "-", "-", "-"],
        &["-", "-", "-", "-", "-", "-"],
        &["a", "b", "c", "d", "e", "f", "g"],
        &["", "", "", "", "", "", "", ""],
    ];
    for (i, args) in all.iter().enumerate() {
        let a = argv(args);
        assert_same(&format!("e01/{}", i), &a, b"");
        // and the exact sentinel: stderr message, empty stdout, exit code 1
        let c = run_exe(c_exe(), &a, b"");
        let r = run_exe(rust_exe(), &a, b"");
        assert_eq!(c.code, Some(1), "C exit code for argc={}", args.len() + 1);
        assert_eq!(r.code, Some(1), "Rust exit code for argc={}", args.len() + 1);
        assert_eq!(c.stderr, ARGC_ERROR);
        assert_eq!(r.stderr, ARGC_ERROR);
        assert!(c.stdout.is_empty() && r.stdout.is_empty());
    }
}

/// Row 7: wrong `argc` with a non-empty stdin (stdin must not be consumed).
#[test]
fn e02_argc_wrong_with_stdin() {
    let data = b"1 L1 F1 AAA BBB comment\n2 L2 F2 CCC DDD c2\n";
    for args in [
        vec![],
        argv(&["-"]),
        argv(&["-", "-", "-"]),
        argv(&["-", "-", "-", "-", "-"]),
    ] {
        assert_same("e02", &args, data);
        let c = run_exe(c_exe(), &args, data);
        assert_eq!(c.code, Some(1));
        assert_eq!(c.stderr, ARGC_ERROR);
        assert!(c.stdout.is_empty());
    }
}

/// Rows 8 + 9 (and 34): `scanf("%d ")` returns EOF → no records at all.
#[test]
fn e03_eof_at_timestamp() {
    for (i, data) in [
        &b""[..],
        b" ",
        b"\n",
        b"\t\t",
        b"\x0b",
        b"\x0c",
        b"\r",
        b" \t\n\x0b\x0c\r",
        b"\n\n\n\n",
    ]
    .iter()
    .enumerate()
    {
        assert_same(&format!("e03/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        let r = run_exe(rust_exe(), &wildcards(), data);
        assert_eq!(c.code, Some(0));
        assert!(c.stdout.is_empty(), "C printed something: {:?}", c);
        assert_eq!(r.code, Some(0));
        assert!(r.stdout.is_empty());
    }
}

/// Row 10: `%d` matching failure (non-digit) is NOT an error.
#[test]
fn e04_matchfail_timestamp() {
    let cases: [&[u8]; 8] = [
        b"x\n",
        b"x ABC FL1 JFK LAX cm\n",
        b"abc\ndef\n",
        b"1 ABC FL1 JFK LAX ok\nx ZZZ FL9 SFO SEA second\n",
        b".\n",
        b"[\n",
        b"1 ABC FL1 JFK LAX ok\n. \n",
        b"z 1 ABC FL1 JFK LAX cm\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e04/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert_eq!(c.code, Some(0), "matching failure must not change the exit code");
    }
}

/// Row 11: a sign with no digits.
#[test]
fn e05_matchfail_sign_only() {
    let cases: [&[u8]; 10] = [
        b"-",
        b"+",
        b"-\n",
        b"+\n",
        b"-x ABC FL1 JFK LAX c\n",
        b"+x ABC FL1 JFK LAX c\n",
        b"++1 ABC FL1 JFK LAX c\n",
        b"--1 ABC FL1 JFK LAX c\n",
        b"- - - -\n",
        b"1 ABC FL1 JFK LAX c\n- \n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e05/{}", i), &wildcards(), data);
    }
}

/// Row 12: EOF right at the luggage id conversion.
#[test]
fn e06_eof_at_luggage() {
    for (i, data) in [&b"5"[..], b"5 ", b"5  \t ", b"5\n", b"7\n\n", b"0 "].iter().enumerate() {
        assert_same(&format!("e06/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert_eq!(c.code, Some(0));
        assert!(c.stdout.is_empty(), "record must be dropped: {:?}", c);
    }
}

/// Row 13: luggage id matching failure.
#[test]
fn e07_matchfail_luggage() {
    let cases: [&[u8]; 6] = [
        b"5 abc FL1 JFK LAX c\n",
        b"5 . FL1 JFK LAX c\n",
        b"1 ABC FL1 JFK LAX ok\n2 abc FL2 SFO SEA c2\n",
        b"5 _ _ _ _\n",
        b"5 abc\n",
        b"5 [ FL1 JFK LAX c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e07/{}", i), &wildcards(), data);
    }
}

/// Row 14: EOF after a successful luggage id (scanf returns 1, not EOF).
#[test]
fn e08_eof_after_luggage() {
    for (i, data) in [&b"5 ABC"[..], b"5 ABC ", b"5 ABC\n", b"5 ABCDEFGH  "].iter().enumerate() {
        assert_same(&format!("e08/{}", i), &wildcards(), data);
    }
}

/// Row 15: flight id matching failure.
#[test]
fn e09_matchfail_flight() {
    let cases: [&[u8]; 5] = [
        b"5 ABC fl1 JFK LAX c\n",
        b"5 ABC . JFK LAX c\n",
        b"1 ABC FL1 JFK LAX ok\n2 ABD .. SFO SEA c2\n",
        b"5 ABC #\n",
        b"5 ABC -1 JFK LAX c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e09/{}", i), &wildcards(), data);
    }
}

/// Row 16: EOF right at the departure conversion.
#[test]
fn e10_eof_at_departure() {
    for (i, data) in [&b"5 ABC FL1"[..], b"5 ABC FL1 ", b"5 ABC FL1\n", b"5 ABC FL1 \t "]
        .iter()
        .enumerate()
    {
        assert_same(&format!("e10/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert_eq!(c.code, Some(0));
        assert!(c.stdout.is_empty(), "record must be dropped: {:?}", c);
    }
}

/// Row 17: departure matching failure (digits/lowercase are not `[A-Z]`).
#[test]
fn e11_matchfail_departure() {
    let cases: [&[u8]; 6] = [
        b"5 ABC FL1 111 LAX c\n",
        b"5 ABC FL1 jfk LAX c\n",
        b"5 ABC FL1 1FK LAX c\n",
        b"1 ABC FL1 JFK LAX ok\n2 ABD FL2 999 SEA c2\n",
        b"5 ABC FL1 . LAX c\n",
        b"5 ABC FL1 0 0 c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e11/{}", i), &wildcards(), data);
    }
}

/// Row 18: EOF after a successful departure.
#[test]
fn e12_eof_after_departure() {
    for (i, data) in [&b"5 ABC FL1 JFK"[..], b"5 ABC FL1 JFK ", b"5 ABC FL1 JFK\n"]
        .iter()
        .enumerate()
    {
        assert_same(&format!("e12/{}", i), &wildcards(), data);
    }
}

/// Row 19: arrival matching failure.
#[test]
fn e13_matchfail_arrival() {
    let cases: [&[u8]; 5] = [
        b"5 ABC FL1 JFK 111 c\n",
        b"5 ABC FL1 JFK lax c\n",
        b"5 ABC FL1 JFK . c\n",
        b"1 ABC FL1 JFK LAX ok\n2 ABD FL2 SFO 123 c2\n",
        b"5 ABC FL1 JFK 1AX c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e13/{}", i), &wildcards(), data);
    }
}

/// Row 20: EOF right at the comments conversion → the whole record is dropped.
#[test]
fn e14_eof_at_comments() {
    for (i, data) in [&b"5 ABC FL1 JFK LAX"[..], b"9 ZZZZZZZZ FFFFFF SFO SEA"]
        .iter()
        .enumerate()
    {
        assert_same(&format!("e14/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert_eq!(c.code, Some(0));
        assert!(
            c.stdout.is_empty(),
            "record must be dropped even though every field parsed: {:?}",
            c
        );
    }
}

/// Row 21: comments matching failure (`\n` right after the arrival).
#[test]
fn e15_matchfail_comments() {
    let cases: [&[u8]; 4] = [
        b"5 ABC FL1 JFK LAX\n",
        b"5 ABC FL1 JFK LAX\n6 ABD FL2 SFO SEA\n",
        b"5 ABC FL1 JFK LAX\n\n",
        b"5 ABC FL1 JFK LAX\n6 ABD FL2 SFO SEA with comment\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e15/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert!(
            !c.stdout.is_empty(),
            "the record must still be created (comments are optional): {:?}",
            c
        );
    }
}

/// Rows 22–25: width truncation of every scan set.
#[test]
fn e16_width_truncation() {
    let cases: [&[u8]; 12] = [
        b"1 ABCDEFGHI FL1234 JFK LAX c\n",
        b"1 ABCDEFGHIJKLMNOP FL1234 JFK LAX c\n",
        b"1 AB ABCDEFG JFK LAX c\n",
        b"1 AB ABCDEFGHIJ JFK LAX c\n",
        b"1 AB CD JFKX LAX c\n",
        b"1 AB CD JFKXYZ LAX c\n",
        b"1 AB CD JFK LAXY c\n",
        b"1 AB CD JFK LAXYZW c\n",
        b"1 ABCDEFGHIJ ABCDEFGH JFKL LAXY tail\n",
        b"1 12345678901234 1234567890 ABCDEF GHIJKL x\n",
        b"1 ABCDEFGH FFFFFF ABC DEF ok\n",
        b"1 ABCDEFGHABCDEFGH FFFFFFFFFFFF ABCABC DEFDEF x\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e16/{}", i), &wildcards(), data);
    }
}

/// Row 26: comment longer than 80 characters.
#[test]
fn e17_comment_overflow() {
    for extra in [0usize, 1, 2, 5, 40, 120] {
        let mut data = b"1 ABC FL1 JFK LAX ".to_vec();
        data.extend(std::iter::repeat(b'q').take(79 + extra));
        data.push(b'\n');
        assert_same(&format!("e17/{}", extra), &wildcards(), &data);
        let mut with_next = data.clone();
        with_next.extend_from_slice(b"2 ABD FL2 SFO SEA next\n");
        assert_same(&format!("e17/{}/next", extra), &wildcards(), &with_next);
    }
}

/// Rows 27–31: `%d` numeric range / sign behaviour.
#[test]
fn e18_numeric_range() {
    let cases: [(&str, &str); 12] = [
        ("2147483647", "2147483647"),
        ("2147483648", "2147483648"),
        ("4294967295", "4294967295"),
        ("4294967296", "0000000000"),
        ("4294967297", "0000000001"),
        ("9223372036854775807", "4294967295"),
        ("9223372036854775808", "4294967295"),
        ("99999999999999999999", "4294967295"),
        ("-1", "4294967295"),
        ("-42", "4294967254"),
        ("-9223372036854775808", "0000000000"),
        ("-99999999999999999999", "0000000000"),
    ];
    for (i, (ts, expected)) in cases.iter().enumerate() {
        let data = format!("{} ABC FL1 JFK LAX c\n", ts).into_bytes();
        assert_same(&format!("e18/{}", i), &wildcards(), &data);
        let c = run_exe(c_exe(), &wildcards(), &data);
        let printed = String::from_utf8_lossy(&c.stdout).to_string();
        assert!(
            printed.starts_with(expected),
            "C printed {:?} for timestamp {}, expected it to start with {}",
            printed,
            ts,
            expected
        );
    }
}

/// Row 32: `supersedes` reaching the end of the list (NULL) returns 0, i.e. the
/// last record of a luggage id is never superseded.
#[test]
fn e19_supersedes_tail() {
    let cases: [&[u8]; 4] = [
        b"1 L1 F1 AAA BBB only\n",
        b"1 L1 F1 AAA BBB a\n2 L2 F2 AAA BBB b\n",
        b"1 L1 F1 AAA BBB a\n2 L1 F2 AAA BBB b\n",
        b"1 L1 F1 AAA BBB a\n2 L1 F2 AAA BBB b\n3 L1 F3 AAA BBB c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e19/{}", i), &wildcards(), data);
        let c = run_exe(c_exe(), &wildcards(), data);
        assert!(!c.stdout.is_empty(), "the tail record always prints: {:?}", c);
    }
}

/// Row 33: the supersede search stops at the FIRST record with the same luggage
/// id — a later record with the same departure does not supersede.
#[test]
fn e20_supersede_stops_at_first_match() {
    let data = b"1 L1 F1 AAA BBB first\n2 L1 F2 XXX CCC second\n3 L1 F3 AAA DDD third\n";
    assert_same("e20", &wildcards(), data);
    let c = run_exe(c_exe(), &wildcards(), data);
    let out = String::from_utf8_lossy(&c.stdout).to_string();
    assert!(
        out.contains("first"),
        "record 1 must NOT be superseded by record 3 (search stops at record 2): {:?}",
        out
    );
}

/// Row 35: an empty filter argument matches only an empty field.
#[test]
fn e21_empty_filter() {
    let data = b"1 L1 F1 AAA BBB one\n2 L2 F2 CCC DDD two\n";
    for args in [
        argv(&["", "-", "-", "-"]),
        argv(&["-", "", "-", "-"]),
        argv(&["-", "-", "", "-"]),
        argv(&["-", "-", "-", ""]),
        argv(&["", "", "", ""]),
    ] {
        assert_same("e21", &args, data);
        let c = run_exe(c_exe(), &args, data);
        assert!(c.stdout.is_empty(), "empty filter cannot match: {:?}", c);
    }
    // ... but it does match a record whose comment-less fields are empty is
    // impossible; the scan sets always assign at least one character.
}

/// Row 36: only `expected[0]` is inspected, so `-anything` is a wildcard.
#[test]
fn e22_dash_prefix_filter() {
    let data = b"1 L1 F1 AAA BBB one\n2 L2 F2 CCC DDD two\n";
    for args in [
        argv(&["-L1", "-", "-", "-"]),
        argv(&["--", "--", "--", "--"]),
        argv(&["-x", "-y", "-z", "-w"]),
        argv(&["-ZZZZZZZZZZ", "-1", "-2", "-3"]),
    ] {
        assert_same("e22", &args, data);
        let c = run_exe(c_exe(), &args, data);
        assert_eq!(
            c.stdout.iter().filter(|&&b| b == b'\n').count(),
            2,
            "a `-` prefix is a wildcard: {:?}",
            c
        );
    }
}

/// Row 37: a NUL byte inside the comment truncates it (strcpy / %s).
#[test]
fn e23_nul_in_comment() {
    let cases: [&[u8]; 5] = [
        b"1 ABC FL1 JFK LAX ab\x00cd\n",
        b"1 ABC FL1 JFK LAX \x00hidden\n",
        b"1 ABC FL1 JFK LAX \x00\x00\x00\n",
        b"1 ABC FL1 JFK LAX x\x00\n2 ABD FL2 SFO SEA y\n",
        b"1 ABC\x00DEF FL1 JFK LAX c\n",
    ];
    for (i, data) in cases.iter().enumerate() {
        assert_same(&format!("e23/{}", i), &wildcards(), data);
    }
}

/// Row 40: unparsable garbage never produces an error exit.
#[test]
fn e24_garbage_stream() {
    let mut rng = Rng::new(0xe024);
    for i in 0..150 {
        let n = rng.below(80);
        let pool: &[u8] = b"abcdefghij .,;!?[]{}()<>+-*/\\\n\t\x00\xff\x80";
        let data: Vec<u8> = (0..n).map(|_| *rng.pick(pool)).collect();
        assert_same(&format!("e24/{}", i), &wildcards(), &data);
        let c = run_exe(c_exe(), &wildcards(), &data);
        assert_eq!(c.code, Some(0), "garbage must still exit(0): {:?}", c);
    }
}

// ===========================================================================
// Heavy mixed fuzzing — every generator, randomized filters, one fixed seed.
// (Covers the CONFIGS.md rows in combination rather than in isolation.)
// ===========================================================================

#[test]
fn p30_heavy_mixed_fuzz() {
    let mut rng = Rng::new(0xf0f0_1234);
    for i in 0..1200 {
        let mut data: Vec<u8> = Vec::new();
        let mut words: Vec<Vec<u8>> = Vec::new();
        match rng.below(6) {
            // structured records from small pools
            0 | 1 => {
                let lugs = rng.pool(1, 4, ALNUM, 5);
                let flights = rng.pool(1, 3, ALNUM, 4);
                let airports = rng.pool(1, 3, UPPER, 3);
                let ts_pool = rng.range(1, 8);
                for _ in 0..rng.below(15) {
                    let fancy = rng.flip();
                    data.extend(
                        gen_pool_record(&mut rng, &lugs, &flights, &airports, ts_pool)
                            .render(&mut rng, fancy),
                    );
                }
                words.extend(lugs);
                words.extend(flights);
                words.extend(airports);
            }
            // fully random records
            2 => {
                for _ in 0..rng.below(10) {
                    let rec = gen_record(&mut rng);
                    words.push(rec.luggage_id.clone());
                    words.push(rec.departure.clone());
                    let fancy = rng.flip();
                    data.extend(rec.render(&mut rng, fancy));
                }
            }
            // records with over-long fields / comments
            3 => {
                for _ in 0..rng.below(8) {
                    let mut rec = gen_record(&mut rng);
                    rec.luggage_id = rng.token(ALNUM, 1, 14);
                    rec.flight_id = rng.token(ALNUM, 1, 12);
                    rec.departure = rng.token(UPPER, 1, 7);
                    rec.arrival = rng.token(UPPER, 1, 7);
                    if rng.flip() {
                        let mut c = vec![b' '];
                        let n = rng.range(1, 120);
                        c.extend(gen_token(&mut rng, b"ABCabc 019-.", n));
                        rec.comment = c;
                    }
                    let fancy = rng.flip();
                    data.extend(rec.render(&mut rng, fancy));
                }
            }
            // random tokens from a small alphabet
            4 => {
                let pool: &[u8] = b"0123456789ABCXYZ abcz\n\t \x0b\x0c\r-+[].\x00\xff";
                let n = rng.below(160);
                data = (0..n).map(|_| *rng.pick(pool)).collect();
            }
            // arbitrary bytes, then optionally a valid record
            _ => {
                let n = rng.below(80);
                data = (0..n).map(|_| rng.byte()).collect();
                if rng.flip() {
                    data.push(b'\n');
                    data.extend(gen_record(&mut rng).render(&mut rng, false));
                }
            }
        }
        // random truncation exercises the EOF-in-the-middle paths
        if rng.below(4) == 0 && !data.is_empty() {
            let keep = rng.below(data.len());
            data.truncate(keep);
        }
        let filters = if rng.flip() {
            wildcards()
        } else {
            gen_filters(&mut rng, &words)
        };
        assert_same(&format!("p30/{}", i), &filters, &data);
    }
}

// ===========================================================================
// Extreme sizes and exotic whitespace (boundaries of the C library itself:
// glibc's %d digit workspace, very long lines, huge whitespace runs).
// ===========================================================================

#[test]
fn p31_extreme_sizes() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    let rec = b" ABC FL1 JFK LAX c\n";

    for digits in [100usize, 1000, 5000] {
        let mut d = vec![b'1'; digits];
        d.extend_from_slice(rec);
        cases.push((format!("{}-digit timestamp", digits), d));
        let mut d = vec![b'-'];
        d.extend(std::iter::repeat(b'7').take(digits));
        d.extend_from_slice(rec);
        cases.push((format!("negative {}-digit timestamp", digits), d));
        let mut d = vec![b'0'; digits];
        d.extend_from_slice(b"42");
        d.extend_from_slice(rec);
        cases.push((format!("{} leading zeros", digits), d));
    }

    let mut d = b"1 ABC FL1 JFK LAX ".to_vec();
    d.extend(std::iter::repeat(b'z').take(10_000));
    d.push(b'\n');
    cases.push(("10k character comment".into(), d));

    cases.push(("100k single token".into(), vec![b'A'; 100_000]));

    let mut d = b"1 ".to_vec();
    d.extend(std::iter::repeat(b'A').take(50_000));
    d.push(b'\n');
    cases.push(("50k letters after a timestamp".into(), d));

    let mut d = b"1".to_vec();
    d.extend(std::iter::repeat(b' ').take(10_000));
    d.extend_from_slice(b"ABC FL1 JFK LAX c\n");
    cases.push(("10k space run".into(), d));

    let mut d = vec![b'\n'; 5_000];
    d.extend_from_slice(b"1 ABC FL1 JFK LAX c\n");
    cases.push(("5k empty lines".into(), d));

    cases.push((
        "vertical tab / form feed / CR as whitespace".into(),
        b"\x0b\x0c\r 5 ABC FL1 JFK LAX c\n".to_vec(),
    ));
    cases.push((
        "exotic whitespace between every field".into(),
        b"\x0b5\x0cABC\rFL1\x0bJFK\x0cLAX c\n".to_vec(),
    ));
    cases.push((
        "80 character comment, no trailing newline".into(),
        {
            let mut d = b"1 ABC FL1 JFK LAX ".to_vec();
            d.extend(std::iter::repeat(b'q').take(79));
            d
        },
    ));
    // 2000 records: exercises the recursive C insertion / supersede walk
    let mut d = Vec::new();
    for i in 0..2000 {
        d.extend_from_slice(format!("{} L{} F{} AAA BBB c{}\n", 2000 - i, i % 23, i % 7, i).as_bytes());
    }
    cases.push(("2000 descending records".into(), d));
    let mut d = Vec::new();
    for i in 0..2000 {
        d.extend_from_slice(format!("{} L{} F{} AAA BBB c{}\n", i, i % 23, i % 7, i).as_bytes());
    }
    cases.push(("2000 ascending records".into(), d));

    for (name, data) in cases {
        assert_same(&name, &wildcards(), &data);
    }
}

// ===========================================================================
// Process-level behaviour that is part of the C program's observable interface
// but not reachable through argv/stdin alone.
// ===========================================================================

/// stdout failures: a reader that closes the pipe early (SIGPIPE), a closed
/// stdout and a full device.  The C program has SIGPIPE at its default
/// disposition, so it is killed by the signal — the Rust program must be too.
#[test]
fn p32_stdout_failure_modes() {
    fn run_shell(exe: &std::path::Path, script_tail: &str, stdin_data: &[u8]) -> (Option<i32>, Vec<u8>) {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let script = format!("'{}' - - - - {}", exe.display(), script_tail);
        let mut child = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn bash");
        {
            let mut si = child.stdin.take().unwrap();
            let _ = si.write_all(stdin_data);
        }
        let out = child.wait_with_output().expect("wait");
        (out.status.code(), out.stderr)
    }

    // 4000 records so that the output is far larger than any stdio buffer
    let mut data = Vec::new();
    for i in 0..4000 {
        data.extend_from_slice(format!("{} L{} F1 AAA BBB c{}\n", i, i, i).as_bytes());
    }

    let scenarios: [(&str, &str); 5] = [
        ("early reader exit (SIGPIPE)", "| head -c 20 > /dev/null; exit ${PIPESTATUS[0]}"),
        ("reader exits immediately", "| true; exit ${PIPESTATUS[0]}"),
        ("closed stdout", ">&-"),
        ("full device", "> /dev/full"),
        ("stdout to /dev/null", "> /dev/null"),
    ];
    for (name, tail) in scenarios.iter() {
        let (c_code, c_err) = run_shell(c_exe(), tail, &data);
        let (r_code, r_err) = run_shell(rust_exe(), tail, &data);
        assert_eq!(
            c_code, r_code,
            "exit status differs for [{}]: C={:?} Rust={:?}",
            name, c_code, r_code
        );
        assert_eq!(
            esc(&c_err),
            esc(&r_err),
            "stderr differs for [{}]",
            name
        );
    }
}

/// argv arguments that are not valid UTF-8 (the C code only ever does
/// `strcmp`, so arbitrary bytes must be compared byte-wise).
#[test]
fn p33_non_utf8_filters() {
    let data = b"1 L1 F1 AAA BBB one\n2 L2 F2 CCC DDD two\n";
    let odd: [&[u8]; 8] = [
        b"\xff",
        b"\xff\xfe",
        b"-\xff",
        b"L1\xff",
        b"\xc3",
        b"\x80\x80",
        b"\t",
        b"\x01",
    ];
    for (i, a) in odd.iter().enumerate() {
        for pos in 0..4 {
            let mut args = wildcards();
            args[pos] = a.to_vec();
            assert_same(&format!("p33/{}/{}", i, pos), &args, data);
        }
    }
    // a record whose comment holds high bytes, filtered with those bytes
    let data2 = b"1 L1 F1 AAA BBB \xff\xfe comment\n";
    for a in odd.iter() {
        let mut args = wildcards();
        args[0] = a.to_vec();
        assert_same("p33/comment", &args, data2);
    }
}
