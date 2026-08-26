// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md.  Every row drives BOTH `.so`s through their
// exported symbols with many randomized inputs (fixed seed) and asserts
// byte-identical stdout.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Expected-output model, transcribed directly from c_src/src/driver.c
// ---------------------------------------------------------------------------

/// `printLine`: `if (line != NULL) printf("%s\n", line);`
fn expect_print_line(buf: &[u8]) -> Vec<u8> {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let mut v = buf[..end].to_vec();
    v.push(b'\n');
    v
}

/// `driver`: `dest` is `char[100] = ""`; `source` is 99 `'A'` then NUL.
/// `if (data < 100) { strncpy(dest, source, data); dest[data] = 0; }`
/// then `printLine(dest)`.  Only defined for `data >= 0`.
fn expect_driver(data: c_int) -> Vec<u8> {
    assert!(data >= 0, "negative data is UB in the C source");
    let n = if data < 100 { data as usize } else { 0 };
    let mut v = vec![b'A'; n];
    v.push(b'\n');
    v
}

fn expect_ops(ops: &[Op]) -> Vec<u8> {
    let mut v = Vec::new();
    for op in ops {
        match op {
            Op::Driver(d) => v.extend_from_slice(&expect_driver(*d)),
            Op::PrintLine(b) => v.extend_from_slice(&expect_print_line(b)),
            Op::PrintLineNull => {}
        }
    }
    v
}

fn ascii_printable(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| 0x20 + rng.below(0x5f) as u8).collect()
}

// ---------------------------------------------------------------------------
// C1 — printLine, empty string
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_print_line_empty() {
    let ops = [Op::PrintLine(cbuf(b""))];
    assert_same_and_eq("C1", &ops, b"\n");
}

// ---------------------------------------------------------------------------
// C2 — printLine, every possible single non-NUL byte
// ---------------------------------------------------------------------------

