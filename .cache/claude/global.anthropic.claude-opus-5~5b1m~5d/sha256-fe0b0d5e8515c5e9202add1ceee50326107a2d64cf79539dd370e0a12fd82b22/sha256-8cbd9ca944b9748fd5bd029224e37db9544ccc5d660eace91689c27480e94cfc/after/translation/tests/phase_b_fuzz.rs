//! Phase B, wide randomized sweep. Complements `CONFIGS.md` rows 43/44 with a
//! much larger input space: mutated well-formed alerts, byte-level noise, and
//! adversarial shapes around every `strncmp` prefix and every `OS_MAXSTR`
//! boundary. Fixed seeds, so failures reproduce exactly.

mod common;
use common::*;

use std::os::raw::c_int;

const PREFIXES: [&[u8]; 8] = [
    b"** Alert",
    b"Rule: ",
    b"Src IP: ",
    b"Src Port: ",
    b"Dst IP: ",
    b"Dst Port: ",
    b"User: ",
    b"Integrity checksum changed for: '",
];

const FLAGS: [c_int; 6] = [
    0,
    CRALERT_MAIL_SET,
    CRALERT_READ_ALL,
    CRALERT_FP_SET,
    CRALERT_MAIL_SET | CRALERT_READ_ALL | CRALERT_FP_SET,
    -1,
];

#[track_caller]
fn cmp_fuzz(tag: &str, bytes: &[u8], flag: c_int) {
    let b = both();
    let d = tmp_root();
    unsafe {
        let c = drain(&b.c, &d, tag, bytes, flag, 24);
        let r = drain(&b.rs, &d, tag, bytes, flag, 24);
        if c != r {
            panic!(
                "fuzz divergence [{tag}] flag={flag:#x}\n--- input ({} bytes) ---\n{:?}\n--- C ---\n{c:#?}\n--- Rust ---\n{r:#?}",
                bytes.len(),
                String::from_utf8_lossy(bytes)
            );
        }
    }
}

/// Prefix-boundary fuzz: every recognized prefix, truncated and extended one
/// byte at a time, so the `strncmp(prefix, str, N)` comparisons are probed at
/// exactly N-1, N and N+1 available bytes.
#[test]
fn fuzz_a_prefix_boundaries() {
    let mut rng = Rng::new(0xA1A1_A1A1_A1A1_A1A1);
    let mut n = 0usize;
    for p in PREFIXES.iter() {
        for cut in 0..=p.len() {
            for extra in [&b""[..], b"x", b" ", b"'", b"0", b" 1 2 3 'c'", b":"] {
                let mut line = p[..cut].to_vec();
                line.extend_from_slice(extra);

                // as the header line
                let mut f1 = line.clone();
                f1.extend_from_slice(b"\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n");
                cmp_fuzz(&format!("fa{n}"), &f1, *rng.pick(&FLAGS));
                n += 1;

                // as a body line of a valid alert
                let mut f2 = b"** Alert 1500000000.1: mail - ossec,syscheck,\n".to_vec();
                f2.extend_from_slice(b"2006 Apr 13 16:15:17 h->/x\n");
                f2.extend_from_slice(&line);
                f2.extend_from_slice(b"\n\n");
                cmp_fuzz(&format!("fb{n}"), &f2, *rng.pick(&FLAGS));
                n += 1;

                // as the date/location line
                let mut f3 = b"** Alert 1500000000.1: mail - ossec,syscheck,\n".to_vec();
                f3.extend_from_slice(&line);
                f3.extend_from_slice(b"\nRule: 1 (level 2) -> 'c'\n\n");
                cmp_fuzz(&format!("fc{n}"), &f3, *rng.pick(&FLAGS));
                n += 1;
            }
        }
    }
    assert!(n > 1000, "expected a wide sweep, only did {n}");
}

