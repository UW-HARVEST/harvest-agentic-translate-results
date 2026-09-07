//! Phase B/C reinforcement — mutation and random-byte fuzzing.
//!
//! The structured rows in `CONFIGS.md` / `ERRORS.md` cover the branches the C
//! source names. This file attacks the *combinations* of them: random byte
//! soup, and mutations of valid alerts (bit flips, byte splices, truncations,
//! line duplication/removal), each run through both `.so`s with a random flag
//! word. Fixed seeds keep every case reproducible.

#![allow(non_snake_case)]

mod harness;
use harness::*;

use std::ffi::c_int;

/// Fast differential `GetAlertData`: reuses one file, drains it on both libs.
struct Fuzzer {
    sc: Scratch,
    path: std::path::PathBuf,
    /// Number of cases in which at least one alert actually parsed, so a test
    /// can assert it is not vacuously comparing NULL against NULL.
    parsed: std::cell::Cell<usize>,
    cases: std::cell::Cell<usize>,
}

impl Fuzzer {
    fn new(tag: &str) -> Fuzzer {
        let sc = Scratch::new(tag);
        let path = sc.path("alerts.log");
        Fuzzer {
            sc,
            path,
            parsed: std::cell::Cell::new(0),
            cases: std::cell::Cell::new(0),
        }
    }

    fn check(&self, tag: &str, bytes: &[u8], flag: c_int) {
        std::fs::write(&self.path, bytes).expect("write");
        let p = libs();
        let _err = stderr_lock(); // some inputs make GetAlertData call perror
        unsafe {
            let fc = open_ro(&self.path);
            let c = drain(&p.c, flag, fc);
            fclose(fc);
            let fr = open_ro(&self.path);
            let r = drain(&p.rs, flag, fr);
            fclose(fr);
            assert!(
                c == r,
                "DIVERGENCE [{tag}] flag={flag:#x}\n  input ({} bytes) = {:?}\n  C   : {c:#?}\n  Rust: {r:#?}",
                bytes.len(),
                String::from_utf8_lossy(bytes)
            );
            if c.iter().any(|(a, _)| *a != AlertSnap::Null) {
                self.parsed.set(self.parsed.get() + 1);
            }
        }
        self.cases.set(self.cases.get() + 1);
        let _ = &self.sc;
    }

    /// Assert the corpus was not degenerate: at least `min` cases must have
    /// produced a real alert on both sides.
    fn assert_productive(&self, min: usize) {
        assert!(
            self.parsed.get() >= min,
            "fuzz corpus was vacuous: only {} of {} cases parsed an alert (need >= {min})",
            self.parsed.get(),
            self.cases.get()
        );
    }
}

const FLAG_POOL: [c_int; 8] = [
    0,
    CRALERT_MAIL_SET,
    CRALERT_EXEC_SET,
    CRALERT_READ_ALL,
    CRALERT_READ_FAILED,
    CRALERT_FP_SET,
    0x1F,
    -1,
];

/// The line prefixes the parser dispatches on, so the fuzzer produces inputs
/// that actually reach the interesting branches.
const PREFIXES: [&str; 14] = [
    "** Alert ",
    "Rule: ",
    "Src IP: ",
    "Src Port: ",
    "Dst IP: ",
    "Dst Port: ",
    "User: ",
    "Integrity checksum changed for: '",
    "Old md5sum was: ",
    "New sha256sum is : ",
    "Size changed from ",
    "Permissions changed from ",
    "2006 Apr 13 16:15:17 ",
    "",
];

fn valid_corpus() -> Vec<String> {
    let mut out = Vec::new();
    out.push(AlertSpec::minimal().render());
    {
        // Negative / overflowing numerics, so `atoi` sign handling is fuzzed.
        let mut a = AlertSpec::minimal();
        a.rule = Some("Rule: -2147483648 (level -7) -> 'negative'".into());
        a.srcport = Some("Src Port: -1".into());
        a.dstport = Some("Dst Port: 4294967296".into());
        out.push(a.render());
    }
    {
        let mut a = AlertSpec::minimal();
        a.rule = Some("Rule: 2147483648 (level 2147483648) -> 'overflow'".into());
        a.srcport = Some("Src Port: 99999999999999999999".into());
        a.dstport = Some("Dst Port: -0".into());
        out.push(a.render());
    }
    {
        let mut a = AlertSpec::minimal();
        a.srcip = Some("Src IP: 10.0.0.9".into());
        a.srcport = Some("Src Port: 1234".into());
        a.dstip = Some("Dst IP: 10.0.0.10".into());
        a.dstport = Some("Dst Port: 80".into());
        a.user = Some("User: root".into());
        out.push(a.render());
    }
    {
        let mut a = AlertSpec::minimal();
        a.group = "ossec,syscheck,".into();
        a.body = vec![
            "Integrity checksum changed for: '/etc/passwd'".into(),
            "Old md5sum was: 1111".into(),
            "New md5sum is : 2222".into(),
        ];
        out.push(a.render());
    }
    {
        let a = AlertSpec::minimal();
        out.push(render_all(&[a.clone(), a.clone(), a]));
    }
    out
}

