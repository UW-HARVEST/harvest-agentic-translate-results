//! Phase B — CONFIGS.md rows 10–29: `GetAlertData` / `FreeAlertData`, the
//! lowest-level public parser, driven directly (not through `Read_FileMon` or
//! `driver`) via the `.so` exports of both implementations.

mod common;

use common::*;
use std::ffi::c_int;

const MAIL: c_int = 0x001;
const EXEC: c_int = 0x002;
const READ_ALL: c_int = 0x004;
const READ_FAILED: c_int = 0x008;
const FP_SET: c_int = 0x010;

const ALL: &[Kind] = &[Kind::File, Kind::Mem];
const FILE_ONLY: &[Kind] = &[Kind::File];
const EVERY: &[Kind] = &[Kind::File, Kind::Mem, Kind::Pipe];

// ---------------------------------------------------------------------------
// Row 10 — one complete alert with every field populated
// ---------------------------------------------------------------------------

#[test]
fn cfg_10_one_complete_alert() {
    let mut rng = Rng::new(0x1010_2024);
    for i in 0..400 {
        let a = rand_alert(&mut rng, i % 2 == 0);
        diff_get_alert_data(&format!("complete#{i}"), &a.bytes(), 0, ALL, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 11 — minimal accepted alert: header + date/location, then EOF
// ---------------------------------------------------------------------------

#[test]
fn cfg_11_minimal_alert() {
    let mut rng = Rng::new(0x1111_2024);
    for i in 0..300 {
        let id = rng.token_len(1, 8);
        let loc = rng.token_len(1, 20);
        let mut b = Vec::new();
        b.extend_from_slice(b"** Alert ");
        b.extend_from_slice(&id);
        b.extend_from_slice(b": whatever\n");
        b.extend_from_slice(b"2006 Apr 13 16:15:17 ");
        b.extend_from_slice(&loc);
        b.push(b'\n');
        diff_get_alert_data(&format!("minimal#{i}"), &b, 0, EVERY, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — two alerts: the first call must fseek back onto the second header
// ---------------------------------------------------------------------------

#[test]
fn cfg_12_two_alerts_fseek_back() {
    let mut rng = Rng::new(0x1212_2024);
    for i in 0..300 {
        let a = rand_alert(&mut rng, false);
        let b = rand_alert(&mut rng, false);
        let mut bytes = a.bytes();
        b.render(&mut bytes);
        diff_get_alert_data(&format!("two#{i}"), &bytes, 0, ALL, 4);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — N alerts, walked to exhaustion
// ---------------------------------------------------------------------------

#[test]
fn cfg_13_many_alerts() {
    let mut rng = Rng::new(0x1313_2024);
    for i in 0..150 {
        let n = 3 + rng.below(4) as usize;
        let mut bytes = Vec::new();
        for _ in 0..n {
            let mail = rng.below(2) == 0;
            rand_alert(&mut rng, mail).render(&mut bytes);
        }
        diff_get_alert_data(&format!("many#{i}"), &bytes, 0, ALL, n + 3);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — CRALERT_MAIL_SET with a matching `mail` header
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_mail_set_matching() {
    let mut rng = Rng::new(0x1414_2024);
    for i in 0..300 {
        let a = rand_alert(&mut rng, true);
        diff_get_alert_data(&format!("mail#{i}"), &a.bytes(), MAIL, ALL, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 15 — CRALERT_MAIL_SET on a mixed mail / non-mail file
// ---------------------------------------------------------------------------

#[test]
fn cfg_15_mail_set_mixed_file() {
    let mut rng = Rng::new(0x1515_2024);
    for i in 0..200 {
        let n = 2 + rng.below(5) as usize;
        let mut bytes = Vec::new();
        for _ in 0..n {
            let mail = rng.below(2) == 0;
            rand_alert(&mut rng, mail).render(&mut bytes);
        }
        for flag in [0, MAIL] {
            diff_get_alert_data(&format!("mixed#{i}"), &bytes, flag, ALL, n + 3);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — every subset of the five CRALERT_* bits on the same alert
// ---------------------------------------------------------------------------

#[test]
fn cfg_16_all_flag_subsets() {
    let mut rng = Rng::new(0x1616_2024);
    let bits = [MAIL, EXEC, READ_ALL, READ_FAILED, FP_SET];
    for i in 0..12 {
        let mut bytes = Vec::new();
        rand_alert(&mut rng, i % 2 == 0).render(&mut bytes);
        rand_alert(&mut rng, i % 2 == 1).render(&mut bytes);
        for mask in 0u32..32 {
            let mut flag = 0;
            for (b, bit) in bits.iter().enumerate() {
                if mask & (1 << b) != 0 {
                    flag |= *bit;
                }
            }
            diff_get_alert_data(&format!("flags#{i}/{flag:#x}"), &bytes, flag, FILE_ONLY, 4);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17 & 18 — header without `-` / with `-` and leading-space runs
// ---------------------------------------------------------------------------

#[test]
fn cfg_17_header_without_dash() {
    let mut rng = Rng::new(0x1717_2024);
    for i in 0..200 {
        let id = rng.token_len(1, 6);
        let mut b = Vec::new();
        b.extend_from_slice(b"** Alert ");
        b.extend_from_slice(&id);
        b.extend_from_slice(b": mail no dash here\n");
        b.extend_from_slice(b"2006 Apr 13 16:15:17 /var/log/x\n");
        b.extend_from_slice(b"Rule: 1002 (level) 7 -> 'nope'\n");
        for flag in [0, MAIL] {
            diff_get_alert_data(&format!("nodash#{i}"), &b, flag, ALL, 3);
        }
    }
}

#[test]
fn cfg_18_header_dash_leading_spaces() {
    let mut rng = Rng::new(0x1818_2024);
    let seps: [&[u8]; 6] = [b"-", b"- ", b"-  ", b"-\t", b"-   \t ", b"-     "];
    for i in 0..80 {
        let id = rng.token_len(1, 6);
        let grp = rng.token_len(0, 15);
        for sep in seps {
            let mut b = Vec::new();
            b.extend_from_slice(b"** Alert ");
            b.extend_from_slice(&id);
            b.extend_from_slice(b": mail ");
            b.extend_from_slice(sep);
            b.extend_from_slice(&grp);
            b.push(b'\n');
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /var/log/x\n");
            for flag in [0, MAIL] {
                diff_get_alert_data(&format!("dash#{i}"), &b, flag, ALL, 3);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19, 20, 21 — the syscheck sub-mode
// ---------------------------------------------------------------------------

fn syscheck_alert(group: &[u8], first_body: &[u8], extra: &[&[u8]]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"** Alert 1234.5: mail - ");
    b.extend_from_slice(group);
    b.push(b'\n');
    b.extend_from_slice(b"2006 Apr 13 16:15:17 (agent) 10.0.0.1->syscheck\n");
    b.extend_from_slice(b"Rule: 550 (level) 7 -> 'Integrity checksum changed.'\n");
    b.extend_from_slice(first_body);
    b.push(b'\n');
    for e in extra {
        b.extend_from_slice(e);
        b.push(b'\n');
    }
    b
}

#[test]
fn cfg_19_syscheck_filename() {
    let mut rng = Rng::new(0x1919_2024);
    for i in 0..250 {
        let path = rng.token_len(1, 60);
        let mut line = b"Integrity checksum changed for: '".to_vec();
        line.extend_from_slice(&path);
        line.push(b'\'');
        let b = syscheck_alert(b"ossec,syscheck,", &line, &[b"Old md5sum was: aaa"]);
        for flag in [0, MAIL] {
            diff_get_alert_data(&format!("syscheck#{i}"), &b, flag, ALL, 3);
        }
    }
}

#[test]
fn cfg_20_syscheck_flag_consumed_by_other_body_line() {
    let mut rng = Rng::new(0x2020_2024);
    for i in 0..200 {
        let path = rng.token_len(1, 40);
        let mut integ = b"Integrity checksum changed for: '".to_vec();
        integ.extend_from_slice(&path);
        integ.push(b'\'');
        let first = rng.token_len(1, 30);
        let b = syscheck_alert(b"syscheck", &first, &[&integ]);
        diff_get_alert_data(&format!("sysconsumed#{i}"), &b, 0, ALL, 3);
    }
}

#[test]
fn cfg_21_syscheck_substring_variants() {
    let groups: [&[u8]; 8] = [
        b"syscheck",
        b"xsyscheckx",
        b"ossec,syscheck,",
        b"SYSCHECK",
        b"syschec",
        b"ysscheck",
        b"aaasyscheck",
        b"syscheckk",
    ];
    let mut rng = Rng::new(0x2121_2024);
    for (i, g) in groups.iter().enumerate() {
        for _ in 0..20 {
            let path = rng.token_len(0, 30);
            let mut line = b"Integrity checksum changed for: '".to_vec();
            line.extend_from_slice(&path);
            line.push(b'\'');
            let b = syscheck_alert(g, &line, &[]);
            diff_get_alert_data(&format!("sysgrp#{i}"), &b, 0, ALL, 3);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — duplicate token lines exercise the os_free + re-strdup paths
// ---------------------------------------------------------------------------

#[test]
fn cfg_22_duplicate_token_lines() {
    let mut rng = Rng::new(0x2222_2024);
    for i in 0..250 {
        let mut b = Vec::new();
        b.extend_from_slice(b"** Alert 99.7: mail - dupgroup\n");
        b.extend_from_slice(b"2006 Apr 13 16:15:17 /var/log/y\n");
        let reps = 2 + rng.below(4) as usize;
        for _ in 0..reps {
            let c = rng.token_len(1, 12);
            let ip1 = rng.token_len(1, 12);
            let ip2 = rng.token_len(1, 12);
            let u = rng.token_len(1, 10);
            b.extend_from_slice(b"Rule: ");
            b.extend_from_slice(format!("{}", rng.below(99999)).as_bytes());
            b.extend_from_slice(b" (level) ");
            b.extend_from_slice(format!("{}", rng.below(16)).as_bytes());
            b.extend_from_slice(b" -> '");
            b.extend_from_slice(&c);
            b.extend_from_slice(b"'\n");
            b.extend_from_slice(b"Src IP: ");
            b.extend_from_slice(&ip1);
            b.push(b'\n');
            b.extend_from_slice(b"Dst IP: ");
            b.extend_from_slice(&ip2);
            b.push(b'\n');
            b.extend_from_slice(b"User: ");
            b.extend_from_slice(&u);
            b.push(b'\n');
            b.extend_from_slice(b"Src Port: ");
            b.extend_from_slice(format!("{}", rng.below(70000)).as_bytes());
            b.push(b'\n');
            b.extend_from_slice(b"Dst Port: ");
            b.extend_from_slice(format!("{}", rng.below(70000)).as_bytes());
            b.push(b'\n');
        }
        diff_get_alert_data(&format!("dup#{i}"), &b, 0, ALL, 3);
    }
}

// ---------------------------------------------------------------------------
// Rows 23 & 24 — atoi value ranges for ports / rule / level
// ---------------------------------------------------------------------------

const NUMS: &[&str] = &[
    "0",
    "1",
    "-1",
    "-0",
    "65535",
    "65536",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967295",
    "4294967296",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "99999999999999999999999999",
    "-99999999999999999999999999",
    "abc",
    "",
    " 12",
    "  -34",
    "+56",
    "12abc",
    "0x1f",
    "007",
    "\t9",
    "1 2",
    ".5",
    "-",
    "+",
];

#[test]
fn cfg_23_port_atoi_ranges() {
    for (i, sp) in NUMS.iter().enumerate() {
        for (j, dp) in NUMS.iter().enumerate() {
            let mut b = Vec::new();
            b.extend_from_slice(b"** Alert 1.2: mail - g\n");
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
            b.extend_from_slice(b"Src Port: ");
            b.extend_from_slice(sp.as_bytes());
            b.push(b'\n');
            b.extend_from_slice(b"Dst Port: ");
            b.extend_from_slice(dp.as_bytes());
            b.push(b'\n');
            diff_get_alert_data(&format!("port#{i}/{j}"), &b, 0, FILE_ONLY, 2);
        }
    }
}

#[test]
fn cfg_24_rule_level_atoi_ranges() {
    for (i, rl) in NUMS.iter().enumerate() {
        for (j, lv) in NUMS.iter().enumerate() {
            let mut b = Vec::new();
            b.extend_from_slice(b"** Alert 1.2: mail - g\n");
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
            b.extend_from_slice(b"Rule: ");
            b.extend_from_slice(rl.as_bytes());
            b.extend_from_slice(b" (level) ");
            b.extend_from_slice(lv.as_bytes());
            b.extend_from_slice(b" -> 'c'\n");
            diff_get_alert_data(&format!("rule#{i}/{j}"), &b, 0, FILE_ONLY, 2);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — comment containing embedded quotes (strrchr takes the LAST one)
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_comment_embedded_quotes() {
    let bodies: [&[u8]; 10] = [
        b"a'b",
        b"'",
        b"''",
        b"'''",
        b"a''b'c",
        b"no quote at all",
        b"'leading",
        b"trailing'",
        b"a'b'c'd'e",
        b"'''''''''",
    ];
    let mut rng = Rng::new(0x2525_2024);
    for (i, body) in bodies.iter().enumerate() {
        for _ in 0..10 {
            let mut b = Vec::new();
            b.extend_from_slice(b"** Alert 1.2: mail - g\n");
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
            b.extend_from_slice(b"Rule: ");
            b.extend_from_slice(format!("{}", rng.below(9999)).as_bytes());
            b.extend_from_slice(b" (level) 7 -> '");
            b.extend_from_slice(body);
            b.push(b'\n');
            diff_get_alert_data(&format!("quote#{i}"), &b, 0, FILE_ONLY, 2);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — date/location lines with many colons and spaces
// ---------------------------------------------------------------------------

#[test]
fn cfg_26_dateline_colon_space_shapes() {
    let lines: [&[u8]; 14] = [
        b"2006 Apr 13 16:15:17 /var/log/auth.log",
        b":x",
        b": x",
        b"a: b: c: d",
        b"a:b c:d",
        b"a :b",
        b"   :   ",
        b"a: ",
        b"::::  ::::",
        b"x:y z",
        b"no-colon but spaces",
        b"lots   of   spaces:   and   colons:   here",
        b"tab\t:\tafter",
        b"trailing colon:",
    ];
    for (i, l) in lines.iter().enumerate() {
        let mut b = Vec::new();
        b.extend_from_slice(b"** Alert 3.4: mail - g\n");
        b.extend_from_slice(l);
        b.push(b'\n');
        b.extend_from_slice(b"Rule: 7 (level) 3 -> 'c'\n");
        diff_get_alert_data(&format!("dateline#{i}"), &b, 0, FILE_ONLY, 2);
    }
}

// ---------------------------------------------------------------------------
// Row 27 — no trailing newline at EOF
// ---------------------------------------------------------------------------

#[test]
fn cfg_27_no_trailing_newline() {
    let mut rng = Rng::new(0x2727_2024);
    for i in 0..300 {
        let a = rand_alert(&mut rng, false);
        let mut bytes = a.bytes();
        while bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        diff_get_alert_data(&format!("nonl#{i}"), &bytes, 0, ALL, 3);
    }
    // header-only / dateline-only with no newline
    for raw in [
        &b"** Alert 1.2: mail - g"[..],
        &b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc"[..],
        &b"** Alert 1.2: mail - g\n2006 Apr 13 16:15:17 /loc\nRule: 1 (level) 2 -> 'c'"[..],
        &b"** Alert"[..],
        &b"** Alert "[..],
        &b"** Alert x"[..],
    ] {
        diff_get_alert_data("nonl-short", raw, 0, ALL, 3);
    }
}

// ---------------------------------------------------------------------------
// Row 28 — lines at and beyond the fgets (OS_MAXSTR) boundary
// ---------------------------------------------------------------------------

#[test]
fn cfg_28_fgets_boundary_lines() {
    let mut rng = Rng::new(0x2828_2024);
    for &total in &[1021usize, 1022, 1023, 1024, 1025, 1026, 2047, 2048, 4096] {
        for which in 0..4 {
            let mut b = Vec::new();
            let pad = |n: usize, rng: &mut Rng| rng.token(n);
            match which {
                // long header line
                0 => {
                    let mut h = b"** Alert 1.2: mail - g".to_vec();
                    while h.len() < total {
                        h.push(b'g');
                    }
                    b.extend_from_slice(&h);
                    b.push(b'\n');
                    b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
                }
                // long date/location line
                1 => {
                    b.extend_from_slice(b"** Alert 1.2: mail - g\n");
                    let mut d = b"2006 Apr 13 16:15:17 ".to_vec();
                    while d.len() < total {
                        d.push(b'L');
                    }
                    b.extend_from_slice(&d);
                    b.push(b'\n');
                }
                // long Rule: line
                2 => {
                    b.extend_from_slice(b"** Alert 1.2: mail - g\n");
                    b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
                    let mut rl = b"Rule: 42 (level) 9 -> '".to_vec();
                    while rl.len() < total - 1 {
                        rl.push(b'C');
                    }
                    rl.push(b'\'');
                    b.extend_from_slice(&rl);
                    b.push(b'\n');
                }
                // long body line then a second alert
                _ => {
                    b.extend_from_slice(b"** Alert 1.2: mail - g\n");
                    b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
                    b.extend_from_slice(&pad(total, &mut rng));
                    b.push(b'\n');
                    b.extend_from_slice(b"** Alert 9.9: mail - h\n");
                    b.extend_from_slice(b"2007 May 14 01:02:03 /loc2\n");
                }
            }
            diff_get_alert_data(&format!("long#{total}/{which}"), &b, 0, ALL, 4);
        }
    }
    // Long token lines: `Src IP: ` etc. spilling over the boundary
    for &total in &[1022usize, 1023, 1024, 1025] {
        for tag in [
            &b"Src IP: "[..],
            &b"Src Port: "[..],
            &b"Dst IP: "[..],
            &b"Dst Port: "[..],
            &b"User: "[..],
        ] {
            let mut line = tag.to_vec();
            while line.len() < total {
                line.push(b'7');
            }
            let mut b = Vec::new();
            b.extend_from_slice(b"** Alert 1.2: mail - g\n");
            b.extend_from_slice(b"2006 Apr 13 16:15:17 /loc\n");
            b.extend_from_slice(&line);
            b.push(b'\n');
            diff_get_alert_data("long-token", &b, 0, ALL, 3);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 29 — the same bytes served from every stream kind
// ---------------------------------------------------------------------------

#[test]
fn cfg_29_stream_kinds_agree() {
    let mut rng = Rng::new(0x2929_2024);
    for i in 0..150 {
        // Single alert only: a second header on a pipe hits the fseek error
        // path, which is ERRORS.md row 5.
        let a = rand_alert(&mut rng, false);
        diff_get_alert_data(&format!("kinds#{i}"), &a.bytes(), 0, EVERY, 2);
    }
}

// ---------------------------------------------------------------------------
// Extra: structured fuzzing over the whole line grammar. Not a CONFIGS row of
// its own — it randomly walks the cross-product of every shape above, which is
// what catches value-dependent and index bugs.
// ---------------------------------------------------------------------------

fn fuzz_line(rng: &mut Rng) -> Vec<u8> {
    match rng.below(24) {
        0 => {
            let mut v = b"** Alert ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 8));
            v.push(b':');
            v.extend_from_slice(&rng.token_len(0, 4));
            v.push(b' ');
            v.extend_from_slice(if rng.below(2) == 0 { b"mail" } else { b"nope" });
            if rng.below(2) == 0 {
                v.extend_from_slice(b" - ");
                v.extend_from_slice(&rng.token_len(0, 12));
                if rng.below(3) == 0 {
                    v.extend_from_slice(b"syscheck");
                }
            }
            v
        }
        1 => b"** Alert".to_vec(),
        2 => b"** Alert ".to_vec(),
        3 => {
            let mut v = b"** Alert ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 10)); // no colon
            v
        }
        4 => {
            let mut v = b"** Alert".to_vec();
            v.push(b':');
            v.extend_from_slice(&rng.token_len(0, 6));
            v
        }
        5 => {
            let mut v = rng.token_len(0, 10);
            v.push(b':');
            v.push(b' ');
            v.extend_from_slice(&rng.token_len(0, 10));
            v
        }
        6 => rng.token_len(0, 20), // no colon at all
        7 => {
            let mut v = rng.token_len(0, 8);
            v.push(b':');
            v
        }
        8 => {
            let mut v = b"Rule: ".to_vec();
            v.extend_from_slice(NUMS[rng.below(NUMS.len() as u64) as usize].as_bytes());
            match rng.below(5) {
                0 => {}
                1 => v.extend_from_slice(b" x"),
                2 => {
                    v.extend_from_slice(b" (level) ");
                    v.extend_from_slice(NUMS[rng.below(NUMS.len() as u64) as usize].as_bytes());
                }
                3 => {
                    v.extend_from_slice(b" (level) 7 -> '");
                    v.extend_from_slice(&rng.token_len(0, 15));
                }
                _ => {
                    v.extend_from_slice(b" (level) ");
                    v.extend_from_slice(NUMS[rng.below(NUMS.len() as u64) as usize].as_bytes());
                    v.extend_from_slice(b" -> '");
                    v.extend_from_slice(&rng.token_len(0, 15));
                    v.push(b'\'');
                }
            }
            v
        }
        9 => {
            let mut v = b"Src IP: ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 16));
            v
        }
        10 => {
            let mut v = b"Dst IP: ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 16));
            v
        }
        11 => {
            let mut v = b"User: ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 16));
            v
        }
        12 => {
            let mut v = b"Src Port: ".to_vec();
            v.extend_from_slice(NUMS[rng.below(NUMS.len() as u64) as usize].as_bytes());
            v
        }
        13 => {
            let mut v = b"Dst Port: ".to_vec();
            v.extend_from_slice(NUMS[rng.below(NUMS.len() as u64) as usize].as_bytes());
            v
        }
        14 => {
            // integrity line with a non-empty tail (empty tail is ERRORS row 30)
            let mut v = b"Integrity checksum changed for: '".to_vec();
            v.extend_from_slice(&rng.token_len(1, 25));
            if rng.below(2) == 0 {
                v.push(b'\'');
            }
            v
        }
        15 => b"Old md5sum was: 0123456789abcdef".to_vec(),
        16 => b"New sha256sum is : deadbeef".to_vec(),
        17 => b"".to_vec(),
        18 => b" ".to_vec(),
        19 => {
            let n = 1 + rng.below(24) as usize;
            rng.wild(n)
        }
        20 => {
            // truncated versions of each token, one byte short
            let toks: [&[u8]; 6] = [
                b"Rule:", b"Src IP:", b"Src Port:", b"Dst IP:", b"Dst Port:", b"User:",
            ];
            toks[rng.below(6) as usize].to_vec()
        }
        21 => {
            // token prefixes with the trailing space but nothing else
            let toks: [&[u8]; 6] = [
                b"Rule: ",
                b"Src IP: ",
                b"Src Port: ",
                b"Dst IP: ",
                b"Dst Port: ",
                b"User: ",
            ];
            toks[rng.below(6) as usize].to_vec()
        }
        22 => {
            let n = 1000 + rng.below(60) as usize;
            let mut v = rng.token(n);
            if rng.below(3) == 0 {
                v.splice(0..0, b"** Alert 5.5: mail - g".iter().cloned());
            }
            v
        }
        _ => {
            let mut v = b"** Alert ".to_vec();
            v.extend_from_slice(&rng.token_len(0, 5));
            v.push(b':');
            v
        }
    }
}

#[test]
fn cfg_fuzz_line_grammar() {
    let mut rng = Rng::new(0xF0F0_2024);
    for i in 0..1200 {
        let nlines = 1 + rng.below(9) as usize;
        let mut bytes = Vec::new();
        for _ in 0..nlines {
            bytes.extend_from_slice(&fuzz_line(&mut rng));
            // sometimes drop the newline (only meaningful on the last line)
            if rng.below(16) != 0 {
                bytes.push(b'\n');
            }
        }
        let flag = match rng.below(4) {
            0 => 0,
            1 => MAIL,
            2 => MAIL | READ_ALL | FP_SET,
            _ => rng.i32_any(),
        };
        diff_get_alert_data(&format!("fuzz#{i}"), &bytes, flag, FILE_ONLY, nlines + 2);
    }
}

/// Pure random bytes — no grammar at all.
#[test]
fn cfg_fuzz_random_bytes() {
    let mut rng = Rng::new(0xBEEF_2024);
    for i in 0..600 {
        let n = rng.below(600) as usize;
        let bytes = rng.wild_nl(n);
        let flag = if i % 3 == 0 { MAIL } else { 0 };
        diff_get_alert_data(&format!("rand#{i}"), &bytes, flag, FILE_ONLY, 6);
    }
}

/// Random bytes with `** Alert` markers sprinkled in, so the state machine
/// actually advances past `_r == 0`.
#[test]
fn cfg_fuzz_random_bytes_with_headers() {
    let mut rng = Rng::new(0xCAFE_2024);
    for i in 0..600 {
        let mut bytes = Vec::new();
        for _ in 0..(1 + rng.below(8)) {
            match rng.below(3) {
                0 => bytes.extend_from_slice(b"** Alert "),
                1 => bytes.extend_from_slice(b"** Alert 1.2: mail - syscheck"),
                _ => {}
            }
            let n = rng.below(40) as usize;
            bytes.extend_from_slice(&rng.wild(n));
            bytes.push(b'\n');
        }
        diff_get_alert_data(&format!("randhdr#{i}"), &bytes, 0, FILE_ONLY, 8);
    }
}

// ---------------------------------------------------------------------------
// Row 50 — FreeAlertData on caller-built structs (mixed NULL / non-NULL)
// ---------------------------------------------------------------------------

#[test]
fn cfg_50_free_alert_data_mixed_null() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x5050_2024);
    for _ in 0..200 {
        let mask = rng.below(1 << 9);
        for api in [c, r] {
            unsafe {
                let p = malloc(size_of::<AlertData>()) as *mut AlertData;
                std::ptr::write_bytes(p as *mut u8, 0, size_of::<AlertData>());
                let slots: [*mut *mut std::ffi::c_char; 9] = [
                    &raw mut (*p).alertid,
                    &raw mut (*p).date,
                    &raw mut (*p).location,
                    &raw mut (*p).comment,
                    &raw mut (*p).group,
                    &raw mut (*p).srcip,
                    &raw mut (*p).dstip,
                    &raw mut (*p).user,
                    &raw mut (*p).filename,
                ];
                for (i, slot) in slots.iter().enumerate() {
                    if mask & (1 << i) != 0 {
                        let s = b"abc\0";
                        let q = malloc(4) as *mut u8;
                        std::ptr::copy_nonoverlapping(s.as_ptr(), q, 4);
                        **slot = q as *mut std::ffi::c_char;
                    }
                }
                (*p).rule = rng.next_u64() as u32;
                (*p).level = rng.next_u64() as u32;
                (*p).srcport = rng.i32_any();
                (*p).dstport = rng.i32_any();
                (api.FreeAlertData)(p);
            }
        }
    }
}