/// Mutation fuzz: take a well-formed multi-alert file and randomly corrupt it
/// (byte flips, deletions, insertions, line duplication / removal / truncation).
#[test]
fn fuzz_b_mutated_alerts() {
    let mut rng = Rng::new(0xB2B2_B2B2_B2B2_B2B2);
    let base = {
        let mut v = Vec::new();
        for i in 0..3 {
            v.extend_from_slice(b"** Alert 15000000");
            v.extend_from_slice(format!("{i:02}").as_bytes());
            v.extend_from_slice(b".1: mail - ossec,syscheck,\n");
            v.extend_from_slice(b"2006 Apr 13 16:15:17 host->/var/log/auth.log\n");
            v.extend_from_slice(b"Rule: 5715 (level 5) -> 'sshd auth ok.'\n");
            v.extend_from_slice(b"Src IP: 192.168.1.1\nSrc Port: 4321\n");
            v.extend_from_slice(b"Dst IP: 10.0.0.5\nDst Port: 22\nUser: root\n");
            v.extend_from_slice(b"Integrity checksum changed for: '/etc/passwd'\n");
            v.extend_from_slice(b"trailing log line\n\n");
        }
        v
    };

    for i in 0..1500 {
        let mut f = base.clone();
        for _ in 0..(1 + rng.below(6)) {
            match rng.below(7) {
                0 => {
                    // byte flip
                    if !f.is_empty() {
                        let k = rng.below(f.len());
                        f[k] = (rng.next_u64() & 0xff) as u8;
                    }
                }
                1 => {
                    // byte delete
                    if !f.is_empty() {
                        let k = rng.below(f.len());
                        f.remove(k);
                    }
                }
                2 => {
                    // byte insert
                    let k = rng.below(f.len() + 1);
                    let c = *rng.pick(&[b'\n', b' ', b':', b'\'', b'-', b'*', b'0', 0xffu8]);
                    f.insert(k, c);
                }
                3 => {
                    // duplicate a line
                    let lines: Vec<Vec<u8>> = f.split(|&c| c == b'\n').map(|s| s.to_vec()).collect();
                    if lines.len() > 1 {
                        let k = rng.below(lines.len());
                        let mut out = Vec::new();
                        for (j, l) in lines.iter().enumerate() {
                            out.extend_from_slice(l);
                            out.push(b'\n');
                            if j == k {
                                out.extend_from_slice(l);
                                out.push(b'\n');
                            }
                        }
                        f = out;
                    }
                }
                4 => {
                    // drop a line
                    let lines: Vec<Vec<u8>> = f.split(|&c| c == b'\n').map(|s| s.to_vec()).collect();
                    if lines.len() > 1 {
                        let k = rng.below(lines.len());
                        let mut out = Vec::new();
                        for (j, l) in lines.iter().enumerate() {
                            if j != k {
                                out.extend_from_slice(l);
                                out.push(b'\n');
                            }
                        }
                        f = out;
                    }
                }
                5 => {
                    // truncate
                    let k = rng.below(f.len() + 1);
                    f.truncate(k);
                }
                _ => {
                    // pad a line past OS_MAXSTR
                    let k = rng.below(f.len() + 1);
                    let pad = vec![*rng.pick(&[b'Z', b' ', b'\'']); 900 + rng.below(300)];
                    for (o, c) in pad.into_iter().enumerate() {
                        f.insert(k + o, c);
                    }
                }
            }
        }
        cmp_fuzz(&format!("fm{i}"), &f, *rng.pick(&FLAGS));
    }
}

/// Pure byte noise, including embedded NULs and no trailing newline.
#[test]
fn fuzz_c_raw_bytes() {
    let mut rng = Rng::new(0xC3C3_C3C3_C3C3_C3C3);
    for i in 0..800 {
        let n = rng.below(1400);
        let mut f: Vec<u8> = Vec::with_capacity(n);
        for _ in 0..n {
            // bias towards structurally interesting bytes
            let c = match rng.below(4) {
                0 => *rng.pick(&[b'\n', b' ', b':', b'\'', b'-', b'*', b'A', b'R', 0u8]),
                1 => *rng.pick(b"** Alert Rule: Src IP Dst Port User "),
                _ => (rng.next_u64() & 0xff) as u8,
            };
            f.push(c);
        }
        cmp_fuzz(&format!("fr{i}"), &f, *rng.pick(&FLAGS));
    }
}

