//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives the lowest-level (and only) public entry point,
//! `parse_number`, through both `.so`s via `libloading`.

mod common;

use common::*;

/// Number of randomized cases per shaped row.
const N: usize = 3000;

/// Random digit string whose length is itself random in `lo..=hi`.
fn rand_digits(rng: &mut Rng, lo: u64, hi: u64) -> String {
    let n = rng.range_incl(lo, hi) as usize;
    digits(rng, n)
}

fn digits(rng: &mut Rng, n: usize) -> String {
    (0..n)
        .map(|_| (b'0' + rng.below(10) as u8) as char)
        .collect()
}

/// Run a scenario both with an in-allocation NUL terminator and with the run
/// ending exactly at `length` (poison after) — both are real shapes (A9).
fn both_terminations(label: &str, s: &str, item: ItemSeed, depth: usize) {
    assert_same(
        label,
        &Scenario::from_str_nul(s).item(item).depth(depth),
    );
    assert_same(
        label,
        &Scenario::from_str_no_term(s).item(item).depth(depth),
    );
    // Terminator that is a plain space (very common in real JSON input).
    let mut sc = Scenario::new(format!("{s} ").into_bytes());
    sc.item = item;
    sc.depth = depth;
    assert_same(label, &sc);
}

// ---------------------------------------------------------------- C1
#[test]
fn c1_plain_positive_int() {
    let mut rng = Rng::new(SEED);
    for _ in 0..N {
        let n = rng.range_incl(1, 9) as usize;
        let s = digits(&mut rng, n);
        both_terminations("C1", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_plain_negative_int() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        let n = rng.range_incl(1, 12) as usize;
        let s = format!("-{}", digits(&mut rng, n));
        both_terminations("C2", &s, random_item_seed(&mut rng), 1);
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_leading_plus_int() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..N {
        let n = rng.range_incl(1, 12) as usize;
        let s = format!("+{}", digits(&mut rng, n));
        both_terminations("C3", &s, random_item_seed(&mut rng), usize::MAX);
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_fraction_no_exponent() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..N {
        let sign = if rng.bool() { "-" } else { "" };
        let a = rand_digits(&mut rng, 0, 10);
        let b = rand_digits(&mut rng, 0, 20);
        let s = format!("{sign}{a}.{b}");
        both_terminations("C4", &s, random_item_seed(&mut rng), 3);
    }
}

// ---------------------------------------------------------------- C5 / C6
fn fraction_with_exponent(label: &str, seed: u64, e: char) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        let sign = if rng.bool() { "-" } else { "" };
        let a = rand_digits(&mut rng, 1, 6);
        let b = rand_digits(&mut rng, 1, 8);
        let x = rand_digits(&mut rng, 1, 3);
        let s = format!("{sign}{a}.{b}{e}{x}");
        both_terminations(label, &s, random_item_seed(&mut rng), 7);
    }
}

#[test]
fn c5_fraction_lower_e() {
    fraction_with_exponent("C5", SEED ^ 5, 'e');
}

#[test]
fn c6_fraction_upper_e() {
    fraction_with_exponent("C6", SEED ^ 6, 'E');
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_full_float_all_exponent_spellings() {
    let mut rng = Rng::new(SEED ^ 7);
    let spellings = ["e+", "e-", "E+", "E-"];
    for _ in 0..N {
        let sign = *rng.pick(&["", "-", "+"]);
        let a = rand_digits(&mut rng, 1, 8);
        let b = rand_digits(&mut rng, 0, 12);
        let sp = *rng.pick(&spellings);
        let x = rand_digits(&mut rng, 1, 3);
        let s = format!("{sign}{a}.{b}{sp}{x}");
        both_terminations("C7", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_int_mantissa_with_exponent() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..N {
        let sign = *rng.pick(&["", "-", "+"]);
        let a = rand_digits(&mut rng, 1, 10);
        let sp = *rng.pick(&["e", "E", "e+", "e-", "E+", "E-"]);
        let x = rand_digits(&mut rng, 1, 3);
        let s = format!("{sign}{a}{sp}{x}");
        both_terminations("C8", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C9 / C10
#[test]
fn c9_positive_infinity() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..N {
        let a = rand_digits(&mut rng, 1, 5);
        let exp = rng.range_incl(309, 400000);
        let sp = *rng.pick(&["e", "E", "e+", "E+"]);
        let s = format!("{a}.5{sp}{exp}");
        both_terminations("C9", &s, random_item_seed(&mut rng), 0);
    }
    for s in ["1e999", "9e308", "1.7976931348623159e308", "2e308", "1e4932"] {
        both_terminations("C9", s, ItemSeed::default(), 0);
    }
}

#[test]
fn c10_negative_infinity() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..N {
        let a = rand_digits(&mut rng, 1, 5);
        let exp = rng.range_incl(309, 400000);
        let sp = *rng.pick(&["e", "E", "e+", "E+"]);
        let s = format!("-{a}.5{sp}{exp}");
        both_terminations("C10", &s, random_item_seed(&mut rng), 0);
    }
    for s in ["-1e999", "-9e308", "-2e308", "-1e4932"] {
        both_terminations("C10", s, ItemSeed::default(), 0);
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_int_max_boundary() {
    let mut rng = Rng::new(SEED ^ 11);
    let bases: [i64; 9] = [
        2147483640, 2147483645, 2147483646, 2147483647, 2147483648, 2147483649, 2147483650,
        4294967295, 4294967296,
    ];
    let fracs = ["", ".0", ".1", ".4999999999", ".5", ".9", ".9999999999999999", ".99999999999"];
    for b in bases {
        for f in fracs {
            both_terminations("C11", &format!("{b}{f}"), ItemSeed::default(), 0);
            both_terminations("C11", &format!("+{b}{f}"), ItemSeed::default(), 0);
        }
    }
    // exponent-expressed boundary values
    for s in [
        "2.147483647e9",
        "2.1474836470000001e9",
        "2.147483646e9",
        "21474836.47e2",
        "2147483647e0",
        "2147483648e0",
        "0.2147483647e10",
    ] {
        both_terminations("C11", s, ItemSeed::default(), 0);
    }
    // randomized near-boundary sweep
    for _ in 0..N {
        let delta = rng.range_incl(0, 40) as i64 - 20;
        let v = 2147483647i64 + delta;
        let frac = rand_digits(&mut rng, 0, 6);
        let s = if frac.is_empty() {
            format!("{v}")
        } else {
            format!("{v}.{frac}")
        };
        both_terminations("C11", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_int_min_boundary() {
    let mut rng = Rng::new(SEED ^ 12);
    let bases: [i64; 8] = [
        -2147483640, -2147483645, -2147483646, -2147483647, -2147483648, -2147483649, -2147483650,
        -4294967296,
    ];
    let fracs = ["", ".0", ".1", ".5", ".9", ".9999999999999999", ".00000000001"];
    for b in bases {
        for f in fracs {
            both_terminations("C12", &format!("{b}{f}"), ItemSeed::default(), 0);
        }
    }
    for s in [
        "-2.147483648e9",
        "-2.1474836480000001e9",
        "-2.147483647e9",
        "-21474836.48e2",
        "-2147483648e0",
        "-0.2147483648e10",
    ] {
        both_terminations("C12", s, ItemSeed::default(), 0);
    }
    for _ in 0..N {
        let delta = rng.range_incl(0, 40) as i64 - 20;
        let v = -2147483648i64 + delta;
        let frac = rand_digits(&mut rng, 0, 6);
        let s = if frac.is_empty() {
            format!("{v}")
        } else {
            format!("{v}.{frac}")
        };
        both_terminations("C12", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_signed_zero() {
    let mut rng = Rng::new(SEED ^ 13);
    let fixed = [
        "0", "-0", "+0", "0.0", "-0.0", "+0.0", "0e0", "-0e0", "-0e5", "-0.0e-5", "0.000",
        "-0.000000", "00", "-00", "000.000", "-0E10", "0.0e999", "-0.0e999",
    ];
    for s in fixed {
        both_terminations("C13", s, ItemSeed::default(), 0);
    }
    for _ in 0..N {
        let sign = *rng.pick(&["", "-", "+"]);
        let z = "0".repeat(rng.range_incl(1, 6) as usize);
        let f = "0".repeat(rng.range_incl(0, 6) as usize);
        let s = if f.is_empty() {
            format!("{sign}{z}")
        } else {
            format!("{sign}{z}.{f}")
        };
        both_terminations("C13", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_subnormal_and_underflow() {
    let mut rng = Rng::new(SEED ^ 14);
    let fixed = [
        "1e-320", "4.9e-324", "5e-324", "2.4703282292062328e-324", "1e-999", "-1e-320",
        "-4.9e-324", "-1e-999", "1e-308", "2.2250738585072011e-308", "1e-400000",
    ];
    for s in fixed {
        both_terminations("C14", s, ItemSeed::default(), 0);
    }
    for _ in 0..N {
        let sign = if rng.bool() { "-" } else { "" };
        let a = rand_digits(&mut rng, 1, 4);
        let exp = rng.range_incl(300, 400);
        let s = format!("{sign}{a}.7e-{exp}");
        both_terminations("C14", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_high_precision_mantissa() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..N {
        let sign = if rng.bool() { "-" } else { "" };
        let a = rand_digits(&mut rng, 1, 20);
        let b = rand_digits(&mut rng, 17, 40);
        let s = format!("{sign}{a}.{b}");
        both_terminations("C15", &s, random_item_seed(&mut rng), 0);
    }
    // Classic hard-rounding strtod inputs.
    for s in [
        "0.500000000000000166533453693773481063544750213623046875",
        "2.22507385850720113605740979670913197593481954635164564802342610972482222202107694551652952390813508791414915891303962110687008643869459464552765720740782062174337998814106326732925355228688137214901298112245145188984905722230728525513315575501591439747639798341180199932396254828901710708185069063066665599493827577257201576306269066333264756530000924588831643303777979186961204949739037782970490505108060994073026293712895895000358379996720725430436028407889577179615094551674824347103070260914462157228988025818254518032570701886087211312807951223342628836862232150377566662250398253433597456888442390026549819838548794829220689472168983109969836584681402285424333066033985088644580400103493397042756718644338377048603786162277173854562306587467901408672332763671875e-308",
        "1.00000000000000011102230246251565404236316680908203125",
        "9007199254740993",
        "123456789012345678901234567890",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
    ] {
        both_terminations("C15", s, ItemSeed::default(), 0);
    }
}

// ---------------------------------------------------------------- C16
#[test]
fn c16_run_ends_at_length_with_poison_after() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..N {
        let n = rng.range_incl(1, 16) as usize;
        let run: String = (0..n).map(|_| *rng.pick(&ACCEPTED) as char).collect();
        // Allocate run + poison; length covers only the run.
        let mut data = run.clone().into_bytes();
        let real_len = data.len();
        for _ in 0..12 {
            data.push(*rng.pick(&ACCEPTED));
        }
        let sc = Scenario::new(data)
            .length(real_len)
            .item(random_item_seed(&mut rng));
        assert_same("C16", &sc);
    }
}

// ---------------------------------------------------------------- C17
#[test]
fn c17_random_terminator_byte() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..N {
        let n = rng.range_incl(1, 12) as usize;
        let run: String = (0..n).map(|_| *rng.pick(&ACCEPTED) as char).collect();
        let mut term = rng.byte();
        while is_accepted(term) {
            term = rng.byte();
        }
        let mut data = run.into_bytes();
        data.push(term);
        for _ in 0..8 {
            data.push(*rng.pick(&ACCEPTED));
        }
        let sc = Scenario::new(data).item(random_item_seed(&mut rng));
        assert_same("C17", &sc);
    }
}

// ---------------------------------------------------------------- C18
#[test]
fn c18_terminator_sweep_all_256() {
    for prefix in ["12", "1.5", "-3e2", "0", "+", ".", "e"] {
        for b in 0u16..=255 {
            let mut data = prefix.as_bytes().to_vec();
            data.push(b as u8);
            data.extend_from_slice(b"78\0");
            let sc = Scenario::new(data);
            assert_same("C18", &sc);
            // Same byte, but as the very first byte at `offset`.
            let mut data2 = vec![b as u8];
            data2.extend_from_slice(prefix.as_bytes());
            data2.push(0);
            assert_same("C18", &Scenario::new(data2));
        }
    }
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_interior_offset() {
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..N {
        let junk_len = rng.range_incl(1, 8) as usize;
        let junk: String = (0..junk_len).map(|_| *rng.pick(&ACCEPTED) as char).collect();
        let sign = *rng.pick(&["", "-", "+"]);
        let num = format!(
            "{sign}{}.{}",
            rand_digits(&mut rng, 1, 6),
            rand_digits(&mut rng, 0, 6)
        );
        let mut data = junk.clone().into_bytes();
        data.extend_from_slice(num.as_bytes());
        data.push(*rng.pick(&[b' ', b',', b']', b'}', 0u8, b'x']));
        data.extend_from_slice(b"55555");
        let sc = Scenario::new(data)
            .offset(junk_len)
            .item(random_item_seed(&mut rng))
            .depth(rng.next_u64() as usize);
        assert_same("C19", &sc);
    }
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_partial_strtod_consumption() {
    let mut rng = Rng::new(SEED ^ 20);
    let fixed = [
        "1.2.3", "1e", "1e+", "1e-", "1-2", "12e-", "1.2E", "5+", ".5.5", "1..2", "1.2e3.4",
        "0e", "0e+", "-1e", "+1e", "1E", "1E+", "3-4e5", "1.e", "1.2e+e3", "9e+-3", "1..",
        "1.2.3.4.5", "12345e", "0.0e", "-.5e", "1e1e1", "2E2E2", "1+2-3", "7..8e9",
    ];
    for s in fixed {
        both_terminations("C20", s, ItemSeed::default(), 0);
    }
    for _ in 0..N {
        // Build a valid number then append a dangling tail of accepted chars.
        let head = format!(
            "{}{}.{}",
            *rng.pick(&["", "-", "+"]),
            rand_digits(&mut rng, 1, 5),
            rand_digits(&mut rng, 0, 5)
        );
        let tail_len = rng.range_incl(1, 5) as usize;
        let tail: String = (0..tail_len)
            .map(|_| *rng.pick(&[b'.', b'e', b'E', b'+', b'-']) as char)
            .collect();
        let s = format!("{head}{tail}");
        both_terminations("C20", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C21
#[test]
fn c21_duplicated_and_interior_signs() {
    let mut rng = Rng::new(SEED ^ 21);
    let fixed = [
        "+-1", "-+2", "++3", "--4", "1+1", "1-1", "3-4e5", "-", "+", "+-", "-+", "--", "++",
        "-e5", "+e5", "e+5", "e-5", "+.5", "-.5", ".+5", ".-5", "1e++5", "1e--5", "1e+-5",
    ];
    for s in fixed {
        both_terminations("C21", s, ItemSeed::default(), 0);
    }
    for _ in 0..N {
        let n = rng.range_incl(1, 8) as usize;
        let s: String = (0..n)
            .map(|_| *rng.pick(&[b'+', b'-', b'1', b'2', b'e', b'E', b'.']) as char)
            .collect();
        both_terminations("C21", &s, random_item_seed(&mut rng), 0);
    }
}

// ---------------------------------------------------------------- C22
#[test]
fn c22_long_random_accepted_runs() {
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..400 {
        let n = rng.range_incl(256, 4096) as usize;
        let mut data: Vec<u8> = (0..n).map(|_| *rng.pick(&ACCEPTED)).collect();
        data.push(0);
        let sc = Scenario::new(data).item(random_item_seed(&mut rng));
        assert_same("C22", &sc);
    }
    // Very long pure-digit runs (mantissa far beyond double precision).
    for n in [100usize, 500, 1000, 4096] {
        let mut data = vec![b'9'; n];
        data.push(0);
        assert_same("C22", &Scenario::new(data));
        let mut data = vec![b'0'; n];
        data.extend_from_slice(b"1\0");
        assert_same("C22", &Scenario::new(data));
    }
}

// ---------------------------------------------------------------- C23
#[test]
fn c23_many_decimal_points() {
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..600 {
        let n = rng.range_incl(1, 512) as usize;
        let mut data: Vec<u8> = (0..n)
            .map(|_| if rng.below(3) == 0 { b'.' } else { *rng.pick(&ACCEPTED) })
            .collect();
        data.push(0);
        let sc = Scenario::new(data).item(random_item_seed(&mut rng));
        assert_same("C23", &sc);
    }
    for n in [1usize, 2, 3, 64, 300] {
        let mut data = vec![b'.'; n];
        data.push(0);
        assert_same("C23", &Scenario::new(data));
        let mut data = vec![b'1'];
        data.extend(std::iter::repeat(b'.').take(n));
        data.extend_from_slice(b"5\0");
        assert_same("C23", &Scenario::new(data));
    }
}

// ---------------------------------------------------------------- C24
#[test]
fn c24_depth_and_item_garbage_roundtrip() {
    let mut rng = Rng::new(SEED ^ 24);
    let depths = [0usize, 1, 2, 1000, usize::MAX, usize::MAX - 1];
    // Success path and failure path, with every depth and garbage item state.
    for s in ["123", "1.5e2", "", "x", ".", "-", "1e999", "-2147483649"] {
        for d in depths {
            for _ in 0..64 {
                let item = random_item_seed(&mut rng);
                assert_same("C24", &Scenario::from_str_nul(s).depth(d).item(item));
                let mut sc = Scenario::from_str_no_term(s).depth(d).item(item);
                if s.is_empty() {
                    sc.length = 0;
                }
                assert_same("C24", &sc);
            }
        }
    }
    // Explicit NaN / inf / signalling-NaN bit patterns pre-loaded into valuedouble.
    let poison_bits = [
        0x7FF8_0000_0000_0000u64, // quiet NaN
        0x7FF0_0000_0000_0001,    // signalling NaN
        0xFFF8_0000_0000_0000,    // negative quiet NaN
        0x7FF0_0000_0000_0000,    // +inf
        0xFFF0_0000_0000_0000,    // -inf
        0x0000_0000_0000_0001,    // smallest subnormal
        0x8000_0000_0000_0000,    // -0.0
        u64::MAX,
    ];
    for bits in poison_bits {
        for tv in [i32::MIN, -1, 0, 1, i32::MAX, 0x5A5A_5A5A] {
            let item = ItemSeed {
                type_: tv,
                valueint: tv,
                valuedouble_bits: bits,
            };
            for s in ["7", "", ".", "1e999"] {
                assert_same("C24", &Scenario::from_str_nul(s).item(item));
            }
        }
    }
}

// ---------------------------------------------------------------- C25
#[test]
fn c25_streaming_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..600 {
        // Build a separator-delimited list of numbers.
        let count = rng.range_incl(2, 8) as usize;
        let mut text = String::new();
        for k in 0..count {
            if k > 0 {
                text.push(*rng.pick(&[',', ' ', ':', ']', '\t', '\n']));
            }
            let sign = *rng.pick(&["", "-", "+"]);
            let a = rand_digits(&mut rng, 1, 8);
            let piece = match rng.below(4) {
                0 => format!("{sign}{a}"),
                1 => format!("{sign}{a}.{}", digits(&mut rng, 4)),
                2 => format!("{sign}{a}e{}", digits(&mut rng, 2)),
                _ => format!("{sign}{a}.{}E-{}", digits(&mut rng, 3), digits(&mut rng, 2)),
            };
            text.push_str(&piece);
        }
        text.push('\0');
        let bytes = text.into_bytes();

        // Drive both libraries through the whole stream, in lock-step, letting
        // each one advance its own offset from its own previous result.
        let mut c_data = bytes.clone();
        let mut r_data = bytes.clone();
        let c_base = c_data.as_mut_ptr();
        let r_base = r_data.as_mut_ptr();
        let mut c_buf = parse_buffer {
            content: c_base,
            length: bytes.len(),
            offset: 0,
            depth: 5,
        };
        let mut r_buf = parse_buffer {
            content: r_base,
            length: bytes.len(),
            offset: 0,
            depth: 5,
        };
        let cf = c_parse_number();
        let rf = rust_parse_number();
        for step in 0..(count * 3) {
            let mut c_item = cJSON {
                type_: POISON_TYPE,
                valueint: POISON_VALUEINT,
                valuedouble: f64::from_bits(POISON_DOUBLE_BITS),
            };
            let mut r_item = c_item;
            let cr = unsafe { cf(&mut c_item, &mut c_buf) };
            let rr = unsafe { rf(&mut r_item, &mut r_buf) };
            assert_eq!(
                (
                    cr,
                    c_item.type_,
                    c_item.valueint,
                    c_item.valuedouble.to_bits(),
                    c_buf.offset,
                    c_buf.length,
                    c_buf.depth
                ),
                (
                    rr,
                    r_item.type_,
                    r_item.valueint,
                    r_item.valuedouble.to_bits(),
                    r_buf.offset,
                    r_buf.length,
                    r_buf.depth
                ),
                "[C25] divergence at step {step} in {:?}",
                String::from_utf8_lossy(&bytes)
            );
            if cr == 0 {
                // Skip the separator byte and continue the stream.
                if c_buf.offset >= c_buf.length {
                    break;
                }
                c_buf.offset += 1;
                r_buf.offset += 1;
            }
        }
        assert_eq!(c_data, r_data, "[C25] input buffer was mutated differently");
    }
}

// ---------------------------------------------------------------- C26
#[test]
fn c26_length_shorter_than_allocation() {
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..N {
        let n = rng.range_incl(4, 32) as usize;
        let data: Vec<u8> = (0..n).map(|_| *rng.pick(&ACCEPTED)).collect();
        // length strictly inside the allocation, offset anywhere up to length.
        let length = rng.range_incl(0, n as u64) as usize;
        let offset = rng.range_incl(0, length.max(1) as u64) as usize;
        let sc = Scenario::new(data)
            .length(length)
            .offset(offset)
            .item(random_item_seed(&mut rng))
            .depth(rng.next_u64() as usize);
        assert_same("C26", &sc);
    }
}

// ---------------------------------------------------------------- C27
#[test]
fn c27_random_byte_soup() {
    let mut rng = Rng::new(SEED ^ 27);
    // Biased alphabet: accepted chars over-represented, plus common JSON bytes.
    let alphabet: Vec<u8> = {
        let mut v = Vec::new();
        for _ in 0..6 {
            v.extend_from_slice(&ACCEPTED);
        }
        v.extend_from_slice(b" \t\r\n,:[]{}\"\\/xXnNiIaA\0");
        v
    };
    for _ in 0..20_000 {
        let n = rng.range_incl(0, 64) as usize;
        let data: Vec<u8> = (0..n).map(|_| *rng.pick(&alphabet)).collect();
        let length = rng.range_incl(0, n as u64) as usize;
        let offset = rng.range_incl(0, (length + 2) as u64) as usize;
        let sc = Scenario::new(data)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item(random_item_seed(&mut rng));
        assert_same("C27", &sc);
    }
}

// ---------------------------------------------------------------- C28
#[test]
fn c28_random_full_byte_range_soup() {
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..20_000 {
        let n = rng.range_incl(0, 48) as usize;
        let data: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let length = rng.range_incl(0, n as u64) as usize;
        let offset = rng.range_incl(0, (length + 2) as u64) as usize;
        let sc = Scenario::new(data)
            .length(length)
            .offset(offset)
            .depth(rng.next_u64() as usize)
            .item(random_item_seed(&mut rng));
        assert_same("C28", &sc);
    }
}