#[test]
fn cfg_c2_print_line_every_single_byte() {
    for b in 1u16..=255 {
        let buf = cbuf(&[b as u8]);
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C2", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C3 — printLine, random printable ASCII, len 2..=64
// ---------------------------------------------------------------------------

#[test]
fn cfg_c3_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 0xC3);
    for _ in 0..512 {
        let len = rng.range(2, 64) as usize;
        let buf = cbuf(&ascii_printable(&mut rng, len));
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C3", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C4 — printLine, random arbitrary non-NUL bytes (incl. non-UTF-8 high bytes)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c4_print_line_random_arbitrary_bytes() {
    let mut rng = Rng::new(SEED ^ 0xC4);
    for _ in 0..512 {
        let len = rng.range(1, 256) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let buf = cbuf(&bytes);
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C4", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C5 — printLine, printf format metacharacters
//      (C compiles to `puts(line)`, Rust keeps `printf("%s\n", line)`)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_print_line_format_metacharacters() {
    const SPECS: &[&[u8]] = &[
        b"%s", b"%d", b"%n", b"%%", b"%p", b"%x", b"%99999999s", b"%.*s", b"%hn", b"%lf",
    ];
    let mut rng = Rng::new(SEED ^ 0xC5);
    for _ in 0..256 {
        let len = rng.range(0, 24) as usize;
        let mut bytes = ascii_printable(&mut rng, len);
        let nspec = rng.range(1, 4) as usize;
        for _ in 0..nspec {
            let spec = SPECS[rng.below(SPECS.len() as u64) as usize];
            let at = rng.below(bytes.len() as u64 + 1) as usize;
            for (k, &b) in spec.iter().enumerate() {
                bytes.insert(at + k, b);
            }
        }
        let buf = cbuf(&bytes);
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C5", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C6 — printLine, embedded newline / CR / tab
// ---------------------------------------------------------------------------

#[test]
fn cfg_c6_print_line_embedded_whitespace() {
    let mut rng = Rng::new(SEED ^ 0xC6);
    for _ in 0..256 {
        let len = rng.range(1, 64) as usize;
        let mut bytes = ascii_printable(&mut rng, len);
        let n = rng.range(1, 5) as usize;
        for _ in 0..n {
            let ws = *[b'\n', b'\r', b'\t', 0x0b, 0x0c].get(rng.below(5) as usize).unwrap();
            let at = rng.below(bytes.len() as u64 + 1) as usize;
            bytes.insert(at, ws);
        }
        let buf = cbuf(&bytes);
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C6", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C7 — printLine, embedded NUL: bytes after it must be ignored
// ---------------------------------------------------------------------------

#[test]
fn cfg_c7_print_line_embedded_nul() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    for _ in 0..256 {
        let head = rng.range(0, 32) as usize;
        let tail = rng.range(1, 32) as usize;
        let mut bytes: Vec<u8> = (0..head).map(|_| rng.nonzero_byte()).collect();
        bytes.push(0); // embedded terminator
        bytes.extend((0..tail).map(|_| rng.nonzero_byte()));
        let buf = cbuf(&bytes);
        let ops = [Op::PrintLine(buf.clone())];
        // Everything from the embedded NUL onwards is invisible.
        assert_same_and_eq("C7", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C8 — printLine, long strings straddling the glibc stdout buffer
// ---------------------------------------------------------------------------

#[test]
fn cfg_c8_print_line_long_strings() {
    let mut rng = Rng::new(SEED ^ 0xC8);
    let mut lens: Vec<usize> = vec![4095, 4096, 4097, 8191, 8192, 8193];
    for _ in 0..64 {
        lens.push(rng.range(1000, 20000) as usize);
    }
    for len in lens {
        let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
        let buf = cbuf(&bytes);
        let ops = [Op::PrintLine(buf.clone())];
        assert_same_and_eq("C8", &ops, &expect_print_line(&buf));
    }
}

// ---------------------------------------------------------------------------
// C9 — printLine, repeated calls inside one capture window
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_print_line_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0xC9);
    for _ in 0..128 {
        let n = rng.range(2, 16) as usize;
        let ops: Vec<Op> = (0..n)
            .map(|_| {
                let len = rng.range(0, 48) as usize;
                Op::PrintLine(cbuf(&ascii_printable(&mut rng, len)))
            })
            .collect();
        let expected = expect_ops(&ops);
        assert_same_and_eq("C9", &ops, &expected);
    }
}

// ---------------------------------------------------------------------------
// C10 — driver(0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c10_driver_zero() {
    assert_same_and_eq("C10", &[Op::Driver(0)], b"\n");
}

// ---------------------------------------------------------------------------
// C11 — driver, exhaustive 1..=98 (ordinary in-range path)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c11_driver_in_range_exhaustive() {
    for d in 1..=98i32 {
        assert_same_and_eq("C11", &[Op::Driver(d)], &expect_driver(d));
    }
}

// ---------------------------------------------------------------------------
// C12 — driver(99): last in-bounds dest index, full 99-byte strncpy
// ---------------------------------------------------------------------------

#[test]
fn cfg_c12_driver_99_boundary() {
    let mut expected = vec![b'A'; 99];
    expected.push(b'\n');
    assert_same_and_eq("C12", &[Op::Driver(99)], &expected);
}

// ---------------------------------------------------------------------------
// C13 — driver(100): first value failing `data < 100`
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_driver_100_guard_fails() {
    assert_same_and_eq("C13", &[Op::Driver(100)], b"\n");
}

// ---------------------------------------------------------------------------
// C14 — driver, 101..=INT_MAX (guard-failing path)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c14_driver_above_bound() {
    let mut rng = Rng::new(SEED ^ 0x14);
    let mut vals: Vec<i32> = vec![101, 1 << 8, 1 << 16, 1 << 30, i32::MAX, i32::MAX - 1];
    for _ in 0..512 {
        vals.push(rng.range(101, i32::MAX as i64) as i32);
    }
    for d in vals {
        assert_same_and_eq("C14", &[Op::Driver(d)], b"\n");
    }
}

// ---------------------------------------------------------------------------
// C15 — driver, whole non-crashing domain structure
// ---------------------------------------------------------------------------

#[test]
fn cfg_c15_driver_full_domain_sweep() {
    for d in 0..=200i32 {
        assert_same_and_eq("C15", &[Op::Driver(d)], &expect_driver(d));
    }
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..512 {
        let d = rng.range(0, i32::MAX as i64) as i32;
        assert_same_and_eq("C15", &[Op::Driver(d)], &expect_driver(d));
    }
}

// ---------------------------------------------------------------------------
// C16 — driver, repeated calls inside one capture window
// ---------------------------------------------------------------------------

#[test]
fn cfg_c16_driver_repeated_calls() {
    let mut rng = Rng::new(SEED ^ 0x16);
    for _ in 0..128 {
        let n = rng.range(2, 16) as usize;
        let ops: Vec<Op> = (0..n)
            .map(|_| Op::Driver(rng.range(0, 120) as i32))
            .collect();
        let expected = expect_ops(&ops);
        assert_same_and_eq("C16", &ops, &expected);
    }
}

// ---------------------------------------------------------------------------
// C17 — mixed sequences interleaving both public entry points
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_mixed_entry_points() {
    let mut rng = Rng::new(SEED ^ 0x17);
    for _ in 0..256 {
        let n = rng.range(2, 16) as usize;
        let ops: Vec<Op> = (0..n)
            .map(|_| match rng.below(4) {
                0 => Op::Driver(rng.range(0, 120) as i32),
                1 => Op::Driver(rng.range(0, i32::MAX as i64) as i32),
                2 => Op::PrintLineNull,
                _ => {
                    let len = rng.range(0, 40) as usize;
                    let bytes: Vec<u8> = (0..len).map(|_| rng.nonzero_byte()).collect();
                    Op::PrintLine(cbuf(&bytes))
                }
            })
            .collect();
        let expected = expect_ops(&ops);
        assert_same_and_eq("C17", &ops, &expected);
    }
}

// ---------------------------------------------------------------------------
// C18 — driver after printLine (no reordering across the shared stdout buffer)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c18_driver_after_print_line() {
    let mut rng = Rng::new(SEED ^ 0x18);
    for _ in 0..128 {
        let len = rng.range(0, 64) as usize;
        let prefix = cbuf(&ascii_printable(&mut rng, len));
        let d = rng.range(0, 150) as i32;
        let ops = [
            Op::PrintLine(prefix),
            Op::Driver(d),
            Op::PrintLineNull,
            Op::PrintLine(cbuf(b"tail")),
        ];
        let expected = expect_ops(&ops);
        assert_same_and_eq("C18", &ops, &expected);
    }
}