/// Random byte soup, biased towards the bytes the parser inspects, spliced with
/// real alert lines so a meaningful share of cases actually parse.
#[test]
fn fuzz_random_bytes() {
    let f = Fuzzer::new("fuzzrand");
    let mut rng = Rng::new(0x5EED_0001);
    const POOL: &[u8] = b"*Alert:-' \n\treRuleSrcDstIPPortUser0123456789.mail\
syscheckIntegritychecksumchangedfor";

    // Real lines, so soup can grow into a valid alert instead of always being
    // rejected at the header.
    let real: Vec<String> = AlertSpec {
        srcip: Some("Src IP: 10.0.0.9".into()),
        srcport: Some("Src Port: -1".into()),
        dstip: Some("Dst IP: 10.0.0.8".into()),
        dstport: Some("Dst Port: 65536".into()),
        user: Some("User: root".into()),
        body: vec!["Integrity checksum changed for: '/etc/passwd'".into()],
        ..AlertSpec::minimal()
    }
    .render()
    .lines()
    .map(str::to_string)
    .collect();

    for i in 0..6000 {
        let mut bytes: Vec<u8> = Vec::new();
        // Alternate between soup chunks and real lines.
        for _ in 0..1 + rng.below(8) {
            if rng.bool() {
                let n = rng.below(60);
                bytes.extend((0..n).map(|_| {
                    if rng.below(8) == 0 {
                        (rng.next_u64() & 0xFF) as u8
                    } else {
                        *rng.choice(POOL)
                    }
                }));
            } else {
                // A real line, occasionally with one byte corrupted.
                let mut line = rng.choice(&real).clone().into_bytes();
                if rng.below(3) == 0 && !line.is_empty() {
                    let at = rng.below(line.len());
                    line[at] = *rng.choice(b"*:-' xq0");
                }
                bytes.extend(line);
                bytes.push(b'\n');
            }
        }
        bytes.retain(|&b| b != 0);
        let flag = *rng.choice(&FLAG_POOL);
        f.check(&format!("rand{i}"), &bytes, flag);
    }
    f.assert_productive(300);
}

/// Line-oriented soup: assembles lines from the real dispatch prefixes plus
/// random tails, so `_r` walks 0 -> 1 -> 2 in unusual orders.
#[test]
fn fuzz_prefixed_lines() {
    let f = Fuzzer::new("fuzzpfx");
    let mut rng = Rng::new(0x5EED_0002);

    for i in 0..6000 {
        let nlines = rng.below(9);
        let mut text = String::new();
        for _ in 0..nlines {
            let pre = *rng.choice(&PREFIXES);
            let tail = String::from_utf8_lossy(&rng.token_upto(30)).into_owned();
            text.push_str(pre);
            text.push_str(&tail);
            if rng.below(10) != 0 {
                text.push('\n');
            }
        }
        let flag = *rng.choice(&FLAG_POOL);
        f.check(&format!("pfx{i}"), text.as_bytes(), flag);
    }
    f.assert_productive(20);
}