/// Assembled-from-lines fuzz with a much bigger line vocabulary than row 43,
/// plus randomized whitespace, quote counts and numeric shapes.
#[test]
fn fuzz_d_line_vocabulary() {
    let mut rng = Rng::new(0xD4D4_D4D4_D4D4_D4D4);
    let nums: [&[u8]; 12] = [
        b"0", b"-0", b"1", b"-1", b"2147483647", b"-2147483648", b"2147483648",
        b"9999999999999999999", b"0x10", b"010", b"+5", b" 7 ",
    ];
    for i in 0..1200 {
        let mut f = Vec::new();
        for _ in 0..(1 + rng.below(12)) {
            match rng.below(12) {
                0 => {
                    f.extend_from_slice(b"** Alert ");
                    f.extend_from_slice(&rng.token(12));
                    if rng.bool() {
                        f.push(b':');
                    }
                    if rng.bool() {
                        f.push(b' ');
                        f.extend_from_slice(if rng.bool() { b"mail" } else { b"noop" });
                    }
                    if rng.bool() {
                        f.extend_from_slice(b" - ");
                        f.extend_from_slice(if rng.bool() {
                            &b"ossec,syscheck,"[..]
                        } else {
                            &b"syslog,"[..]
                        });
                    }
                }
                1 => {
                    f.extend_from_slice(b"2006 Apr 13 16:15:17");
                    if rng.bool() {
                        f.push(b' ');
                        f.extend_from_slice(&rng.token(16));
                    }
                }
                2 => {
                    f.extend_from_slice(b"Rule: ");
                    f.extend_from_slice(rng.pick(&nums));
                    for _ in 0..rng.below(4) {
                        f.push(b' ');
                        f.extend_from_slice(rng.pick(&nums));
                    }
                    for _ in 0..rng.below(4) {
                        f.push(b'\'');
                        f.extend_from_slice(&rng.token(8));
                    }
                }
                3 => {
                    f.extend_from_slice(b"Src IP: ");
                    f.extend_from_slice(&rng.token(20));
                }
                4 => {
                    f.extend_from_slice(b"Src Port: ");
                    f.extend_from_slice(rng.pick(&nums));
                }
                5 => {
                    f.extend_from_slice(b"Dst IP: ");
                    f.extend_from_slice(&rng.token(20));
                }
                6 => {
                    f.extend_from_slice(b"Dst Port: ");
                    f.extend_from_slice(rng.pick(&nums));
                }
                7 => {
                    f.extend_from_slice(b"User: ");
                    f.extend_from_slice(&rng.token(20));
                }
                8 => {
                    f.extend_from_slice(b"Integrity checksum changed for: '");
                    if rng.bool() {
                        f.extend_from_slice(&rng.token(24));
                    }
                    if rng.bool() {
                        f.push(b'\'');
                    }
                }
                9 => f.extend_from_slice(&rng.token(80)),
                10 => {}
                _ => f.extend_from_slice(&vec![b' '; rng.below(40)]),
            }
            f.push(b'\n');
        }
        if rng.below(3) == 0 {
            while f.last() == Some(&b'\n') {
                f.pop();
            }
        }
        cmp_fuzz(&format!("fv{i}"), &f, *rng.pick(&FLAGS));
    }
}

/// The one place where the C reads past the NUL `fgets` wrote: a header line of
/// exactly `"** Alert"` (8 bytes) with no newline makes `p = str + 9` point past
/// the terminator. Exercised after a wide variety of preceding lines so the
/// reused buffer contents vary.
#[test]
fn fuzz_e_header_reads_past_terminator() {
    let mut rng = Rng::new(0xE5E5_E5E5_E5E5_E5E5);
    for i in 0..200 {
        let mut f = Vec::new();
        for _ in 0..rng.below(4) {
            f.extend_from_slice(&rng.token(120));
            f.push(b'\n');
        }
        f.extend_from_slice(b"** Alert"); // no trailing newline, at EOF
        cmp_fuzz(&format!("fe{i}"), &f, *rng.pick(&FLAGS));

        // and mid-file, followed by more content
        let mut g = f.clone();
        g.extend_from_slice(b"\n2006 Apr 13 16:15:17 h->/x\nRule: 1 (level 2) -> 'c'\n\n");
        cmp_fuzz(&format!("fe2_{i}"), &g, *rng.pick(&FLAGS));
    }
}
