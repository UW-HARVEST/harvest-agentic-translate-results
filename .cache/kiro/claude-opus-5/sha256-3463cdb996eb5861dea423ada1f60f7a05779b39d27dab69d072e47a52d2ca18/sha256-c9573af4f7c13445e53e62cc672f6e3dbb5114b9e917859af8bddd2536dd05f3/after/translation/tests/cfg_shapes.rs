//! CONFIGS.md rows C02, C03, C14 — the `struct tm` axis and the input-SHAPE
//! axis (line length vs `OS_MAXSTR`, line endings, trailing newline).

mod common;

use common::*;
use std::ffi::c_int;

const READ_ALL: c_int = 0x004;

fn init_snap(lib: &Lib, flags: c_int, t: &tm) -> (c_int, QueueSnap) {
    let mut fq = file_queue::zeroed();
    let rc = unsafe { (lib.init_file_queue)(&mut fq, t, flags) };
    let s = unsafe { snap_queue(&fq) };
    unsafe {
        if !fq.fp.is_null() {
            fclose(fq.fp);
        }
    }
    (rc, s)
}

fn assert_init_tm_eq(flags: c_int, t: &tm, what: &str) -> (c_int, QueueSnap) {
    let (c, r) = libs();
    let a = init_snap(c, flags, t);
    let b = init_snap(r, flags, t);
    assert_eq!(a.0, b.0, "[{what}] rc differs");
    assert_eq!(a.1, b.1, "[{what}] queue differs");
    a
}

/// C02 / G6 — all twelve `s_month[]` entries, checking `mon`, `day`, `year`,
/// `file_name` and `last_change`.
#[test]
fn c02_all_months() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    const NAMES: [&[u8; 3]; 12] = [
        b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov",
        b"Dec",
    ];
    for mon in 0..12 {
        for flags in [0, READ_ALL] {
            let t = tm::new(19, mon, 116);
            let (rc, q) = assert_init_tm_eq(flags, &t, &format!("C02 mon={mon} flags={flags:#x}"));
            assert_eq!(rc, 0);
            assert_eq!(
                &q.mon[..3],
                &NAMES[mon as usize][..],
                "C02 mon={mon}: wrong abbreviation"
            );
            assert_eq!(
                q.mon[3], 0,
                "C02: strncpy(_,_,3) must not write a 4th byte"
            );
            assert_eq!(q.day, 19);
            assert_eq!(q.year, 2016);
            assert_eq!(q.flags, flags);
            assert_eq!(q.file_name, b"alerts.log".to_vec());
            assert_eq!(q.last_change, PINNED_MTIME);
        }
    }
    drop(g);
}

/// C03 / G7 — `tm_mday` / `tm_year` extremes. `tm_mon` stays in 0..11 because
/// `s_month[tm_mon]` is an unchecked lookup into a 12-element table (see the
/// ERRORS.md G7 row).
#[test]
fn c03_day_year_extremes() {
    let g = world();
    write_alerts_log(MINIMAL.as_bytes());
    let mut days: Vec<c_int> = vec![c_int::MIN, c_int::MIN + 1, -1, 0, 1, 31, 32, c_int::MAX];
    let mut years: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1900,
        -1901,
        -1899,
        -1,
        0,
        70,
        116,
        c_int::MAX - 1900,
        c_int::MAX - 1899,
        c_int::MAX,
    ];
    let mut rng = Rng::new(0xC03);
    for _ in 0..60 {
        days.push(rng.i32());
        years.push(rng.i32());
    }

    for &d in &days {
        for &y in &years {
            let t = tm::new(d, (d.rem_euclid(12)) as c_int, y);
            let (rc, q) = assert_init_tm_eq(READ_ALL, &t, &format!("C03 day={d} year={y}"));
            assert_eq!(rc, 0);
            assert_eq!(q.day, d, "day is copied verbatim");
            assert_eq!(
                q.year,
                y.wrapping_add(1900),
                "year is tm_year + 1900 with C's two's-complement wrap"
            );
        }
    }
    drop(g);
}