/// Mutations of valid alerts: bit flips, byte deletion/insertion/duplication,
/// line shuffling and truncation.
#[test]
fn fuzz_mutated_valid_alerts() {
    let f = Fuzzer::new("fuzzmut");
    let mut rng = Rng::new(0x5EED_0003);
    let corpus = valid_corpus();

    for i in 0..8000 {
        let base = rng.choice(&corpus).clone();
        let mut b = base.into_bytes();
        for _ in 0..1 + rng.below(4) {
            if b.is_empty() {
                break;
            }
            match rng.below(7) {
                // Bit flip.
                0 => {
                    let at = rng.below(b.len());
                    let bit = rng.below(8);
                    b[at] ^= 1 << bit;
                    if b[at] == 0 {
                        b[at] = b'?';
                    }
                }
                // Replace with a byte from the significant set.
                1 => {
                    let at = rng.below(b.len());
                    b[at] = *rng.choice(b"*:-' \nAlertRuleSrcDstUsermail0189.");
                    if b[at] == 0 {
                        b[at] = b'?';
                    }
                }
                // Delete a byte.
                2 => {
                    let at = rng.below(b.len());
                    b.remove(at);
                }
                // Insert a byte.
                3 => {
                    let at = rng.below(b.len() + 1);
                    let v = *rng.choice(b"*:-' \nxq0");
                    b.insert(at, v);
                }
                // Truncate.
                4 => {
                    let at = rng.below(b.len() + 1);
                    b.truncate(at);
                }
                // Duplicate a line.
                5 => {
                    let lines: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
                    if lines.len() > 1 {
                        let pick = lines[rng.below(lines.len())].to_vec();
                        let mut out: Vec<u8> = Vec::new();
                        let at = rng.below(lines.len());
                        for (k, l) in lines.iter().enumerate() {
                            if k == at {
                                out.extend_from_slice(&pick);
                                out.push(b'\n');
                            }
                            out.extend_from_slice(l);
                            out.push(b'\n');
                        }
                        b = out;
                    }
                }
                // Drop a line.
                _ => {
                    let lines: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
                    if lines.len() > 1 {
                        let at = rng.below(lines.len());
                        let mut out: Vec<u8> = Vec::new();
                        for (k, l) in lines.iter().enumerate() {
                            if k != at {
                                out.extend_from_slice(l);
                                out.push(b'\n');
                            }
                        }
                        b = out;
                    }
                }
            }
        }
        b.retain(|&c| c != 0);
        let flag = *rng.choice(&FLAG_POOL);
        f.check(&format!("mut{i}"), &b, flag);
    }
    f.assert_productive(2000);
}

/// Inputs built to straddle the 1023-byte `fgets` chunk boundary at every
/// dispatch prefix, since a split line changes which branch each half hits.
#[test]
fn fuzz_fgets_boundary() {
    let f = Fuzzer::new("fuzzbnd");
    let mut rng = Rng::new(0x5EED_0004);

    for i in 0..1500 {
        // Pad so that a chosen prefix lands exactly across the boundary.
        let pre = *rng.choice(&PREFIXES);
        let target = 1023usize;
        let split_at = target.saturating_sub(rng.below(pre.len().max(1) + 2));
        let mut text = String::new();
        text.push_str(&AlertSpec::minimal().render());
        // A body line whose length pushes the next prefix across the boundary.
        let pad = split_at.saturating_sub(text.len() % 1023);
        text.push_str(&"P".repeat(pad));
        text.push_str(pre);
        text.push_str(&String::from_utf8_lossy(&rng.token_upto(40)));
        text.push('\n');
        text.push_str(&AlertSpec::minimal().render());
        let flag = *rng.choice(&FLAG_POOL);
        f.check(&format!("bnd{i}"), text.as_bytes(), flag);
    }
    f.assert_productive(500);
}

/// End-to-end fuzz through `driver`: random file contents, random dates, random
/// flag words. `CRALERT_FP_SET` is excluded because it forces a 5 s
/// `file_sleep` per call (covered deterministically in phase_b_driver row 43).
#[test]
fn fuzz_driver_end_to_end() {
    let p = libs();
    let mut rng = Rng::new(0x5EED_0005);
    let corpus = valid_corpus();
    let sc = Scratch::new("fuzzdrv");
    let path = sc.path("alerts.log");
    let mut produced = 0usize;

    for i in 0..600 {
        let mut b = if rng.below(3) == 0 {
            let n = rng.below(200);
            (0..n)
                .map(|_| *rng.choice(b"*Alert:-' \nRuleSrcUsermail019."))
                .collect::<Vec<u8>>()
        } else {
            let mut v = rng.choice(&corpus).clone().into_bytes();
            for _ in 0..1 + rng.below(3) {
                if v.is_empty() {
                    break;
                }
                let at = rng.below(v.len());
                v[at] = *rng.choice(b"*:-' \nxq0");
                if v[at] == 0 {
                    v[at] = b'?';
                }
            }
            v
        };
        b.retain(|&c| c != 0);
        std::fs::write(&path, &b).expect("write");

        let flags = *rng.choice(&[
            CRALERT_READ_ALL,
            CRALERT_READ_ALL | CRALERT_MAIL_SET,
            CRALERT_READ_ALL | CRALERT_EXEC_SET | CRALERT_READ_FAILED,
            0,
            CRALERT_MAIL_SET,
        ]);
        let day = rng.i32();
        let mon = rng.below(12) as c_int;
        let year = rng.i32();

        let _cwd = enter_dir(&sc.dir);
        unsafe {
            let (c, ce) = capture_stderr(|| {
                snap_and_free(&p.c, (p.c.driver)(day, mon, year, 0, flags))
            });
            let (r, re) = capture_stderr(|| {
                snap_and_free(&p.rs, (p.rs.driver)(day, mon, year, 0, flags))
            });
            assert!(
                c == r,
                "DIVERGENCE in driver [fuzz{i}] flags={flags:#x} d={day} m={mon} y={year}\n  input={:?}\n  C   : {c:#?}\n  Rust: {r:#?}",
                String::from_utf8_lossy(&b)
            );
            assert!(
                ce == re,
                "DIVERGENCE in driver stderr [fuzz{i}]\n  C   : {:?}\n  Rust: {:?}",
                String::from_utf8_lossy(&ce),
                String::from_utf8_lossy(&re)
            );
            if c != AlertSnap::Null {
                produced += 1;
            }
        }
    }
    assert!(
        produced >= 100,
        "driver fuzz was vacuous: only {produced} of 600 cases returned an alert"
    );
}

/// End-to-end fuzz through the low-level pair `Init_FileQueue` + `Read_FileMon`,
/// with the whole `file_queue` compared after each call.
#[test]
fn fuzz_filequeue_pipeline() {
    let p = libs();
    let mut rng = Rng::new(0x5EED_0006);
    let corpus = valid_corpus();
    let sc = Scratch::new("fuzzfq");
    let path = sc.path("alerts.log");
    let mut produced = 0usize;

    for i in 0..400 {
        let mut b = rng.choice(&corpus).clone().into_bytes();
        for _ in 0..rng.below(4) {
            if b.is_empty() {
                break;
            }
            let at = rng.below(b.len());
            b[at] = *rng.choice(b"*:-' \nxq0");
            if b[at] == 0 {
                b[at] = b'?';
            }
        }
        b.retain(|&c| c != 0);
        std::fs::write(&path, &b).expect("write");

        // No CRALERT_FP_SET: it costs a 5 s sleep per call.
        let flags = *rng.choice(&[
            CRALERT_READ_ALL,
            CRALERT_READ_ALL | CRALERT_MAIL_SET,
            CRALERT_READ_ALL | CRALERT_READ_FAILED,
        ]);
        let rounds = 1 + rng.below(3);
        let t_init = tm {
            tm_mday: rng.i32(),
            tm_mon: rng.below(12) as c_int,
            tm_year: rng.i32(),
            ..Default::default()
        };
        let t_read = tm {
            tm_mday: rng.i32(),
            tm_mon: rng.below(12) as c_int,
            tm_year: rng.i32(),
            ..Default::default()
        };

        let _cwd = enter_dir(&sc.dir);
        unsafe {
            let run = |lib: &Lib| {
                let mut fq = file_queue::default();
                let rc = (lib.Init_FileQueue)(&mut fq, &t_init, flags);
                let mut snaps = vec![snap_fq(&fq)];
                let mut alerts = Vec::new();
                for _ in 0..rounds {
                    let a = (lib.Read_FileMon)(&mut fq, &t_read, 0);
                    alerts.push(snap_and_free(lib, a));
                    snaps.push(snap_fq(&fq));
                }
                if !fq.fp.is_null() {
                    fclose(fq.fp);
                }
                (rc, snaps, alerts)
            };
            let c = run(&p.c);
            let r = run(&p.rs);
            assert!(
                c == r,
                "DIVERGENCE in Init+Read pipeline [fuzz{i}] flags={flags:#x}\n  input={:?}\n  C   : {c:#?}\n  Rust: {r:#?}",
                String::from_utf8_lossy(&b)
            );
            if c.2.iter().any(|a| *a != AlertSnap::Null) {
                produced += 1;
            }
        }
    }
    assert!(
        produced >= 100,
        "Init+Read fuzz was vacuous: only {produced} of 400 cases returned an alert"
    );
}