/// C14 / G4 — line-length shapes around `OS_MAXSTR` (1024) and the `fgets`
/// read limit of 1023 bytes, plus line-ending and trailing-newline variants.
#[test]
fn c14_oversized_line() {
    let head = "** Alert 1461102540.1234: mail - syslog,\n2016 Apr 19 20:29:00 h->/var/log/m\n";

    for n in [
        0usize, 1, 2, 32, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 2046, 2047, 2048, 4096, 65536,
    ] {
        // A long free-form log line.
        let mut content = head.as_bytes().to_vec();
        content.extend(std::iter::repeat_n(b'x', n));
        content.push(b'\n');
        assert_gad_eq(0, &content, 0, &format!("C14 log line of {n} bytes"));

        // A long comment inside the Rule: line (crosses the fgets boundary
        // mid-token, so `strrchr(comment, '\'')` sees a truncated string).
        let mut content = head.as_bytes().to_vec();
        content.extend_from_slice(b"Rule: 1002 (level 7) -> '");
        content.extend(std::iter::repeat_n(b'c', n));
        content.extend_from_slice(b"'\n");
        assert_gad_eq(0, &content, 0, &format!("C14 comment of {n} bytes"));

        // A long Src IP: value.
        let mut content = head.as_bytes().to_vec();
        content.extend_from_slice(b"Src IP: ");
        content.extend(std::iter::repeat_n(b'9', n));
        content.push(b'\n');
        assert_gad_eq(0, &content, 0, &format!("C14 srcip of {n} bytes"));

        // A long alert header (the alertid / group parsing crosses the split).
        let mut content = b"** Alert ".to_vec();
        content.extend(std::iter::repeat_n(b'i', n));
        content.extend_from_slice(b": mail - syslog,\n");
        content.extend_from_slice(b"2016 Apr 19 20:29:00 h->/var/log/m\n");
        assert_gad_eq(0, &content, 0, &format!("C14 header with {n}-byte id"));

        // A long group.
        let mut content = b"** Alert 1.2: mail - ".to_vec();
        content.extend(std::iter::repeat_n(b'g', n));
        content.extend_from_slice(b"\n2016 Apr 19 20:29:00 h->/var/log/m\n");
        assert_gad_eq(0, &content, 0, &format!("C14 group of {n} bytes"));

        // A long syscheck path.
        let mut content = b"** Alert 1.2: mail - ossec,syscheck,\n".to_vec();
        content.extend_from_slice(b"2016 Apr 19 20:29:00 h->/var/log/m\n");
        content.extend_from_slice(b"Integrity checksum changed for: '");
        content.extend(std::iter::repeat_n(b'p', n));
        content.extend_from_slice(b"'\n");
        assert_gad_eq(0, &content, 0, &format!("C14 syscheck path of {n} bytes"));
    }

    // A line that spans exactly the fgets boundary such that the continuation
    // itself starts with a recognised prefix.
    for pad in 1015..1030 {
        let mut content = head.as_bytes().to_vec();
        content.extend(std::iter::repeat_n(b'x', pad));
        content.extend_from_slice(b"Rule: 7 (level 8) -> 'split'\n");
        assert_gad_eq(0, &content, 0, &format!("C14 split before Rule:, pad={pad}"));
    }

    // CRLF and lone CR.
    for eol in [&b"\r\n"[..], &b"\r"[..], &b"\n"[..]] {
        let mut content = Vec::new();
        for line in [
            "** Alert 1461102540.1234: mail - ossec,syscheck,",
            "2016 Apr 19 20:29:00 myhost->/var/log/messages",
            "Rule: 550 (level 7) -> 'Integrity checksum changed.'",
            "Src IP: 1.2.3.4",
            "Src Port: 22",
            "User: root",
            "Integrity checksum changed for: '/etc/passwd'",
        ] {
            content.extend_from_slice(line.as_bytes());
            content.extend_from_slice(eol);
        }
        assert_gad_eq(0, &content, 0, &format!("C14 eol={eol:?}"));
        assert_drain_eq(0, &content, 6, &format!("C14 drain eol={eol:?}"));
    }

    // No trailing newline at EOF, for each line kind.
    for last in [
        "Rule: 1002 (level 7) -> 'no trailing newline'",
        "Src IP: 1.2.3.4",
        "Src Port: 22",
        "User: root",
        "Integrity checksum changed for: '/etc/passwd'",
        "free form",
        "2016 Apr 19 20:29:00 h->/l",
    ] {
        let content = format!("{head}{last}");
        assert_gad_eq(0, content.as_bytes(), 0, &format!("C14 EOF after {last:?}"));
    }

    // Bytes 0x01..0xFF (no NUL — the C's line-oriented parsing terminates on
    // NUL, which cannot appear in a `char*`-based field anyway).
    let mut content = head.as_bytes().to_vec();
    content.extend((1u8..=255).collect::<Vec<u8>>());
    content.push(b'\n');
    assert_gad_eq(0, &content, 0, "C14 high bytes");

    // Randomized line lengths across the boundary.
    let mut rng = Rng::new(0xC14);
    for i in 0..200 {
        let n = 1000 + rng.below(60);
        let mut content = head.as_bytes().to_vec();
        let fill: Vec<u8> = (0..n)
            .map(|_| *rng.pick(b"abcdefgh:'- 0123456789"))
            .collect();
        content.extend_from_slice(&fill);
        content.push(b'\n');
        content.extend_from_slice(b"Rule: 1 (level 2) -> 'tail'\n");
        assert_gad_eq(0, &content, 0, &format!("C14 random#{i} len={n}"));
    }
}
